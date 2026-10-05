// The opponent's aim (SPEC §9.5, R661): what a player is aiming a play, an Activate or an attack
// at, as the opponent's board draws it — Hearthstone's targeting arrow, shown to both players.
//
// Cosmetic, like an emote (R643): an aim is never an `ActionBody`, never reaches `reduce`, the
// action log, the replay hash or a game record, is never part of `PlayerView`, and no rule is ever
// decided from it (CLAUDE.md rule 7). This module holds only the shape both ends of the wire agree
// on, and its shape check, because `apps/web` and `apps/server` may not import each other.
//
// Every end is a public handle, so nothing hidden can ride on it (R97, R177):
//  - a hero, by its seat;
//  - a zone of the field, by seat, row and lane — a face-down backrow card is named only by the
//    zone it lies in, and a card on the field is never named by its instance id;
//  - a card in a hand, by its position in that hand, which the opponent's view draws as the card
//    back at that position — never its instance id, its definition or anything on its face.

import type { PlayerId, Row } from "./catalog-types.ts";

export type AimEnd =
  | { at: "hero"; player: PlayerId }
  | { at: "zone"; player: PlayerId; row: Row; lane: number }
  | { at: "hand"; player: PlayerId; index: number };

/**
 * One aim: where it starts and what it is over now. `target` is null while the aim is over
 * nothing it may land on, so the opponent sees what is being aimed with and not yet at what.
 */
export type Aim = { source: AimEnd; target: AimEnd | null };

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isPlayerId = (value: unknown): value is PlayerId => value === "p1" || value === "p2";

const isRow = (value: unknown): value is Row => value === "units" || value === "backrow";

const isIndex = (value: unknown, least: number): value is number =>
  typeof value === "number" && Number.isInteger(value) && value >= least;

/**
 * One end, rebuilt field by field so nothing else on the wire survives (an instance id, a def id),
 * or null when it is not one of the three handles. Lanes count from 1, as the board's do.
 */
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

/**
 * An aim, `null` (the aim has ended), or `undefined` when the value is neither — the shape check
 * the server parses a client's frame with and the client parses a relay with. A hand is never a
 * target: no play, Activate or attack aims at a hand card through the arrow.
 */
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

/** A stable key for an aim, so two equal aims compare equal (the sender sends only changes). */
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
