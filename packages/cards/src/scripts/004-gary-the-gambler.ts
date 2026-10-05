// SPEC §8.1 #4 Gary the Gambler — 1/1 → 2/2 Unit, Human, cost 1.
// Base: "Cry: flip 5 coins; +1 attack per heads, +1 max health per tails. Then flip a coin:
// heads gains Divine Shield, tails gains Rush".
// Radiant: "7 coins; +2 per heads, +2 per tails" plus the same rider verbatim — the cell changes
// only the numbers of the base clause (§8 Conventions), so it is the same flip at 7 coins and 2
// per side with an identical keyword coin.
//
// Engine cell: 5 or 7 seeded rolls and a permanent buff layer (§10.4 layer 4), then one seeded
// coin granting Divine Shield on heads, Rush on tails. R32/R130: Lucky has no defined "best" for
// a coin effect that pays on both faces, so it does not apply — the flips never consult it, which
// is why this file asks for plain coins and no `lucky` option.
//
// A card file may not call `ctx.rng` itself — that is state and randomness in a card script
// (CLAUDE.md rules 4 and 5) — so the rolls live in `flipCoins` and `flipCoinKeyword`
// (engine/src/effects/coins.ts). The stat flip makes exactly `coins` seeded `ctx.rng.coin()`
// calls and applies the totals as ONE layer-4 buff (one `buffed` event), so heads + tails always
// accounts for every flip; the rider takes exactly one more seeded draw and grants the keyword
// (§10.4 granted keywords, one `keywordGranted` event), so a replay at the same seed and cursor
// reproduces both (§10.7). Stats first, rider second: the draw prefix is unchanged.

import type { Script } from "@jackioh/engine";
import { flipCoinKeyword, flipCoins } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-004");

export const base: Script = {
  cry: () => [
    flipCoins({ target: { of: "self" }, coins: 5, perHeads: { attack: 1 }, perTails: { health: 1 } }),
    flipCoinKeyword({ target: { of: "self" }, headsKeyword: { kind: "Divine Shield" }, tailsKeyword: { kind: "Rush" } }),
  ],
};

export const radiant: Script = {
  cry: () => [
    flipCoins({ target: { of: "self" }, coins: 7, perHeads: { attack: 2 }, perTails: { health: 2 } }),
    flipCoinKeyword({ target: { of: "self" }, headsKeyword: { kind: "Divine Shield" }, tailsKeyword: { kind: "Rush" } }),
  ],
};
