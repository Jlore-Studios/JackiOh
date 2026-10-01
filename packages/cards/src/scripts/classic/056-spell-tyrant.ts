// C #56 Spell Tyrant (SPEC §8.6 row 56; §6.3 Cast, §10.6; R70, R386). Unit 5/5 → 10/10, cost 4, Legendary.
//   Base:    "Cry: Choose up to {spells|Spell|Spells} in your graveyard. Cast them, then exile them." (3)
//   Radiant: "Cry: Cast every Spell in your graveyard, oldest first, then exile them."
//   Engine:  casts from the graveyard one at a time, each free, counted as played and with your choices
//            (R70), each exiled after it resolves instead of going to the graveyard. The base face's
//            three are your pick as the Cry resolves (a `pick` prompt, its budget `spells` cards); the
//            Radiant face casts the Spells there as the Cry begins, oldest first, so a Spell a cast puts
//            in the graveyard is not cast. "Spell" is the Spell type.

import { cardTypeOf, param, zoneCards, type Script } from "@jackioh/engine";
import { castEach, choosePick } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-056");

export const base: Script = {
  cry: (ctx) => [
    choosePick({
      step: "cast",
      from: [{ zone: "graveyard" }],
      filter: { type: "Spell" },
      max: param(ctx, "spells"),
      prompt: "Choose Spells in your graveyard to cast",
    }),
  ],
  resume: {
    cast: (ctx) => {
      const picked = ctx.targets.flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []));
      return [castEach({ cards: () => picked, afterward: "exile" })];
    },
  },
};

export const radiant: Script = {
  cry: () => [
    castEach({
      cards: (ctx) => zoneCards(ctx.state, ctx.controller, "graveyard").filter((card) => cardTypeOf(ctx.state, card) === "Spell"),
      afterward: "exile",
    }),
  ],
};
