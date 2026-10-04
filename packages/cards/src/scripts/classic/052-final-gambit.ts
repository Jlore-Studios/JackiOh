// C #52 Final Gambit (SPEC §8.6 row 52; §4.4 step 4a, §6.3 Redirect; R18, R33, R44, R58, R125, R216,
// R317, R386). Trap, cost 2, Epic.
//   Base:    "Activates when a hit would bring your hero to 0 or less: Redirect the hit to the enemy
//            hero. Then heal your hero {heal} and draw {draw}." (10, 3)
//   Radiant: "… Then heal your hero {heal} and draw your deck." (20)
//   Engine:  a replacement at "would take lethal damage" (after Armor, multipliers and caps; this hit
//            alone, R44): the trap fires and the hit goes on to the enemy hero as a new instance from
//            the same source, through their Armor, multipliers and caps; then the heal and the draws
//            ("draw your deck" counts the deck as the step starts, R58). Fatigue counts; losing health
//            does not (R18). Either player's turn.
//
// The replacement is data (`Script.replacements`); its `then` step is owed on `state.work` and runs
// after the hit has landed — never, when that hit ended the game (R216). A play's or a combat's state
// check usually ends the game first; where none runs before the step (owed work drained at the start
// of a turn), the step itself finds the enemy hero at 0 or less and does nothing. That is what stops a
// Final Gambit fused into a Field Trap that stays (C+ #74) from re-aiming its own fatigue for ever.

import { heroOf, param, replacementOf, zoneCount, type EffectContext, type Script } from "@jackioh/engine";
import { draw, heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-052");

/** How many cards the follow-up draws: the declared number, or the deck as the step begins (R58). */
type DrawCount = (ctx: EffectContext) => number;

function finalGambit(drawCount: DrawCount): Script {
  return {
    replacements: [{ id: "final-gambit", on: "lethalHit", instead: { redirect: "enemyHero" }, then: "afterRedirect" }],
    resume: {
      afterRedirect: (ctx) => {
        // R216: the re-aimed hit left the enemy hero at 0 or less, so the game is over.
        const to = replacementOf(ctx)?.redirectedTo;
        if (to !== undefined && heroOf(ctx.state, to).health <= 0) return [];
        return [heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") }), draw({ count: drawCount(ctx) })];
      },
    },
  };
}

export const base: Script = finalGambit((ctx) => param(ctx, "draw"));

export const radiant: Script = finalGambit((ctx) => zoneCount(ctx.state, ctx.controller, "library"));
