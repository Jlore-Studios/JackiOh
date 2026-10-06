// C #15 Nose Hunter (SPEC §8.6 row 15). Unit 3/1 → 6/2, Human, cost 1, Common.
//   Base:    "Activate ♾️: Discard a random card. Exile the bottom {exile|card|cards} of your opponent's deck."
//   Radiant: "… Exile the bottom {exile|card|cards} of your opponent's deck and a random card from their hand."
//
// R392: the designer's "Discard a random card: Exile …" is an Activate ♾️ ability (balance patch 1;
// R384) — "cost: effect" is how a card is clicked to do an effect — so the random discard is the
// ability's cost, paid as it is activated (`cost.discardRandom`), and with an empty hand it cannot be
// activated at all (`subsystems/activate.ts` refuses it, and so `legalActions` never lists it). The
// discard is an ordinary discard (§6.3), so C #64 Malzahar's Recycler sees it. "Each opponent" in the
// designer's text is the multiplayer phrasing Heroic Power uses (R45): with two players, the opponent.
//
// Activating is not attacking, so it is usable the turn Nose Hunter arrives and never spends an
// exertion; nor is it a play, so nothing that counts plays sees it (R384).
//
// The exile reads the bottom of the deck as `exileBottomOfLibrary` does (the last element; an empty
// deck exiles nothing and deals no fatigue, since this is not a draw). The Radiant face's "a random
// card from their hand" is `exileRandomFromHand` (R60; an empty hand: nothing). No event names a deck
// position; the exiled cards are public once in exile (§3.2).

import type { ActivationDecl, EffectContext, Effect, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { exileBottomOfLibrary, exileRandomFromHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-015");

/** "Exile the bottom {exile} card(s) of your opponent's deck" (Radiant: "… and a random card from their hand"). */
function hunt(ctx: EffectContext, fromHand: boolean): Effect[] {
  const bottom = exileBottomOfLibrary({ player: "enemy", count: param(ctx, "exile") });
  return fromHand ? [bottom, exileRandomFromHand({ player: "enemy", count: 1 })] : [bottom];
}

function ability(fromHand: boolean): ActivationDecl {
  return {
    id: "hunt",
    label: fromHand
      ? "Discard a random card. Exile the bottom of your opponent's deck and a random card from their hand"
      : "Discard a random card. Exile the bottom of your opponent's deck",
    uses: "unlimited",
    cost: { discardRandom: 1 },
    run: (ctx) => hunt(ctx, fromHand),
  };
}

export const base: Script = { activations: [ability(false)] };

export const radiant: Script = { activations: [ability(true)] };
