// #90.1 CN-Virus (SPEC §8.5, §7, §4.4, R57, R58, R70, R80, R316, R350).
//
// Base: "Cast on draw: take 1 damage; at end of turn, shuffle 2 copies of this into your deck".
// Radiant: "Cast on draw: take 2 damage; at end of turn, shuffle 3 copies of this into your deck"
// (§8's cell "Take 2 damage; 3 copies", R275: both numbers scale).
// Engine cell: "Damage goes through the pipeline; cast-on-draw chains, capped by R58; the copies wait
// for the end of the turn it was cast on (R350); copies stop at the library cap (R80)."
//
// §7's token rules say the rest: "CN-Virus damage is a normal damage instance to your own hero
// (Armor and Anti-oneshot Armor apply)." It is a SPELL token, so unlike a unit token it lives in a
// hand and a library like a real card and goes to the graveyard after it resolves (§7, R11).
//
// NOTHING HERE CASTS OR DRAWS. `staticFlags.castOnDraw` is the whole of "Cast on draw": `drawOne`
// (engine/src/draw.ts) reads the flag off the drawn card, casts it, and repeats the draw, stopping
// at `CAST_ON_DRAW_CHAIN_CAP` casts — after which the next such card goes to hand uncast and ends
// the chain (R58). `castCard` makes the cast free, counts it as a card played (R70) and sends the
// spell to the graveyard afterwards. The flag is on BOTH faces: R58's cap is a property of the
// draw, not of the card, and the radiant cell changes only the two numbers.
//
// "TAKE 1 DAMAGE" (radiant 2) IS DAMAGE, TO THE DRAWER'S OWN HERO, AND AT ONCE. `{ of: "selfHero" }`
// resolves to `ctx.controller`, and a cast-on-draw card resolves with `controller === owner` (off
// the field control follows ownership, R12), so the player who drew it takes the hit — which is
// exactly why #90 hands the token to the OPPONENT. It is damage and not "lose health" (R18), so it
// runs the whole §4.4 pipeline: step 2 subtracts the hero's Armor, so Going Long (#84) reduces or
// removes it, and step 3 applies any Anti-oneshot Armor cap. A hit reduced to 0 emits nothing and
// triggers nothing (R63) — and the copies are still owed, because the two clauses are independent.
//
// THE COPIES WAIT FOR THE END OF THE TURN (R350, patch v0.1.1, issue #27). They were shuffled in at
// once, so a chain could draw the copies it had just made. Now the cast arms a delayed effect for the
// end of the turn it is cast on — whoever's turn that is (`THIS_TURN`), since a draw can come on
// either player's turn (#32 Prem Panther defending) — and the copies go in at §2.2's end-of-turn
// delayed-effect point, in R68's creation order with the turn's other delayed effects (#39's copies,
// #78's exile). So a chain only ever casts the viruses the library already held. Each cast arms its
// own shuffle, so three viruses cast in one turn shuffle in 2 + 2 + 2 at its end. One cast after
// that turn's delayed effects have begun is due at the next such point for the same player (R62).
//
// THE COPIES ARE FRESH INSTANCES CARRYING THE RADIANT FLAG. The continuation is the face that was
// cast (`resume.radiant`), and each copy is created with that flag — R57's rule for copies shuffled
// into a library — which is what makes a Radiant virus breed Radiant viruses: each drawn copy casts
// the radiant face, takes 2 and arms 3 more. They go to the caster's own library ("into YOUR deck";
// the delayed effect runs as the virus's controller, its owner), each at a uniformly random position
// from the match rng, and `shuffleIntoLibrary` stops at `LIBRARY_CAP`, reporting each copy a full
// library refuses with `libraryOverflow` (R80, R316).

import type { Effect, Script } from "@jackioh/engine";
import { RESUME_HOOK } from "@jackioh/engine";
import { THIS_TURN, damage, delay, shuffleInto } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-090-1");

/** R350: the step the end-of-turn delayed effect re-enters (§10.6: `script.resume[step]`). */
const COPIES_STEP = "copies";

/** "take 1 damage; shuffle 2 copies", radiant "take 2 damage; 3 copies". */
const BASE = { damage: 1, copies: 2 } as const;
const RADIANT = { damage: 2, copies: 3 } as const;

/** The two numbers are the whole of the radiant text. */
function virus(face: { damage: number; copies: number }): Script {
  return {
    staticFlags: { castOnDraw: true },
    cry: (): Effect[] => [
      damage({ to: { of: "selfHero" }, amount: face.damage }),
      // R350: the copies at the end of the turn this is cast on, whoever's turn that is.
      delay({ at: { phase: "end", player: THIS_TURN }, step: COPIES_STEP, hook: RESUME_HOOK }),
    ],
    resume: {
      // R57: fresh copies with the flag of the face that was cast; R80 and R316 at a full library.
      [COPIES_STEP]: (ctx): Effect[] => [
        shuffleInto({
          defId: def.id,
          count: face.copies,
          player: "self",
          radiant: ctx.radiant,
          ...(ctx.self === null ? {} : { copyOf: ctx.self.id }),
        }),
      ],
    },
  };
}

export const base: Script = virus(BASE);

export const radiant: Script = virus(RADIANT);
