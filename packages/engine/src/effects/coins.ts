// Coin flips that buff a unit or grant it a keyword (SPEC §8.1 #4 Gary the Gambler, §10.4
// layer 4, §10.7, R32, R130).
//
// "Cry: flip 5 coins; +1 attack per heads, +1 max health per tails. Then flip a coin: heads
// gains Divine Shield, tails gains Rush", radiant "7 coins; +2 per heads, +2 per tails" plus the
// same rider verbatim. #4's Engine cell reads "5 or 7 seeded rolls; permanent buff layer; then
// one seeded coin granting Divine Shield on heads, Rush on tails; Lucky has no defined 'best'
// here so does not apply", and all four clauses are decisions this file keeps rather than
// re-makes:
//
//   * SEEDED ROLLS. Every flip is `ctx.rng.coin()`, the only source of randomness §10.7 allows.
//     `Math.random` is banned and lint-enforced (CLAUDE.md rule 4), and a card file may not roll
//     for itself, which is the whole reason these verbs exist.
//   * PERMANENT BUFF LAYER. The totals are applied through `buff` from `./buff`, and the rider
//     keyword through `grantKeyword` from the same module, so there is ONE implementation of
//     §10.4's layer 4: nothing here writes `card.buffs` or `card.grantedKeywords`, and the unit's
//     totals stay something `unitView` computes on every read rather than something stored.
//   * NO LUCKY. §6.1's Lucky X is "repeat a luck-based roll X extra times, keep the best", and a
//     flip that pays out on heads AND on tails has no better side, so R32/R130 leave Gary out of
//     it. There is deliberately no `lucky` option to wire in.
//   * ONE DRAW FOR THE RIDER. `flipCoinKeyword` takes exactly one seeded draw, and like
//     `flipCoins` it pays on both faces, so R32/R130 leave it without Lucky too.
//
// THE FIZZLE IS TOTAL, AND THAT IS A DETERMINISM RULE, NOT TIDINESS. §10.7 stores `rngCursor` in
// state, so the cursor is part of the match: every later draw in the game depends on how many were
// taken before it. If a flip happened only when a target was there, the whole rest of the match's
// randomness would depend on the board at this moment — two replays of the same action list could
// diverge, and §9.3 requires they never do. So the target is resolved FIRST and a missing one takes
// no draws at all, leaving the cursor exactly where it was.

import type { Keyword } from "@jackioh/shared";
import type { Effect } from "../script";
import { buff, grantKeyword, type BuffAmount } from "./buff";
import { instanceOf, type TargetSpec } from "./targets";

/** Heads × per-heads plus tails × per-tails, in one stat. */
function totalFor(
  heads: number,
  tails: number,
  perHeads: BuffAmount,
  perTails: BuffAmount,
  key: "attack" | "health",
): number {
  return heads * Math.trunc(perHeads[key] ?? 0) + tails * Math.trunc(perTails[key] ?? 0);
}

/**
 * §8.1 #4: flip `coins` coins and buff the target by `perHeads` per heads and `perTails` per tails.
 *
 * The two totals are applied as ONE layer-4 buff, so heads + tails always accounts for every flip
 * and there is a single `buffed` event carrying the whole result — which is what §10.10 animates
 * and what a test can read back. A buff that works out to +0/+0 (all five coins landing on a side
 * the card pays nothing for) is silently nothing, like every other no-op buff.
 */
export function flipCoins(args: {
  target: TargetSpec;
  coins: number;
  perHeads?: BuffAmount;
  perTails?: BuffAmount;
}): Effect {
  return {
    kind: "flipCoins",
    apply(ctx): void {
      // Resolved before a single draw: see the determinism note above.
      const unit = instanceOf(ctx, args.target);
      if (unit === null) return;

      const coins = Math.max(0, Math.trunc(args.coins));
      let heads = 0;
      for (let flip = 0; flip < coins; flip += 1) {
        if (ctx.rng.coin()) heads += 1;
      }
      const tails = coins - heads;

      const perHeads = args.perHeads ?? {};
      const perTails = args.perTails ?? {};
      buff({
        target: { of: "instance", instanceId: unit.id },
        attack: totalFor(heads, tails, perHeads, perTails, "attack"),
        health: totalFor(heads, tails, perHeads, perTails, "health"),
      }).apply(ctx);
    },
  };
}

/**
 * §8.1 #4: flip one coin and grant the target a keyword — `headsKeyword` on heads,
 * `tailsKeyword` on tails (§10.4 granted keywords, §10.7, R32/R130).
 *
 * Gary the Gambler's rider after its stat flips: heads gains Divine Shield, tails gains Rush.
 * Exactly one seeded `ctx.rng.coin()` draw, resolved against the target FIRST so a missing
 * target takes zero draws, like `flipCoins`. It pays on both faces, so there is no Lucky to
 * consult and no `lucky` option, per R32/R130.
 */
export function flipCoinKeyword(args: {
  target: TargetSpec;
  headsKeyword: Keyword;
  tailsKeyword: Keyword;
}): Effect {
  return {
    kind: "flipCoinKeyword",
    apply(ctx): void {
      // Resolved before the single draw: see the determinism note above.
      const unit = instanceOf(ctx, args.target);
      if (unit === null) return;

      const heads = ctx.rng.coin();
      grantKeyword({
        target: { of: "instance", instanceId: unit.id },
        keyword: heads ? args.headsKeyword : args.tailsKeyword,
      }).apply(ctx);
    },
  };
}
