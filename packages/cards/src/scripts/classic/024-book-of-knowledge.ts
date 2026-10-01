// C #24 Book of Knowledge (SPEC §8.6 row 24). (1) Spell, Book, Epic.
//   Base:    "Draw {draw}." — draw 3
//   Radiant: "Draw {draw}." — draw 6
//   Engine:  "Three draws; the hand cap applies (R4). Tunes: draw 3 ↑."
//
// §2.4 and R58: "Draw N" is N separate draws, each with its own cast-on-draw chain, and each one
// fatigues on an empty deck, burns into a full hand (R4, R317) or stops at a draw limit (B5 E3,
// R457). `effects/draw` is that pipeline, so this file only names how many.
//
// The count is the declared number `draw` (R386), 3 or 6, read through `param`; both faces run this
// one script.

import { param, type Script } from "@jackioh/engine";
import { draw } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-024");

export const base: Script = {
  cry: (ctx) => [draw({ count: param(ctx, "draw") })],
};

// The same script: the Radiant face's 6 is its declared `draw`, which `param` reads off the running face.
export const radiant: Script = base;
