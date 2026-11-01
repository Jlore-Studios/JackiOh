// R639 logs only cards the viewer could read (SPEC §10.8, R97; CLAUDE.md rule 7). R1582000 adds Allure's takings, lethal blows, Traps fired and survivors; R1743000 adds the damage, the biggest hit, the overkill, the attacks, the healing, the draws and the Radiant changes of the viewer's cards, and a game's turns. R2648000: a death counts only when the other seat caused it; a Lifesteal hit credits its heal.

import { opponentOf, type CardView, type GameEvent, type PlayerId, type PlayerView, type SideView } from "@jackioh/shared";

import { ALLURE_MARK, markEventOf } from "../cards/marks.ts";
import { HIDDEN_ID } from "../game/animations.ts";
import { logMax, logSeen, logWith, outcomeFor, type GameLog, type GameOutcome } from "./model.ts";

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

/** Count plays, draws, Radiant changes, turns and the viewer's Traps fired by the viewer-relative counter. */
export function countEvent(log: GameLog, event: GameEvent, viewer: PlayerView["viewer"]): GameLog {
  if (event.type === "cardPlayed") {
    const id = readable(event.defId);
    if (id === null) return log;
    return logWith(log, event.player === viewer ? "played" : "playedAgainst", id);
  }
  // R1582000: a Trap the viewer controlled fired; another's reads as the sentinel to them (R97).
  if (event.type === "trapFired") {
    const id = readable(event.defId);
    if (id === null || event.controller !== viewer) return log;
    return logWith(log, "sprung", id);
  }
  // R1743000: the viewer's own draws show the card; the opponent's read as the sentinel (R97).
  if (event.type === "drawn") {
    const id = readable(event.defId);
    return id === null || event.player !== viewer ? log : logWith(log, "drawn", id);
  }
  if (event.type === "radiantSet") {
    const id = readable(event.defId);
    return id === null || event.zone.player !== viewer ? log : logWith(log, "radiant", id);
  }
  if (event.type === "turnStarted") return event.turn > log.turns ? { ...log, turns: event.turn } : log;
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

/** R2648000: the seat whose side shows `instanceId` in the first of `views` that does; null for none or the sentinel (R97). */
function seatOf(views: readonly (PlayerView | null)[], instanceId: string | null): PlayerId | null {
  if (instanceId === null || instanceId === HIDDEN_ID) return null;
  for (const view of views) {
    if (view === null) continue;
    for (const side of [view.you, view.opponent]) {
      if (cardsOnSide(side).some((card) => card.instanceId === instanceId)) return side.player;
    }
  }
  return null;
}

/**
 * R2648000: a death counts (`destroyed` for the viewer's card, `defeated` for the opponent's) only when the
 * seat across from its owner caused it: its `killerId` names a card on that side (in the view before the
 * batch, else this one), or the last `cardPlayed`, `activated`, `trapFired` or `attackDeclared` before it in
 * the batch was that seat's (a destroy Spell names no killer, R89). Never after a `crumbled` of the same
 * card (Brittle, R638), nor when the owner's side acted last with no killer of the other's (a Tribute, a
 * Sacrifice, its own sweeper), nor with no actor and no killer.
 */
export function countDeaths(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const views = [before, view];
  const crumbled = new Set<string>();
  let actor: PlayerId | null = null;
  let next = log;
  for (const event of fresh) {
    if (event.type === "cardPlayed" || event.type === "activated") {
      actor = event.player;
    } else if (event.type === "trapFired") {
      actor = event.controller;
    } else if (event.type === "attackDeclared") {
      actor = seatOf(views, event.attackerId);
    } else if (event.type === "crumbled") {
      crumbled.add(event.instanceId);
    } else if (event.type === "destroyed") {
      const id = readable(event.defId);
      const foe = opponentOf(event.owner);
      if (id === null || crumbled.has(event.instanceId)) continue;
      if (seatOf(views, event.killerId) !== foe && actor !== foe) continue;
      next = logWith(next, event.owner === view.viewer ? "destroyed" : "defeated", id);
    }
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

/** The health the first of `side`'s hero and units that `targetId` names has; null for anything else, the sentinel included. */
function healthIn(view: PlayerView, targetId: string): number | null {
  if (targetId === HIDDEN_ID) return null;
  for (const side of [view.you, view.opponent]) {
    if (targetId === `hero-${side.player}`) return side.hero.health;
    for (const unit of [...side.units, ...(side.carried ?? [])]) {
      if (unit !== null && unit.instanceId === targetId) return unit.health;
    }
  }
  return null;
}

/**
 * R1743000: what the viewer's cards attacked and hit in one batch. An `attackDeclared` by a card on the
 * viewer's side counts an attack. A `damage` of one counts its amount, may raise the card's biggest hit
 * and, when it went past what its target had left, its overkill. The target's health is read from the
 * view before the batch (`before`) less the hits of the batch already walked; with no view before, or a
 * target the view does not show, there is no overkill. A hidden or opposing source counts nothing.
 */
export function countHits(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const left = new Map<string, number>();
  let next = log;
  for (const event of fresh) {
    if (event.type === "attackDeclared") {
      const id = defIdOn([view.you], event.attackerId);
      if (id !== null) next = logWith(next, "attacks", id);
    } else if (event.type === "damage" && event.amount > 0) {
      const health = left.get(event.targetId) ?? (before === null ? null : healthIn(before, event.targetId));
      if (health !== null) left.set(event.targetId, Math.max(0, health - event.amount));
      const id = defIdOn([view.you], event.sourceId);
      if (id === null) continue;
      next = logMax(logWith(next, "damageDealt", id, event.amount), "biggestHit", id, event.amount);
      if (health !== null && event.amount > health) next = logMax(next, "overkill", id, event.amount - health);
    }
  }
  return next;
}

/**
 * R1743000: `healed` names no source, so a heal counts toward the viewer's card whose `cardPlayed`,
 * `activated` or `trapFired` came last before it in the batch. With none, or one of the opponent's
 * (or a hidden card's), it counts nothing. R2648000: the viewer's attacker (`attackDeclared`), or the
 * viewer's card that dealt the damage (`damage`), becomes the source, so a Lifesteal hit's heal is its
 * card's; any other attack or damage clears it.
 */
export function countHealing(log: GameLog, view: PlayerView, fresh: readonly GameEvent[]): GameLog {
  let next = log;
  let source: string | null = null;
  for (const event of fresh) {
    if (event.type === "cardPlayed" || event.type === "activated") {
      source = event.player === view.viewer ? readable(event.defId) : null;
    } else if (event.type === "trapFired") {
      source = event.controller === view.viewer ? readable(event.defId) : null;
    } else if (event.type === "attackDeclared") {
      source = defIdOn([view.you], event.attackerId);
    } else if (event.type === "damage") {
      source = defIdOn([view.you], event.sourceId);
    } else if (event.type === "healed" && event.amount > 0 && source !== null) {
      next = logWith(next, "healing", source, event.amount);
    }
  }
  return next;
}

/**
 * Views carry a sliding event window (SPEC §10.8), so callers supply only fresh events. `before` is the
 * view the batch followed (R1743000's overkill reads the targets' health in it), null for the first.
 */
export function observe(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null = null): GameLog {
  let next = logSeen(log, seenIn(view));
  for (const event of fresh) next = countEvent(next, event, view.viewer);
  return countWin(
    countHealing(countHits(countDeaths(countAllured(next, view, fresh), view, fresh, before), view, fresh, before), view, fresh),
    view,
    fresh,
  );
}

export function outcomeOfView(view: PlayerView): GameOutcome | null {
  return view.result === null ? null : outcomeFor(view.result.winner, view.viewer, view.result.reason);
}
