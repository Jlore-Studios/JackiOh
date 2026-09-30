// The opponent's play, held up (Hearthstone's "your opponent played …"): when the other player plays
// a card, its face stands beside the field for about a second (SHOWCASE_HOLD_MS, divided by the
// viewer's effects speed, R201) so it can be read before the game moves on.
//
// What it shows is decided in plan.ts from the view's own redacted events (CLAUDE.md rule 7, R97,
// R202): the opponent's `cardPlayed`, and a card back with a caption when the view hides the card
// (a Trap or Field Trap set face down, R227). The viewer's own plays are never shown.
//
// R502: a card cast the moment it was drawn (#21 Hinder, #27 Blood Ridden, any card given Cast on
// draw) is shown on BOTH seats, the drawer's own too, for longer (SHOWCASE_CAST_HOLD_MS over the
// speed, capped), under a "Cast on draw!" ribbon, bursting out of its drawer's Deck pile as it
// appears. When Game hands over its animation runner (`queue`), a cast on draw waits for the runner to
// reach its `cardPlayed`, so it appears as the board plays it and its effect plays out while it is
// up; the runner going idle, draining or resetting lets it go at once, and it never waits longer than
// SHOWCASE_GATE_MAX_MS. It listens to the runner and never answers it: nothing here paces the board.
//
// R436: a Call to Chaos roll is announced in words on both seats through its own polite live region,
// and where the effects layer draws nothing (reduced motion, or the effects off) it is shown still
// (ChaosBanner) for as long as the effects layer's reveal would have stood.
//
// Which plays are new is kept PER VIEWER. Online and in practice the viewer never changes, so this
// is "as they happen". On a hotseat device each seat catches up on the plays made since it last
// held the device, which is how the arriving player sees what was played while the other one had it.
//
// It paces nothing and blocks nothing. It is portalled to <body>, click-through (`pointer-events:
// none`) and hidden from assistive tech, which hears a polite live region instead; it never touches
// the animation runner, `data-animating` or `legal`, so it holds no view back and `cy.settled()`
// never waits for it. It goes when its hold runs out, when a newer play replaces it, on Escape, and
// on the viewer's first pointer down anywhere (a player who starts to act has moved on). While it is
// up, `data-showcase` is on it: the practice route holds the AI's next step on that attribute, as it
// does on `data-speaking`, so the AI never plays its next card over the one being read.
//
// Reduced motion (the media query, the settings panel's switch or the effects store's "reduce"):
// the card is information, not decoration, so it is still shown for the same hold, but it appears
// and goes without the fade, the rise and the burst (showcase.css keys that off `data-motion="reduce"`).

import {
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
} from "react";
import { createPortal } from "react-dom";

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { CardBack, CardFace, costPhrase } from "../../cards/index.ts";
import { chaosNames, type ChaosRoll } from "../../fx/chaos.ts";
import { FX_BANNER_TAIL_MS } from "../../fx/constants.ts";
import { getFxSettings, normalizeSpeed } from "../../fx/settings.ts";
import { ANIMATIONS, reducedMotionNow, type AnimationQueue, type RunnerSignal } from "../animations.ts";
import { CatalogContext } from "../catalog.ts";
import { sideOf, type Side } from "../contract.ts";
import { namedFace } from "../faces.ts";
import ChaosBanner from "./ChaosBanner.tsx";
import {
  CAST_ON_DRAW_TEXT,
  CHAOS_TEXT,
  SHOWCASE_BURST_MS,
  SHOWCASE_FADE_MS,
  SHOWCASE_GATE_MAX_MS,
  SHOWCASE_QUEUE_MAX,
  showcaseCastHoldMs,
  showcaseHoldMs,
  showcaseTestid,
  type ShowcaseKind,
} from "./constants.ts";
import { capQueue, chaosRollsIn, eventsSince, showcasePlays, type ShowcasePlay } from "./plan.ts";
import "./showcase.css";

export type CardShowcaseProps = {
  /** The newest view (Game's `view`, not the one the board is still showing): a play is held up as it arrives. */
  view: PlayerView;
  /**
   * The animation runner (Game's), which a cast on draw and a Call to Chaos roll wait on (R502,
   * R436). Absent: they are held up as they arrive, like every other play.
   */
  queue?: AnimationQueue;
};

