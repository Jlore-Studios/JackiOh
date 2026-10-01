// Damage in rounds until a Unit dies (SPEC §8.7 C+ #32.3 Blade Storm, §4.5, R59, R283).
//
// R59 checks state after a whole effect list; Blade Storm's is one of the two lists that check inside
// themselves (R283's is the other): each round is its own list, a sweep and then the state check
// (`afterStateCheck`), so the round's deaths and their Death hooks resolve before the next round is
// built. The storm stops after a round in which a Unit died (a Reborn death counts; R55's destroyed
// count says so), after `rounds` rounds, or when no Unit in scope is left.
//
// Each round is a part (`lazyPart`) holding the next, built when the list reaches it. A Death hook
// that asks parks the rest on `state.work` (R113); the part's memo is the destroyed count it began
// with, so a resumed part rebuilds exactly as it ran and the round after it decides afresh.

import { lazyPart } from "../resolve";
import type { Effect } from "../script";
import { afterStateCheck } from "./afterCheck";
import { damageAll } from "./damage";
import { cardsInScope, type BoardScope } from "./targets";

type Storm = { amount: number; rounds: number; side: BoardScope["side"] };

function round(storm: Storm, at: number, deathsBefore: number | null): Effect {
  return lazyPart("damageRound", (ctx, memo) => {
    const deaths = ctx.state.counters.destroyed;
    if (typeof memo !== "number") {
      const died = deathsBefore !== null && deaths > deathsBefore;
      if (died || at >= storm.rounds || cardsInScope(ctx, { side: storm.side }).length === 0) return { effects: [] };
    }
    const start = typeof memo === "number" ? memo : deaths;
    return {
      effects: [damageAll({ amount: storm.amount, side: storm.side }), afterStateCheck(() => []), round(storm, at + 1, start)],
      memo: start,
    };
  });
}

/** `amount` to every Unit in scope, round after round, each round with its own state check (R59). */
export function damageRoundsUntilDeath(args: Storm): Effect {
  return round(args, 0, null);
}
