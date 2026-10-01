// The last 30 seconds of a turn clock (R439), as pure functions of the readout `Clock.tsx` already
// computes. Presentation only: the server runs every clock (R79), and nothing here decides or
// predicts a timeout (CLAUDE.md rule 7). A paused turn clock, a prompt's clock and the mulligan's
// clock are never urgent: the stretch belongs to the turn clock, the one the issue names ("the final
// 30s of your turn"). A prompt clock is short and already its own countdown, and the mulligan is
// nobody's turn (§2.1, R268).

import type { ClockLine, ClockReadout, ClockSide } from "./Clock.tsx";
import { FUSE_EDGES, MS_PER_SECOND, PERCENT, TURN_CLOCK_FINAL_MS, TURN_CLOCK_LAST_MS } from "./clockConstants.ts";

/** R439: none, the final 30 seconds, or the last 10 of them. */
export type TurnClockUrgencyLevel = "none" | "final" | "last10";

/**
 * R439: what the clock root carries as `data-clock-urgency` and `data-clock-side`, and what a sound
 * hook is handed: the level, whose turn clock it is, and the whole seconds left (null when none).
 */
export type TurnClockUrgency = {
  level: TurnClockUrgencyLevel;
  side: ClockSide | null;
  secondsLeft: number | null;
};

export const NO_URGENCY: TurnClockUrgency = { level: "none", side: null, secondsLeft: null };

/**
 * R439: a running turn clock at or under 30 seconds is "final", at or under 10 it is "last10".
 * Anything else — no line, a prompt or mulligan clock, a paused turn clock (the server gave it no
 * deadline, R79), no number at all, or more time than that — is "none".
 */
export function turnClockUrgency(line: ClockLine | null): TurnClockUrgencyLevel {
  if (line === null || line.kind !== "turn" || line.paused || line.remainingMs === null) return "none";
  if (line.remainingMs > TURN_CLOCK_FINAL_MS) return "none";
  return line.remainingMs > TURN_CLOCK_LAST_MS ? "final" : "last10";
}

/** Whole seconds left, as the readout prints them (rounded up, never negative). */
export function secondsLeft(remainingMs: number): number {
  return Math.max(0, Math.ceil(remainingMs / MS_PER_SECOND));
}

/**
 * R439: the readout's turn clock and whose it is. A turn clock the frame names no holder of
 * (`side: null`) has nobody to warn, so it is not urgent.
 */
export function readUrgency(readout: ClockReadout): TurnClockUrgency {
  const line = readout.turn;
  if (line === null || line.side === null) return NO_URGENCY;
  const level = turnClockUrgency(line);
  if (level === "none" || line.remainingMs === null) return NO_URGENCY;
  return { level, side: line.side, secondsLeft: secondsLeft(line.remainingMs) };
}

/** How much of the final stretch is left, 1 at its start and 0 at the deadline. */
export function finalFraction(remainingMs: number): number {
  return Math.min(1, Math.max(0, remainingMs / TURN_CLOCK_FINAL_MS));
}

/**
 * The fuse at `fraction` of the final stretch left: how much of each edge (top, right, bottom,
 * left, 0 to 1) is still unburnt, and where the burning head is, in percent of the screen. It burns
 * clockwise from the top-left corner, so an edge's unburnt part is always at its far end.
 */
export type FuseGeometry = {
  edges: readonly [number, number, number, number];
  spark: { x: number; y: number };
};

export function fuseGeometry(fraction: number): FuseGeometry {
  const left = Math.min(1, Math.max(0, fraction));
  /** How far round the burn has come, in edges: 0 at the start, FUSE_EDGES at the end. */
  const burnt = (1 - left) * FUSE_EDGES;
  const edge = (index: number): number => Math.min(1, Math.max(0, index + 1 - burnt));
  const edges = [edge(0), edge(1), edge(2), edge(3)] as const;

  const at = Math.min(FUSE_EDGES - 1, Math.floor(burnt));
  const along = (burnt - at) * PERCENT;
  const spark =
    at === 0
      ? { x: along, y: 0 }
      : at === 1
        ? { x: PERCENT, y: along }
        : at === 2
          ? { x: PERCENT - along, y: PERCENT }
          : { x: 0, y: PERCENT - along };
  return { edges, spark };
}

/** R439's readout says whose turn is running out in words, never by colour alone. */
export function urgencyLabel(side: ClockSide): string {
  return side === "you" ? "Your turn" : "Their turn";
}

/** The urgent readout's accessible name: "Your turn ends in 12 seconds". */
export function urgencyAccessibleName(side: ClockSide, seconds: number): string {
  const unit = seconds === 1 ? "second" : "seconds";
  return `${urgencyLabel(side)} ends in ${String(seconds)} ${unit}`;
}
