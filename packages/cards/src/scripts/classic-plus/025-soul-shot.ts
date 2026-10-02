// C+ #25 Soul Shot (SPEC §8.7 row 25): destroy a random enemy Unit a Spell may affect; Lucky X picks X
// more and keeps the best by R414's comparator.

import {
  cardKeywords,
  costNow,
  numberedSum,
  slotOf,
  unitView,
  type CardInstance,
  type EffectContext,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { cardsInScope, destroy, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-025");

/** R414: higher attack plus current health, then higher cost, then lower lane. */
function better(state: GameState): (a: CardInstance, b: CardInstance) => CardInstance {
  const worth = (card: CardInstance): number => {
    const view = unitView(state, card);
    return view.attack + view.health;
  };
  const lane = (card: CardInstance): number => slotOf(state, card)?.lane ?? Number.MAX_SAFE_INTEGER;
  return (a, b) => {
    if (worth(a) !== worth(b)) return worth(a) > worth(b) ? a : b;
    if (costNow(state, a) !== costNow(state, b)) return costNow(state, a) > costNow(state, b) ? a : b;
    return lane(b) < lane(a) ? b : a;
  };
}

/** One random enemy Unit a Spell may affect, Lucky X picks keeping the best; none on an empty board. */
function pick(ctx: EffectContext): CardInstance[] {
  const pool = cardsInScope(ctx, { side: "enemy" });
  if (pool.length === 0) return [];
  const lucky = ctx.self === null ? 0 : (numberedSum(cardKeywords(ctx.state, ctx.self), "Lucky") ?? 0);
  const roll = (): CardInstance | undefined => ctx.rng.pick(pool);
  const chosen =
    lucky > 0
      ? ctx.rng.lucky(lucky, roll, (a, b) => (a === undefined ? b : b === undefined ? a : better(ctx.state)(a, b)))
      : roll();
  return chosen === undefined ? [] : [chosen];
}

export const base: Script = {
  cry: () => [forEachCard({ cards: pick, each: (instanceId) => destroy({ target: { of: "instance", instanceId } }) })],
};

// The same script: Lucky 1 is the Radiant face's catalog keyword, which `pick` reads.
export const radiant: Script = base;
