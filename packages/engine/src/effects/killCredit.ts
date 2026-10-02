// Kills an effect watches, and kills it credits to another unit (R42, R412): Classic+ #19.2 Jungle
// Loser's "If it destroys the enemy Unit across from your Bot Loser, your Bot Loser goes Berserk" and
// its Radiant "your Bot Loser gets the kill instead".
//
// One effect wraps the one it watches (`during`, a forced attack): the credit is in force while that
// effect's hits land and gone after it, and the kills are read off that effect's own `destroyed`
// events in the same `apply`, so a Death that asks (a prompt pausing the list) loses none of them.

import { KILL_CREDIT_KEY, type KillCredit } from "../killCredit";
import { applyEffects } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { findInstance, type CardInstance } from "../state";
import { instanceOf, type TargetSpec } from "./targets";

/**
 * R42, R412: apply `during` with `killer`'s kills watched. `pairs`, read before `during`, names each
 * victim that matters and the unit it is paired with. With `transfer`, `killer`'s lethal hit on a
 * paired victim names that unit as R42's killer (the `destroyed` event's `killerId` and its kill
 * triggers). Then `then(pair)` applies for each pair whose victim `during` destroyed, killed by the
 * credited unit with `transfer` or by `killer` without; its effects must not ask. A killer not on the
 * field watches nothing, and `during` still applies.
 */
export function withKillCredit(args: {
  killer: TargetSpec;
  pairs: (ctx: EffectContext, killer: CardInstance) => readonly KillCredit[];
  transfer: boolean;
  during: Effect;
  then?: (pair: KillCredit) => readonly Effect[];
}): Effect {
  return {
    kind: "withKillCredit",
    apply(ctx): void {
      const killer = instanceOf(ctx, args.killer);
      const pairs = killer === null || killer.zone.z !== "field" ? [] : args.pairs(ctx, killer);
      if (killer !== null && args.transfer && pairs.length > 0) killer.memory[KILL_CREDIT_KEY] = pairs.map((pair) => ({ ...pair }));
      const from = ctx.events.length;
      args.during.apply(ctx);
      if (killer === null) return;
      const live = findInstance(ctx.state, killer.id);
      if (live !== undefined) delete live.memory[KILL_CREDIT_KEY];
      for (const event of ctx.events.slice(from)) {
        if (event.type !== "destroyed") continue;
        const pair = pairs.find((each) => each.victimId === event.instanceId);
        if (pair === undefined || event.killerId !== (args.transfer ? pair.toId : killer.id)) continue;
        if (args.then !== undefined) applyEffects(args.then(pair), ctx);
      }
    },
  };
}
