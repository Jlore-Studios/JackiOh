// The match clock (BUILD M7-T1). It decides nothing: R79 fixes the behaviour and the server's match
// actor runs it, so a paused turn clock is a `turnDeadline` of `null` in the frame the server pushed,
// not an inference drawn here (CLAUDE.md rule 7).
//
// The mulligan is both seats' at once (R265) on one clock (R268): while `mulligan` is set both sides show
// the frame's one `promptDeadline` over `MULLIGAN_CLOCK_MS`, and there is no turn clock (§2.1).
//
// Every length comes from the server's config (`TICK_MS` below is only a repaint cadence). Time is
// monotonic: each frame is anchored to `performance.now()` on arrival and every repaint adds the delta
// to the server's `now`, as `protocol.rs`'s `clock` message says, never `Date.now()`.
//
// The last 30 seconds of a turn (R439): the running side's line turns urgent (`data-clock-urgency`,
// `data-clock-side`) and the viewer's own turn adds an ember fuse (`TurnFuse`). Only a running turn clock
// has a final stretch (`clockUrgency.ts`); reduced motion keeps the static readout, no fuse.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { useTurnClockAlarm } from "../audio/index.ts";

import {
  DISCONNECT_GRACE_MS,
  MATCH_CEILING_MS,
  MULLIGAN_CLOCK_MS,
  PROMPT_CLOCK_MS,
  TURN_CLOCK_MS,
} from "@jackioh/server-config";
import { prefersReducedMotion } from "./animations.ts";
import {
  finalFraction,
  fuseGeometry,
  readUrgency,
  secondsLeft,
  urgencyAccessibleName,
  urgencyLabel,
  type TurnClockUrgency,
  type TurnClockUrgencyLevel,
} from "./clockUrgency.ts";
import { useSetting } from "../settings/index.ts";
import "./clock.css";

/** How often the readout repaints. Not a SPEC value; see the header. */
const TICK_MS = 200;

export type Seat = "p1" | "p2";

/** Viewer-relative, as everywhere else in the client (`game/contract.ts`). */
export type ClockSide = "you" | "opponent";

/** `crates/server/src/db/store.rs` `MatchClocks`, restated structurally. */
export type MatchClocksView = {
  /** Epoch ms the active player's turn clock expires, or null while it is paused. */
  turnDeadline: number | null;
  /** Epoch ms the open prompt's own clock expires (R79), or null. */
  promptDeadline: number | null;
  graceDeadline: { p1: number | null; p2: number | null };
  /** Epoch ms the hard ceiling is reached (R79). */
  ceilingAt: number;
};

/** The `clock` WebSocket frame (`crates/server/src/actor/protocol.rs` `ClockMessage`). */
export type ClockFrame = { now: number; clocks: MatchClocksView };

export type ClockProps = {
  /** `PlayerView.clockMs` for the viewer's own side, or null when no clock is running. */
  youMs: number | null;
  /** `PlayerView.clockMs` for the opponent. */
  opponentMs: number | null;
  /** Per-player disconnect grace remaining (§9.5), or null when nobody is away. */
  graceMs?: { you: number | null; opponent: number | null };
  /**
   * The last `clock` frame: its absolute deadlines let the readout tick between pushes. Without one
   * (a hotseat game runs no clock) `youMs`/`opponentMs` answer.
   */
  frame?: ClockFrame | null;
  /** The viewer's seat, to map the frame's `p1`/`p2` grace onto `you`/`opponent`. */
  viewer?: Seat;
  /** R79: the seat the turn clock belongs to. */
  activePlayer?: Seat | null;
  /** R79: the seat holding the open prompt, when one is open. */
  promptHolder?: Seat | null;
  /**
   * R265, R268: both mulligans are open (the view carries `view.mulligan`). The frame's
   * `promptDeadline` is then the one mulligan deadline, and `youMs` / `opponentMs` both carry it.
   */
  mulligan?: boolean;
  /** Monotonic source; `performance.now` in a browser, injected in tests. */
  monotonic?: () => number;
};

export type ClockKind = "turn" | "prompt" | "mulligan" | "idle";

export type ClockLine = {
  /** null when the frame arms a clock nobody has been named the holder of. */
  side: ClockSide | null;
  kind: ClockKind;
  remainingMs: number | null;
  /** The clock's full length, for a bar: R79's own duration for this kind. */
  totalMs: number;
  /** R79: the turn clock is stopped while the non-active player answers a prompt. */
  paused: boolean;
};

