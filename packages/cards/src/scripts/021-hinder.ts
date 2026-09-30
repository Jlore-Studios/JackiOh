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
// THE DISCARD (R431, R16, R81, R70, R158). "Discard 1" names no "random", so the caster picks the
// card (R16). It is the card's own choice, so it is declared (R81, as #26 declares its hand pick):
// played from hand it travels in the play action, and cast — on a draw, which is how Hinder almost
// always resolves — the caster is asked it as the cast begins (R70), a hand prompt in the middle of
// the draw, which pauses the draw and owes its remainder on `state.work` (R158). The pick offers the
// caster's own hand and never Hinder itself (§9.1, R90); with an empty hand there is nothing to offer,
// the declaration takes nothing, and the discard fizzles while the mana clause still lands (R90). The
// discard comes second, after the mana clause, as the text reads.

import type { Script } from "@jackioh/engine";
import { discard, nextTurnMana } from "@jackioh/engine/effects";
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
    // R16, R81: "Discard 1" — one card of the caster's own hand, their choice.
    targets: [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"] } }],
    cry: () => [nextTurnMana({ amount: -lower, player: "enemy" }), discard({ target: { of: "chosen" } })],
  };
}

export const base: Script = hinder(1, true);

export const radiant: Script = hinder(2, false);
