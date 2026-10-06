// C+ #3 Second Amendment Snake (SPEC §8.7 row 3). (2) Unit, Rare, 1/6 → 2/12.
// End of turn: one placement of {tokens} Plague Counters on itself (E19). Death: its last-known tokens
// (R78) become that many hits of {damage}, each on a random enemy still standing (E37, R59). Both faces
// run this script; the numbers are the face's declared `params` (R386). Preview: the hits (R280).

import { param, type CardInstance, type ConditionContext, type Script } from "@jackioh/engine";
import { damageSplit, placePlague } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-003");

/** R280: the words the preview's value follows, on both faces. */
const HITS_LABEL = "for each Plague Counter on this";

/** One hit per Plague Counter on the card — the Death's count and the preview's (R78: last-known). */
function hits(self: CardInstance | null): number {
  return self?.counters.plague ?? 0;
}

export const base: Script = {
  endOfTurn: (ctx) => [placePlague({ amount: param(ctx, "tokens") })],
  death: (ctx) => {
    const perHit = param(ctx, "damage");
    return [damageSplit({ amount: hits(ctx.self) * perHit, perHit, among: "enemies" })];
  },
  preview: (ctx: ConditionContext) => [{ label: HITS_LABEL, value: hits(ctx.self) }],
};

/** The Radiant face differs only in its stats and its `tokens` value (3), both catalog data. */
export const radiant: Script = base;
