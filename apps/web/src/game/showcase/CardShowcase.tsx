// A visual-only, redacted-event display (CLAUDE.md rule 7; R97, R202, R227), held long enough to read (R201).
// R502: casts on draw (Hinder #21, Blood Ridden #27) appear on both seats, gated to the runner without pacing it.
// A card another card casts (C+ #47 Jogg's Box) replaces the current display on its runner entry; per-viewer
// `runs.ts` tracking identifies those casts. R436 announces Call to Chaos and shows it still when effects do not draw.
// Per-viewer windows let a hotseat player catch up; the portal is click-through and holds practice's AI via `data-showcase`.
// Reduced motion preserves the information without animation.

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
import { createPlayTracker, type PlayTracker } from "../runs.ts";
import ChaosBanner from "./ChaosBanner.tsx";
import {
  CAST_ON_DRAW_TEXT,
  CHAOS_TEXT,
  MULTICAST_TEXT,
  SHOWCASE_BURST_MS,
  SHOWCASE_FADE_MS,
  SHOWCASE_GATE_MAX_MS,
  SHOWCASE_QUEUE_MAX,
  showcaseCastHoldMs,
  showcaseHoldMs,
  showcaseMulticastHoldMs,
  showcaseTestid,
  type ShowcaseKind,
} from "./constants.ts";
import { capQueue, chaosRollsIn, eventsSince, showcasePlays, type ShowcasePlay } from "./plan.ts";
import "./showcase.css";

export type CardShowcaseProps = {
  view: PlayerView;
  /** Runner for R502 and R436 entries; absent runners present immediately. */
  queue?: AnimationQueue;
};

type Showing = { play: ShowcasePlay; seq: number; holdMs: number; reduced: boolean; viewer: PlayerId; side: Side };

type ChaosShowing = { names: string[]; seq: number; holdMs: number; still: boolean };

function kindOf(play: ShowcasePlay): ShowcaseKind {
  if (play.castOnDraw === true) return "cast";
  if (play.castBy !== undefined) return "multicast";
  if (play.defId !== null) return "played";
  return play.set ? "set" : "hidden";
}

const CAPTION: Readonly<Record<Exclude<ShowcaseKind, "cast" | "multicast">, string>> = {
  played: "Opponent played",
  set: "Opponent set a card",
  hidden: "Opponent played a card",
};

function castCaption(play: ShowcasePlay, viewer: PlayerId): string {
  if (play.player === viewer) return CAST_ON_DRAW_TEXT.you;
  return play.defId === null ? CAST_ON_DRAW_TEXT.hidden : CAST_ON_DRAW_TEXT.opponent;
}

function multicastCaption(byName: string | undefined): string {
  return byName === undefined ? MULTICAST_TEXT.unknown : `${MULTICAST_TEXT.by} ${byName}`;
}

function captionOf(play: ShowcasePlay, kind: ShowcaseKind, viewer: PlayerId, byName: string | undefined): string {
  if (kind === "cast") return castCaption(play, viewer);
  return kind === "multicast" ? multicastCaption(byName) : CAPTION[kind];
}

function saidOf(play: ShowcasePlay, kind: ShowcaseKind, viewer: PlayerId, name: string | undefined, byName: string | undefined): string {
  if (kind === "multicast") {
    const ordinal = play.castBy?.ordinal ?? 1;
    return `${byName ?? "A card"} ${MULTICAST_TEXT.said} ${name ?? "a card"} (${String(ordinal)})`;
  }
  if (kind === "cast") {
    const drew = castCaption(play, viewer);
    return play.defId === null ? `${drew}: ${CAST_ON_DRAW_TEXT.said}` : `${drew} ${name ?? "a card"}: ${CAST_ON_DRAW_TEXT.said}`;
  }
  return kind === "played" ? `${CAPTION.played} ${name ?? "a card"}` : CAPTION[kind];
}

function effectsDrawing(): boolean {
  const settings = getFxSettings();
  return !reducedMotionNow(settings) && settings.intensity !== "off";
}

export function chaosHoldMs(speed: number): number {
  return Math.round(ANIMATIONS.chaosRolled.durationMs / normalizeSpeed(speed)) + FX_BANNER_TAIL_MS;
}

/** `reached` distinguishes runner-started entries from other gate releases. */
type Gated = { event: GameEvent; release: (reached: boolean) => void };

