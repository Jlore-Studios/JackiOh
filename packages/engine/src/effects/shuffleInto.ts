// Shuffle into a library at a uniformly random position, stopping at the library cap (§6.3, R80).

import { shuffleIntoLibrary } from "../draw";
import { unitedEnchantments } from "../enchantments";
import type { Effect } from "../script";
import { findInstance, newInstance, type CardInstance, type GameState } from "../state";
import { copyTuning } from "../tuning";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * R57 as patch v0.2.0 extends it (B3.4 rule 4, R443): a copy shuffled into a library carries its
 * source's `tuning` and enchantments beside the radiant flag — never its Brittle count, which a copy
 * never inherits. `source` is the card copied, when it still exists and is of the copy's definition.
 */
function carryFrom(copy: CardInstance, source: CardInstance | undefined): void {
  if (source === undefined || source.defId !== copy.defId) return;
  const tuning = copyTuning(source.tuning);
  if (tuning !== undefined) copy.tuning = tuning;
  const enchantments = unitedEnchantments([source]);
  if (enchantments !== undefined) copy.enchantments = enchantments;
}

function sourceOf(state: GameState, id: string | undefined): CardInstance | undefined {
  return id === undefined ? undefined : findInstance(state, id);
}

/**
 * Shuffle fresh copies of a definition into a library (CN-Viral Injection's CN-Virus, Unstable Clone
 * Machine's copies). `copyOf` is the instance the copies are copies of, when they copy a card rather
 * than make one the text names: a copy a full library refuses is judged by that card (R316), which
 * may be a Trap its controller has just set face-down (#33).
 */
export function shuffleInto(args: {
  defId: string;
  count: number;
  player?: PlayerSpec;
  radiant?: boolean;
  copyOf?: string;
}): Effect {
  return {
    kind: "shuffleInto",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      for (let i = 0; i < args.count; i += 1) {
        const card = newInstance(ctx.state, args.defId, player, { z: "library", player });
        if (args.radiant === true) card.radiant = true;
        carryFrom(card, sourceOf(ctx.state, args.copyOf));
        shuffleIntoLibrary(ctx, card, false, args.copyOf);
      }
    },
  };
}

/** Shuffle copies of the card that is resolving (CN-Virus's own copies). */
export function shuffleCopiesOfSelf(args: { count: number; player?: PlayerSpec }): Effect {
  return {
    kind: "shuffleCopiesOfSelf",
    apply(ctx): void {
      if (ctx.self === null) return;
      const player = playerOf(ctx, args.player ?? "self");
      for (let i = 0; i < args.count; i += 1) {
        const card = newInstance(ctx.state, ctx.self.defId, player, { z: "library", player });
        card.radiant = ctx.self.radiant;
        carryFrom(card, ctx.self);
        shuffleIntoLibrary(ctx, card, false, ctx.self.id);
      }
    },
  };
}
