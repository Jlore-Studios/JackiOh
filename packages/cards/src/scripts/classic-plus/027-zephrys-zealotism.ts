// C+ #27 Zephrys Zealotism (SPEC §8.7 row 27; R29, R364, R387, R416). (4) Spell, Mythic.
//   Base:    "Replace your hand with the perfect hand of Classic and Classic+ cards. Refresh your mana."
//   Radiant: "Replace your hand with the perfect Radiant hand of Classic and Classic+ cards. Refresh
//            your mana."
//
// THE HAND IS THE SUBSYSTEM'S. B5 E34, the perfect-hand scorer (`engine/src/subsystems/perfectHand.ts`):
// R29's Zephyrs scorer ranks every non-token Classic and Classic+ card but this one (R387) for the
// state as this resolves, each on the face it would arrive with, ties by card id; each other card in
// the caster's hand goes to their graveyard (not a discard; a unit-token card ceases to exist, R11) and
// the top N distinct cards arrive in rank order, N being how many there were (R416). This card only
// names the verb and the face, so no weight, pool or tie-break is restated here.
//
// "Refresh your mana" is §6.3 Refresh (R364): current mana rises toward max and never past it, so a
// Refresh of every crystal gives back what was spent — this card's 4 included when max is 4 — and a
// player at or above max gains nothing. It follows the hand, as the text orders them.

import type { Script } from "@jackioh/engine";
import { refreshMana, replaceHandWithPerfect } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-027");

/**
 * "Refresh your mana": every spent crystal, which R364 caps at max mana (a Refresh never takes
 * current mana above it), so this is a full refill, whatever the player's max is (§9.9's handicaps).
 */
const ALL_MANA = Number.POSITIVE_INFINITY;

/** The faces differ only in which face the scorer ranks and the new cards arrive with. */
function zealotism(radiant: boolean): Script {
  return {
    cry: () => [replaceHandWithPerfect({ radiant }), refreshMana({ amount: ALL_MANA })],
  };
}

export const base: Script = zealotism(false);

export const radiant: Script = zealotism(true);
