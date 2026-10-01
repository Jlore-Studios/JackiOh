// C #35 Prep (SPEC §8.6 row 35). (0) Spell, Common.
//   Base:    "Your next Spell this turn costs ({discount}) less." — discount 2
//   Radiant: "Your next Spell this turn costs ({discount}) less." — discount 4
//   Engine:  "#35 Lunar Eclipse's modifier (`costDiscount`, `onlyType: "Spell"`, this turn, consumed
//            on use or at cleanup; Cost, §6.3, R65). Tunes: discount 2 ↑."
//
// The discount is Core #35 Lunar Eclipse's player modifier, the same shape:
//   * `onlyType: "Spell"` — `effectiveCost` skips it for a Unit, a Field Spell or a Trap, so those
//     plays neither pay less nor spend it;
//   * `oncePerTurn` — §10.5 step 2 (`playSteps.consumeUsedDiscounts`) removes it on the first Spell
//     it priced, so only the next Spell is cheaper;
//   * `{ until: "thisTurn" }` — §2.2's cleanup takes it if no Spell used it.
// R65 floors the price at (0), so a (1) Spell under a 2 discount costs (0) and gives nothing back.
// R70: a cast pays nothing and never uses a discount, so a Spell cast this turn leaves it for the
// next Spell played.
//
// The amount is the declared number `discount` (R386), 2 or 4, read through `param`; both faces run
// this one script.

import { param, type Script } from "@jackioh/engine";
import { addPlayerModifier } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-035");

export const base: Script = {
  cry: (ctx) => [
    addPlayerModifier({
      player: "self",
      mod: {
        kind: "costDiscount",
        amount: param(ctx, "discount"),
        // "your next Spell": Spells only, and only the first one.
        onlyType: "Spell",
        oncePerTurn: true,
        // "this turn": §2.2's cleanup takes it if unused.
        expiry: { until: "thisTurn", turn: ctx.state.turn },
      },
    }),
  ],
};

// The same script: the Radiant face's 4 is its declared `discount`, which `param` reads off the running face.
export const radiant: Script = base;
