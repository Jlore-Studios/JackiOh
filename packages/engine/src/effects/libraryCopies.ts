// T-AI-3 Hallucination's verb (SPEC §8.7 row T-AI-3; R57, R60, R129, R385): copies of `count`
// different random cards of a player's deck, added to the running card's controller's hand as new
// cards they own, each carrying its source's definition, radiant flag, `statsOverride` and `tuning`
// (R57's copy, R386) — never a Brittle count or a cost rider — the source staying where it is. A copy
// that reaches the hand is given Brittle `brittle` (B3.3); one the hand cap burns is not (§2.4, R586).
//
// R586: the copies go to the hand in the order drawn, never the deck's, so their order says nothing of where
// the sources lay (§9.1). A deck of no more than `count` cards gives a copy of each with no draw
// (R129), ordered by definition id for the same reason. The other player reads only that cards
// reached the hand (`addedToHand` under the sentinel, R97); the deck they came from is untouched.

import { addToHand } from "../draw";
import type { Effect } from "../script";
import { newInstance } from "../state";
import { copyTuning } from "../tuning";
import { giveBrittle } from "./brittle";
import { playerOf, type PlayerSpec } from "./targets";

export function addLibraryCopies(args: { of: PlayerSpec; count: number; brittle?: number }): Effect {
  return {
    kind: "addLibraryCopies",
    apply(ctx): void {
      const library = ctx.state.players[playerOf(ctx, args.of)].library;
      const count = Math.max(0, Math.trunc(args.count));
      const picked =
        library.length <= count
          ? [...library].sort((a, b) => (a.defId < b.defId ? -1 : a.defId > b.defId ? 1 : 0))
          : ctx.rng.shuffle([...library]).slice(0, count);
      for (const source of picked) {
        const copy = newInstance(ctx.state, source.defId, ctx.controller, { z: "hand", player: ctx.controller });
        copy.radiant = source.radiant;
        if (source.statsOverride !== undefined) copy.statsOverride = { ...source.statsOverride };
        const tuning = copyTuning(source.tuning);
        if (tuning !== undefined) copy.tuning = tuning;
        if (addToHand(ctx, copy) === "hand" && args.brittle !== undefined) {
          giveBrittle({ instanceId: copy.id, n: args.brittle }).apply(ctx);
        }
      }
    },
  };
}
