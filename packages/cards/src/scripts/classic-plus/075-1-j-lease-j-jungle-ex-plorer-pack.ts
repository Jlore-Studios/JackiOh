// C+ #75.1 J-lease J-Jungle EX-plorer Pack (SPEC §8.7 row 75.1). (2) Spell, Token (printed Legendary).
//   Base:    "Cast on draw: Add {cards|random Radiant Classic or Classic+ card|…cards} to your hand."
//   Radiant: "… Each costs (0)."
//   Engine:  "Cast on draw (§6.2, R58). Non-token cards of the Classic and Classic+ sets (the text names
//            them, R380), repeats allowed (R60), made Radiant; the hand cap burns what doesn't fit
//            (§2.4); the Radiant sets `costOverride` 0. A spell token, so it goes to the graveyard
//            after it resolves (§7). Tunes: cards 5 ↑."
//
// Cast on draw is the static flag §2.4's draw reads (R58): drawn, it casts itself at once and the draw
// repeats, so its owner draws again; played from a hand it resolves the same way. The pool names its two
// sets, so Core never comes up; a Pack is a token, so it is never in its own pool (§5.1, R387). Each card
// is made Radiant before it reaches the hand (R74), and the Radiant face's price lands only on a card that
// reached it (§2.4, R4).

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-075-1");

/** §8.7 row 75.1: "Each costs (0)" on the Radiant face. */
const SET_COST = 0;

function pack(free: boolean): Script {
  return {
    staticFlags: { castOnDraw: true },
    cry: (ctx) => [
      addRandomFromCatalog({
        query: { set: ["Classic", "Classic+"] },
        count: param(ctx, "cards"),
        radiant: true,
        ...(free ? { costOverride: SET_COST } : {}),
      }),
    ],
  };
}

export const base: Script = pack(false);

export const radiant: Script = pack(true);
