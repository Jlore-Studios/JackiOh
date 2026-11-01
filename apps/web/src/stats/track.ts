// R639 logs only cards the viewer could read (SPEC §10.8, R97; CLAUDE.md rule 7). R1582000 adds Allure's
// takings, lethal blows, Traps fired and survivors; R1743000 the damage, biggest hit, overkill, attacks,
// healing, draws, Radiant changes and turns; R2648000: a death counts only when the other seat caused it,
// a Lifesteal hit credits its heal. R2649000 adds the game's own mechanics, each read from the viewer's seat
// (R97) and, where it needs what a card was, from the view before the batch.

import { CATALOG } from "@jackioh/cards";
import {
  hasKeyword,
  opponentOf,
  type CardView,
  type GameEvent,
  type Keyword,
  type PlayerId,
  type PlayerView,
  type SideView,
  type UnitView,
} from "@jackioh/shared";

import { ALLURE_MARK, markEventOf } from "../cards/marks.ts";
import { HIDDEN_ID } from "../game/animations.ts";
import { printedCost } from "../game/decks.ts";
import { FROZEN_MARK } from "./config.ts";
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

/**
 * R2649000: the mana a play of the viewer's cost under its printed cost (`printedCost`: an X card's is 0, an
 * embiggen card's its base); 0 for a play with an X, at its embiggen price, cast on draw, or of an unknown card.
 */
function manaSavedBy(event: Extract<GameEvent, { type: "cardPlayed" }>, id: string): number {
  const def = CATALOG[id];
  if (def === undefined || event.x !== undefined || event.embiggened === true || event.onDraw === true) return 0;
  return Math.max(0, printedCost(def.cost) - event.costPaid);
}

/**
 * Count plays, draws, Radiant changes, turns and the viewer's Traps fired by the viewer-relative counter.
 * R2649000 adds Flashback plays (R783012) and the mana they saved, crumbles (Brittle, R638), plays
 * countered, cards burned from a full hand (R4) and bounced, and Starship pieces launched (R1852003).
 */