function holdFor(play: ShowcasePlay, speed: number): number {
  if (play.castOnDraw === true) return showcaseCastHoldMs(speed);
  return play.castBy !== undefined ? showcaseMulticastHoldMs(speed) : showcaseHoldMs(speed);
}

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
  const seen = useRef(new Map<PlayerId, readonly GameEvent[]>());
  const trackers = useRef(new Map<PlayerId, PlayTracker>());
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
      holdMs: holdFor(play, speed),
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

  /** Runner-reached casts replace the current display. */
  const replace = useCallback(
    (play: ShowcasePlay) => {
      waiting.current = [play, ...waiting.current];
      advance();
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

  // R502 and R436 wait for their runner entry; restarting the gate on each entry prevents long cast bursts timing out.
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
    for (const item of go) item.release(match !== undefined);
  }, []);
  const refreshGate = useCallback(() => {
    if (gated.current.length === 0) return;
    if (gateTimer.current !== null) clearTimeout(gateTimer.current);
    gateTimer.current = setTimeout(() => {
      gateTimer.current = null;
      release();
    }, SHOWCASE_GATE_MAX_MS);
  }, [release]);
  const gate = useCallback(
    (event: GameEvent, then: (reached: boolean) => void) => {
      if (queue === undefined) {
        then(false);
        return;
      }
      gated.current.push({ event, release: then });
      refreshGate();
    },
    [queue, refreshGate],
  );

  // Subscribe before Game feeds this view's runner events.
  useLayoutEffect(() => {
    if (queue === undefined) return undefined;
    const onSignal = (signal: RunnerSignal): void => {
      if (signal.kind === "start") {
        release((event) => signal.entry.events.some((played) => sameEvent(played, event)));
        refreshGate();
      } else release();
    };
    const unsubscribe = queue.subscribeSignals(onSignal);
    return () => {
      unsubscribe();
      release();
    };
  }, [queue, release, refreshGate]);

  useEffect(
    () => () => {
      if (gateTimer.current !== null) clearTimeout(gateTimer.current);
    },
    [],
  );

  // Show a play in the same paint as its view.
  useLayoutEffect(() => {
    const previous = seen.current.get(view.viewer);
    seen.current.set(view.viewer, view.events);
    let tracker = trackers.current.get(view.viewer);
    if (tracker === undefined) {
      tracker = createPlayTracker();
      trackers.current.set(view.viewer, tracker);
    }
    if (view.result !== null) {
      gated.current = [];
      dismiss();
      return;
    }
    // Seed the tracker from the first window so its next casts are identified.
    if (previous === undefined) {
      for (const event of view.events) tracker.see(event);
      return;
    }
    const fresh = eventsSince(previous, view.events);
    const now: ShowcasePlay[] = [];
    for (const item of showcasePlays(fresh, view, tracker)) {
      if (queue !== undefined && item.play.castOnDraw === true) gate(item.event, () => hold([item.play]));
      else if (queue !== undefined && item.play.castBy !== undefined) {
        const play = item.play;
        gate(item.event, (reached) => (reached ? replace(play) : hold([play])));
      } else now.push(item.play);
    }
    hold(now);
    for (const { roll, event } of chaosRollsIn(fresh)) gate(event, () => announceChaos(roll));
  }, [view, queue, hold, replace, gate, announceChaos, dismiss]);

  useEffect(() => {
    if (showing === null) return undefined;
    const timer = setTimeout(advance, showing.holdMs);
    return () => {
      clearTimeout(timer);
    };
  }, [showing, advance]);

  useEffect(() => {
    if (chaos === null) return undefined;
    const timer = setTimeout(() => setChaos(null), chaos.holdMs);
    return () => {
      clearTimeout(timer);
    };
  }, [chaos]);

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

  // R502: measure before paint so casts burst from their deck pile when both boxes exist.
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
  // SPEC §10.10: use the current face, or the paid-cost definition once it has left the view.
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
  const castBy = play?.castBy;
  const byName =
    castBy === undefined || castBy.defId === null ? undefined : (lookup?.(castBy.defId, false)?.name ?? view.defs?.[castBy.defId]?.name);
  const said = play === null || kind === null || showing === null ? "" : saidOf(play, kind, showing.viewer, face?.name, byName);

  // Inline click-through keeps the board usable before its stylesheet loads.
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
                {captionOf(showing.play, kind, showing.viewer, byName)}
              </span>
              {face !== null ? (
                <span ref={cardRef} className="showcase-card" data-testid={showcaseTestid.face}>
                  <CardFace face={face} layout="full" />
                  {kind === "cast" ? <CastRibbon /> : null}
                  {castBy !== undefined ? <CastOrdinal ordinal={castBy.ordinal} /> : null}
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
                  {castBy !== undefined ? <CastOrdinal ordinal={castBy.ordinal} /> : null}
                </span>
              )}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}

function CastOrdinal({ ordinal }: { ordinal: number }): ReactElement {
  return (
    <span className="showcase-ordinal" data-testid={showcaseTestid.ordinal} data-ordinal={ordinal}>
      {`${MULTICAST_TEXT.ordinal}${String(ordinal)}`}
    </span>
  );
}

function CastRibbon(): ReactElement {
  return (
    <span className="showcase-ribbon" data-testid={showcaseTestid.ribbon}>
      {CAST_ON_DRAW_TEXT.ribbon}
    </span>
  );
}
