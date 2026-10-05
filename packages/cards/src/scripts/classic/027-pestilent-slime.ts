// C #27 Pestilent Slime (SPEC §8.6 row 27). (0) Unit, Common, 1/1 → 2/2.
//   Base:    "Plague Counters placed on this are multiplied by {multiplier}." — ×2
//   Radiant: "Plague Counters placed on this are multiplied by {multiplier}." — ×3
//   Engine:  "Plague Counters (§6.3): a placement multiplier (×2, Radiant ×3) on every placement onto it,
//            whoever places them. Tunes: multiplier 2 ↑."
//
// B5 E19, R471: the multiplier is the card's `plagueMultiplier`, which `plague.placePlagueOn` reads on
// every placement onto it — "Place N Plague Counters on this" puts N × multiplier, one placement of a
// "Place N Plague Counters" split puts 1 × multiplier — so each placement is still ONE placement,
// reported by one `counterChanged` carrying how many it put (`placed`), and a "whenever Plague Counters
// are placed on this" answers it once. It is the card's text, so it holds whoever places the tokens,
// and a Vanilla Slime multiplies by 1. Its tokens are counters, which R78 clears when it leaves the
// field.
//
// The multiplier is the declared `multiplier` (R386), read through `param` on the face it wears; both
// faces run this one script.

import { param, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-027");

export const base: Script = {
  plagueMultiplier: ({ state, self, radiant }) => param({ state, self, radiant }, "multiplier"),
};

// The same script: the Radiant face's ×3 is its declared `multiplier`, which `param` reads off the face it wears.
export const radiant: Script = base;
