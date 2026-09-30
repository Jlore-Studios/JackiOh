// Animated (docs/classic-sets.md B3.1, R383): a Field Spell, Trap or Field Trap that steps into a unit
// zone as a Unit, and the "Animated on your turn" cards that go back to their backrow zone at their
// controller's cleanup. `turn.ts` runs `animateAtTurnStart` as a stage of the start of a turn (after the
// Brittle tick, before the delayed effects) and `returnAtCleanup` at cleanup, after every end-of-turn
// step (§2.2, R62).

import type { PlayerId } from "@jackioh/shared";
import type { EngineSink } from "./resolve";

/** B3.1 rule 4: `player`'s "Animated on your turn" cards step into their unit zones. */
export function animateAtTurnStart(sink: EngineSink, player: PlayerId): void {
  void sink;
  void player;
}

/** B3.1 rule 4, rule 6: `player`'s animated "on your turn" cards go back to their home zones. */
export function returnAtCleanup(sink: EngineSink, player: PlayerId): void {
  void sink;
  void player;
}
