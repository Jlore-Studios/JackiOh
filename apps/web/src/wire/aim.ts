// Wire aim contract (docs/v0.3.0/SURFACE.md §10.4).
//
// The opponent's aim (SPEC §9.5, R738) is a public targeting arrow shown to both players.
//
// Cosmetic only (R643; CLAUDE.md rule 7): aim never reaches actions, logs, replays, records, or
// `PlayerView`, and decides no rule. Its shared shape is checked locally because `apps/web` and
// `crates/server` cannot import each other.
//
// Every end is public (R97, R177): seat, zone, or hand position, never an instance id, definition,
// or face.

import type { PlayerId, Row } from "./catalog.ts";

export type AimEnd =
  | { at: "hero"; player: PlayerId }
  | { at: "zone"; player: PlayerId; row: Row; lane: number }
  | { at: "hand"; player: PlayerId; index: number };

/** An in-progress aim; `target` is null off a valid landing spot. */
export type Aim = { source: AimEnd; target: AimEnd | null };

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isPlayerId = (value: unknown): value is PlayerId => value === "p1" || value === "p2";

const isRow = (value: unknown): value is Row => value === "units" || value === "backrow";

const isIndex = (value: unknown, least: number): value is number =>
  typeof value === "number" && Number.isInteger(value) && value >= least;

/** Rebuild only public fields; zone lanes start at 1. */
export function parseAimEnd(value: unknown): AimEnd | null {
  if (!isRecord(value) || !isPlayerId(value.player)) return null;
  switch (value.at) {
    case "hero":
      return { at: "hero", player: value.player };
    case "zone":
      return isRow(value.row) && isIndex(value.lane, 1)
        ? { at: "zone", player: value.player, row: value.row, lane: value.lane }
        : null;
    case "hand":
      return isIndex(value.index, 0) ? { at: "hand", player: value.player, index: value.index } : null;
    default:
      return null;
  }
}

/** A finished aim is null; invalid input is undefined. A hand is never an aim target. */
export function parseAim(value: unknown): Aim | null | undefined {
  if (value === null) return null;
  if (!isRecord(value)) return undefined;
  const source = parseAimEnd(value.source);
  if (source === null) return undefined;
  if (value.target === null) return { source, target: null };
  const target = parseAimEnd(value.target);
  if (target === null || target.at === "hand") return undefined;
  return { source, target };
}

/** Stable key: send only changed aims. */
export function aimKey(aim: Aim | null): string {
  if (aim === null) return "none";
  return `${endKey(aim.source)}>${aim.target === null ? "none" : endKey(aim.target)}`;
}

function endKey(end: AimEnd): string {
  switch (end.at) {
    case "hero":
      return `hero:${end.player}`;
    case "zone":
      return `zone:${end.player}:${end.row}:${String(end.lane)}`;
    case "hand":
      return `hand:${end.player}:${String(end.index)}`;
  }
}
