// Pointer drag and targeting (docs/polish/7-mobile-ux.md S9, B36-B39; R384, R510, R658).
// A press below the threshold stays a click; a drag lifts its interaction and releases through the
// usual target path. Window capture always sees pointer press/release, while bubble keys and menus
// let dialogs handle them. Nothing here decides a rule (CLAUDE.md rule 7) or sends until release.
// `drag-landing` keeps a played card out of the fan until a newer view arrives, so animation delay
// does not look like a refusal.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { ActionBody, CardView, PlayerView } from "@jackioh/shared";

import { readSettings } from "../../settings/index.ts";
import { IDLE, isBuilding, type Interaction } from "../actions.ts";
import { MatchCardsProvider, useCardInfo, useCopiedDef } from "../catalog.ts";
import { testid, type ClickTarget } from "../contract.ts";
import { liveFace } from "../faces.ts";
import BlockedMark, { type Blocked } from "./BlockedMark.tsx";
import { setLanding } from "./landing.ts";
import { DRAG_THRESHOLD_PX, planDrag, resolveDrop, type DragPlan, type DropSpot } from "./model.ts";
import OptionDrag from "./OptionDrag.tsx";
import { lockedZoneAt, pickDropSpot, targetFromElement } from "./targets.ts";
import "./drag.css";

export type DragLayerProps = {
  view: PlayerView;
  legal: readonly ActionBody[];
  interaction: Interaction;
  onInteraction: (next: Interaction) => void;
  onAction: (body: ActionBody) => void;
  /**
   * R655: a press has just become a drag, once per lift. `Game` plays a lifted attacker's `attack`
   * hook from it; the drop, a cancel and the attack itself report nothing.
   */
  onLift?: (plan: DragPlan) => void;
};

type Point = { x: number; y: number };
type Box = { x: number; y: number; size: number; width: number; height: number };
type ReticleShape = "ring" | "frame" | "pad";
type ArrowStop = { shape: "circle" | "box"; halfWidth: number; halfHeight: number };

type Press = { pointerId: number; start: Point; source: ClickTarget; testid: string; element: Element };

type Flight = {
  pointerId: number;
  plan: DragPlan;
  element: Element;
  pointer: Point;
  from: Point;
  touch: boolean;
  spot: DropSpot;
  reticle: Box | null;
};

type Landed = { instanceId: string; card: CardView | null; at: Point; view: PlayerView };

const BOARD = '[data-testid="board"]';
/**
 * The longest a landed card waits for the board to catch up. A drop only ever sends an action the
 * engine listed, so the view always moves on; this only covers a refusal the client cannot see
 * coming (a networked match whose clock ran out as the card was dropped).
 */
const LANDING_TIMEOUT_MS = 4_000;
const RETICLE_MIN_PX = 44;
/** A hero ring hugs its health gem to avoid covering the nearby name and pile count. */
const HERO_GEM = ".hero-health";
const RETICLE_GEM_MARGIN_PX = 10;
const RETICLE_GEM_MIN_PX = 36;
const RETICLE_FRAME_OUTSET_PX = 6;
const ARROW_HEAD_PX = 30;
const ARROW_HEAD_HALF_WIDTH_PX = 19;
const ARROW_HEAD_NOTCH_PX = 9;
const ARROW_BEND_SHARE = 0.22;
const ARROW_BEND_MAX_PX = 90;
const ARROW_GLOW_BLUR_PX = 4;
const ARROW_GLOW_MARGIN_PX = 40;

function round(n: number): number {
  return Math.round(n * 10) / 10;
}

