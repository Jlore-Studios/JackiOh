// C #36 Burn (SPEC §8.6 row 36). (0) Spell, Common.
//   Base:    "Deal {damage} damage. If you have {threshold} or more mana left, draw {draw}." — 2, 4, 1
//   Radiant: "Deal {damage} damage. If your max mana is {threshold} or more, draw {draw}." — 4, 4, 1
//   Engine:  "Mana left is current mana as it resolves; max mana is §2.3's. The name is also a rules
//            word, which the reference proof never reads as this card unless `refs` lists it (R381).
//            Tunes: damage 2 ↑; threshold 4 ↓; draw 1 ↑."
//
// "Deal N damage" with no target named is targeted (§8's Conventions, as #68's is): any unit or hero,
// either side, chosen with the play (R81). One §4.4 hit, then the draw if the condition holds.
//
// The condition, read as the Spell resolves: the base face compares the mana its controller has left
// then (current mana, after paying for Burn, §2.3's temporary mana included); the Radiant face
// compares §2.3's max mana, which paying does not move. `drawCondition` holds both readings, so the
// Cry and the glow cannot disagree.
//
// R195: in hand the card glows when it would draw if played now — the base face with the mana that
// would be left after paying its price now (R65's `effectiveCost`, so a discount or a surcharge moves
// it), the Radiant face with max mana as it stands.
//
// The numbers are the declared `damage`, `threshold` and `draw` (R386), read through `param`.

import {
  effectiveCost,
  maxManaOf,
  param,
  unspentManaOf,
  type ConditionContext,
  type EffectContext,
  type Effect,
  type Script,
} from "@jackioh/engine";
import { damage, draw } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-036");

/** §8 Conventions: any unit or hero, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/**
 * The mana the face's condition compares, and whether it reaches the threshold. `leftNow` is the
 * base face's "mana left": current mana as the Spell resolves, or, asked of a card in hand, what
 * paying for it now would leave.
 */
function drawCondition(ctx: EffectContext | ConditionContext, radiant: boolean, leftNow: number): boolean {
  const mana = radiant ? maxManaOf(ctx.state, ctx.controller) : leftNow;
  return mana >= param(ctx, "threshold");
}

function burn(radiant: boolean): Script {
  return {
    targets,
    cry: (ctx): Effect[] => [
      damage({ to: { of: "chosen" }, amount: param(ctx, "damage") }),
      ...(drawCondition(ctx, radiant, unspentManaOf(ctx.state, ctx.controller)) ? [draw({ count: param(ctx, "draw") })] : []),
    ],
    // R195: in hand, whether playing it now would draw.
    conditionMet: (ctx) =>
      ctx.zone === "hand" &&
      drawCondition(ctx, radiant, unspentManaOf(ctx.state, ctx.controller) - effectiveCost(ctx.state, ctx.self)),
  };
}

export const base: Script = burn(false);

export const radiant: Script = burn(true);
