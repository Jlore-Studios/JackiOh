// C+ #39 Book Worm (SPEC §8.7 row 39): (1) Unit, Common, 1/4 → 2/8.
//   Base:    "Start of Turn: Place a Plague Token on this. Death: Add a random Book to your hand for
//            each Plague Token on this."
//   Radiant: the same, the Books Radiant.
//   Engine:  "It starts with no Plague Tokens (balance patch 1: no N counter, no start-of-turn
//            increment — the tokens on itself are the count). At each start of its controller's turn
//            one Plague Token is placed on it; its Death reads those tokens last-known (R78, R89) and
//            adds that many random Books. The pool is the non-token Books of every set (R380, R60);
//            a full hand burns what doesn't fit (§2.4). Tunes: none."
//
// The Death's count is the snapshot's own counters (`plagueOn(ctx.self)`), so a bounced worm keeps
// nothing (R78) and a worm that died with none adds none.

import { plagueOn, type EffectContext, type Script } from "@jackioh/engine";
import { addRandomFromCatalog, placePlague } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-039");

/** The printed "a Plague Token": one per start of turn. */
const PLAGUE_PER_TURN = 1;

/** The tokens on itself as it died (R89), or none when it has no self. */
function tokensOnSelf(ctx: Pick<EffectContext, "self">): number {
  return ctx.self === null ? 0 : plagueOn(ctx.self);
}

function bookWorm(radiant: boolean): Script {
  return {
    startOfTurn: () => [placePlague({ target: { of: "self" }, amount: PLAGUE_PER_TURN })],
    death: (ctx) => [
      addRandomFromCatalog({ query: { tags: ["Book"] }, count: tokensOnSelf(ctx), ...(radiant ? { radiant } : {}) }),
    ],
    // R280: the tokens stacked on itself are the Books its Death would add — the public count.
    preview: (ctx) => [{ label: "Plague Token", value: tokensOnSelf(ctx) }],
  };
}

export const base: Script = bookWorm(false);

export const radiant: Script = bookWorm(true);