function centreOf(element: Element): Point | null {
  if (!element.isConnected) return null;
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

function byTestid(id: string): Element | null {
  return document.querySelector(`[data-testid="${id.replace(/["\\]/g, "\\$&")}"]`);
}

/** Heroes are ringed at their health gem; other targets are ringed at centre. */
function reticleFor(spot: DropSpot): Box | null {
  if (spot.at !== "target") return null;
  const element = byTestid(spot.testid);
  if (element === null) return null;
  const rect = element.getBoundingClientRect();
  const gem = spot.testid.startsWith("hero-") ? element.querySelector(HERO_GEM) : null;
  const aim = gem === null ? rect : gem.getBoundingClientRect();
  const size =
    gem === null
      ? Math.max(RETICLE_MIN_PX, Math.min(rect.width, rect.height))
      : Math.max(RETICLE_GEM_MIN_PX, Math.max(aim.width, aim.height) + RETICLE_GEM_MARGIN_PX);
  return {
    x: aim.left + aim.width / 2,
    y: aim.top + aim.height / 2,
    size,
    width: rect.width,
    height: rect.height,
  };
}

function isValidSpot(plan: DragPlan, spot: DropSpot): boolean {
  if (spot.at === "target") return true;
  return spot.at === "board" && plan.freeDrop;
}

/** A card dropped on a Locked zone is refused with a mark where it landed (BlockedMark.tsx). */
export default function DragLayer(props: DragLayerProps): ReactElement {
  const [blocked, setBlocked] = useState<Blocked | null>(null);
  const clearBlocked = useCallback(() => setBlocked(null), []);
  return (
    <>
      <PlayDrag {...props} onBlocked={setBlocked} />
      <BlockedMark blocked={blocked} onDone={clearBlocked} />
      <OptionDrag />
    </>
  );
}

let blockedKey = 0;

function PlayDrag(props: DragLayerProps & { onBlocked: (blocked: Blocked) => void }): ReactElement | null {
  // Listeners read this ref so a mid-drag render does not drop the drag.
  const latest = useRef(props);
  latest.current = props;

  const [drawn, setDrawn] = useState<Flight | null>(null);
  const [landed, setLanded] = useState<Landed | null>(null);

  // A landed card leaves on a newer view or timeout, never after this layer.
  useEffect(() => {
    if (landed === null) return undefined;
    if (props.view !== landed.view) {
      setLanded(null);
      return undefined;
    }
    const timer = setTimeout(() => setLanded(null), LANDING_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, [landed, props.view]);

  // Keep the card out of the fan in the landing frame.
  useLayoutEffect(() => {
    setLanding(landed === null ? null : landed.instanceId);
  }, [landed]);

  useEffect(() => () => setLanding(null), []);

  useEffect(() => {
    const root = document.documentElement;
    let press: Press | null = null;
    let flight: Flight | null = null;
    let swallowClick = false;

    /** jsdom lacks `elementsFromPoint`, so tests stub it on `document`. */
    function hits(x: number, y: number): readonly Element[] {
      try {
        return document.elementsFromPoint(x, y);
      } catch {
        // jsdom has no elementsFromPoint: nothing is under the pointer.
        return [];
      }
    }

    function spotAt(plan: DragPlan, x: number, y: number): DropSpot {
      return pickDropSpot(hits(x, y), plan.dropTestids);
    }

    function stopDragging(): void {
      const ending = flight;
      flight = null;
      press = null;
      root.removeAttribute("data-dragging");
      if (ending !== null) {
        try {
          if (ending.element.hasPointerCapture(ending.pointerId)) ending.element.releasePointerCapture(ending.pointerId);
        } catch {
          // jsdom has no pointer capture.
        }
      }
      swallowClick = true;
      setDrawn(null);
    }

    /**
     * Back to idle with nothing sent, or (R658) to the build a drag lifted again from its picks. A
     * press that never became a drag is simply forgotten.
     */
    function cancel(): void {
      if (flight === null) {
        press = null;
        return;
      }
      const missed = flight.plan.missed ?? IDLE;
      stopDragging();
      latest.current.onInteraction(missed);
    }

    function onPointerDown(event: PointerEvent): void {
      if (flight !== null) {
        // A repeated pointer means its release was lost, so drop the stale drag.
        if (event.pointerId !== flight.pointerId) return;
        cancel();
      }
      swallowClick = false;
      if (press !== null && press.pointerId !== event.pointerId) return;
      press = null;

      if (event.button !== 0) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (target.closest(BOARD) === null) return;
      const hit = targetFromElement(target);
      if (hit === null) return;
      const source = hit.target;
      const yours = (source.on === "unit" || source.on === "backrow") && source.side === "you";
      const building = isBuilding(latest.current.interaction);
      if (source.on !== "hand" && source.on !== "activate" && !yours && !building) return;
      if (!readSettings().dragToPlay) return;

      const element = target.closest(`[data-testid="${hit.testid.replace(/["\\]/g, "\\$&")}"]`) ?? target;
      press = {
        pointerId: event.pointerId,
        start: { x: event.clientX, y: event.clientY },
        source,
        testid: hit.testid,
        element,
      };
    }

    function onPointerMove(event: PointerEvent): void {
      if (flight !== null) {
        if (event.pointerId !== flight.pointerId) return;
        if (event.button === 2) {
          cancel();
          return;
        }
        const pointer = { x: event.clientX, y: event.clientY };
        const spot = spotAt(flight.plan, pointer.x, pointer.y);
        flight = {
          ...flight,
          pointer,
          from: centreOf(flight.element) ?? flight.from,
          spot,
          reticle: reticleFor(spot),
        };
        setDrawn(flight);
        return;
      }

      if (press === null || event.pointerId !== press.pointerId) return;
      const travelled = Math.hypot(event.clientX - press.start.x, event.clientY - press.start.y);
      if (travelled < DRAG_THRESHOLD_PX) return;

      const { view, legal, interaction, onInteraction, onLift } = latest.current;
      const plan = planDrag(view, legal, interaction, press.source, press.testid);
      const element = press.element;
      const start = press.start;
      press = null;
      if (plan === null) return;

      onInteraction(plan.lifted);
      onLift?.(plan);
      root.setAttribute("data-dragging", plan.kind);
      try {
        element.setPointerCapture(event.pointerId);
      } catch {
        // jsdom lacks pointer capture; an already-gone pointer cannot be captured.
      }

      const pointer = { x: event.clientX, y: event.clientY };
      const spot = spotAt(plan, pointer.x, pointer.y);
      flight = {
        pointerId: event.pointerId,
        plan,
        element,
        pointer,
        from: centreOf(element) ?? start,
        touch: event.pointerType === "touch",
        spot,
        reticle: reticleFor(spot),
      };
      setDrawn(flight);
    }

    function onPointerUp(event: PointerEvent): void {
      if (flight !== null) {
        if (event.pointerId !== flight.pointerId) return;
        const { view, legal, onInteraction, onAction, onBlocked } = latest.current;
        const spot = spotAt(flight.plan, event.clientX, event.clientY);
        const result = resolveDrop(view, legal, flight.plan, spot);
        const refused = result.action === undefined ? lockedZoneAt(hits(event.clientX, event.clientY), flight.plan) : null;
        const { plan } = flight;
        const reticle = reticleFor(spot);
        const at = reticle === null ? { x: event.clientX, y: event.clientY } : { x: reticle.x, y: reticle.y };
        stopDragging();
        onInteraction(result.interaction);
        if (refused !== null) {
          const rect = refused.zone.getBoundingClientRect();
          blockedKey += 1;
          onBlocked({
            key: blockedKey,
            testid: refused.testid,
            x: round(rect.left + rect.width / 2),
            y: round(rect.top + rect.height / 2),
            size: round(Math.min(rect.width, rect.height)),
          });
        }
        if (result.action !== undefined) {
          const card = plan.kind === "play" && plan.source.on === "hand" ? handCard(view, plan.source.instanceId) : null;
          if (card !== null) {
            // R658: a play lifted again from its zone lands in that zone, not on the Cry's target.
            const zone = plan.missed?.stage === "playing" ? plan.missed.picked.zone : undefined;
            const zoneBox = zone === undefined ? null : byTestid(testid.zone("you", zone.row, zone.lane));
            const settled = zoneBox === null ? null : centreOf(zoneBox);
            setLanded({ instanceId: card.instanceId, card, at: settled ?? at, view });
          }
          onAction(result.action);
        }
        return;
      }
      if (press !== null && event.pointerId === press.pointerId) press = null;
    }

    function onPointerCancel(event: PointerEvent): void {
      if (flight !== null && event.pointerId === flight.pointerId) {
        cancel();
        return;
      }
      if (press !== null && event.pointerId === press.pointerId) press = null;
    }

    function onBlur(): void {
      cancel();
    }

    function onKeyDown(event: KeyboardEvent): void {
      swallowClick = false;
      if (event.key !== "Escape") return;
      if (flight !== null) {
        cancel();
        return;
      }
      press = null;
      if (latest.current.interaction.stage !== "idle") latest.current.onInteraction(IDLE);
    }

    function onContextMenu(event: MouseEvent): void {
      if (flight !== null) {
        event.preventDefault();
        cancel();
        return;
      }
      const target = event.target;
      if (!(target instanceof Element) || target.closest(BOARD) === null) return;
      if (latest.current.interaction.stage === "idle") return;
      event.preventDefault();
      press = null;
      latest.current.onInteraction(IDLE);
    }

    function onClickCapture(event: MouseEvent): void {
      if (!swallowClick) return;
      swallowClick = false;
      event.stopPropagation();
      event.preventDefault();
    }

    /** Native HTML5 drag cancels pointer drag, so block it while tracking one. */
    function onDragStart(event: DragEvent): void {
      if (press === null && flight === null) return;
      const target = event.target;
      if (!(target instanceof Element) || target.closest(BOARD) === null) return;
      event.preventDefault();
    }

    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("pointermove", onPointerMove, true);
    window.addEventListener("pointerup", onPointerUp, true);
    window.addEventListener("pointercancel", onPointerCancel, true);
    window.addEventListener("click", onClickCapture, true);
    window.addEventListener("dragstart", onDragStart, true);
    window.addEventListener("blur", onBlur);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("contextmenu", onContextMenu);

    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("pointermove", onPointerMove, true);
      window.removeEventListener("pointerup", onPointerUp, true);
      window.removeEventListener("pointercancel", onPointerCancel, true);
      window.removeEventListener("click", onClickCapture, true);
      window.removeEventListener("dragstart", onDragStart, true);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("contextmenu", onContextMenu);
      if (flight !== null) {
        try {
          if (flight.element.hasPointerCapture(flight.pointerId)) flight.element.releasePointerCapture(flight.pointerId);
        } catch {
          // jsdom has no pointer capture.
        }
      }
      flight = null;
      press = null;
      root.removeAttribute("data-dragging");
    };
  }, []);

  if (drawn === null) {
    if (landed === null) return null;
    return (
      <div className="drag-layer drag-landing" data-testid="drag-landing" aria-hidden="true" style={{ pointerEvents: "none" }}>
        <MatchCardsProvider view={props.view}>
          <DragGhost
            testId="drag-landing-card"
            card={landed.card}
            instanceId={landed.instanceId}
            at={landed.at}
            touch={false}
            valid
            landing
          />
        </MatchCardsProvider>
      </div>
    );
  }

  const { plan, spot } = drawn;
  const valid = isValidSpot(plan, spot);
  const locked = spot.at === "target" ? drawn.reticle : null;
  const shape: ReticleShape =
    spot.at !== "target" || !plan.arrow ? "pad" : spot.testid.startsWith("card-") ? "frame" : "ring";

  return (
    <div
      className="drag-layer"
      data-testid="drag-layer"
      data-kind={plan.kind}
      aria-hidden="true"
      style={{ pointerEvents: "none" }}
    >
      {plan.arrow ? (
        <DragArrow
          from={drawn.from}
          to={locked === null ? drawn.pointer : { x: locked.x, y: locked.y }}
          sourceTestid={plan.sourceTestid}
          valid={valid}
          stop={locked === null ? null : arrowStop(locked, shape)}
        />
      ) : (
        <MatchCardsProvider view={props.view}>
          <DragGhost
            card={sourceCard(props.view, plan.source.instanceId)}
            instanceId={plan.source.instanceId}
            at={drawn.pointer}
            touch={drawn.touch}
            valid={valid}
          />
        </MatchCardsProvider>
      )}
      {spot.at === "target" ? (
        <Reticle testid={spot.testid} box={drawn.reticle} shape={shape} />
      ) : null}
    </div>
  );
}

function arrowStop(box: Box, shape: ReticleShape): ArrowStop {
  return shape === "frame"
    ? {
        shape: "box",
        halfWidth: box.width / 2 + RETICLE_FRAME_OUTSET_PX,
        halfHeight: box.height / 2 + RETICLE_FRAME_OUTSET_PX,
      }
    : { shape: "circle", halfWidth: box.size / 2, halfHeight: box.size / 2 };
}

function handCard(view: PlayerView, instanceId: string): CardView | null {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) return null;
  return hand.find((card) => card.instanceId === instanceId) ?? null;
}

function sourceCard(view: PlayerView, instanceId: string): CardView | null {
  const inHand = handCard(view, instanceId);
  if (inHand !== null) return inHand;
  for (const entry of view.you.backrow) {
    if (entry !== null && !entry.faceDown && entry.instanceId === instanceId) return entry;
  }
  return null;
}

/** The ghost has no card testid, so it cannot be mistaken for the card it represents. */
function DragGhost(props: {
  card: CardView | null;
  instanceId: string;
  at: Point;
  touch: boolean;
  valid: boolean;
  landing?: boolean;
  testId?: string;
}): ReactElement {
  const info = useCardInfo(props.card?.defId ?? "", props.card?.radiant ?? false);
  const copied = useCopiedDef(props.card);
  const face = props.card === null ? null : liveFace(info, props.card, copied === undefined ? {} : { copied });
  const text = face === null ? info.text : face.text.full;
  const stats = face === null ? (info.attack === undefined || info.health === undefined ? null : { attack: info.attack, health: info.health }) : face.stats;
  const style: CSSProperties = { left: props.at.x, top: props.at.y };
  return (
    <div
      className="drag-ghost"
      data-testid={props.testId ?? "drag-ghost"}
      data-instance-id={props.instanceId}
      data-pointer={props.touch ? "touch" : "mouse"}
      data-valid={props.valid ? "true" : "false"}
      data-landing={props.landing === true ? "true" : undefined}
      data-radiant={props.card?.radiant === true ? "true" : undefined}
      style={style}
    >
      {props.card === null ? null : <span className="drag-ghost-cost">{props.card.cost}</span>}
      <span className="drag-ghost-name">{face?.name ?? info.name}</span>
      {text === "" ? null : <span className="drag-ghost-text">{text}</span>}
      {stats === null ? null : (
        <span className="drag-ghost-stats" aria-hidden="true">
          <span className="drag-ghost-attack">{stats.attack}</span>
          <span className="drag-ghost-health">{stats.health}</span>
        </span>
      )}
    </div>
  );
}

function DragArrow(props: {
  from: Point;
  to: Point;
  sourceTestid: string;
  valid: boolean;
  stop: ArrowStop | null;
}): ReactElement {
  const { from, to } = props;
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;

  const bend = Math.min(ARROW_BEND_MAX_PX, length * ARROW_BEND_SHARE);
  let nx = -dy / length;
  let ny = dx / length;
  if (ny > 0) {
    nx = -nx;
    ny = -ny;
  }
  const cx = (from.x + to.x) / 2 + nx * bend;
  const cy = (from.y + to.y) / 2 + ny * bend;

  const tx = to.x - cx;
  const ty = to.y - cy;
  const tangent = Math.hypot(tx, ty) || 1;
  const ux = tx / tangent;
  const uy = ty / tangent;

  const withHead = props.stop === null;
  const head = Math.min(ARROW_HEAD_PX, length);
  const cut =
    props.stop === null ? head - ARROW_HEAD_NOTCH_PX : Math.min(stopDistance(props.stop, ux, uy), length * 0.6);
  const endX = to.x - ux * cut;
  const endY = to.y - uy * cut;

  const baseX = to.x - ux * head;
  const baseY = to.y - uy * head;
  const notchX = to.x - ux * (head - ARROW_HEAD_NOTCH_PX);
  const notchY = to.y - uy * (head - ARROW_HEAD_NOTCH_PX);
  const leftX = baseX - uy * ARROW_HEAD_HALF_WIDTH_PX;
  const leftY = baseY + ux * ARROW_HEAD_HALF_WIDTH_PX;
  const rightX = baseX + uy * ARROW_HEAD_HALF_WIDTH_PX;
  const rightY = baseY - ux * ARROW_HEAD_HALF_WIDTH_PX;

  const shaft = `M ${round(from.x)} ${round(from.y)} Q ${round(cx)} ${round(cy)} ${round(endX)} ${round(endY)}`;
  const tip = [
    `${round(to.x)},${round(to.y)}`,
    `${round(leftX)},${round(leftY)}`,
    `${round(notchX)},${round(notchY)}`,
    `${round(rightX)},${round(rightY)}`,
  ].join(" ");

  // Use screen space: an arrow-sized region collapses for straight arrows.
  const glowX = Math.min(from.x, to.x, cx) - ARROW_GLOW_MARGIN_PX;
  const glowY = Math.min(from.y, to.y, cy) - ARROW_GLOW_MARGIN_PX;
  const glowW = Math.max(from.x, to.x, cx) + ARROW_GLOW_MARGIN_PX - glowX;
  const glowH = Math.max(from.y, to.y, cy) + ARROW_GLOW_MARGIN_PX - glowY;

  return (
    <svg
      className="drag-arrow"
      data-testid="drag-arrow"
      data-from={props.sourceTestid}
      data-valid={props.valid ? "true" : "false"}
      width="100%"
      height="100%"
      aria-hidden="true"
      focusable="false"
    >
      <defs>
        {/* Along the arrow in screen space: dim at the source, full at the tip. */}
        <linearGradient
          id="drag-arrow-fade"
          gradientUnits="userSpaceOnUse"
          x1={round(from.x)}
          y1={round(from.y)}
          x2={round(to.x)}
          y2={round(to.y)}
        >
          <stop offset="0" className="drag-arrow-stop-tail" />
          <stop offset="1" className="drag-arrow-stop-tip" />
        </linearGradient>
        <filter
          id="drag-arrow-glow"
          filterUnits="userSpaceOnUse"
          x={round(glowX)}
          y={round(glowY)}
          width={round(glowW)}
          height={round(glowH)}
        >
          <feGaussianBlur in="SourceGraphic" stdDeviation={ARROW_GLOW_BLUR_PX} result="glow" />
          <feMerge>
            <feMergeNode in="glow" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <path className="drag-arrow-shadow" d={shaft} />
      <g className="drag-arrow-lit" filter="url(#drag-arrow-glow)">
        <path className="drag-arrow-shaft" d={shaft} />
        <path className="drag-arrow-core" d={shaft} />
        <path className="drag-arrow-pulse" d={shaft} />
        {withHead ? <polygon className="drag-arrow-head" points={tip} /> : null}
      </g>
      <circle className="drag-arrow-origin" cx={round(from.x)} cy={round(from.y)} r={7} />
    </svg>
  );
}

function stopDistance(stop: ArrowStop, ux: number, uy: number): number {
  if (stop.shape === "circle") return stop.halfWidth;
  const alongX = Math.abs(ux) < 1e-6 ? Infinity : stop.halfWidth / Math.abs(ux);
  const alongY = Math.abs(uy) < 1e-6 ? Infinity : stop.halfHeight / Math.abs(uy);
  return Math.min(alongX, alongY);
}

/** A frame keeps a targeted card's name and stats readable; a pad marks a placement zone. */
function Reticle(props: { testid: string; box: Box | null; shape: ReticleShape }): ReactElement {
  const { box } = props;
  const style: CSSProperties =
    box === null
      ? { display: "none" }
      : props.shape === "ring"
        ? { left: box.x, top: box.y, width: box.size, height: box.size }
        : props.shape === "frame"
          ? {
              left: box.x,
              top: box.y,
              width: box.width + 2 * RETICLE_FRAME_OUTSET_PX,
              height: box.height + 2 * RETICLE_FRAME_OUTSET_PX,
            }
          : { left: box.x, top: box.y, width: box.width, height: box.height };
  return (
    <div
      className="drag-reticle"
      data-testid="drag-reticle"
      data-target={props.testid}
      data-shape={props.shape}
      style={style}
    />
  );
}
