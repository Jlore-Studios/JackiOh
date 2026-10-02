// C+ #12.3 Fluffy Grip (SPEC §8.7 row 12.3): (1) Spell, Pancake, Token (printed Legendary).
//   Base:    "Steal a random Unit from your opponent's deck and put it in your hand. It costs (0)."
//   Radiant: "… It costs (0) and becomes Radiant."
// E2/E16 (`takeFromLibrary`): a random Unit card of their library becomes yours, in your hand, with
// `costOverride` 0; a full hand burns it into your graveyard, now its owner's; none, nothing (R129).

import type { Script } from "@jackioh/engine";
import { takeFromLibrary } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-3");

export const base: Script = {
  cry: () => [takeFromLibrary({ from: "enemy", pick: "random", filter: { type: "Unit" }, costOverride: 0 })],
};

export const radiant: Script = {
  cry: () => [takeFromLibrary({ from: "enemy", pick: "random", filter: { type: "Unit" }, costOverride: 0, radiant: true })],
};
