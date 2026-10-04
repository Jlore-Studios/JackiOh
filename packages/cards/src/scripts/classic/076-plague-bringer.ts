// C #76 Plague Bringer (SPEC §8.6 row 76). (2) Unit, Rare, 4/4 → 8/8.
//   Base:    "Rush
//             Cry: Place {tokens|Plague Counter|Plague Counters}. Draw {draw}." — 2 tokens, draw 1
//   Radiant: the same text — 4 tokens, draw 2
//   Engine:  "Plague Counters (§6.3): two (Radiant four) placements of 1, each on a permanent you choose
//            (either side, face-down cards included, repeats allowed), one prompt per token. Tunes:
//            tokens 2 ↑; draw 1 ↑."
//
// Rush is printed on both faces. `placePlagueTokens` asks one `target` prompt per token (a face-down
// card the chooser may not read offered by its id alone, R177) and parks the draw behind them (R113).
// Both numbers are declared (R386).

import { param, type Script } from "@jackioh/engine";
import { draw, placePlagueTokens } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-076");

export const base: Script = {
  cry: (ctx) => [placePlagueTokens({ count: param(ctx, "tokens") }), draw({ count: param(ctx, "draw") })],
};

// The same script: the Radiant face's 4 tokens and draw 2 are its declared numbers.
export const radiant: Script = base;
