// C+ #64 Mulch Muncher (SPEC §8.7 row 64; BUILD M9 row C+ 64). (10) Unit, Rare, 9/9 → 18/18.
//   Base:    "Rush, Trample / Costs ({discount}) less for each Fruit you've played this game." — discount 1
//   Radiant: "Rush, Trample, Divine Shield / (the same)"
//
// A `cost` hook (Core #100's pattern, R55) reading the per-game count of Fruit-tagged plays
// (`playedThisGameWithTag`, B5 E4: casts count, R70; a Grape is a Fruit; never reset), floored at 0.
// R584: the discount is a price for a play, so it holds where a play takes the card from — its
// player's hand, or a graveyard a permission lets them play it from (E11) — and counts that player's
// Fruits; anywhere else (a deck, a graveyard, the field, a pool) it costs its printed (10) (R65).
// The keywords are the faces' printed ones, so both faces run this one script.

import { param, playableFromGraveyard, playedThisGameWithTag, queryCost, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-064");

const PRINTED_COST = queryCost(def);

export const base: Script = {
  cost: ({ state, instance }) => {
    const at = instance.zone;
    const forPlay = at.z === "hand" || (at.z === "graveyard" && playableFromGraveyard(state, instance));
    if (!forPlay) return PRINTED_COST;
    const discount = param({ state, self: instance, radiant: instance.radiant }, "discount");
    return Math.max(0, PRINTED_COST - discount * playedThisGameWithTag(state, at.player, "Fruit"));
  },
};

export const radiant: Script = base;
