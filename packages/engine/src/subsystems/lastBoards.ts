// Last boards: a match setup input from outside the match (docs/classic-sets.md B5 E30; SPEC §8.7
// C+ #29 Portal to the Past, §9.3, §10.1, R417, R564).
//
// "Your last game" is a game this match never saw, so its board is an INPUT: one list per seat,
// handed to `createGame` beside the decks and handicaps and frozen into the match
// (`GameState.lastBoards`). Nothing writes it again, so the match folds exactly from
// `(seed, decks, handicaps, lastBoards, log)` (§9.3, `replay.ReplayInput.lastBoards`).
//
//  - `freezeLastBoards`: what `createGame` keeps — `{ defId, radiant }` per entry, nothing else, and
//    only entries this match can rebuild from the id alone (R564). Pure, and it reads the catalog
//    and the id's text only, so every process freezes the same input the same way.
//  - `lastBoardFor`: the reader the server calls for each seat as a game ends.
//  - `lastBoardCandidates`: what C+ #29 picks among (R564).
//
// `viewFor` never sends a last board (it copies what a view may hold and names no such field), and
// the AI's redaction keeps only its own seat's (`packages/ai/src/observe.ts`, R185).

import type { CardDefs, PlayerId } from "@jackioh/shared";
import { PLAYER_IDS } from "@jackioh/shared";
import { fusedIdSpecs, isDigestId } from "../catalog";
import { isFaceDown } from "../preview";
import type { CardInstance, GameState, LastBoardEntry, LastBoardInput } from "../state";
import { cardAt, carriedAt, slotsOf } from "../zones";

/** R77's smallest fusion, as `subsystems/fuse`'s `FUSE_MIN_INGREDIENTS` (not imported: fuse imports state). */
const FUSED_MIN_PARTS = 2;

/**
 * R564: whether this match can have the definition `defId` names, from the id alone: a catalog card,
 * or a fused id (R179) whose every ingredient, all the way down, is one. A digest id (R468) names its
 * list only to the process that minted it, so it never can.
 */
export function rebuildableFromId(defId: string, catalog: CardDefs): boolean {
  if (catalog[defId] !== undefined) return true;
  if (isDigestId(defId)) return false;
  const specs = fusedIdSpecs(defId);
  if (specs === null || specs.length < FUSED_MIN_PARTS) return false;
  return specs.every((spec) => rebuildableFromId(spec.defId, catalog));
}

function isEntry(value: unknown): value is { defId: string; radiant?: unknown } {
  return typeof value === "object" && value !== null && typeof (value as { defId?: unknown }).defId === "string";
}

/**
 * R417, R564: what `createGame` keeps of its `lastBoards` input — per seat, in order, each entry as
 * `{ defId, radiant }`, every entry this match cannot rebuild dropped. Undefined when no seat keeps one.
 */
export function freezeLastBoards(
  input: LastBoardInput | undefined,
  catalog: CardDefs,
): Partial<Record<PlayerId, LastBoardEntry[]>> | undefined {
  if (input === undefined) return undefined;
  const frozen: Partial<Record<PlayerId, LastBoardEntry[]>> = {};
  PLAYER_IDS.forEach((player, seat) => {
    const kept = ((input[seat] ?? []) as readonly unknown[])
      .filter(isEntry)
      .filter((entry) => rebuildableFromId(entry.defId, catalog))
      .map((entry): LastBoardEntry => ({ defId: entry.defId, radiant: entry.radiant === true }));
    if (kept.length > 0) frozen[player] = kept;
  });
  return Object.keys(frozen).length === 0 ? undefined : frozen;
}

function entryOf(card: CardInstance): LastBoardEntry {
  return { defId: card.defId, radiant: card.radiant };
}

/**
 * R417: the board `seat` takes away from this game — every card on the field, both sides, in board
 * order (p1's unit lanes, then p1's backrow lanes, a carried Unit before its carrier, R446, then
 * p2's), each as its card and face, except a backrow card `seat` may not read (R33: a face-down Trap
 * or Field Trap it does not control). Dormant cards are not on the field (R13).
 */
export function lastBoardFor(state: GameState, seat: PlayerId): LastBoardEntry[] {
  const board: LastBoardEntry[] = [];
  for (const player of PLAYER_IDS) {
    for (const ref of slotsOf(player, "units")) {
      const top = cardAt(state, ref);
      if (top !== null) board.push(entryOf(top));
    }
    for (const ref of slotsOf(player, "backrow")) {
      const carried = carriedAt(state, ref);
      if (carried !== null) board.push(entryOf(carried));
      const card = cardAt(state, ref);
      if (card !== null && !(isFaceDown(state, card) && card.controller !== seat)) board.push(entryOf(card));
    }
  }
  return board;
}

/**
 * R564: the different cards C+ #29 picks among on `player`'s frozen board — each definition once,
 * where it first appears, on its Radiant face if any of its entries was Radiant — minus `exclude`
 * (R387: the generating card's own definitions).
 */
export function lastBoardCandidates(state: GameState, player: PlayerId, exclude: readonly string[] = []): LastBoardEntry[] {
  const candidates: LastBoardEntry[] = [];
  for (const entry of state.lastBoards?.[player] ?? []) {
    if (exclude.includes(entry.defId)) continue;
    const held = candidates.find((candidate) => candidate.defId === entry.defId);
    if (held === undefined) candidates.push({ ...entry });
    else held.radiant ||= entry.radiant;
  }
  return candidates;
}