type Showing = { play: ShowcasePlay; seq: number; holdMs: number; reduced: boolean; viewer: PlayerId; side: Side };

type ChaosShowing = { names: string[]; seq: number; holdMs: number; still: boolean };

function kindOf(play: ShowcasePlay): ShowcaseKind {
  if (play.castOnDraw === true) return "cast";
  if (play.defId !== null) return "played";
  return play.set ? "set" : "hidden";
}

const CAPTION: Readonly<Record<Exclude<ShowcaseKind, "cast">, string>> = {
  played: "Opponent played",
  set: "Opponent set a card",
  hidden: "Opponent played a card",
};

/** R502: a cast on draw's caption: whose draw it was, and whether the view names the card. */
function castCaption(play: ShowcasePlay, viewer: PlayerId): string {
  if (play.player === viewer) return CAST_ON_DRAW_TEXT.you;
  return play.defId === null ? CAST_ON_DRAW_TEXT.hidden : CAST_ON_DRAW_TEXT.opponent;
}

function captionOf(play: ShowcasePlay, kind: ShowcaseKind, viewer: PlayerId): string {
  return kind === "cast" ? castCaption(play, viewer) : CAPTION[kind];
}

/** What the live region says for a play. */
function saidOf(play: ShowcasePlay, kind: ShowcaseKind, viewer: PlayerId, name: string | undefined): string {
  if (kind === "cast") {
    const drew = castCaption(play, viewer);
    return play.defId === null ? `${drew}: ${CAST_ON_DRAW_TEXT.said}` : `${drew} ${name ?? "a card"}: ${CAST_ON_DRAW_TEXT.said}`;
  }
  return kind === "played" ? `${CAPTION.played} ${name ?? "a card"}` : CAPTION[kind];
}

/** Whether the effects layer draws right now (FxLayer's own test): otherwise a roll is shown still. */
function effectsDrawing(): boolean {
  const settings = getFxSettings();
  return !reducedMotionNow(settings) && settings.intensity !== "off";
}

/** R436: as long as the effects layer's reveal stands at this speed (the entry and its tail). */
export function chaosHoldMs(speed: number): number {
  return Math.round(ANIMATIONS.chaosRolled.durationMs / normalizeSpeed(speed)) + FX_BANNER_TAIL_MS;
}

/** Something waiting for the runner to reach `event`, and what to do then. */
type Gated = { event: GameEvent; release: () => void };

function sameEvent(a: GameEvent, b: GameEvent): boolean {
  return a === b || JSON.stringify(a) === JSON.stringify(b);
}

