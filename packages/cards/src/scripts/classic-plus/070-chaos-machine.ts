// C+ #70 Chaos Machine (SPEC §8.7 row 70, R386; BUILD M9 row C+ 70). (2) Field Spell, Rare.
//   Both faces: "Start of turn and end of turn: Upgrade {cards} random other card(s) in your hand or
//   on your side of the field. Degrade {cards} random card(s) in your opponent's hand or on their
//   side of the field." — cards 1, Radiant 2
//
// At its controller's start and end of turn (R62). Each pick is uniform over the named hand and field
// cards but Chaos Machine itself (the tops of piles, face-down ones included, R585), different cards (R60);
// the engine draws it over the piles' sizes alone and reports in R242's order, public cards first, and
// a hidden card's change reaches the other seat as a bare cue (R177, R440). Empty zones draw nothing
// (R129). The Radiant face's two is its declared `cards`, so both faces run this one script.

import { param, type Hook, type Script } from "@jackioh/engine";
import { degrade, upgrade } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-070");

const tick: Hook = (ctx) => [
  upgrade({
    scope: { side: "self", zones: ["hand", "field"], excludeSelf: true },
    random: param(ctx, "cards"),
  }),
  degrade({ scope: { side: "enemy", zones: ["hand", "field"] }, random: param(ctx, "cards") }),
];

export const base: Script = { startOfTurn: tick, endOfTurn: tick };

export const radiant: Script = base;