export type ClockReadout = {
  you: ClockLine;
  opponent: ClockLine;
  /** The active player's turn clock, when the frame or the props name one. */
  turn: ClockLine | null;
  /** The open prompt's own clock (R79), when the frame arms one. */
  prompt: ClockLine | null;
  grace: { you: number | null; opponent: number | null };
  /** Remaining wall clock before `ceilingReached` (R79), or null without a frame. */
  ceilingMs: number | null;
};

function sideOf(seat: Seat, viewer: Seat): ClockSide {
  return seat === viewer ? "you" : "opponent";
}

function seatOf(side: ClockSide, viewer: Seat): Seat {
  const other: Seat = viewer === "p1" ? "p2" : "p1";
  return side === "you" ? viewer : other;
}

/**
 * The whole readout, as a function of the frame and the monotonic time since it arrived. Pure, so
 * R79's pause is testable without a timer: `elapsedMs` is the only thing that moves.
 */
export function readClock(props: ClockProps, elapsedMs: number): ClockReadout {
  const viewer: Seat = props.viewer ?? "p1";
  const frame = props.frame ?? null;
  const clocks = frame?.clocks ?? null;
  const serverNow = frame === null ? null : frame.now + elapsedMs;
  const mulligan = props.mulligan === true;
  /** R268: the one mulligan deadline, when a frame reports it. */
  const mulliganMs =
    mulligan && clocks !== null && serverNow !== null && clocks.promptDeadline !== null
      ? clocks.promptDeadline - serverNow
      : null;

  const lineFor = (side: ClockSide): ClockLine => {
    const seat = seatOf(side, viewer);
    const fallback = side === "you" ? props.youMs : props.opponentMs;
    const isActive = props.activePlayer === seat;
    const holdsPrompt = props.promptHolder === seat;

    // R268: one clock for both seats, answered or not.
    if (mulligan) {
      return { side, kind: "mulligan", remainingMs: mulliganMs ?? fallback, totalMs: MULLIGAN_CLOCK_MS, paused: false };
    }

    if (clocks !== null && serverNow !== null) {
      if (holdsPrompt && clocks.promptDeadline !== null) {
        return {
          side,
          kind: "prompt",
          remainingMs: clocks.promptDeadline - serverNow,
          totalMs: PROMPT_CLOCK_MS,
          paused: false,
        };
      }
      if (isActive) {
        if (clocks.turnDeadline !== null) {
          return {
            side,
            kind: "turn",
            remainingMs: clocks.turnDeadline - serverNow,
            totalMs: TURN_CLOCK_MS,
            paused: false,
          };
        }
        // R79, as the server reports it: the turn clock has no deadline right now. What is left on
        // it is whatever the last view carried, and it is not counting down.
        return { side, kind: "turn", remainingMs: fallback, totalMs: TURN_CLOCK_MS, paused: true };
      }
    }

    const kind: ClockKind = isActive ? "turn" : holdsPrompt ? "prompt" : "idle";
    return {
      side,
      kind,
      remainingMs: fallback,
      totalMs: kind === "prompt" ? PROMPT_CLOCK_MS : TURN_CLOCK_MS,
      paused: false,
    };
  };

  const you = lineFor("you");
  const opponent = lineFor("opponent");

  const active = props.activePlayer ?? null;
  // §2.1: setup is nobody's turn, so the mulligan window shows no turn clock (R268).
  const turn: ClockLine | null = mulligan
    ? null
    : active === null
      ? clocks === null || clocks.turnDeadline === null || serverNow === null
        ? null
        : {
            side: null,
            kind: "turn",
            remainingMs: clocks.turnDeadline - serverNow,
            totalMs: TURN_CLOCK_MS,
            paused: false,
          }
      : sideOf(active, viewer) === "you"
        ? you
        : opponent;

  let prompt: ClockLine | null = null;
  if (mulligan) {
    prompt =
      mulliganMs === null
        ? null
        : { side: null, kind: "mulligan", remainingMs: mulliganMs, totalMs: MULLIGAN_CLOCK_MS, paused: false };
  } else if (clocks !== null && serverNow !== null && clocks.promptDeadline !== null) {
    const holder = props.promptHolder ?? null;
    prompt =
      holder === null
        ? {
            side: null,
            kind: "prompt",
            remainingMs: clocks.promptDeadline - serverNow,
            totalMs: PROMPT_CLOCK_MS,
            paused: false,
          }
        : sideOf(holder, viewer) === "you"
          ? you
          : opponent;
  }

  const graceFrom = (side: ClockSide): number | null => {
    if (clocks !== null && serverNow !== null) {
      const deadline = clocks.graceDeadline[seatOf(side, viewer)];
      if (deadline !== null) return deadline - serverNow;
      return null;
    }
    const held = props.graceMs;
    if (held === undefined) return null;
    return side === "you" ? held.you : held.opponent;
  };

  return {
    you,
    opponent,
    turn,
    prompt,
    grace: { you: graceFrom("you"), opponent: graceFrom("opponent") },
    ceilingMs:
      clocks === null || serverNow === null || clocks.ceilingAt === 0
        ? null
        : clocks.ceilingAt - serverNow,
  };
}

