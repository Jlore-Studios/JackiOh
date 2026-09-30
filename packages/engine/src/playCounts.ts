// What every play leaves behind to be counted (docs/classic-sets.md B5 E4, R451).
//
// §10.5 step 4 counts a play the moment it places the card: `turnLog.playedIds`, `cardsPlayed`,
// R213's `costsPaid` and the game's `played` counter (R55), and now these too —
//   * per player, per turn (on both players' turns): plays by the type each was played as (B2.7),
//     in the turn log, which `startTurn` rebuilds for both players (Classic+ #37 Wardrum);
//   * per player, per game: plays by tag in the player's `gameLog`, never reset (Classic+ #64 Mulch
//     Muncher's Fruit, AI Scaling Law's AI generated cards);
//   * game-wide: the last Spell anyone played (Classic #57 Echo, `state.lastSpell`), and per player
//     the last face-up card they played (AI Autocomplete, `gameLog.lastFaceUpPlay`). Each "last"
//     record is overwritten by the next play and never cleared.
// A cast is a play and counts (R70). A countered play never reaches step 4, so it counts for nothing
// (R448). An Echo repeat is the same play resolving again, not a play (§6.2).
//
// The readers are `query.ts`'s (the read half of the card-facing surface); this module only writes.

import type { PlayerId } from "@jackioh/shared";
import { defOf, fusedIdParts } from "./catalog";
import { LAST_FACE_UP_SKIPPED_TAGS } from "./config";
import { cardTypeOf } from "./faces";
import { scriptOf } from "./scripts";
import type { CardInstance, GameLog, GameState, PlayRecord } from "./state";

/**
 * R451: what a play records as the card played — the card itself, or what its `recordsPlayAs` hook
 * names (Classic #57 Echo records the Spell it copied; null records nothing). A fused card records
 * itself: its combined text is its ingredients' (R102), and no ingredient names another card for it.
 */
export function playRecordOf(state: GameState, card: CardInstance): PlayRecord | null {
  const own: PlayRecord = { defId: card.defId, radiant: card.radiant };
  if (fusedIdParts(card.defId) !== null) return own;
  const hook = scriptOf(card).recordsPlayAs;
  if (hook === undefined) return own;
  const named = hook({ state, self: card, radiant: card.radiant });
  if (named === null) return null;
  return { defId: named.defId, radiant: named.radiant === true };
}

/** A Trap or Field Trap is set face-down (§3.2, R33), so its play is never a face-up one (R451). */
function playedFaceDown(type: string): boolean {
  return type === "Trap" || type === "Field Trap";
}

/**
 * §10.5 step 4, B5 E4: count one play of `card` by `player`, read as the card is placed (its face and
 * type as they stand after step 3 made it Radiant, if it did).
 */
export function recordPlay(state: GameState, player: PlayerId, card: CardInstance): void {
  const side = state.players[player];
  const type = cardTypeOf(state, card);
  const def = defOf(state, card.defId);

  const types = { ...(side.turnLog.playedByType ?? {}) };
  types[type] = (types[type] ?? 0) + 1;
  side.turnLog.playedByType = types;

  const tags = { ...(side.gameLog?.playedByTag ?? {}) };
  for (const tag of new Set(def.tags)) tags[tag] = (tags[tag] ?? 0) + 1;
  const log: GameLog = { ...side.gameLog, playedByTag: tags };
  side.gameLog = log;

  const record = playRecordOf(state, card);
  if (record === null) return;
  if (type === "Spell") state.lastSpell = record;
  // R451: the last face-up card, passing over Traps (set face-down, so nothing hidden is ever
  // recorded) and the tags `LAST_FACE_UP_SKIPPED_TAGS` names (the AI generated cards).
  if (playedFaceDown(type) || def.tags.some((tag) => LAST_FACE_UP_SKIPPED_TAGS.includes(tag))) return;
  log.lastFaceUpPlay = { ...record, type };
}
