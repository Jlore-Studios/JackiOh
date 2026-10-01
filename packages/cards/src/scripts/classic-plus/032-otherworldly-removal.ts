// C+ #32 Otherworldly Removal (SPEC §8.7 row 32; §2.4, §7, R4, R11). (2) Spell, Epic.
//   Base:    "Add an Execute, a Brawl and a Blade Storm to your hand."
//   Radiant: "Add a Radiant Execute, a Radiant Brawl and a Radiant Blade Storm to your hand."
//
// The three Spell tokens C+ #32.1 to #32.3, created in that order and added to the caster's hand
// through §2.4's pipeline, so a full hand burns what doesn't fit (R4). They are Spell tokens: each
// lives in hand like a real card and goes to the graveyard once played (R11). The opponent sees three
// cards added and never which (R97).

import type { Script } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-032");

/** §8.7 C+ #32: Execute, Brawl, Blade Storm — the order they are added in. */
const TOKENS = [cardDef("classicplus-032-1").id, cardDef("classicplus-032-2").id, cardDef("classicplus-032-3").id];

function otherworldlyRemoval(radiant: boolean): Script {
  return {
    cry: () => TOKENS.map((defId) => addToHand({ defId, ...(radiant ? { radiant: true } : {}) })),
  };
}

export const base: Script = otherworldlyRemoval(false);

export const radiant: Script = otherworldlyRemoval(true);
