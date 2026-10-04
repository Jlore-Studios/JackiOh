// C+ #57 Book of Stats (SPEC §8.7 row 57). (1) Spell, Book, Epic.
//   Base:    "Give a Unit +{buff}/+{buff}." — buff 5
//   Radiant: the same text, buff 10.
//   Engine:  "A target Unit on either side, declared at play (R81); a permanent buff (§10.4 layer 4).
//            Tunes: buff 5 ↑."

import { param, type Script } from "@jackioh/engine";
import { buff } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-057");

// R651: a buff helps, so a random cast that targets enemies aims this at friends.
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] }, aim: "help" }];

export const base: Script = {
  targets,
  cry: (ctx) => {
    const amount = param(ctx, "buff");
    return [buff({ target: { of: "chosen" }, attack: amount, health: amount })];
  },
};

// The same script: the Radiant face's 10 is its declared `buff`, which `param` reads.
export const radiant: Script = base;
