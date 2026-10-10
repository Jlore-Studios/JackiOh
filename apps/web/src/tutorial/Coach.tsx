// Coach bubble and anchor ring (SPEC §9.10, R292): wait for board animation and keep controls clear.
// R314: it never skips or blocks play; focus stays in prompts and dialogs.
// CLAUDE.md rule 7: anchors come from view testids.

import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type CSSProperties,
  type ReactElement,
} from "react";

import { LANES, testid } from "../game/contract.ts";
import {
  COACH_BUBBLE_GAP_PX,
  COACH_BUBBLE_MIN_HEIGHT_PX,
  COACH_DOCK_QUERY,
  COACH_RING_PAD_PX,
  COACH_SHOWCASE_WAIT_MAX_MS,
  COACH_TRACK_INTERVAL_MS,
  COACH_VIEWPORT_MARGIN_PX,
} from "./config.ts";
import { padRect, placeBubble, unionRect, type BubblePlacement, type Rect } from "./layout.ts";
import { tutorialTestid } from "./testids.ts";
import { displayKey, type CoachTracker, type CoachView } from "./tracker.ts";
import "./tutorial.css";

const SOFT_OBSTACLES: readonly string[] = ["prompt-modal", "hand-you", "end-turn"];

/** Keep unit rows clear when an anchored bubble leaves one side free. */
const UNIT_ROWS: readonly string[] = (["opponent", "you"] as const).flatMap((side) =>
  LANES.map((lane) => testid.zone(side, "units", lane)),
);

/** Keep legal targets clear during a play or attack; otherwise keep ready units clear. */
const IN_PROGRESS = `[data-testid="${testid.board}"] [data-selected="true"]`;
const ASKED_FOR = ["zone-", "card-", "hero-"].map((prefix) => `[data-testid^="${prefix}"][data-legal="true"]`).join(", ");
const READY_UNITS = '[data-testid^="card-"][data-glow]';

const PROMPT_MODAL = "prompt-modal";

const FOCUS_KEEPERS = '[data-testid="prompt-modal"], [role="dialog"][aria-modal="true"], [role="alertdialog"]';

const YOUR_MOVE = "Your move: play cards and attack, then press End turn.";
const AI_MOVE = "The AI is taking its turn.";

function byTestid(id: string): Element | null {
  return document.querySelector(`[data-testid="${id.replace(/["\\]/g, "\\$&")}"]`);
}

function rectOf(element: Element | null): Rect | null {
  if (element === null) return null;
  const box = element.getBoundingClientRect();
  // An unlaid-out element has no ring.
  if (box.width <= 0 && box.height <= 0) return null;
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}

/** Do not ring an anchor hidden beneath an open prompt. */
function visibleRect(element: Element | null, prompt: Element | null): Rect | null {
  const rect = rectOf(element);
  if (rect === null || element === null || prompt === null || prompt.contains(element)) return rect;
  const cover = rectOf(prompt);
  if (cover === null) return rect;
  const x = rect.left + rect.width / 2;
  const y = rect.top + rect.height / 2;
  const hidden = x >= cover.left && x <= cover.left + cover.width && y >= cover.top && y <= cover.top + cover.height;
  return hidden ? null : rect;
}

export type CoachDock = "panel" | "float";

function dockQuery(): MediaQueryList | null {
  try {
    return typeof window.matchMedia === "function" ? window.matchMedia(COACH_DOCK_QUERY) : null;
  } catch {
    return null;
  }
}

function currentDock(): CoachDock {
  return dockQuery()?.matches === true ? "panel" : "float";
}

function serverDock(): CoachDock {
  return "float";
}

/** Resize supports old Safari and test stubs that do not emit MediaQueryList changes. */
function subscribeDock(onChange: () => void): () => void {
  const query = dockQuery();
  window.addEventListener("resize", onChange);
  if (typeof query?.addEventListener === "function") query.addEventListener("change", onChange);
  else query?.addListener?.(onChange);
  return () => {
    window.removeEventListener("resize", onChange);
    if (typeof query?.removeEventListener === "function") query.removeEventListener("change", onChange);
    else query?.removeListener?.(onChange);
  };
}

type Geometry = { ring: Rect | null; place: BubblePlacement | null; clamped: boolean };

const NO_GEOMETRY: Geometry = { ring: null, place: null, clamped: false };

function sameRect(a: Rect | null, b: Rect | null): boolean {
  if (a === null || b === null) return a === b;
  return (
    Math.round(a.left) === Math.round(b.left) &&
    Math.round(a.top) === Math.round(b.top) &&
    Math.round(a.width) === Math.round(b.width) &&
    Math.round(a.height) === Math.round(b.height)
  );
}

function sameGeometry(a: Geometry, b: Geometry): boolean {
  if (!sameRect(a.ring, b.ring) || a.clamped !== b.clamped) return false;
  const p = a.place;
  const q = b.place;
  if (p === null || q === null) return p === q;
  return (
    p.side === q.side &&
    Math.round(p.left) === Math.round(q.left) &&
    Math.round(p.top) === Math.round(q.top) &&
    p.maxHeight === q.maxHeight
  );
}

