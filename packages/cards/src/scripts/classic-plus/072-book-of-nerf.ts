// C+ #72 Book of Nerf (SPEC §8.7 row 72, R386; BUILD M9 row C+ 72). (1) Spell, Book, Epic.
//   Base:    "Degrade a permanent {times|time|times}." — times 5
//   Radiant: "Degrade a card {times|time|times}. It may be a card in your hand." — times 10
//
// A declared target (R81): a permanent on either side, and on the Radiant face also a card in your own
// hand (a broader scope, R275). `times` separate Degrades, each its own draw (R386): attack floors at
// 0, current health at 1, cost stops at (4), no harmful keyword goes; an Immutable card is left alone.

import { param, type Script } from "@jackioh/engine";
import { degrade } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-072");

const cry: Script["cry"] = (ctx) => [degrade({ target: { of: "chosen" }, times: param(ctx, "times") })];

export const base: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }],
  cry,
};

export const radiant: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow", "hand"] } }],
  cry,
};
