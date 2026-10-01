// C #34 Ancient Acquisition (SPEC §8.6 row 34, §6.3 Add to hand, §10.6; R4, R70, R81, R97, R317).
// Spell, cost 1, Rare.
//   Base:    "Return {cards|card|cards} from your graveyard to your hand."
//   Radiant: "Return {cards|card|cards} from your graveyard or exile to your hand."
//   Engine:  "Your choice of up to 2 (Radiant 4) from the pile or piles: a `pick` prompt (§10.6)
//            budgeted by count, without Discover's three-option limit; the hand cap applies (R4).
//            C #47 Recurring Felinor casts it. Tunes: cards 2 ↑."
//
// THE PICK is chosen as the Spell resolves, so it is a prompt (R81), the engine's `choosePick` (B5
// E18): every card in the pile is an option — no three-card Discover limit — and the answer may take
// up to the card's number of them (`param(ctx, "cards")`: 2, or 4 on the Radiant face), fewer when
// the piles hold fewer, and none at all ("up to"). An empty pile asks nothing. The Spell itself is
// resolving (§10.5), in no pile, so it is never one of its own options.
//
// Each pick then MOVES to your hand (`addToHand({ instance })`, §6.3's "creates or moves"), keeping
// its radiant flag and cost riders (R78), through §2.4's pipeline: a full hand burns it into your
// graveyard (R4, R317). A card in your hand is yours to read alone again (R97).
//
// Cast by C #47 Recurring Felinor, the picks are still its caster's (R70).

import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { addToHand, choosePick, type PileSpec } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-034");

function returnPicks(ctx: EffectContext): Effect[] {
  return ctx.targets.map((_, index) => addToHand({ instance: { of: "chosen", index } }));
}

function acquire(from: readonly PileSpec[], prompt: string): Script {
  return {
    cry: (ctx) => [choosePick({ step: "picked", from, max: param(ctx, "cards"), prompt })],
    resume: { picked: returnPicks },
  };
}

export const base: Script = acquire([{ zone: "graveyard" }], "Return cards from your graveyard to your hand");

export const radiant: Script = acquire(
  [{ zone: "graveyard" }, { zone: "exile" }],
  "Return cards from your graveyard or exile to your hand",
);
