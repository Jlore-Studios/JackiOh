// #21 Hinder (SPEC §8.2): "Cast on draw: Your opponent has 1 less mana next turn. Discard 1.",
// radiant "Cast on draw: Your opponent has 2 less mana next turn." — patch v0.2.0 (R431) gives the
// base face its discard and leaves the Radiant face as it was, without one.
//
// Nothing here casts the card or draws again: `staticFlags.castOnDraw` is the whole of that, and
// `drawOne` (engine/src/draw.ts) casts it, repeats the draw and stops at CAST_ON_DRAW_CHAIN_CAP
// (R58), while `castCard` makes the cast free and counts it as a card played (R40, R70).
//
// The floor is not this card's either: `nextTurnMana` moves `mana.nextTurnMod`, and `refreshMana`
// fills current mana to §2.3's max plus that one-shot rider, floored at 0, then clears it. So a −2
// against a 1-mana refresh is 0, not −1, and max mana itself is untouched (§2.3).
//
// THE DISCARD (R431, R640, R70). "Discard 1" names no "of your choice", so the discard is random
// from the caster's hand (R640) — no declaration travels in the play action (R81), and a cast, on a
// draw, which is how Hinder almost always resolves, asks nothing as it begins (R70): no hand prompt
// pauses the draw. With an empty hand there is nothing to discard, and the mana clause still lands.
// The discard comes second, after the mana clause, as the text reads.

import type { Script } from "@jackioh/engine";
import { discardRandom, nextTurnMana } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-021");

/** One Hinder face: how much lower the opponent's refresh is, and whether the caster discards 1. */
function hinder(lower: number, discards: boolean): Script {
  if (!discards) {
    return {
      staticFlags: { castOnDraw: true },
      cry: () => [nextTurnMana({ amount: -lower, player: "enemy" })],
    };
  }
  return {
    staticFlags: { castOnDraw: true },
    // R640: "Discard 1" — one random card of the caster's own hand.
    cry: () => [nextTurnMana({ amount: -lower, player: "enemy" }), discardRandom({ count: 1 })],
  };
}

export const base: Script = hinder(1, true);

export const radiant: Script = hinder(2, false);
