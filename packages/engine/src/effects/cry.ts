// Trigger a Cry (docs/classic-sets.md B5 E13, R467): the card-facing verb over `../cryTrigger`, which
// owns the sequence — who runs the Cry, how its choices are asked, and what "this" means out of a
// graveyard. Classic #54 Rewind: "Trigger the Cry of one of your Units on the field or in your
// graveyard"; its Radiant triggers any Unit's twice, which is two of these, each with its own choices.

import { cryPlaceOf, triggerCryOf } from "../cryTrigger";
import type { Effect } from "../script";
import type { CardInstance, GameState } from "../state";
import { instanceOnItsStay, resolveTarget, type TargetSpec } from "./targets";

/**
 * B5 E13: trigger the Cry of the Unit `target` names (default the play's first choice) or the card
 * `instanceId` names, for this card's controller, who makes the Cry's choices. A Unit on top of a
 * unit pile runs it as itself; one in a graveyard runs it with no "this". Anything else — a card with
 * no Cry, a dormant card, a card that has left — triggers nothing (`cryTrigger.cryPlaceOf`).
 */
export function triggerCry(args: { target?: TargetSpec; instanceId?: string } = {}): Effect {
  return {
    kind: "triggerCry",
    apply(ctx): void {
      const card =
        args.instanceId !== undefined
          ? instanceOnItsStay(ctx, args.instanceId)
          : (() => {
              const target = resolveTarget(ctx, args.target ?? { of: "chosen" });
              return target !== null && target.kind === "unit" ? target.instance : null;
            })();
      if (card === null) return;
      triggerCryOf(ctx, card, ctx.controller);
    },
  };
}

/**
 * B5 E13: whether this card's Cry can be triggered where it lies now — a Unit with a Cry on top of a
 * unit pile or in a graveyard. A pure read, for a card's target check (Classic #54's "a Unit that has
 * a Cry") and its `conditionMet`.
 */
export function hasTriggerableCry(state: GameState, card: CardInstance): boolean {
  return cryPlaceOf(state, card) !== null;
}
