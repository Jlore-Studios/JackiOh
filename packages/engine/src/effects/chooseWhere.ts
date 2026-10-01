// A target prompt narrowed by a condition no `TargetScope` field can say (§10.6): Classic #32 Felinor
// Feelings' Radiant face, "steal an enemy permanent in a lane where you control a (1) Cost Unit",
// asked after its Felinor Token lands, so the token's lane counts. A card-specific verb of the Classic
// #1–#45 workstream, the prompt half of what `TargetFilter.check` / `Script.targetChecks` do for a
// play's declared targets.
//
// `where` is read once, as the effect applies, to build the options; the prompt that opens holds only
// data (the options and the continuation), so a pause survives `JSON.parse(JSON.stringify(state))` as
// every prompt does (§9.3). Options are built exactly as `chooseTarget`'s — the scope's cards in its
// deterministic order, keyed by their selection, never by a name — and `viewFor` redacts an option
// that offers a card the chooser may not read, a face-down card of the other player's (R177).

import type { Selection } from "@jackioh/shared";
import { defOf } from "../catalog";
import { openPrompt, resumeSelf } from "../prompts";
import type { Effect, EffectContext } from "../script";
import { findInstance, type CardInstance } from "../state";
import { targetsInScope, type TargetScope } from "./choose";

function keyOf(selection: Selection): string {
  if (selection.pick === "instance") return `instance:${selection.instanceId}`;
  if (selection.pick === "hero") return `hero:${selection.player}`;
  return "none";
}

/**
 * A target prompt for `ctx.controller` over the scope's cards that `where` admits (a hero, which is no
 * card, only when `where` admits null). With no card admitted the effect fizzles and asks nothing, as
 * `chooseTarget` does; the answer re-enters the card's script at `step` with the pick in `ctx.targets`.
 */
export function chooseTargetWhere(args: {
  step: string;
  scope?: TargetScope;
  where: (ctx: EffectContext, card: CardInstance | null) => boolean;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseTargetWhere",
    apply(ctx): void {
      const options = targetsInScope(ctx, args.scope).filter((selection) => {
        if (selection.pick !== "instance") return args.where(ctx, null);
        const card = findInstance(ctx.state, selection.instanceId);
        return card !== undefined && args.where(ctx, card);
      });
      if (options.length === 0) return;
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: args.prompt ?? "Choose a target",
        options: options.map((selection) => {
          const card = selection.pick === "instance" ? findInstance(ctx.state, selection.instanceId) : undefined;
          const label =
            card !== undefined
              ? defOf(ctx.state, card.defId).name
              : selection.pick === "hero"
                ? `${selection.player}'s hero`
                : "nothing";
          return { key: keyOf(selection), label, selection };
        }),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}
