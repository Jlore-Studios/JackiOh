// #78 /fullsend (SPEC §8.3, R62, R65, R364, §2.2, §2.3, §10.5 step 5, §10.1).
//
// Base: "Refresh 3 mana. Your cards cost (1) less this turn. End of turn: Exile your hand."
// Radiant: "Refresh 3 mana. Your cards cost (1) less and gain "Combo: Draw 1" this turn. End of
// turn: Exile your hand." Patch v0.1.1 made the mana a Refresh of 3 instead of a gain of 4 on both
// faces, took the Combo draw off the base face, and set the Radiant discount back to 1, so the
// Combo draw is the Radiant face's rider.
//
// §8's Engine cell: "Turn-scoped player modifiers plus an end-of-turn delayed exile." The effects:
//
//  1. `refreshMana({ amount: 3 })` — §6.3's Refresh (R364): up to 3 spent mana back, never past max,
//     so unlike the old temporary gain it cannot take current above MAX_MANA.
//  2. a `costDiscount` with `{ until: "thisTurn", turn }`. The text says "your CARDS", not "your
//     spells", so there is no `onlyType` and no `minCurrentCost`: it is the flat discount R65
//     applies before Curvature. R65 also settles the X-cost case with no help from this card: "An
//     X-cost card being played costs exactly X: `costMod` and discounts don't change it", and
//     `mana.effectiveCost` returns early for an X card, so /fullsend never cheapens an X card.
//  3. the Radiant face only: a `comboDraw` modifier with the same expiry. §10.5 step 5 resolves
//     "Combo checks, Quickstriker, /fullsend's Combo draw, then the card's own … script", so this is
//     a PLAYER-scoped rider that the play pipeline reads once per card played this turn.
//  4. a delayed effect at `{ phase: "end", player: controller }` whose hook exiles the hand.
//
// R62 places that last one precisely: "… → end-of-turn triggers → end-of-turn trap window (Bread and
// Butter and Intern Stimmy on both sides, in R68 order) → end-of-turn delayed effects → cleanup".
// `turn.ts` matches for the two neighbours it has (`endOfTurn` hooks, then `runDelayed(sink, "end",
// player)`, then `cleanup`), so the exile lands AFTER the traps have had their window — a Bread and
// Butter token still reaches the hand and is then exiled with it — and BEFORE cleanup, so the
// modifiers above are still live while the exile runs. The trap window itself is not in `endTurn`
// yet (see the report).
//
// The continuation is one entry in this card's `resume` step table, named by the `delay` that
// schedules it (`hook: RESUME_HOOK`). R126: `turn.runDelayed` re-enters a delayed effect through
// `prompts.runResume`, the one reader that resolves either shape — a `Hook` on the script or a step
// table — so a card registers its continuation once and never twice. /fullsend is a Spell, so by
// the time the step runs the instance is in the graveyard; the stored `Resume` names the script and
// the face, and its `radiant` flag persists in every zone (R78), so the radiant face's step is the
// one that runs — and R127 has it run even if there were no instance left to find at all.

import type { Effect, EffectContext, Hook, Script } from "@jackioh/engine";
import { RESUME_HOOK } from "@jackioh/engine";
import { addPlayerModifier, delay, exileHand, refreshMana } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-078");

/** §6.3 Refresh (R364): spent mana given back, never past max. */
const MANA_REFRESH = 3;
/** "Your cards cost (1) less this turn", on both faces. */
const DISCOUNT = 1;
/** The Radiant face's "gain 'Combo: Draw 1'" — one card per play. */
const COMBO_DRAW = 1;
/** The step name the delayed effect carries; `turn.ts` labels the pause with it. */
const EXILE_STEP = "exileHand";

/**
 * R62: the end-of-turn delayed effect. It runs after the trap window and before cleanup, and it
 * exiles whatever the hand holds then — including cards drawn by the Combo rider this turn.
 */
const exileTheHand: Hook = () => [exileHand({ player: "self" })];

/** The Radiant face's rider: each card played this turn draws 1 (§10.5 step 5). */
function comboDraw(ctx: EffectContext): Effect {
  return addPlayerModifier({
    player: "self",
    mod: { kind: "comboDraw", amount: COMBO_DRAW, expiry: { until: "thisTurn", turn: ctx.state.turn } },
  });
}

/** The two faces differ only in whether the turn's cards gain the Combo draw. */
function fullsend(withComboDraw: boolean): Script {
  return {
    cry: (ctx) => [
      refreshMana({ amount: MANA_REFRESH }),
      addPlayerModifier({
        player: "self",
        // "your cards", so no `onlyType`; R65 applies it as a flat discount before Curvature.
        mod: {
          kind: "costDiscount",
          amount: DISCOUNT,
          expiry: { until: "thisTurn", turn: ctx.state.turn },
        },
      }),
      ...(withComboDraw ? [comboDraw(ctx)] : []),
      // R62: `delay` takes a PlayerSpec, so "my own end of turn" is "self" (§6.3, §2.2).
      delay({ at: { phase: "end", player: "self" }, step: EXILE_STEP, hook: RESUME_HOOK }),
    ],
    // The one registration (R126): the step table the `delay` above names.
    resume: { [EXILE_STEP]: exileTheHand },
  };
}

export const base: Script = fullsend(false);

export const radiant: Script = fullsend(true);
