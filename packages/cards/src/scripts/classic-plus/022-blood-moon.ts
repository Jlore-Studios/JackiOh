// C+ #22 Blood Moon (SPEC §8.7 row 22): the "would be healed" replacement (B5 E5, E8, R413): an enemy heal
// becomes Pierce damage from it, for the rest of the turn; the Radiant face is a Field Trap that keeps it.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-022");

export const base: Script = {
  replacements: [{ id: "blood-moon", on: "healed", instead: { damage: "pierce", lasting: "thisTurn" } }],
};

export const radiant: Script = {
  staticFlags: { healToDamage: true },
  replacements: [{ id: "blood-moon", on: "healed", instead: { damage: "pierce" } }],
};
