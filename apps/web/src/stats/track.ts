// R639 logs only cards the viewer could read (SPEC §10.8, R97; CLAUDE.md rule 7). R1582000 adds Allure's takings, lethal blows, Traps fired and survivors.

import type { CardView, GameEvent, PlayerView, SideView } from "@jackioh/shared";

import { ALLURE_MARK, markEventOf } from "../cards/marks.ts";
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

/** Count plays, destroyed cards and the viewer's Traps fired by the viewer-relative counter. */
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
  // R1582000: a Trap the viewer controlled fired; another's reads as the sentinel to them (R97).
  if (event.type === "trapFired") {
    const id = readable(event.defId);
    if (id === null || event.controller !== viewer) return log;
    return logWith(log, "sprung", id);
  }
  return log;
}

/** The readable definition of the card the first of `sides` that shows `instanceId` shows under it. */
function defIdOn(sides: readonly SideView[], instanceId: string | null): string | null {
  if (instanceId === null || instanceId === HIDDEN_ID) return null;
  for (const side of sides) {
    const found = cardsOnSide(side).find((card) => card.instanceId === instanceId);
    if (found !== undefined) return readable(found.defId);
  }
  return null;
}

/**
 * R1582000: a card of the viewer's that the opponent's Allure took. The steal (`controlChanged`,
 * `how: "steal"`, to the other seat) and the `allure` mark leaving that card come in one action's
 * events, so in one batch; the card is named where the view shows it now. A face-down Trap taken reads
 * as the sentinel (R33) and counts nothing, as does a steal no Allure mark left.
 */
export function countAllured(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  const lifted = new Set<string>();
  for (const event of fresh) {
    const change = markEventOf(event);
    if (change !== null && change.mark === ALLURE_MARK && !change.added) lifted.add(change.instanceId);
  }
  let next = log;
  for (const event of fresh) {
    if (event.type !== "controlChanged" || event.how !== "steal" || event.controller === view.viewer) continue;
    if (!lifted.has(event.instanceId)) continue;
    const id = defIdOn([view.opponent, view.you], event.instanceId);
    if (id !== null) next = logWith(next, "allured", id);
  }
  return next;
}

/**
 * R1582000: at the `gameOver` of a game the viewer won, each Unit on their side survived, a copy each;
 * and when the opponent's hero died, the card on the viewer's side that dealt it the last damage before
 * the end dealt the lethal blow. `gameOver` arrives once, so a game is counted once.
 */
export function countWin(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  const end = fresh.findIndex((event) => event.type === "gameOver");
  const over = fresh[end];
  if (over === undefined || over.type !== "gameOver" || over.winner !== view.viewer) return log;
  let next = log;
  for (const unit of view.you.units) {
    const id = unit === null ? null : readable(unit.defId);
    if (id !== null) next = logWith(next, "survived", id);
  }
  if (over.reason !== "hero-death") return next;
  const enemyHero = `hero-${view.opponent.player}`;
  const blow = fresh
    .slice(0, end)
    .findLast((event) => event.type === "damage" && event.targetId === enemyHero && event.amount > 0);
  if (blow === undefined || blow.type !== "damage") return next;
  const id = defIdOn([view.you], blow.sourceId);
  return id === null ? next : logWith(next, "lethal", id);
}

/** Views carry a sliding event window (SPEC §10.8), so callers supply only fresh events. */
export function observe(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  let next = logSeen(log, seenIn(view));
  for (const event of fresh) next = countEvent(next, event, view.viewer);
  return countWin(countAllured(next, view, fresh), view, fresh);
}

export function outcomeOfView(view: PlayerView): GameOutcome | null {
  return view.result === null ? null : outcomeFor(view.result.winner, view.viewer);
}
