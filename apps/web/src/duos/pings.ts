// The five pings (#936 B64, S6): what each looks like on the wheel and on its marker, what it says,
// and the shape check a relayed `ping` frame passes before the board shows it. A ping is cosmetic:
// it never reaches the engine, a view or the log (B39), so nothing here is a rule.

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import { LANES } from "../game/contract.ts";
import { PING_KINDS, type PingKind, type PingTarget } from "../net/duos.ts";

/** The glyph a ping wears on the wheel and on its marker. */
export const PING_GLYPH: Readonly<Record<PingKind, string>> = {
  thumbsUp: "\u{1F44D}",
  thumbsDown: "\u{1F44E}",
  portal: "\u{1F300}",
  careful: "⚠️",
  killThis: "\u{1F3AF}",
};

/** What a ping says, in English for `t()` (never called at module scope). */
export const PING_LABEL: Readonly<Record<PingKind, string>> = {
  thumbsUp: "Good idea",
  thumbsDown: "Bad idea",
  portal: "Look at my board",
  careful: "Careful",
  killThis: "Kill this",
};

const SEATS: readonly string[] = ["a1", "a2", "b1", "b2"];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isWholeIn(value: unknown, below: number): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value < below;
}

export function isPingKind(value: unknown): value is PingKind {
  return typeof value === "string" && (PING_KINDS as readonly string[]).includes(value);
}

export function isDuosSeat(value: unknown): value is DuosSeat {
  return typeof value === "string" && SEATS.includes(value);
}

/**
 * A relayed target, or null when it is not one of S6's three shapes. Only the shape: a hand index
 * past the hand is the server's to drop (B39), and the board simply finds no card to mark.
 */
export function parsePingTarget(value: unknown): PingTarget | null {
  if (!isRecord(value)) return null;
  const { duel, at, side, lane, index } = value;
  if (duel !== 0 && duel !== 1) return null;
  if (side !== "p1" && side !== "p2") return null;
  // S6's `lane: 0..4`: 0-based on the wire, where the board's zone testids count 1..5.
  if (at === "hero") return { duel, at: "hero", side };
  // Both rows have a slot per lane (`LANES`); a ping names one 0-based.
  if (at === "unit") return isWholeIn(lane, LANES.length) ? { duel, at: "unit", side, lane } : null;
  if (at === "backrow") return isWholeIn(lane, LANES.length) ? { duel, at: "backrow", side, lane } : null;
  if (at === "hand") return isWholeIn(index, Number.MAX_SAFE_INTEGER) ? { duel, at: "hand", side, index } : null;
  return null;
}
