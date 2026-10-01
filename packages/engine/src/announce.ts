// The announce window's record (docs/classic-sets.md B5 E1, R448): the plays and casts that have been
// paid for and announced (`cardAnnounced`) but not yet moved by §10.5 step 4.
//
// §10.5 gains a step between 3 and 4 (`playSteps.announceStep`): once the price is paid and before
// the card moves, the engine announces the play and runs a window in which Counters answer it, traps
// first and then the other triggers the announce woke (§10.3). The card waits in its player's
// resolving zone for the whole window — Hearthstone's stack — so nothing that reaches a hand (a
// discard, a hand count, a steal out of the hand) reaches it (R448). This module is only the record
// of which announces are open, read by every module that must know: the Counter verb
// (`effects/move.counterPlay`), the trap dispatch and the trigger queue (a response to an announce
// that a Counter has already cancelled finds no card: a trap stays set, a trigger fizzles), and
// `viewFor` and the AI's redaction (a card being set face-down is read by its player alone while it
// waits). It is a leaf: it reads and writes `state.announcing` and nothing else.

import type { PlayerId } from "@jackioh/shared";
import type { AnnounceRecord, CardInstance, GameState } from "./state";

/** Every open announce, outermost first (a cast a responder makes announces inside the window). */
export function openAnnounces(state: GameState): readonly AnnounceRecord[] {
  return state.announcing ?? [];
}

/** The open announce of this card, or undefined when its window is closed or never opened. */
export function announceOf(state: GameState, instanceId: string): AnnounceRecord | undefined {
  return openAnnounces(state).find((record) => record.instanceId === instanceId);
}

/**
 * R448: whether this card's play is still announced and uncancelled — what a Counter can still
 * answer. False once a Counter has cancelled it (the rest find no card and stay set), and once
 * step 4 has moved it (the window is over).
 */
export function isAnnounceLive(state: GameState, instanceId: string): boolean {
  const record = announceOf(state, instanceId);
  return record !== undefined && record.countered !== true;
}

/** The innermost announce still live: what a Counter that names no card answers. */
export function innermostLiveAnnounce(state: GameState): AnnounceRecord | undefined {
  const open = openAnnounces(state);
  for (let at = open.length - 1; at >= 0; at -= 1) {
    const record = open[at];
    if (record !== undefined && record.countered !== true) return record;
  }
  return undefined;
}

/** Open a card's announce (§10.5 between steps 3 and 4). */
export function beginAnnounce(state: GameState, record: AnnounceRecord): void {
  state.announcing = [...openAnnounces(state).filter((open) => open.instanceId !== record.instanceId), record];
}

/**
 * Close a card's announce once its window is over, and return its record. The field goes when the
 * last one closes, so a state with no window open hashes as it did before the record existed.
 */
export function endAnnounce(state: GameState, instanceId: string): AnnounceRecord | undefined {
  const record = announceOf(state, instanceId);
  const rest = openAnnounces(state).filter((open) => open.instanceId !== instanceId);
  if (rest.length === 0) delete state.announcing;
  else state.announcing = rest;
  return record;
}

/** R448: a Counter cancelled this announce. Returns false when there was nothing live to cancel. */
export function markCountered(state: GameState, instanceId: string): boolean {
  const record = announceOf(state, instanceId);
  if (record === undefined || record.countered === true) return false;
  record.countered = true;
  return true;
}

/**
 * R448, R97, R227: a card waiting in the resolving zone to be set face-down is read by its player
 * alone, as the face-down card it is about to be; a card being played face-up is public there (R98).
 */
export function announcedFaceDownTo(state: GameState, card: CardInstance, viewer: PlayerId): boolean {
  if (card.zone.z !== "resolving") return false;
  const record = announceOf(state, card.id);
  return record !== undefined && record.faceDown === true && record.player !== viewer;
}
