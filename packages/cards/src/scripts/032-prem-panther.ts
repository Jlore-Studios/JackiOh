// #32 Prem Panther (SPEC §8.2 row 32): 5/4 "Rush / After this attacks and survives, draw 2 for each
// Unit that attack destroyed", radiant 10/8 "Rush, Cleave / the same" (patch v0.2.0, R426).
//
// Rush and Cleave are printed on the catalog faces, so §10.4 layer 1 grants them. The text is the
// engine's `afterAttack` hook, which runs once the state check that closes each combat the Panther
// attacked in has run, declared or forced (R53, R59), and hands it the combat's facts
// (`afterAttackOf`): the Units whose lethal hit it dealt in that combat (its strike and its Cleave's;
// a defender's strike back is the defender's, and a defending Panther runs no hook), and whether it
// is still on the field on the stay it attacked from (R174), so one that died, Reborn or not, did not
// survive (R212). The hook acts for the player who controlled it in that combat, though a Death in
// the check (#86 Mrow's) took it since.

import { afterAttackOf, type Hook, type Script } from "@jackioh/engine";
import { draw } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-032");

/** §8 row 32: "draw 2 for each Unit that attack destroyed". */
const DRAW_PER_UNIT = 2;

const afterAttack: Hook = (ctx) => {
  const facts = afterAttackOf(ctx);
  if (facts === null || !facts.survived || facts.destroyedIds.length === 0) return [];
  return [draw({ count: DRAW_PER_UNIT * facts.destroyedIds.length })];
};

export const base: Script = { afterAttack };

// "Rush, Cleave / the same": the keyword list is the radiant face's, the text is unchanged.
export const radiant: Script = { afterAttack };
