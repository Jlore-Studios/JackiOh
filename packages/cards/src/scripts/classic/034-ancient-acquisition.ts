// C #34 Ancient Acquisition (SPEC §8.6 row 34, §6.3 Add to hand; R4, R70, R97, R317, R664).
// Spell, cost 1, Rare.
//   Base:    "Return {cards|random card|random cards} from your graveyard to hand." (2)
//   Radiant: "Return {cards|random card|random cards} from your graveyard or exile to hand." (4)
//   Engine:  "That many random cards from the pile or piles, drawn uniformly through the match rng
//            (balance patch 1: no pick prompt; R664); fewer cards than asked ends it; the hand cap
//            applies (R4). C #47 Recurring Felinor casts it. Tunes: cards 2 ↑."
//
// Each return moves through §2.4's pipeline (`addRandomFromGraveyard`): a full hand burns it into
// your graveyard (R4, R317). A card in your hand is yours to read alone again (R97). The Spell
// itself is resolving (§10.5), in no pile, so it is never one of its own returns.
//
// Cast by C #47 Recurring Felinor, the returns are still its caster's (R70).

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { addRandomFromGraveyard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-034");

function acquire(exile: boolean): Script {
  return {
    cry: (ctx) => [addRandomFromGraveyard({ count: param(ctx, "cards"), ...(exile ? { exile: true } : {}) })],
  };
}

export const base: Script = acquire(false);

export const radiant: Script = acquire(true);
