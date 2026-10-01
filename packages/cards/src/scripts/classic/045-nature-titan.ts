// C #45 Nature Titan (SPEC §8.6 row 45). (2) Unit, Legendary, 6/6 → 12/12.
//   Base:    "Tribute 1\nCry and whenever this attacks: Draw {draw} and heal your hero {heal}." — 1, 3
//   Radiant: "Tribute 1\nCry and whenever this attacks: Draw {draw} and heal your hero {heal}." — 2, 6
//   Engine:  "A Tribute cost (§6.3, R101), which may pay for its own zone (§3.2, R391). One script for
//            the Cry and the attack trigger, as #22 Carnivorous Cube's Cry and Death share one; the
//            trigger answers `attackDeclared` for this unit, forced attacks included. Tunes: draw 1 ↑;
//            heal 3 ↑."
//
// Tribute 1 is the play validator's (`playChoices.ts`), read off `staticFlags.tribute` as #66 The
// Rock's is: the units travel in the play action's `tributes` (R81, R90), a Sheep Token pays it alone
// (R101), and a board with no Unit to Tribute refuses the play. Whether a full board may be played
// into the zone the Tribute empties (R391) is the validator's as well; nothing here names a zone.
// The 1 is the printed keyword's, not a declared number, so it is written here once.
//
// "Cry and whenever this attacks" is one effect list, `titan` below, run by the Cry and by a trigger
// on `attackDeclared` whose attacker is this card. Every attack it makes emits one — declared by its
// controller or forced (R53, #9 Moths to the Flame) — and the trigger answers each; an attack on it
// names it as the target, not the attacker, so defending does nothing.
//
// "Heal your hero" is §6.3 Heal on a hero: no cap (§3). The two numbers are the declared `draw` and
// `heal` (R386), read through `param`; the faces differ only in them, so both run this one script.

import { param, type EffectContext, type Effect, type Script, type TriggerDef } from "@jackioh/engine";
import { draw, heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-045");

/** "Tribute 1" (§6.3): one of your units, or one Sheep Token. */
const TRIBUTE_COST = 1;

/** "Draw {draw} and heal your hero {heal}", the text the Cry and the attack trigger share. */
function titan(ctx: EffectContext): Effect[] {
  return [draw({ count: param(ctx, "draw") }), heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") })];
}

/**
 * "Whenever this attacks": every `attackDeclared` whose attacker is this card, forced ones included.
 * The test is in `run`: a `when` is read for traps only (R99, `traps.ts`), and an ordinary trigger's
 * `run` is its whole condition, as #32 Prem Panther's is.
 */
const onAttack: TriggerDef = {
  id: "45-whenever-this-attacks",
  on: ["attackDeclared"],
  run: (ctx) => {
    if (ctx.event.type !== "attackDeclared" || ctx.self === null || ctx.event.attackerId !== ctx.self.id) return [];
    return titan(ctx);
  },
};

export const base: Script = {
  staticFlags: { tribute: TRIBUTE_COST },
  cry: titan,
  triggers: [onAttack],
};

// The same script: the Radiant face's draw 2 and heal 6 are its declared numbers, which `param` reads off the running face.
export const radiant: Script = base;
