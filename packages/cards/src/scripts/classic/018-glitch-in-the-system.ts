// C #18 Glitch in the System (SPEC §8.6 row 18, §6.3 Exile, §10.6; R13, R65, R66, R81, R113, R135,
// R396). Spell, cost 3, Common.
//   Base:    "Choose a number. Exile every card on the field, in hands and in decks that costs that
//            much."
//   Radiant: "Choose a number. Exile every card on your opponent's field, in their hand and in their
//            deck that costs that much."
//   Engine:  "A `number` choice (§10.6), declared with the play (R81) from a fixed list, 0 to 10
//            (`GLITCH_NUMBERS`), so the options reveal nothing. Costs read per R65 at resolution, as
//            R66 reads #94 Genn's Greed's: a hand card at its hand cost, a deck or field card at its
//            own; an X-cost card counts the X it was played for on the field and 0 anywhere else
//            (R396). The Spell itself is resolving, in no pile, and is spared. The base face reaches
//            both players' field, hand and deck, the zones its Radiant face names: the Radiant narrows
//            whose, not where. Graveyards and exile are untouched. Tunes: none (the number is chosen)."
//
// THE NUMBER is a play-time choice (R81), so it is declared — a `ModeDecl` of kind `number` whose
// options are the eleven numbers — and it travels in the `play` action's `modes`; `legalActions`
// offers each, and the options are the same whatever the board holds, so choosing reveals nothing.
//
// THE SWEEP is C #25 Lag in the System's with an exact cost: the set is read once as the Spell
// resolves (`forEachCard`, R66, R113), each card its own exile (R135). The zones are the field (the
// tops of the unit piles and every backrow card, face-down ones included, both sides in R68's walk;
// a card dormant under a Stack pile is not on the field, §3.2, R13), then each side's hand and deck,
// the Spell's controller first. The cost is `costNow` (R396): R65's cost as it stands at resolution,
// an X card its X on the field and 0 anywhere else or with none chosen. Graveyards and exile are never
// read, and the Spell itself is resolving, in none of those zones.

import type { CardInstance, EffectContext, Script } from "@jackioh/engine";
import { GLITCH_NUMBERS, costNow, zoneCards } from "@jackioh/engine";
import type { ModeDecl } from "@jackioh/shared";
import { cardsInScope, exile, forEachCard, sidesOf } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-018");

/** The fixed range the number is chosen from (BUILD §2's `GLITCH_NUMBERS`), as the choice's options. */
const GLITCH_OPTIONS: string[] = GLITCH_NUMBERS.map(String);

const NUMBER_CHOICE: ModeDecl[] = [{ kind: "number", options: GLITCH_OPTIONS }];

type Whose = "any" | "enemy";

/** The number the play carried, or null (§10.6's choices are strings). */
function chosenNumber(ctx: EffectContext): number | null {
  const picked = ctx.modes[0];
  if (picked === undefined || !GLITCH_OPTIONS.includes(picked)) return null;
  return Number(picked);
}

/** Every card on the field and in the hands and decks of `whose` sides, in the order the header gives. */
function reachable(ctx: EffectContext, whose: Whose): CardInstance[] {
  const field = cardsInScope(ctx, { side: whose, rows: ["units", "backrow"] });
  const players = whose === "enemy" ? sidesOf(ctx, "enemy") : [ctx.controller, ...sidesOf(ctx, "enemy")];
  const piles = players.flatMap((player) => [...zoneCards(ctx.state, player, "hand"), ...zoneCards(ctx.state, player, "library")]);
  return [...field, ...piles];
}

function glitch(whose: Whose): Script {
  return {
    modes: NUMBER_CHOICE,
    cry: (ctx) => {
      const number = chosenNumber(ctx);
      if (number === null) return [];
      return [
        forEachCard({
          cards: (read) => reachable(read, whose).filter((card) => costNow(read.state, card) === number),
          each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
        }),
      ];
    },
  };
}

export const base: Script = glitch("any");

export const radiant: Script = glitch("enemy");
