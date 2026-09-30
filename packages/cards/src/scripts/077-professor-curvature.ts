// #77 Professor Curvature (SPEC §8.3, R48, R65, R363, §2.2, §10.1).
//
// Base: "Cry: (4)+ Cost cards cost (1) less on your next turn."; radiant "... cost (2) less ...".
// Patch v0.1.1 widened it from cards whose cost is exactly 4 to cards whose cost is 4 or more
// (R363), and cut its body to 3/3 → 6/6; the stats come from the catalog.
//
// §8's Engine cell: "Delayed player modifier, checked against current cost at play; expires at that
// turn's cleanup." That is one `PlayerModifier`, not a delayed effect:
//
//   R363 "Cost (4)+" → `minCurrentCost: 4`, which `mana.effectiveCost` reads AFTER `costMod` and the
//        flat discounts, exactly where R65 puts it: "add player discounts; apply Professor
//        Curvature if the result is then 4 or more; floor at 0". So a card the board has already
//        discounted from 5 to 4 is caught, and a printed-4 card another discount has already taken
//        to 3 is not.
//   R48  "on your NEXT turn" → `{ until: "nextTurnOf", player, fromTurn }`. `mana.modifierIsLive`
//        answers false while `state.turn === fromTurn`, so the discount does nothing on the turn
//        Curvature was played, and `modifiers.expireModifiers` drops it at the cleanup of that
//        player's next turn (§2.2: "Cleanup expires … Professor Curvature's discount on its turn").
//
// The modifier sits on the controller's own `mods`, so it is read only when that player's cards are
// costed; the opponent's turn in between cannot reach it even while it is live.
//
import type { Script } from "@jackioh/engine";
import { addPlayerModifier } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-077");

/** R363: the least current cost the discount reaches, checked after every other modifier (R65). */
const TARGET_COST = 4;

/** The two faces differ only in how much the discount is worth. */
function professorCurvature(amount: number): Script {
  return {
    cry: (ctx) => [
      addPlayerModifier({
        player: "self",
        mod: {
          kind: "costDiscount",
          amount,
          minCurrentCost: TARGET_COST,
          // R48: it covers the controller's NEXT turn, so it survives the turn it was created on.
          expiry: { until: "nextTurnOf", player: ctx.controller, fromTurn: ctx.state.turn },
        },
      }),
    ],
  };
}

export const base: Script = professorCurvature(1);

export const radiant: Script = professorCurvature(2);
