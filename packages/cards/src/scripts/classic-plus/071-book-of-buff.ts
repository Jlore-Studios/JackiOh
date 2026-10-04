// C+ #71 Book of Buff (SPEC §8.7 row 71, R386; BUILD M9 row C+ 71). (1) Spell, Book, Epic.
//   Both faces: "Upgrade a card {times|time|times}. It may be a card in your hand." — times 5, Radiant 10
//
// A declared target (R81): a permanent on either side or a card in your own hand. `times` separate
// Upgrades, each its own draw (R386); an Immutable card is left alone. The Radiant ten is the declared
// `times`, so both faces run this one script.

import { param, type Script } from "@jackioh/engine";
import { upgrade } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-071");

export const base: Script = {
  // R654: an Upgrade helps, so a random cast that targets enemies aims this at friends.
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow", "hand"] }, aim: "help" }],
  cry: (ctx) => [upgrade({ target: { of: "chosen" }, times: param(ctx, "times") })],
};

export const radiant: Script = base;
