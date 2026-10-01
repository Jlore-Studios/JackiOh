// C+ #17 Conjure Rush Token++ (SPEC §8.7 row 17): summon a Rush Token (Radiant: 3) with {keywords} (6)
// different random keywords from R21's pool it lacks; a summon with no open zone rolls nothing (R129).

import { param, type Effect, type EffectContext, type Script } from "@jackioh/engine";
import { summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-017");

/** §7's shared Rush Token, 3/3 with Rush. */
const RUSH_TOKEN = "core-t-rush";

/** "Summon 3 Rush Tokens" on the Radiant face; the base face summons one. */
const RADIANT_TOKENS = 3;

function conjure(tokens: number): (ctx: EffectContext) => Effect[] {
  return (ctx) => {
    const keywords = param(ctx, "keywords");
    return Array.from({ length: tokens }, () => summon({ defId: RUSH_TOKEN, randomKeywords: keywords }));
  };
}

export const base: Script = { cry: conjure(1) };

export const radiant: Script = { cry: conjure(RADIANT_TOKENS) };
