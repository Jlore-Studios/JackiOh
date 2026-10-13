// R639 logs only cards the viewer could read (SPEC §10.8, R97; CLAUDE.md rule 7).

import type { CardView, GameEvent, PlayerView, SideView } from "@jackioh/shared";

import { HIDDEN_ID } from "../game/animations.ts";
import { logSeen, logWith, outcomeFor, type GameLog, type GameOutcome } from "./model.ts";

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

/** Visible definitions only; never libraries, the opponent's hand, or face-down cards. */
export function seenIn(view: PlayerView): string[] {
  const ids = new Set<string>();
  for (const card of [...cardsOnSide(view.you), ...cardsOnSide(view.opponent)]) {
    const id = readable(card.defId);
    if (id !== null) ids.add(id);
  }
  return [...ids];
}

/** Count plays and destroyed cards by the viewer-relative counter. */
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

/** Views carry a sliding event window (SPEC §10.8), so callers supply only fresh events. */
export function observe(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  let next = logSeen(log, seenIn(view));
  for (const event of fresh) next = countEvent(next, event, view.viewer);
  return next;
}

export function outcomeOfView(view: PlayerView): GameOutcome | null {
  return view.result === null ? null : outcomeFor(view.result.winner, view.viewer);
}
