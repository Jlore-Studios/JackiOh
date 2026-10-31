// R439 (CLAUDE.md rule 7): presentation for turn clock's final 30s.
// Server controls clock (R79); paused, prompt, and mulligan (§2.1, R268) clocks are never urgent.

import type { ClockLine, ClockReadout, ClockSide } from "./Clock.tsx";
import { FUSE_EDGES, MS_PER_SECOND, PERCENT, TURN_CLOCK_FINAL_MS, TURN_CLOCK_LAST_MS } from "./clockConstants.ts";
import { t } from "../i18n/i18n.ts";

/** R439: none, the final 30 seconds, or the last 10 of them. */
export type TurnClockUrgencyLevel = "none" | "final" | "last10";

/** R439: data attributes and audio parameters for urgent clock state. */
export type TurnClockUrgency = {
  level: TurnClockUrgencyLevel;
  side: ClockSide | null;
  secondsLeft: number | null;
};

export const NO_URGENCY: TurnClockUrgency = { level: "none", side: null, secondsLeft: null };

/** R439: urgency level ("final" <= 30s, "last10" <= 10s, else "none"; R79). */
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

/** Fuse geometry at fraction left: edge coverage and burning head position (clockwise from top-left). */
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

/** R439's readout says whose turn is running out in words, never by color alone. */
export function urgencyLabel(side: ClockSide): string {
  return side === "you" ? "Your turn" : "Their turn";
}

/** The urgent readout's accessible name: "Your turn ends in 12 seconds". */
export function urgencyAccessibleName(side: ClockSide, seconds: number): string {
  const count = String(seconds);
  if (side === "you") {
    return seconds === 1 ? t("Your turn ends in {count} second", { count }) : t("Your turn ends in {count} seconds", { count });
  }
  return seconds === 1 ? t("Their turn ends in {count} second", { count }) : t("Their turn ends in {count} seconds", { count });
}