export default function CardShowcase({ view, queue }: CardShowcaseProps): ReactElement {
  const lookup = useContext(CatalogContext);
  const [showing, setShowing] = useState<Showing | null>(null);
  const [chaos, setChaos] = useState<ChaosShowing | null>(null);
  const [chaosSaid, setChaosSaid] = useState("");
  const current = useRef<Showing | null>(null);
  const waiting = useRef<ShowcasePlay[]>([]);
  /** Per viewer, the last event window this component saw (see the header). */
  const seen = useRef(new Map<PlayerId, readonly GameEvent[]>());
  const counter = useRef(0);
  const cardRef = useRef<HTMLSpanElement | null>(null);
  const viewRef = useRef<PlayerView>(view);
  viewRef.current = view;

  const show = useCallback((next: Showing | null) => {
    current.current = next;
    setShowing(next);
  }, []);

  const advance = useCallback(() => {
    const play = waiting.current.shift();
    if (play === undefined) {
      show(null);
      return;
    }
    counter.current += 1;
    const speed = getFxSettings().speed;
    const now = viewRef.current;
    show({
      play,
      seq: counter.current,
      holdMs: play.castOnDraw === true ? showcaseCastHoldMs(speed) : showcaseHoldMs(speed),
      reduced: reducedMotionNow(),
      viewer: now.viewer,
      side: sideOf(now, play.player),
    });
  }, [show]);

  const hold = useCallback(
    (plays: readonly ShowcasePlay[]) => {
      if (plays.length === 0) return;
      waiting.current = capQueue([...waiting.current, ...plays], SHOWCASE_QUEUE_MAX);
      if (current.current === null) advance();
    },
    [advance],
  );

  const announceChaos = useCallback((roll: ChaosRoll) => {
    const names = chaosNames(roll);
    counter.current += 1;
    setChaosSaid(`${CHAOS_TEXT.said}: ${names.join(", ")}`);
    setChaos({ names, seq: counter.current, holdMs: chaosHoldMs(getFxSettings().speed), still: !effectsDrawing() });
  }, []);

  const dismiss = useCallback(() => {
    waiting.current = [];
    if (current.current !== null) show(null);
    setChaos(null);
  }, [show]);

  // What waits for the runner (R502, R436): let go as the runner starts the entry holding its event,
  // or all at once when the runner goes idle, drains or resets, or when SHOWCASE_GATE_MAX_MS runs out.
  const gated = useRef<Gated[]>([]);
  const gateTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const release = useCallback((match?: (event: GameEvent) => boolean) => {
    const go: Gated[] = [];
    const keep: Gated[] = [];
    for (const item of gated.current) (match === undefined || match(item.event) ? go : keep).push(item);
    gated.current = keep;
    if (keep.length === 0 && gateTimer.current !== null) {
      clearTimeout(gateTimer.current);
      gateTimer.current = null;
    }
    for (const item of go) item.release();
  }, []);
  const gate = useCallback(
    (event: GameEvent, then: () => void) => {
      if (queue === undefined) {
        then();
        return;
      }
      gated.current.push({ event, release: then });
      if (gateTimer.current === null) {
        gateTimer.current = setTimeout(() => {
          gateTimer.current = null;
          release();
        }, SHOWCASE_GATE_MAX_MS);
      }
    },
    [queue, release],
  );

  // A LAYOUT effect, so it listens before Game (the parent) feeds the runner this view's events.
  useLayoutEffect(() => {
    if (queue === undefined) return undefined;
    const onSignal = (signal: RunnerSignal): void => {
      if (signal.kind === "start") release((event) => signal.entry.events.some((played) => sameEvent(played, event)));
      else release();
    };
    const unsubscribe = queue.subscribeSignals(onSignal);
    return () => {
      unsubscribe();
      release();
    };
  }, [queue, release]);

  useEffect(
    () => () => {
      if (gateTimer.current !== null) clearTimeout(gateTimer.current);
    },
    [],
  );

  // A layout effect, so a play is up in the same paint as the view that brought it.
  useLayoutEffect(() => {
    const previous = seen.current.get(view.viewer);
    seen.current.set(view.viewer, view.events);
    if (view.result !== null) {
      gated.current = [];
      dismiss();
      return;
    }
    // The first view a seat is given has no "since": the board it shows has always been there.
    if (previous === undefined) return;
    const fresh = eventsSince(previous, view.events);
    const now: ShowcasePlay[] = [];
    for (const item of showcasePlays(fresh, view)) {
      // With a runner to wait on, a cast on draw waits for it; without one, every play keeps its order.
      if (item.play.castOnDraw === true && queue !== undefined) gate(item.event, () => hold([item.play]));
      else now.push(item.play);
    }
    hold(now);
    for (const { roll, event } of chaosRollsIn(fresh)) gate(event, () => announceChaos(roll));
  }, [view, queue, hold, gate, announceChaos, dismiss]);

  // The hold. A newer play keeps its own: `showing` is a new object per play.
  useEffect(() => {
    if (showing === null) return undefined;
    const timer = setTimeout(advance, showing.holdMs);
    return () => {
      clearTimeout(timer);
    };
  }, [showing, advance]);

  // The still reveal of a roll goes when its time is up.
  useEffect(() => {
    if (chaos === null) return undefined;
    const timer = setTimeout(() => setChaos(null), chaos.holdMs);
    return () => {
      clearTimeout(timer);
    };
  }, [chaos]);

  // Escape, or the viewer's first pointer down anywhere, puts it away (and whatever was waiting).
  const up = showing !== null || (chaos !== null && chaos.still);
  useEffect(() => {
    if (!up) return undefined;
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") dismiss();
    };
    const onPointerDown = (): void => {
      dismiss();
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [up, dismiss]);

  // R502: a cast on draw bursts out of its drawer's Deck pile. Measured before the first paint, so the
  // card starts on the pile; a pile or a card with no box (not laid out) leaves it where it holds.
  useLayoutEffect(() => {
    const element = cardRef.current;
    if (showing === null || showing.play.castOnDraw !== true || showing.reduced || element === null) return;
    const pile = document.querySelector(`[data-testid="library-${showing.side}"]`);
    const card = element.getBoundingClientRect();
    const box = pile?.getBoundingClientRect();
    if (box === undefined || box.width === 0 || card.width === 0 || card.height === 0) return;
    element.style.setProperty("--showcase-from-x", `${(box.left + box.width / 2 - (card.left + card.width / 2)).toFixed(1)}px`);
    element.style.setProperty("--showcase-from-y", `${(box.top + box.height / 2 - (card.top + card.height / 2)).toFixed(1)}px`);
    element.style.setProperty("--showcase-from-s", (box.height / card.height).toFixed(3));
  }, [showing]);

  const play = showing?.play ?? null;
  // The card in play (SPEC §10.10): as it stands where the view lists it — a unit's numbers, a Heroic
  // Power's rolled power, a fused card's own text — else its definition at the price that was paid.
  const face =
    play === null || play.defId === null
      ? null
      : namedFace(lookup, view, {
          defId: play.defId,
          radiant: play.radiant,
          ...(play.instanceId === undefined ? {} : { instanceId: play.instanceId }),
          ...(play.costPaid === undefined ? {} : { cost: play.costPaid }),
        });
  const kind = play === null ? null : kindOf(play);
  const said = play === null || kind === null || showing === null ? "" : saidOf(play, kind, showing.viewer, face?.name);

  // Click-through inline as well as in showcase.css, as HoverPreview is: it never takes a click aimed
  // at the board under it, whatever stylesheet has loaded.
  const style = {
    pointerEvents: "none",
    "--showcase-ms": `${String(showing?.holdMs ?? 0)}ms`,
    "--showcase-fade-ms": `${String(SHOWCASE_FADE_MS)}ms`,
    "--showcase-burst-ms": `${String(SHOWCASE_BURST_MS)}ms`,
  } as CSSProperties;

  return (
    <>
      <p className="showcase-live" data-testid={showcaseTestid.live} role="status" aria-live="polite">
        {said}
      </p>
      <p className="showcase-live" data-testid={showcaseTestid.chaosLive} role="status" aria-live="polite">
        {chaosSaid}
      </p>
      {chaos !== null && chaos.still ? <ChaosBanner key={chaos.seq} names={chaos.names} seq={chaos.seq} /> : null}
      {showing !== null && kind !== null
        ? createPortal(
            <div
              key={showing.seq}
              className="showcase"
              data-testid={showcaseTestid.root}
              data-showcase={kind}
              data-showcase-def={face?.defId}
              data-seq={showing.seq}
              data-motion={showing.reduced ? "reduce" : "full"}
              aria-hidden="true"
              style={style}
            >
              <span className="showcase-caption" data-testid={showcaseTestid.caption}>
                {captionOf(showing.play, kind, showing.viewer)}
              </span>
              {face !== null ? (
                <span ref={cardRef} className="showcase-card" data-testid={showcaseTestid.face}>
                  <CardFace face={face} layout="full" />
                  {kind === "cast" ? <CastRibbon /> : null}
                </span>
              ) : (
                <span ref={cardRef} className="showcase-card showcase-card--back" data-testid={showcaseTestid.back}>
                  <CardBack />
                  {/* R370: a card set face down shows the cost its back shows on the board. */}
                  {showing.play.cost === undefined ? null : (
                    <span
                      className="facedown-cost facedown-cost--large"
                      data-testid={showcaseTestid.cost}
                      data-cost={showing.play.cost}
                      title={costPhrase(showing.play.cost)}
                    >
                      {showing.play.cost}
                    </span>
                  )}
                  {kind === "cast" ? <CastRibbon /> : null}
                </span>
              )}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}

/** R502: the sash across a card cast as it was drawn. */
function CastRibbon(): ReactElement {
  return (
    <span className="showcase-ribbon" data-testid={showcaseTestid.ribbon}>
      {CAST_ON_DRAW_TEXT.ribbon}
    </span>
  );
}
