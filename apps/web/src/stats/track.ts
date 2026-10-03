// From views to a game's log (SPEC R639): which cards the player saw, played, met, lost and took down.
//
// The inputs are what the client already holds and nothing more: a `PlayerView` (its cards and its
// events are redacted for this viewer, SPEC §10.8, R97) and the events that arrived since the last one.
// A card the view hides carries the sentinel `"hidden"` in place of its definition, and a face-down
// backrow card carries none, so a statistic never names a card the player was not allowed to read
// (CLAUDE.md rule 7). Nothing here decides a rule: it counts what the engine said happened.

import type { CardView, GameEvent, PlayerView, SideView } from "@jackioh/shared";

import { HIDDEN_ID } from "../game/animations.ts";
import { logSeen, logWith, outcomeFor, type GameLog, type GameOutcome } from "./model.ts";

/** A definition the viewer may read, or null for the redaction sentinel. */
function readable(defId: string): string | null {
  return defId === HIDDEN_ID ? null : defId;
}

function cardsOnSide(side: SideView): CardView[] {
  const out: CardView[] = [];
  if (Array.isArray(side.hand)) out.push(...side.hand);
  out.push(...side.graveyard, ...side.exile, ...side.resolving);
  for (const unit of side.units) if (unit !== null) out.push(unit);
  for (const unit of side.carried ?? []) if (unit !== null) out.push(unit);
  for (const card of side.backrow) if (card !== null && !card.faceDown) out.push(card);
  return out;
}

/**
 * Every definition in front of the viewer in this view: their own hand, both boards, both
 * graveyards and exile piles, and what is resolving. The opponent's hand and library, a face-down
 * trap and the viewer's own library are not in front of them, so they are not here.
 */
export function seenIn(view: PlayerView): string[] {
  const ids = new Set<string>();
  for (const card of [...cardsOnSide(view.you), ...cardsOnSide(view.opponent)]) {
    const id = readable(card.defId);
    if (id !== null) ids.add(id);
  }
  return [...ids];
}

/**
 * The log with one event counted. `cardPlayed` is a play (the viewer's own, or one they watched the
 * opponent make); `destroyed` is a copy of the viewer's own lost, or an opponent's they saw fall.
 */
export function countEvent(log: GameLog, event: GameEvent, viewer: PlayerView["viewer"]): GameLog {
  if (event.type === "cardPlayed") {
    const id = readable(event.defId);
    if (id === null) return log;
    return logWith(log, event.player === viewer ? "played" : "playedAgainst", id);
  }
  if (event.type === "destroyed") {
    const id = readable(event.defId);
    if (id === null) return log;
    return logWith(log, event.owner === viewer ? "destroyed" : "defeated", id);
  }
  return log;
}

/**
 * The log after a new view: the cards the view shows, and the events that are new with it. The
 * caller works out which events are new (`newEventsSince`, as the board's animation queue does),
 * since a view carries a sliding window of the match's events (SPEC §10.8), not an action's delta.
 */
export function observe(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  let next = logSeen(log, seenIn(view));
  for (const event of fresh) next = countEvent(next, event, view.viewer);
  return next;
}

/** How the finished game reads from the viewer's seat, or null while it is still on. */
export function outcomeOfView(view: PlayerView): GameOutcome | null {
  return view.result === null ? null : outcomeFor(view.result.winner, view.viewer);
}
