// Brittle X's verbs (docs/classic-sets.md B3.3 rule 4, R385): "Give Brittle N" sets a card's count to
// N, starting now; "gain +N Brittle" adds N to it. A card anywhere may take one — on the field, in a
// hand, in a deck (Classic+ #23 Dropshipping gives Brittle 2 to the cards it adds to your hand, T-AI-3
// Hallucination to its copy, #74 Twice Forward One Step Backwards gains +1 on the field). The count
// lives on the instance (`brittleCount.ts`) and ticks at its controller's start of turn (`brittle.ts`).
//
// Events (R440): a named card's count is reported by `counterChanged` "brittle" wherever it is, hidden
// by where it sits (R97); a scope's cards are reported only where both players read them, since a cue
// on each hidden card a scope reached would count the ones its filters let through.

import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { activeBrittleCount, gainBrittleCount, giveBrittleCount } from "../brittleCount";
import { cardsInCardScope, type CardScope } from "./cardScope";
import { instanceOnItsStay, resolveTarget, type TargetSpec } from "./targets";

/** Which cards a Brittle verb reaches: one named card (a spec or an id a script captured), or a scope. */
export type BrittleTarget = { target?: TargetSpec; instanceId?: string; scope?: CardScope };

function reached(ctx: EffectContext, args: BrittleTarget): { card: CardInstance; report: boolean }[] {
  if (args.scope !== undefined) {
    return cardsInCardScope(ctx, args.scope).map(({ card, readers }) => ({ card, report: readers === "everyone" }));
  }
  let card: CardInstance | null = null;
  // R174: a card named by id is aimed at the stay it had when the run began.
  if (args.instanceId !== undefined) card = instanceOnItsStay(ctx, args.instanceId);
  else if (args.target !== undefined) {
    const target = resolveTarget(ctx, args.target);
    card = target?.kind === "unit" ? target.instance : null;
  }
  // A card that has ceased to exist (R11, R86) is in no pile to take a count.
  return card === null || card.zone.z === "gone" ? [] : [{ card, report: true }];
}

function report(ctx: EffectContext, card: CardInstance): void {
  const count = activeBrittleCount(card);
  if (count === null) return;
  ctx.events.push({ type: "counterChanged", instanceId: card.id, counter: "brittle", value: count });
}

/** B3.3 rule 4: "Give Brittle N" — the count is N from now, whatever it was (a given count, R385). */
export function giveBrittle(args: BrittleTarget & { n: number }): Effect {
  return {
    kind: "giveBrittle",
    apply(ctx): void {
      for (const entry of reached(ctx, args)) {
        giveBrittleCount(ctx.state, entry.card, args.n);
        if (entry.report) report(ctx, entry.card);
      }
    },
  };
}

/**
 * B3.3 rule 4: "gain +N Brittle" — N more on the count in force; a card with none starts one now, at
 * N more than it prints (R441).
 */
export function gainBrittle(args: BrittleTarget & { n: number }): Effect {
  return {
    kind: "gainBrittle",
    apply(ctx): void {
      for (const entry of reached(ctx, args)) {
        gainBrittleCount(ctx.state, entry.card, args.n);
        if (entry.report) report(ctx, entry.card);
      }
    },
  };
}
