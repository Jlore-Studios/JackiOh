// C #70 Book of Plague (SPEC §8.6 row 70). (1) Spell, Book, Epic.
//   Base:    "Place {tokens|Plague Token|Plague Tokens}." — 5; Radiant: the same text — 10.
//   Engine:  "Plague Tokens (§6.3): five (Radiant ten) placements of 1, each on a permanent you choose
//            (either side, face-down cards included, repeats allowed), one prompt per token. Tunes:
//            tokens 5 ↑."
//
// `placePlagueTokens`: one `target` prompt naming the single permanent every placement lands on (R661),
// over every permanent on the field (a face-down card the chooser may not read offered by its id
// alone, R177); with none on the field nothing is placed. Each placement is its own (C #53 answers
// each; C #27 multiplies its own). The count is the declared `tokens` (R386).

import { param, type Script } from "@jackioh/engine";
import { placePlagueTokens } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-070");

export const base: Script = { cry: (ctx) => [placePlagueTokens({ count: param(ctx, "tokens") })] };

// The same script: the Radiant face's 10 is its declared `tokens`.
export const radiant: Script = base;
