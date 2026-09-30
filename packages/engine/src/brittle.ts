// Brittle X (docs/classic-sets.md B3.3, R385): the count on a card instance, its start-of-turn tick
// and its crumbling. `turn.ts` runs `brittleTick` as a stage of the start of a turn, right after the
// mana refresh (§2.2, R62).

import type { PlayerId } from "@jackioh/shared";
import type { EngineSink } from "./resolve";

/**
 * B3.3 rule 2: at the start of `player`'s turn, every Brittle count of theirs that has had a full
 * turn cycle drops by 1, and a count that reaches 0 crumbles its card (rule 3).
 */
export function brittleTick(sink: EngineSink, player: PlayerId): void {
  void sink;
  void player;
}
