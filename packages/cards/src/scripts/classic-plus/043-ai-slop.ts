// C+ #43 AI Slop (SPEC §8.7 row 43, E23, R60, R77, R102, R179, R468, R469, R582). (4) Spell, Legendary.
//   Fuse {cards} random AI generated cards and add the result to your hand. It costs (0).
//   Radiant: the AI generated cards are Radiant.
//
// `fuseGenerated` is the whole fusion: independent picks from the ten AI generated cards (a pool the
// text names, so tokens reach it; repeats allowed, R60), fused with no target (R77, R102: the shared
// type, else the first's; a Token with the AI tag; its id names them, R179), into your hand at
// `costOverride` 0 (the hand cap burns it). Radiant picks go in on their Radiant face (R469).
// R582: tuned down to 1, "fuse 1 card" is no fusion (R77), so that one card is added as it is, at (0).

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog, fuseGenerated } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-043");

const AI_CARDS = { tags: ["AI" as const], token: true };

export const base: Script = {
  cry: (ctx) => {
    const count = param(ctx, "cards");
    return count < 2
      ? [addRandomFromCatalog({ query: AI_CARDS, count, costOverride: 0, radiant: ctx.radiant })]
      : [fuseGenerated({ count, query: AI_CARDS, radiant: ctx.radiant })];
  },
};

// The same script: the Radiant face's picks are Radiant through `ctx.radiant`.
export const radiant: Script = base;