/** Keep the prior display until board animation and any showcased card have cleared. */
function useCaughtUp(latest: CoachView, root: HTMLElement | null): CoachView | null {
  const [shown, setShown] = useState<CoachView | null>(null);
  const newest = useRef(latest);
  newest.current = latest;
  const showing = useRef<CoachView | null>(null);
  showing.current = shown;

  useEffect(() => {
    let live = true;
    let cap: ReturnType<typeof setTimeout> | null = null;
    /** Do not restart a timed-out showcase wait until it disappears. */
    let capped = false;
    const adopt = (): void => {
      if (!live) return;
      if (root !== null && root.querySelector("[data-animating]") !== null) return;
      if (document.querySelector("[data-showcase]") !== null) {
        if (!capped) {
          cap ??= setTimeout(() => {
            cap = null;
            capped = true;
            adopt();
          }, COACH_SHOWCASE_WAIT_MAX_MS);
          return;
        }
      } else {
        capped = false;
        if (cap !== null) clearTimeout(cap);
        cap = null;
      }
      setShown(newest.current);
    };
    const now = showing.current;
    if (now === null || now.ctx === latest.ctx) adopt();
    else queueMicrotask(adopt);
    if (typeof MutationObserver !== "function") {
      return () => {
        live = false;
        if (cap !== null) clearTimeout(cap);
      };
    }
    const observer = new MutationObserver(adopt);
    if (root !== null) {
      observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-animating"] });
    }
    // The showcase is portalled to <body>.
    observer.observe(document.body, { childList: true });
    return () => {
      live = false;
      observer.disconnect();
      if (cap !== null) clearTimeout(cap);
    };
  }, [latest, root]);

  return shown;
}

export type CoachProps = {
  tracker: CoachTracker;
  boardRoot: HTMLElement | null;
};

