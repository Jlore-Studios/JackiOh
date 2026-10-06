// C #28 Second Wind (SPEC §8.6 row 28, §6.2 Replacement, §6.3 Play; R1, R3, R65, R78, R393).
// Field Spell, cost 0, Legendary.
//   Base:    "Cry: Exile your deck. Discard your hand.\nAura: You may play cards from your graveyard
//            that cost ({minCost}) or more. Cards that would go to your graveyard are exiled instead."
//            (balance patch 1: the base face's permission needs the same minimum price)
//   Radiant: "Cry: Exile your deck. Discard your hand.\nAura: You may play cards from your graveyard
//            that cost ({minCost}) or more."
//   Engine:  "Play from the graveyard (§6.3 Play) for every card type while this is on the field: such
//            a play costs, chooses and counts as one from hand, fires its Cry (R1) and takes R65's
//            player discounts. The base face adds a replacement (§6.2 Replacement) at the "would go to
//            a graveyard" point for cards you own: they are exiled instead. Both faces' play from the
//            graveyard needs a price of at least (1) as it would be paid, which stops a loop of free
//            plays. The Cry's own discard lands in the graveyard before the Aura starts exiling, so the
//            discarded hand is playable (R393), the reading that gives the card its name. With no deck
//            left, every draw is fatigue (§2.4). Tunes: minimum price 1 ↓."
//
// THE CRY exiles your whole deck, top to bottom (`exileMatching` over the library with no cost
// filter: each card its own exile, R135), then discards your hand (`discardHand`, a discard of each
// card, R16's "whole hand" needing no choice).
//
// THE PERMISSION is `Script.graveyardPlay` (B5 E11): while the card acts on the field its controller
// may play any card of their own graveyard, which `legalActions` offers and §10.5 takes from there as
// from a hand — its cost, its choices, R65's player discounts, its Cry, its count as a play. The
// Radiant face's permission carries a minimum price as it would be paid (`param(ctx, "minCost")`), so
// a card that would cost less is not offered. It ends when the card leaves the field.
//
// THE BASE REPLACEMENT is `Script.replacements` (B5 E5): at the "would go to a graveyard" point, a
// card its controller owns goes to exile instead — a Spell played from the graveyard is exiled after it
// resolves, a destroyed Unit is exiled. R393 has the Cry's own discard land in the graveyard before
// the Aura starts: the card is on the field while its Cry runs, so the Cry marks itself as running
// (`remember`) around the discard and the replacement declines while the mark is on. A Second Wind
// that arrives without its Cry (a Recruit, a summon) has no mark, so its Aura is on from the start.

import type { Script } from "@jackioh/engine";
import { param, recalled } from "@jackioh/engine";
import { discardHand, exileMatching, remember } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-028");

/** The instance-memory mark the Cry holds while its own discard lands (R393). */
const CRY_RUNNING = "secondWindCry";

const cry: Script["cry"] = () => [
  remember({ key: CRY_RUNNING, value: true }),
  exileMatching({ zones: ["library"] }),
  discardHand(),
  remember({ key: CRY_RUNNING, value: false }),
];

export const base: Script = {
  cry,
  graveyardPlay: (args) => [{ minPrice: param(args, "minCost") }],
  replacements: [
    {
      id: "second-wind-exile",
      on: "toGraveyard",
      when: (ctx) => ctx.event.owner === ctx.controller && recalled({ self: ctx.self, data: {} }, CRY_RUNNING) !== true,
      instead: { to: "exile" },
    },
  ],
};

export const radiant: Script = {
  cry,
  graveyardPlay: (args) => [{ minPrice: param(args, "minCost") }],
};