/** Whole seconds remaining, never negative. `null` reads as an em dash, not as zero. */
export function formatClock(remainingMs: number | null): string {
  if (remainingMs === null) return "—";
  return `${String(Math.max(0, Math.ceil(remainingMs / 1000)))}s`;
}

function attributes(line: ClockLine): Record<string, string> {
  return {
    "data-kind": line.kind,
    "data-paused": line.paused ? "true" : "false",
    "data-remaining-ms": line.remainingMs === null ? "" : String(Math.max(0, Math.round(line.remainingMs))),
    "data-total-ms": String(line.totalMs),
  };
}

const SIDE_LABEL: Readonly<Record<ClockSide, string>> = { you: "You", opponent: "Opponent" };

/** The gauge round the hourglass (R439): a ring that empties with the final stretch. Decoration. */
function UrgencyGauge({ fraction }: { fraction: number }): ReactElement {
  return (
    <svg className="clock-gauge" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <circle className="clock-gauge-track" cx="12" cy="12" r="10" pathLength={1} />
      <circle
        className="clock-gauge-fill"
        cx="12"
        cy="12"
        r="10"
        pathLength={1}
        strokeDasharray={`${String(fraction)} 1`}
        transform="rotate(-90 12 12)"
      />
      <path className="clock-gauge-glass" d="M8 6.5h8M8 17.5h8M9 6.5c0 3 6 3.5 6 5.5s-6 2.5-6 5.5M15 6.5c0 3-6 3.5-6 5.5s6 2.5 6 5.5" />
    </svg>
  );
}

/**
 * One side's line: the side's name and the seconds, or in the turn clock's final stretch (R439) the
 * urgent readout, its digits keyed on the second so each one pops in.
 */
function SideLine({
  side,
  line,
  urgency,
}: {
  side: ClockSide;
  line: ClockLine;
  urgency: TurnClockUrgency;
}): ReactElement {
  const urgent = urgency.side === side && urgency.level !== "none" ? urgency.level : "none";
  const text = formatClock(line.remainingMs);
  if (urgent === "none" || line.remainingMs === null) {
    return (
      <span className="clock-line clock-side" data-testid={`clock-${side}`} data-urgency="none" {...attributes(line)}>
        <span className="clock-side-label">{SIDE_LABEL[side]}</span>
        <span className="clock-digits">{text}</span>
      </span>
    );
  }
  const seconds = secondsLeft(line.remainingMs);
  return (
    <span
      className="clock-line clock-side"
      data-testid={`clock-${side}`}
      data-urgency={urgent}
      role="timer"
      aria-label={urgencyAccessibleName(side, seconds)}
      {...attributes(line)}
    >
      <UrgencyGauge fraction={finalFraction(line.remainingMs)} />
      <span className="clock-side-label">{urgencyLabel(side)}</span>
      <span key={seconds} className="clock-digits">
        {text}
      </span>
    </span>
  );
}

/**
 * R439: the ember fuse, on the viewer's own turn clock only and never under reduced motion. It burns
 * clockwise from the top-left corner and is gone at the deadline. Fixed, blind to the pointer.
 */
function TurnFuse({ remainingMs, level }: { remainingMs: number; level: TurnClockUrgencyLevel }): ReactElement {
  const { edges, spark } = fuseGeometry(finalFraction(remainingMs));
  const edge = (name: string, lit: number): ReactElement => (
    <span className={`clock-fuse-edge clock-fuse-edge--${name}`} style={{ "--lit": lit } as CSSProperties} />
  );
  return (
    <div className="clock-fuse" data-testid="turn-clock-fuse" data-urgency={level} aria-hidden="true">
      {edge("top", edges[0])}
      {edge("right", edges[1])}
      {edge("bottom", edges[2])}
      {edge("left", edges[3])}
      <span className="clock-fuse-spark" style={{ left: `${String(spark.x)}%`, top: `${String(spark.y)}%` }} />
    </div>
  );
}

/**
 * The latest `clock` frame, or null while it belongs to an earlier turn than the view beside it. The
 * server pushes views then clocks, so for a moment the old turn's frame sits beside the new turn's view
 * and would count down on it (and sound R439's alarm). A frame is handed on only while the view still
 * names the turn (`turnKey`) it arrived in.
 */