export function Coach({ tracker, boardRoot }: CoachProps): ReactElement | null {
  const latest = useSyncExternalStore(tracker.subscribe, tracker.getState, tracker.getState);
  const shown = useCaughtUp(latest, boardRoot);
  const display = shown?.display ?? null;
  const visible = display !== null && display.mode !== "finished";
  const key = display === null ? "none" : displayKey(display);
  const stale = shown !== null && key !== displayKey(latest.display);
  const targets = shown?.targets ?? [];
  const targetKey = targets.join(" ");
  const slim = display?.mode === "waiting";
  const dock = useSyncExternalStore(subscribeDock, currentDock, serverDock);
  const panel = dock === "panel";

  const bubble = useRef<HTMLElement>(null);
  const text = useRef<HTMLParagraphElement>(null);
  const ackButton = useRef<HTMLButtonElement>(null);
  const refocus = useRef(false);
  const titleId = useId();
  const textId = useId();
  const [geometry, setGeometry] = useState<Geometry>(NO_GEOMETRY);
  const [expandedFor, setExpandedFor] = useState<string | null>(null);
  const expanded = panel && expandedFor === key;

  // Re-measure as cards animate or fan on hover; neither resizes a page-observable element.
  useLayoutEffect(() => {
    if (!visible) {
      setGeometry((prev) => (sameGeometry(prev, NO_GEOMETRY) ? prev : NO_GEOMETRY));
      return;
    }
    const ids = targetKey.length === 0 ? [] : targetKey.split(" ");
    const measure = (): void => {
      const prompt = byTestid(PROMPT_MODAL);
      const found = ids.map((id) => visibleRect(byTestid(id), prompt)).filter((rect): rect is Rect => rect !== null);
      const union = unionRect(found);
      const ring = union === null ? null : padRect(union, COACH_RING_PAD_PX);
      if (panel) {
        // Preserve the previous overflow result while expanded so Less remains available.
        const box = text.current;
        const over = box !== null && box.scrollHeight > box.clientHeight + 1;
        setGeometry((prev) => {
          const next: Geometry = { ring, place: null, clamped: expanded ? prev.clamped : over };
          return sameGeometry(prev, next) ? prev : next;
        });
        return;
      }
      const own = new Set(ids);
      const avoid = (ring === null ? SOFT_OBSTACLES : [...SOFT_OBSTACLES, ...UNIT_ROWS])
        .filter((id) => !own.has(id))
        .map((id) => rectOf(byTestid(id)))
        .filter((rect): rect is Rect => rect !== null);
      const inProgress = document.querySelector(IN_PROGRESS) !== null || document.documentElement.hasAttribute("data-dragging");
      const keepClear = [...document.querySelectorAll(inProgress ? ASKED_FOR : READY_UNITS)]
        .filter((element) => !own.has(element.getAttribute("data-testid") ?? ""))
        .map((element) => rectOf(element))
        .filter((rect): rect is Rect => rect !== null);
      const hud = rectOf(byTestid(tutorialTestid.hud));
      const element = bubble.current;
      const place = placeBubble({
        anchor: ring,
        bubble: { width: element?.offsetWidth ?? 0, height: element?.offsetHeight ?? 0 },
        viewport: { width: window.innerWidth, height: window.innerHeight },
        slim,
        insetTop: hud === null ? 0 : hud.top + hud.height,
        avoid,
        keepClear,
        gap: COACH_BUBBLE_GAP_PX,
        margin: COACH_VIEWPORT_MARGIN_PX,
        minHeight: COACH_BUBBLE_MIN_HEIGHT_PX,
      });
      const next: Geometry = { ring, place, clamped: false };
      setGeometry((prev) => (sameGeometry(prev, next) ? prev : next));
    };
    measure();
    const frame = typeof requestAnimationFrame === "function" ? requestAnimationFrame(measure) : null;
    const timer = setInterval(measure, COACH_TRACK_INTERVAL_MS);
    window.addEventListener("resize", measure);
    window.addEventListener("scroll", measure, { capture: true, passive: true });
    return () => {
      if (frame !== null) cancelAnimationFrame(frame);
      clearInterval(timer);
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, { capture: true });
    };
  }, [visible, shown, targetKey, slim, panel, expanded]);

  // Never move focus out of an open prompt or dialog.
  useEffect(() => {
    if (!visible || display === null) return;
    const wantsAck = (display.mode === "tip" || display.mode === "step") && display.ack;
    const active = document.activeElement;
    const kept = active instanceof Element && active.closest(FOCUS_KEEPERS) !== null;
    if (wantsAck && !kept) {
      ackButton.current?.focus({ preventScroll: true });
    } else if (refocus.current && !kept) {
      bubble.current?.focus({ preventScroll: true });
    }
    refocus.current = false;
    // Only a new display moves focus, never a re-render of the same one.
  }, [visible, key]);

  const answer = useCallback(() => {
    refocus.current = bubble.current?.contains(document.activeElement) === true;
    tracker.ack(key);
  }, [tracker, key]);

  if (!visible || display === null || shown === null) return null;

  const place = panel ? null : geometry.place;
  const style: CSSProperties | undefined = panel
    ? undefined
    : {
        left: place?.left ?? 0,
        top: place?.top ?? 0,
        ...(place?.maxHeight == null ? {} : { maxHeight: place.maxHeight }),
      };
  const count = `${String(display.stepNumber)} / ${String(display.stepCount)}`;
  const countLabel = `Step ${String(display.stepNumber)} of ${String(display.stepCount)}`;
  const full = display.mode === "tip" || display.mode === "step";
  const yourMove = !full && !shown.aiBusy && shown.yourMove;
  const waitingLine = shown.aiBusy ? AI_MOVE : yourMove ? YOUR_MOVE : "";
  const className = ["coach", panel ? "coach--panel" : "coach--float", ...(full ? [] : ["coach--slim"])].join(" ");
  const more = panel && (expanded || geometry.clamped);

  return (
    <>
      {geometry.ring === null ? null : (
        <div
          className="coach-ring"
          data-testid={tutorialTestid.coachRing}
          data-stale={stale ? "true" : undefined}
          aria-hidden="true"
          style={{
            left: geometry.ring.left,
            top: geometry.ring.top,
            width: geometry.ring.width,
            height: geometry.ring.height,
          }}
        />
      )}
      <section
        ref={bubble}
        className={className}
        data-testid={tutorialTestid.coach}
        data-coach-mode={display.mode}
        data-coach-step={full ? display.id : undefined}
        data-coach-anchor={targetKey}
        data-coach-dock={dock}
        data-coach-side={place?.side}
        data-placed={panel || place !== null ? "true" : "false"}
        data-expanded={panel ? (expanded ? "true" : "false") : undefined}
        data-stale={stale ? "true" : undefined}
        role="region"
        aria-label={full ? undefined : "Tutorial coach"}
        aria-labelledby={full ? titleId : undefined}
        tabIndex={-1}
        style={style}
      >
        <div key="heading" className="coach__heading">
          <header className="coach__head">
            <span className="coach__eyebrow">{display.mode === "tip" ? "Tip" : "Coach"}</span>
            <span className="coach__count" aria-label={countLabel}>
              {count}
            </span>
          </header>
          {full ? (
            <h2 className="coach__title" id={titleId}>
              {display.title}
            </h2>
          ) : null}
        </div>
        <div key="body" className="coach__body">
          <p ref={text} id={textId} className="coach__text" aria-live="polite">
            {full ? display.text : waitingLine}
          </p>
          {more ? (
            <button
              type="button"
              className="coach__more"
              aria-expanded={expanded}
              aria-controls={textId}
              onClick={() => {
                setExpandedFor(expanded ? null : key);
              }}
            >
              {expanded ? "Less" : "More"}
            </button>
          ) : null}
        </div>
        <div key="actions" className="coach__actions">
          {full && display.ack ? (
            <button
              ref={ackButton}
              type="button"
              className="coach__ack"
              data-testid={tutorialTestid.coachAck}
              onClick={answer}
            >
              Got it
            </button>
          ) : null}
        </div>
      </section>
    </>
  );
}
