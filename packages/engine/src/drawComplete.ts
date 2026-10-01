// The "draw complete" point of a cast-on-draw draw (SPEC §2.4, R58, R70; Classic #9 Income Tax).
//
// R58: a cast-on-draw card is cast the moment it is drawn, and the draw is complete once that cast
// has resolved — so a trap or trigger answering the draw (its `drawn` event) answers it then, after
// the cast, and never inside it. The cast is a play with windows of its own (its announce, its step
// 4), and each window offers every event so far to the traps (R70), which would hand them the
// `drawn` that found the card before the card had resolved. So the draw holds its `drawn` back from
// every dispatch (`triggers.dispatchNewEvents` passes over a held one and keeps its place) until the
// cast's pipeline has finished (`playSteps.drive`), and the loop that runs next delivers it.
//
// A leaf over `state.heldDraws`: the instance ids the drawn cards had when they were drawn (a cast
// Trap set face-down takes a fresh id, R227, and a Devil's Pact replacement is a new card, R449 —
// neither moves the hold). Plain JSON, so a cast that pauses keeps its draw held across the answer.

import type { GameEvent } from "@jackioh/shared";
import type { GameState } from "./state";

/**
 * Where a cast-on-draw cast carries the id its card was drawn under (`resolve.castCard`'s options
 * data), so the play pipeline knows which draw its finish completes.
 */
export const CAST_ON_DRAW_KEY = "__castOnDraw";

/** R58: hold this draw's `drawn` until the cast of the card it drew has resolved. */
export function holdDraw(state: GameState, instanceId: string): void {
  const held = state.heldDraws ?? [];
  if (!held.includes(instanceId)) state.heldDraws = [...held, instanceId];
}

/** R58: the cast has resolved, so the draw is complete and its `drawn` may be answered. */
export function releaseDraw(state: GameState, instanceId: string): void {
  const held = state.heldDraws;
  if (held === undefined || !held.includes(instanceId)) return;
  const rest = held.filter((id) => id !== instanceId);
  if (rest.length === 0) delete state.heldDraws;
  else state.heldDraws = rest;
}

/** Whether a dispatch must pass over this event for now: the `drawn` of a draw not yet complete. */
export function heldBack(state: GameState, event: GameEvent): boolean {
  return event.type === "drawn" && (state.heldDraws?.includes(event.instanceId) ?? false);
}
