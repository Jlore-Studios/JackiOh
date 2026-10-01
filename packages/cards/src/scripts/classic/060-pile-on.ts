// C #60 Pile On (SPEC §8.6 row 60; §6.2 Replacement, §6.3 Recruit, §2.3; R1, R11, R33, R80, R275). Spell,
// cost 5, Rare.
//   Base:    "Recruit every permanent in your deck.\nIf this would go to your graveyard, put it on the
//            bottom of your deck instead."
//   Radiant: "Recruit every permanent in your deck." (the clause dropped on purpose, R275)
//   Engine:  `recruitAll`: the deck top down, each permanent into its row while that row has an open
//            zone, Traps face-down (R33), no Cry (R1); Spells and unit-token cards stay (R11). The base
//            clause is a replacement on this card's own move to its graveyard — resolving, discarded,
//            burned — to the bottom of the deck; a full deck turns it away (R80).

import type { Script } from "@jackioh/engine";
import { recruitAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-060");

export const base: Script = {
  cry: () => [recruitAll()],
  replacements: [{ id: "pile-on", on: "toGraveyard", where: "self", instead: { to: "bottomOfLibrary" } }],
};

// The Radiant face drops the return (SPEC §8.6 row 60): the same Recruit, and this Spell goes to the
// graveyard as any Spell does.
export const radiant: Script = { cry: base.cry };