export function countEvent(log: GameLog, event: GameEvent, viewer: PlayerView["viewer"]): GameLog {
  if (event.type === "cardPlayed") {
    const id = readable(event.defId);
    if (id === null) return log;
    if (event.player !== viewer) return logWith(log, "playedAgainst", id);
    let next = logWith(log, "played", id);
    if (event.from === "graveyard") next = logWith(next, "flashback", id);
    const saved = manaSavedBy(event, id);
    return saved > 0 ? logWith(next, "manaSaved", id, saved) : next;
  }
  if (event.type === "crumbled") {
    const id = readable(event.defId);
    return id === null ? log : logWith(log, event.owner === viewer ? "crumbled" : "crumbledAgainst", id);
  }
  if (event.type === "countered") {
    const id = readable(event.defId);
    return id === null || event.player !== viewer ? log : logWith(log, "countered", id);
  }
  if (event.type === "burned" || event.type === "bounced") {
    const id = readable(event.defId);
    return id === null || event.owner !== viewer ? log : logWith(log, event.type, id);
  }
  if (event.type === "starshipLaunched") {
    if (event.player !== viewer) return log;
    let next = log;
    for (const piece of event.pieces) {
      const id = readable(piece);
      if (id !== null) next = logWith(next, "launched", id);
    }
    return next;
  }
  // R1582000: a Trap the viewer controlled fired; another's reads as the sentinel to them (R97). A
  // face-up Field Trap firing again reveals nothing (R2657001) but still fires, so it counts each time.
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

/** R2649000: the readable definition of the viewer's card `instanceId` names, where this view shows it, else where the view before did. */
function ownDefId(view: PlayerView, before: PlayerView | null, instanceId: string | null): string | null {
  return defIdOn(before === null ? [view.you] : [view.you, before.you], instanceId);
}

/** R2649000: the card `side` shows under `instanceId`; none for no id or the sentinel (R97). */
function cardOn(side: SideView, instanceId: string | null): CardView | undefined {
  if (instanceId === null || instanceId === HIDDEN_ID) return undefined;
  return cardsOnSide(side).find((card) => card.instanceId === instanceId);
}

/** R2649000: the unit on `side`'s field (a carried one included) under `instanceId`; none for no id or the sentinel. */
function unitOn(side: SideView, instanceId: string | null): UnitView | undefined {
  if (instanceId === null || instanceId === HIDDEN_ID) return undefined;
  return [...side.units, ...(side.carried ?? [])].find((unit): unit is UnitView => unit !== null && unit.instanceId === instanceId);
}

/** R2649000: a card's keywords as the view shows them; a hand card that sets none has its catalog face's printed ones (R243). */
function keywordsOf(card: CardView): readonly Keyword[] {
  return card.keywords ?? CATALOG[card.defId]?.[card.radiant ? "radiant" : "base"].keywords ?? [];
}

/**
 * R1743000, R2648000: the viewer's card an event makes the source of what follows it in the batch (their
 * play, activation, Trap, attacker or damage dealer). Null for the opponent's event or a card the view does
 * not show; undefined for an event that sets none.
 */
function sourceAfter(event: GameEvent, view: PlayerView): string | null | undefined {
  if (event.type === "cardPlayed" || event.type === "activated") return event.player === view.viewer ? readable(event.defId) : null;
  if (event.type === "trapFired") return event.controller === view.viewer ? readable(event.defId) : null;
  if (event.type === "attackDeclared") return defIdOn([view.you], event.attackerId);
  if (event.type === "damage") return defIdOn([view.you], event.sourceId);
  return undefined;
}

/** R2648000: the seat an event acts for (a play, an activation, a Trap, an attack); undefined for one that acts for none. */
function actorOf(event: GameEvent, views: readonly (PlayerView | null)[]): PlayerId | null | undefined {
  if (event.type === "cardPlayed" || event.type === "activated") return event.player;
  if (event.type === "trapFired") return event.controller;
  if (event.type === "attackDeclared") return seatOf(views, event.attackerId);
  return undefined;
}

/**
 * R1582000: a card of the viewer's the opponent's Allure took; the steal and the `allure` mark leaving it
 * are one batch. A face-down Trap taken reads as the sentinel (R33) and counts nothing, nor a steal with no mark.
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
 * seat across from its owner caused it: its `killerId` names a card on that side, or that seat acted last
 * before it in the batch (a destroy Spell names no killer, R89). Never after a `crumbled` of the same card
 * (Brittle, R638), nor when the owner's side acted last with no killer of the other's (a Tribute).
 */
export function countDeaths(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const views = [before, view];
  const crumbled = new Set<string>();
  let actor: PlayerId | null = null;
  let next = log;
  for (const event of fresh) {
    const acted = actorOf(event, views);
    if (acted !== undefined) {
      actor = acted;
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
 * R1582000: at the `gameOver` of a game the viewer won, each Unit on their side survived; if the opponent's
 * hero died, the viewer's card that dealt it the last damage dealt the lethal blow. `gameOver` comes once.
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

/** The health of the first hero or unit that `targetId` names; null otherwise, the sentinel included. */
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
 * R1743000: the viewer's cards' attacks, damage, biggest hit and overkill in one batch; overkill reads the
 * target's health in `before` less earlier hits, none without it. An opposing or hidden source counts nothing.
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
 * R1743000: `healed` names no source, so a heal counts toward the viewer's card that acted last before it in
 * the batch, and toward nothing after the opponent's or a hidden card. R2648000: the viewer's attacker or
 * damage dealer becomes the source, so a Lifesteal hit's heal is its card's. R2649000 (Vampire): the first
 * heal after a `damage` by a card that had Lifesteal in the view before also counts as drained.
 */
export function countHealing(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null = null): GameLog {
  let next = log;
  let source: string | null = null;
  let drain = false;
  for (const event of fresh) {
    const after = sourceAfter(event, view);
    if (after !== undefined) {
      source = after;
      const card = event.type === "damage" && after !== null && before !== null ? cardOn(before.you, event.sourceId) : undefined;
      drain = card !== undefined && hasKeyword(keywordsOf(card), "Lifesteal");
    } else if (event.type === "healed" && event.amount > 0 && source !== null) {
      next = logWith(next, "healing", source, event.amount);
      if (drain) next = logWith(next, "lifesteal", source, event.amount);
      drain = false;
    }
  }
  return next;
}

/**
 * R2649000: what the viewer's Units took: a `divineShieldLost` counts a shield popped (Bubble Wrap); a
 * `damage` its amount (Punching Bag) and what Armor took off (`absorbed`, R1360); a `damageAbsorbed` the hit
 * Armor took whole (R1361) (Iron Wall). The card is named where this view shows it, else the view before.
 */
export function countTaken(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  let next = log;
  for (const event of fresh) {
    if (event.type === "divineShieldLost") {
      const id = ownDefId(view, before, event.instanceId);
      if (id !== null) next = logWith(next, "shieldsLost", id);
    } else if (event.type === "damage") {
      const id = ownDefId(view, before, event.targetId);
      if (id === null) continue;
      if (event.amount > 0) next = logWith(next, "damageTaken", id, event.amount);
      if (event.absorbed !== undefined && event.absorbed > 0) next = logWith(next, "armorAbsorbed", id, event.absorbed);
    } else if (event.type === "damageAbsorbed" && event.absorbed > 0) {
      const id = ownDefId(view, before, event.targetId);
      if (id !== null) next = logWith(next, "armorAbsorbed", id, event.absorbed);
    }
  }
  return next;
}

/**
 * R2649000 (Martyr): a death of the viewer's card that R2648000 counts for neither seat because the viewer
 * gave it up: the opponent neither its killer nor the last to act before it, and the viewer's side its killer
 * or the batch's actor, the last before it or else the first after it (a Tribute's death precedes its play,
 * R119). Never after a `crumbled` of the same card (Brittle, R638), nor with no actor and no killer.
 */
export function countSacrificed(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const views = [before, view];
  const foe = opponentOf(view.viewer);
  const crumbled = new Set<string>();
  const waiting: string[] = [];
  let actor: PlayerId | null | undefined;
  let next = log;
  for (const event of fresh) {
    const acted = actorOf(event, views);
    if (acted !== undefined) {
      if (actor === undefined && acted === view.viewer) for (const id of waiting) next = logWith(next, "sacrificed", id);
      actor = acted;
    } else if (event.type === "crumbled") {
      crumbled.add(event.instanceId);
    } else if (event.type === "destroyed" && event.owner === view.viewer) {
      const id = readable(event.defId);
      const killer = seatOf(views, event.killerId);
      if (id === null || crumbled.has(event.instanceId) || killer === foe || actor === foe) continue;
      if (killer === view.viewer || actor === view.viewer) next = logWith(next, "sacrificed", id);
      else if (actor === undefined) waiting.push(id);
    }
  }
  return next;
}

/**
 * R2649000: the opponent's cards the viewer's cards killed. A `destroyed` of the opponent's card whose
 * `killerId` names the viewer's card (in this view, else the view before: the killer may die in the same
 * combat) counts a kill (Executioner) and may raise the killer's biggest kill to the victim's `maxHealth`
 * (Giant Slayer). A crumble (R638) is no kill, nor is a victim the sentinel hides (R97). A victim Frozen in
 * the view before was shattered (Shatter) by its killer, or with no `killerId` by the viewer's card that
 * acted last before it; an opponent's killer or actor shatters nothing.
 */
export function countKills(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const crumbled = new Set<string>();
  let actorCard: string | null = null;
  let next = log;
  for (const event of fresh) {
    if (event.type === "cardPlayed" || event.type === "activated") {
      actorCard = event.player === view.viewer ? readable(event.defId) : null;
    } else if (event.type === "trapFired") {
      actorCard = event.controller === view.viewer ? readable(event.defId) : null;
    } else if (event.type === "attackDeclared") {
      actorCard = ownDefId(view, before, event.attackerId);
    } else if (event.type === "crumbled") {
      crumbled.add(event.instanceId);
    } else if (event.type === "destroyed" && event.owner !== view.viewer && readable(event.defId) !== null && !crumbled.has(event.instanceId)) {
      const killer = ownDefId(view, before, event.killerId);
      if (killer !== null) next = logMax(logWith(next, "kills", killer), "biggestKill", killer, event.maxHealth);
      const victim = before === null ? undefined : (unitOn(before.opponent, event.instanceId) ?? unitOn(before.you, event.instanceId));
      const by = event.killerId === null ? actorCard : killer;
      if (victim?.frozen === true && by !== null) next = logWith(next, "shattered", by);
    }
  }
  return next;
}

/**
 * R2649000 (Double Tap): a Unit of the viewer's second `attackDeclared` in one turn (Windfury, R636). The log
 * keeps this turn's attacks by instance id (`attackers`, never stored), emptied at each `turnStarted`.
 */
export function countDoubleAttacks(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  let next = log;
  for (const event of fresh) {
    if (event.type === "turnStarted") {
      if (Object.keys(next.attackers).length > 0) next = { ...next, attackers: {} };
    } else if (event.type === "attackDeclared") {
      const id = ownDefId(view, before, event.attackerId);
      if (id === null) continue;
      const times = (next.attackers[event.attackerId] ?? 0) + 1;
      next = { ...next, attackers: { ...next.attackers, [event.attackerId]: times } };
      if (times === 2) next = logWith(next, "doubleAttacks", id);
    }
  }
  return next;
}

/**
 * R2649000: the freezes and thaws (`marked` with the `frozen` mark, R1174000: `added` a freeze, else a thaw).
 * A freeze on a Unit the opponent's side shows (in the view before, else this one) counts for the viewer's
 * card R2648000's source rule names (Cold Snap); a freeze on the viewer's Unit counts for it (Brain Freeze);
 * a thaw of the viewer's Unit still on the field counts for it (Spring Thaw), a death never.
 */
export function countFreezes(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  const foe = opponentOf(view.viewer);
  let source: string | null = null;
  let next = log;
  for (const event of fresh) {
    const after = sourceAfter(event, view);
    if (after !== undefined) {
      source = after;
      continue;
    }
    const change = markEventOf(event);
    if (change === null || change.mark !== FROZEN_MARK) continue;
    if (change.added) {
      if (source !== null && seatOf([before, view], change.instanceId) === foe) next = logWith(next, "froze", source);
      const id = ownDefId(view, before, change.instanceId);
      if (id !== null) next = logWith(next, "frozen", id);
    } else {
      const unit = unitOn(view.you, change.instanceId);
      const id = unit === undefined ? null : readable(unit.defId);
      if (id !== null) next = logWith(next, "thawed", id);
    }
  }
  return next;
}

/**
 * R2649000: the viewer's cards that were Temporary in their hand in the view before (R637, R2179001). One
 * discarded after the viewer's `turnEnded` and before a later `turnStarted` (the cleanup's discard, R637)
 * slipped away (Use It or Lose It); one played, by its hand id (R227's `formerId` when the play set it
 * face-down), was played in time (Just in Time). Without that view, or a hand it shows, nothing counts.
 */
export function countTemporary(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  if (before === null || !Array.isArray(before.you.hand)) return log;
  const hand: readonly CardView[] = before.you.hand;
  const temporary = new Set(hand.filter((card) => hasKeyword(keywordsOf(card), "Temporary")).map((card) => card.instanceId));
  if (temporary.size === 0) return log;
  let ended = false;
  let next = log;
  for (const event of fresh) {
    if (event.type === "turnEnded" && event.player === view.viewer) {
      ended = true;
    } else if (event.type === "turnStarted") {
      ended = false;
    } else if (event.type === "discarded" && ended && event.owner === view.viewer && temporary.has(event.instanceId)) {
      const id = readable(event.defId);
      if (id !== null) next = logWith(next, "tempExpired", id);
    } else if (event.type === "cardPlayed" && event.player === view.viewer && temporary.has(event.formerId ?? event.instanceId)) {
      const id = readable(event.defId);
      if (id !== null) next = logWith(next, "tempPlayed", id);
    }
  }
  return next;
}

/**
 * R2649000: the viewer's trades (R1838001: `traded` names no card, and the card it shuffles in is hidden from
 * the trader too). The first `drawn` of theirs after each `traded` was pulled from a trade (Trade-In Value).
 * For each `traded`, one Tradable card of their hand in the view before that this view shows nowhere was
 * traded away (Hot Potato), in hand order; with no view before, none.
 */
export function countTrades(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  let trades = 0;
  let owed = 0;
  let next = log;
  for (const event of fresh) {
    if (event.type === "traded" && event.player === view.viewer) {
      trades += 1;
      owed += 1;
    } else if (event.type === "drawn" && event.player === view.viewer && owed > 0) {
      owed -= 1;
      const id = readable(event.defId);
      if (id !== null) next = logWith(next, "tradeDrawn", id);
    }
  }
  if (trades === 0 || before === null || !Array.isArray(before.you.hand)) return next;
  const hand: readonly CardView[] = before.you.hand;
  const gone = hand.filter(
    (card) => card.tradable === true && cardOn(view.you, card.instanceId) === undefined && cardOn(view.opponent, card.instanceId) === undefined,
  );
  for (const card of gone.slice(0, trades)) {
    const id = readable(card.defId);
    if (id !== null) next = logWith(next, "traded", id);
  }
  return next;
}

/**
 * R2649000 (Second Wind): a `destroyed` of the viewer's Unit that had Reborn in the view before, then a
 * `summoned` of theirs of the same definition later in the batch, each summon taking one such death.
 */
export function countReborn(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  if (before === null) return log;
  const dying: string[] = [];
  let next = log;
  for (const event of fresh) {
    if (event.type === "destroyed" && event.owner === view.viewer) {
      const unit = unitOn(before.you, event.instanceId);
      const id = unit === undefined ? null : readable(unit.defId);
      if (unit !== undefined && id !== null && hasKeyword(unit.keywords, "Reborn")) dying.push(id);
    } else if (event.type === "summoned" && event.player === view.viewer) {
      const at = dying.indexOf(event.defId);
      if (at < 0) continue;
      dying.splice(at, 1);
      next = logWith(next, "reborn", event.defId);
    }
  }
  return next;
}

/**
 * R2649000: a Feng Shui's judgement of the viewer's play (R983), counted for the card it names where the
 * view shows it, good or bad; and a `countered` of the opponent's play whose `byInstanceId` is the viewer's (Nope).
 */
export function countFengShuiAndCounters(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null): GameLog {
  let next = log;
  for (const event of fresh) {
    if (event.type === "fengShui" && event.player === view.viewer) {
      const id = ownDefId(view, before, event.instanceId);
      if (id !== null) next = logWith(next, event.outcome === "positive" ? "fengShuiGood" : "fengShuiBad", id);
    } else if (event.type === "countered" && event.player !== view.viewer) {
      const id = ownDefId(view, before, event.byInstanceId);
      if (id !== null) next = logWith(next, "counterspells", id);
    }
  }
  return next;
}

/**
 * R2649000 (Mass Produced): each Created card (R943's `created`) this view shows on the viewer's side, once
 * per instance in a game (`createdIds`, never stored), a fresh id it takes (R227's `formerId`) included. The
 * seat holding it is credited, not R2089000's `creator`, which no view forwards; nothing else counts.
 */
export function countCreated(log: GameLog, view: PlayerView, fresh: readonly GameEvent[] = []): GameLog {
  const counted = new Set(log.createdIds);
  const known = counted.size;
  // R227: a card set face-down or changing control takes a fresh id but is the same card, so its new
  // id joins the ones counted when its `formerId` is among them, in the batch's order.
  for (const event of fresh) {
    if (event.type !== "cardPlayed" && event.type !== "summoned" && event.type !== "controlChanged") continue;
    if (event.formerId !== undefined && counted.has(event.formerId)) counted.add(event.instanceId);
  }
  let next = log;
  for (const card of cardsOnSide(view.you)) {
    const id = readable(card.defId);
    if (card.created !== true || id === null || counted.has(card.instanceId)) continue;
    counted.add(card.instanceId);
    next = logWith(next, "created", id);
  }
  return next === log && counted.size === known ? log : { ...next, createdIds: [...counted] };
}

/**
 * Views carry a sliding event window (SPEC §10.8), so callers supply only fresh events. `before` is the view
 * the batch followed (R1743000's overkill and R2649000's counters read what a card was in it), null for the first.
 */
export function observe(log: GameLog, view: PlayerView, fresh: readonly GameEvent[], before: PlayerView | null = null): GameLog {
  let next = countCreated(logSeen(log, seenIn(view)), view, fresh);
  for (const event of fresh) next = countEvent(next, event, view.viewer);
  next = countAllured(next, view, fresh);
  next = countDeaths(next, view, fresh, before);
  next = countSacrificed(next, view, fresh, before);
  next = countHits(next, view, fresh, before);
  next = countHealing(next, view, fresh, before);
  next = countTaken(next, view, fresh, before);
  next = countKills(next, view, fresh, before);
  next = countDoubleAttacks(next, view, fresh, before);
  next = countFreezes(next, view, fresh, before);
  next = countTemporary(next, view, fresh, before);
  next = countTrades(next, view, fresh, before);
  next = countReborn(next, view, fresh, before);
  next = countFengShuiAndCounters(next, view, fresh, before);
  return countWin(next, view, fresh);
}

export function outcomeOfView(view: PlayerView): GameOutcome | null {
  return view.result === null ? null : outcomeFor(view.result.winner, view.viewer, view.result.reason);
}
