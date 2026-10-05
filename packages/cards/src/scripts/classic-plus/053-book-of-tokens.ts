// C+ #53 Book of Tokens (SPEC §8.7 row 53). (1) Spell, Book, Epic.
//   Base:    "Summon 1-2 Rush Tokens." — the count is random (balance patch 1)
//   Radiant: "Lucky 1 / Summon 1-2 Radiant Rush Tokens."
//   Engine:  "Rush Tokens (§7) placed per R64; a full board takes fewer. Tunes: none."
//
// One laneless `summon` per token: each takes the leftmost empty, unlocked, unreserved unit zone (R64)
// and fizzles silently on a full row (§3.2), as Core #15's do. The token's stats are its own (§7).
// The count is one draw from the match rng (1 or 2), so the curve stays near a single 1-cost token
// summon; a tuned count would be a param, and there is none. Lucky X (§6.1) rolls X more times and
// keeps the most tokens: the Radiant face prints Lucky 1, and the Lucky read is the running card's
// own, so a Degrade or an Upgrade of that number moves it (B3.4), as Two Grapes reads its Lucky.

import { numberedKeywordsOn, type EffectContext, type Script } from "@jackioh/engine";
import { summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-053");

const RUSH_TOKEN = cardDef("core-t-rush").id;

/** The printed "1-2": the fewest and most tokens one cast summons. */
const MIN_TOKENS = 1;
const MAX_TOKENS = 2;

/** §6.1: the Lucky X the running card has now (its Radiant face prints Lucky 1). */
function luckyOf(ctx: EffectContext): number {
  if (ctx.self === null) return 0;
  return numberedKeywordsOn(ctx.state, ctx.self).find((keyword) => keyword.key === "Lucky")?.value ?? 0;
}

function bookOfTokens(radiant: boolean): Script {
  return {
    cry: (ctx) => {
      const roll = (): number => MIN_TOKENS + ctx.rng.int(MAX_TOKENS - MIN_TOKENS + 1);
      const lucky = luckyOf(ctx);
      const count = lucky > 0 ? ctx.rng.lucky(lucky, roll, (a, b) => Math.max(a, b)) : roll();
      return Array.from({ length: count }, () => summon({ defId: RUSH_TOKEN, radiant }));
    },
  };
}

export const base: Script = bookOfTokens(false);

export const radiant: Script = bookOfTokens(true);
