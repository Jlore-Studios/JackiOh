// C #61 Plague Bringer Goliath (SPEC §8.6 row 61). (3) Unit, Legendary, 7/7 → 14/14.
//   Base:    "Tribute 1, Rush, Trample
//             Cry: Place {tokens|Plague Token|Plague Tokens}. Draw {draw}." — 3 tokens, draw 1
//   Radiant: the same text — 3 tokens, draw 3
//   Engine:  "Tribute (§6.3, R101), which may pay for its own zone (R391, §3.2). Plague Tokens (§6.3):
//            three placements of 1, each on a permanent you choose (either side, face-down cards
//            included, repeats allowed), one prompt per token; then the draw. Tunes: tokens 3 ↑;
//            draw 1 ↑."
//
// Rush and Trample are printed on both catalog faces and §10.4 layer 1 reads them there. The Tribute is
// the play's price (§6.3, R101): one of your Units, or one worth more (a Sheep Token's 2 overpays it),
// paid at §10.5 step 2, so with no Unit to tribute the card can't be played at all; on a full unit row
// it may take the zone its own Tribute empties (R391).
//
// "Place N Plague Tokens" names no card, so it is N placements (`placePlagueTokens`), all on the one
// permanent a single `target` prompt names for the Goliath's controller over every permanent on the
// field — either side, the Goliath itself and face-down cards included (a face-down card the chooser
// may not read is offered by its id alone, R177) (R689). Each placement is one placement of 1,
// multiplied by the card that receives it (C #27), and each is its own for "whenever Plague Tokens
// are placed on this" (C #53). The prompt parks the rest of the Cry on `state.work` (R113), so the
// draw comes after the placements. The other player sees only that a prompt is open.
//
// Both numbers are the declared `tokens` and `draw` (R386), read through `param`.

import { param, type Script } from "@jackioh/engine";
import { draw, placePlagueTokens } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-061");

/** §6.3 "Tribute 1": one of your Units, as #66 The Rock's. */
const TRIBUTE_COST = 1;

export const base: Script = {
  staticFlags: { tribute: TRIBUTE_COST },
  cry: (ctx) => [placePlagueTokens({ count: param(ctx, "tokens") }), draw({ count: param(ctx, "draw") })],
};

// The same script: the Radiant face's draw 3 is its declared `draw`, which `param` reads off the running
// face; its stats are the catalog's.
export const radiant: Script = base;
