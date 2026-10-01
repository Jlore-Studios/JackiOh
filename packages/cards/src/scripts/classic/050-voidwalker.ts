// C #50 Voidwalker (SPEC §8.6 row 50; §6.2 Replacement; R11, R135, R398). Unit 6/3 → 12/6, cost 2, Rare.
//   Base:    "Cry: Exile every card in both graveyards.\nAura: Cards that would go to a graveyard are
//            exiled instead."
//   Radiant: "Cry: Exile every card in your opponent's graveyard.\nAura: Cards your opponent owns that
//            would go to a graveyard are exiled instead."
//   Engine:  the aura is a replacement at "would go to a graveyard" while Voidwalker is on the field,
//            whatever sends the card there; the Radiant face judges by owner. Its own card reaches the
//            graveyard when it dies: its aura left the field with it (R398). Unit tokens still cease to
//            exist (R11).

import type { Script } from "@jackioh/engine";
import { exileMatching } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-050");

export const base: Script = {
  cry: () => [exileMatching({ zones: ["graveyard"], player: "self" }), exileMatching({ zones: ["graveyard"], player: "enemy" })],
  replacements: [{ id: "voidwalker", on: "toGraveyard", instead: { to: "exile" } }],
};

export const radiant: Script = {
  cry: () => [exileMatching({ zones: ["graveyard"], player: "enemy" })],
  replacements: [
    {
      id: "voidwalker",
      on: "toGraveyard",
      when: (ctx) => ctx.event.owner !== ctx.controller,
      instead: { to: "exile" },
    },
  ],
};