export function useFrameFor<F>(frame: F | null, turnKey: string | null): F | null {
  const paired = useRef<{ frame: F; turnKey: string | null } | null>(null);
  if (frame === null) paired.current = null;
  else if (paired.current === null || paired.current.frame !== frame) paired.current = { frame, turnKey };
  return paired.current !== null && paired.current.turnKey === turnKey ? frame : null;
}

/** The turn a view is on, for `useFrameFor`: whose it is and its number; null once the game is over. */
export function turnKeyOf(view: { active: Seat; turn: number; result: unknown } | null): string | null {
  if (view === null || view.result !== null) return null;
  return `${view.active}:${String(view.turn)}`;
}

/** R439: the media query or the settings panel's "Reduce motion": nothing of the urgency moves. */
function useReducedMotion(): boolean {
  const panel = useSetting("reduceMotion");
  return panel || prefersReducedMotion();
}

export default function Clock(props: ClockProps) {
  const monotonic = props.monotonic ?? defaultMonotonic;
  const frame = props.frame ?? null;
  const reduced = useReducedMotion();

  // Anchored during render so the first paint is right; a ref, so it schedules nothing.
  const anchor = useRef<{ frame: ClockFrame; at: number } | null>(null);
  if (frame === null) anchor.current = null;
  else if (anchor.current === null || anchor.current.frame !== frame) {
    anchor.current = { frame, at: monotonic() };
  }

  const [, setTick] = useState(0);
  useEffect(() => {
    if (frame === null) return;
    const id = setInterval(() => {
      setTick((tick) => tick + 1);
    }, TICK_MS);
    return () => {
      clearInterval(id);
    };
  }, [frame]);

  const elapsed = anchor.current === null ? 0 : Math.max(0, monotonic() - anchor.current.at);
  const readout = readClock(props, elapsed);
  // R439: the turn clock's final stretch, recomputed on every repaint.
  const urgency = readUrgency(readout);
  const turnRemaining = readout.turn?.remainingMs ?? null;
  // R439, R506: the viewer's own running turn clock beats in its last 30 seconds (audio/clockAlarm.ts).
  useTurnClockAlarm(turnRemaining, readout.turn !== null && readout.turn.side === "you" && !readout.turn.paused);
  const fuseMs = !reduced && urgency.side === "you" && urgency.level !== "none" ? turnRemaining : null;

  return (
    <div
      className="clock"
      data-clock-urgency={urgency.level}
      data-clock-side={urgency.side ?? ""}
      data-motion={reduced ? "reduced" : "full"}
    >
      <SideLine side="you" line={readout.you} urgency={urgency} />
      <SideLine side="opponent" line={readout.opponent} urgency={urgency} />

      {/* The turn and prompt lines repeat a side's line (the active player's, the prompt holder's)
          and the ceiling is hours off, so clock.css keeps these three for tools and tests only. */}
      {readout.turn === null ? null : (
        <span
          className="clock-line clock-line--aux"
          data-testid="turn-clock"
          data-side={readout.turn.side ?? ""}
          data-urgency={urgency.level}
          {...attributes(readout.turn)}
        >
          {formatClock(readout.turn.remainingMs)}
        </span>
      )}

      {readout.prompt === null ? null : (
        <span
          className="clock-line clock-line--aux"
          data-testid="prompt-clock"
          data-side={readout.prompt.side ?? ""}
          {...attributes(readout.prompt)}
        >
          {formatClock(readout.prompt.remainingMs)}
        </span>
      )}

      {readout.grace.you === null ? null : (
        <span
          className="clock-line"
          data-testid="grace-you"
          data-remaining-ms={String(Math.max(0, Math.round(readout.grace.you)))}
          data-total-ms={String(DISCONNECT_GRACE_MS)}
        >
          {formatClock(readout.grace.you)}
        </span>
      )}

      {readout.grace.opponent === null ? null : (
        <span
          className="clock-line"
          data-testid="grace-opponent"
          data-remaining-ms={String(Math.max(0, Math.round(readout.grace.opponent)))}
          data-total-ms={String(DISCONNECT_GRACE_MS)}
        >
          {formatClock(readout.grace.opponent)}
        </span>
      )}

      {readout.ceilingMs === null ? null : (
        <span
          className="clock-line clock-line--aux"
          data-testid="match-ceiling"
          data-remaining-ms={String(Math.max(0, Math.round(readout.ceilingMs)))}
          data-total-ms={String(MATCH_CEILING_MS)}
        >
          {formatClock(readout.ceilingMs)}
        </span>
      )}

      {fuseMs === null ? null : <TurnFuse remainingMs={fuseMs} level={urgency.level} />}
    </div>
  );
}

function defaultMonotonic(): number {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return 0;
}
