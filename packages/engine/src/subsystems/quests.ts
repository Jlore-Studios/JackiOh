// Quests (docs/classic-sets.md B5 E33): the machinery of Classic #90 In Too Deep (SPEC §8.6 row 90,
// §10.1, §10.6, §10.8; R404). The TREE — which quests there are, what completes each, which rewards
// each offers and where each reward leads — is data the card's script declares (`Script.quests`, a
// `QuestBook`); the REWARDS are ordinary verbs in the card file, and the card's own trigger answers
// `questCompleted` with them. This module is what no card file may do: keep the count.
//
// STATE (§10.1). Everything lives on the card's instance, `memory.quest` (`QuestMemory`): the open
// quests, each counted quest's progress, the completed quests, the auras held (rewards L and M), and
// the quests opened by a reward whose opening the event stream has not reached yet (`waiting`). Plain
// JSON, so a paused state survives a round trip and a log folds to the same hash (§9.3). R78 resets
// memory as a card leaves the field, which is the whole of "leaving the field resets the quest line":
// nothing here has to watch for it.
//
// COUNTING, "from the moment it opens" (R404). A count moves as the resolution loop dispatches an event
// (`observeQuestEvent`, called once at the top of `triggers.dispatchEvent`), and the loop dispatches
// events in the order they were emitted — so "after the quest opened" is "after its opening in the
// event stream":
//   - a quest a reward opens is `waiting` until its own opening report (`questProgressed`, emitted as
//     it opens) is dispatched: the events emitted before it in the same action — reward H's own two
//     draws before quest 9 opens — are dispatched first, and count for nothing;
//   - the first quest opens as the card enters the field: at the first dispatch or state check that
//     finds the card on the field with no quest memory, and it is counting at once. R212 keeps an
//     event from before the card's arrival away from it: a card that has moved since an event (its
//     `cardPlayed`, its `summoned` comes after the event) does not answer it.
// The side a card stood on when an event happened is read as R212 reads it, off what happened since
// (`sideWhen`): `destroyed` names the controller a card died under (`destroyed.controller`), a
// draw that took the last card of the drawer's deck says so (`drawn.emptied`).
//
// COMPLETION is noticed at the state check (`noticeQuests`, called once at the end of
// `stateCheck.stateCheck`, when the board has settled): a counted quest whose count reached its goal,
// or a board quest whose condition holds now (3, 6, 10 — read off the board there, never counted), is
// completed and reported by `questCompleted`, which the card's trigger answers — a `reward` prompt for
// its controller on the base face (§10.6), every reward on the Radiant face. A quest reached by two
// paths opens once (`openQuest` opens a quest only if it was never opened), and a reward two
// completed quests offer is granted by each (each completion runs its own rewards).
//
// THE VIEW (§10.8, R404): `questViewOf` — the open quests, their progress and the rewards on offer,
// and the auras held — on every view of the card, which is a face-up Field Spell both players read.

import type { GameEvent, PlayerId, QuestView } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { defOf } from "../catalog";
import { cardTypeOf } from "../faces";
import { unitView } from "../layers";
import type { EngineSink } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { scriptOf } from "../scripts";
import { findInstance, type CardInstance, type GameState } from "../state";
import { movesIn, type LaterMoves } from "../stays";
import { activeUnitsOf, cardAt, isBuried, slotsOf } from "../zones";
import { selfOnItsStay } from "../effects/targets";

// ---------------------------------------------------------------------------
// The tree, as a card declares it
// ---------------------------------------------------------------------------

/**
 * What completes a quest (R404's countable readings). Counted goals move as events are dispatched;
 * board goals are read off the board at each state check.
 *
 * - `draws`: draws of yours — every draw that took a card, one burned at the hand cap or cast on draw
 *   included; a draw a limit stopped never happened, and a fatigue draw takes no card (R541).
 * - `enemyPermanentsDestroyed`: permanents your opponent controlled as they were destroyed, by anything.
 * - `unspentManaAtTurnEnd`: a turn of yours ended with at least `mana` unspent (goal 1).
 * - `damageToEnemies`: damage your cards dealt to your opponent's hero and the units they controlled.
 * - `cardsExiled`: cards entering either exile pile (a unit token ceases to exist instead, R11).
 * - `deckEmptiedByDraw`: a draw of yours took the last card of your deck (goal 1); a deck already empty
 *   when the quest opens completes it at once.
 * - board: `permanentsControlled` (the tops of your unit piles and your backrow cards, this one
 *   included), `unitTotals` (your Units' total attack and total health both at least `total`),
 *   `unitsInGraveyard` (Units in your graveyard).
 */
export type QuestGoal =
  | { kind: "draws"; count: number }
  | { kind: "enemyPermanentsDestroyed"; count: number }
  | { kind: "unspentManaAtTurnEnd"; mana: number }
  | { kind: "damageToEnemies"; amount: number }
  | { kind: "cardsExiled"; count: number }
  | { kind: "deckEmptiedByDraw" }
  | { kind: "permanentsControlled"; count: number }
  | { kind: "unitTotals"; total: number }
  | { kind: "unitsInGraveyard"; count: number };

/** One quest: its text as the view shows it, what completes it, and the rewards it offers, in order. */
export type QuestDef = { id: string; text: string; goal: QuestGoal; rewards: readonly string[] };

/** One reward: its text, and the quest it opens next (null at the end of a line). */
export type QuestRewardDef = { id: string; text: string; next: string | null };

/** A card's quest tree (`Script.quests`): the quest that opens as it enters, every quest, every reward. */
export type QuestBook = { first: string; quests: readonly QuestDef[]; rewards: readonly QuestRewardDef[] };

/** The goals the state check reads off the board rather than counting. */
const BOARD_GOALS: readonly QuestGoal["kind"][] = ["permanentsControlled", "unitTotals", "unitsInGraveyard"];

function isBoardGoal(goal: QuestGoal): boolean {
  return BOARD_GOALS.includes(goal.kind);
}

/** The number a quest's progress is shown against ("1/2"). A goal met once is 1. */
export function questGoalOf(quest: QuestDef): number {
  switch (quest.goal.kind) {
    case "draws":
    case "enemyPermanentsDestroyed":
    case "cardsExiled":
    case "permanentsControlled":
    case "unitsInGraveyard":
      return quest.goal.count;
    case "damageToEnemies":
      return quest.goal.amount;
    case "unitTotals":
      return quest.goal.total;
    case "unspentManaAtTurnEnd":
    case "deckEmptiedByDraw":
      return 1;
  }
}

/** The quest tree a card's running face declares, or null (a Vanilla card has none, R115). */
export function questBookOf(card: CardInstance): QuestBook | null {
  return scriptOf(card).quests ?? null;
}

export function questDefOf(book: QuestBook, id: string): QuestDef | null {
  return book.quests.find((quest) => quest.id === id) ?? null;
}

export function questRewardOf(book: QuestBook, id: string): QuestRewardDef | null {
  return book.rewards.find((reward) => reward.id === id) ?? null;
}

// ---------------------------------------------------------------------------
// The quest line on the instance (§10.1)
// ---------------------------------------------------------------------------

/** Where the quest line lives on the instance (`memory.quest`, §10.1). */
export const QUEST_MEMORY_KEY = "quest";

/** `memory.quest`. All JSON. */
export type QuestMemory = {
  /** The open quests, in the order they opened. */
  active: string[];
  /** Each counted quest's count so far, capped at its goal. */
  progress: Record<string, number>;
  /** The completed quests, in the order they were completed. */
  done: string[];
  /** The auras held (rewards L and M), in the order they were granted. */
  auras: string[];
  /** Quests a reward opened whose opening report the loop has not dispatched yet: not counting. */
  waiting: string[];
};

function stringList(raw: unknown): string[] {
  return Array.isArray(raw) ? raw.filter((item): item is string => typeof item === "string") : [];
}

/** The card's quest line, read back defensively (it may have come through JSON), or null. */
export function questMemoryOf(card: CardInstance): QuestMemory | null {
  const raw: unknown = card.memory[QUEST_MEMORY_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const held = raw as Partial<Record<keyof QuestMemory, unknown>>;
  const progress: Record<string, number> = {};
  if (held.progress !== null && typeof held.progress === "object") {
    for (const [id, value] of Object.entries(held.progress as Record<string, unknown>)) {
      if (typeof value === "number" && Number.isFinite(value)) progress[id] = value;
    }
  }
  return {
    active: stringList(held.active),
    progress,
    done: stringList(held.done),
    auras: stringList(held.auras),
    waiting: stringList(held.waiting),
  };
}

function writeMemory(card: CardInstance, memory: QuestMemory): void {
  card.memory[QUEST_MEMORY_KEY] = memory;
}

/** The open quests of a card, in the order they opened. */
export function openQuestsOf(card: CardInstance): readonly string[] {
  return [...(questMemoryOf(card)?.active ?? [])];
}

/** The quests a card has completed, in order. */
export function completedQuestsOf(card: CardInstance): readonly string[] {
  return [...(questMemoryOf(card)?.done ?? [])];
}

/** The auras a card's quest line holds (rewards L and M): a pure read for its `aura` and `graveyardPlay`. */
export function heldQuestAuras(card: CardInstance): readonly string[] {
  return [...(questMemoryOf(card)?.auras ?? [])];
}

/** The card acting where it is on the field: the top of its pile, or its backrow card (§3.2, R13). */
function actsOnField(state: GameState, card: CardInstance): boolean {
  return card.zone.z === "field" && !isBuried(state, card);
}

// ---------------------------------------------------------------------------
// The board goals, read now
// ---------------------------------------------------------------------------

function backrowCardsOf(state: GameState, player: PlayerId): CardInstance[] {
  return slotsOf(player, "backrow").flatMap((ref) => {
    const card = cardAt(state, ref);
    return card === null ? [] : [card];
  });
}

/** Quest 3: the permanents `player` controls now — the tops of their unit piles and their backrow cards. */
function permanentsControlled(state: GameState, player: PlayerId): number {
  return activeUnitsOf(state, player).length + backrowCardsOf(state, player).length;
}

/** Quest 6: the lower of `player`'s Units' total attack and total health, through the layers (§10.4). */
function unitTotals(state: GameState, player: PlayerId): number {
  let attack = 0;
  let health = 0;
  for (const unit of activeUnitsOf(state, player)) {
    const view = unitView(state, unit);
    attack += view.attack;
    health += Math.max(0, view.health);
  }
  return Math.min(attack, health);
}

/** Quest 10: the Units in `player`'s graveyard, each by the type it has there (B2.7). */
function unitsInGraveyard(state: GameState, player: PlayerId): number {
  return state.players[player].graveyard.filter((card) => cardTypeOf(state, card) === "Unit").length;
}

function boardValue(state: GameState, player: PlayerId, goal: QuestGoal): number {
  switch (goal.kind) {
    case "permanentsControlled":
      return permanentsControlled(state, player);
    case "unitTotals":
      return unitTotals(state, player);
    case "unitsInGraveyard":
      return unitsInGraveyard(state, player);
    default:
      return 0;
  }
}

/** A quest's progress now: its count, or what the board comes to, capped at its goal. */
export function questProgressOf(state: GameState, card: CardInstance, quest: QuestDef): number {
  const goal = questGoalOf(quest);
  const value = isBoardGoal(quest.goal)
    ? boardValue(state, card.controller, quest.goal)
    : (questMemoryOf(card)?.progress[quest.id] ?? 0);
  return Math.max(0, Math.min(goal, value));
}

// ---------------------------------------------------------------------------
// Opening a quest
// ---------------------------------------------------------------------------

type Sink = Pick<EngineSink, "state" | "events">;

function report(sink: Sink, card: CardInstance, quest: QuestDef, progress: number): void {
  sink.events.push({
    type: "questProgressed",
    player: card.controller,
    instanceId: card.id,
    quest: quest.id,
    progress,
    goal: questGoalOf(quest),
  });
}

/**
 * Open `id` on `card` unless it was ever opened (a quest reached by two paths opens once, R404).
 * `live`: counting at once (the first quest, as the card enters); otherwise it waits for its own
 * opening report to be dispatched. Quest 9's "a deck already empty when the quest opens completes it
 * at once" is its count met as it opens.
 */
function openOn(sink: Sink, card: CardInstance, id: string, live: boolean): void {
  const book = questBookOf(card);
  const quest = book === null ? null : questDefOf(book, id);
  if (quest === null) return;
  const memory = questMemoryOf(card) ?? { active: [], progress: {}, done: [], auras: [], waiting: [] };
  if (memory.active.includes(id) || memory.done.includes(id)) return;
  memory.active.push(id);
  if (!isBoardGoal(quest.goal)) {
    const emptyDeck = quest.goal.kind === "deckEmptiedByDraw" && sink.state.players[card.controller].library.length === 0;
    memory.progress[id] = emptyDeck ? questGoalOf(quest) : 0;
  }
  if (!live) memory.waiting.push(id);
  writeMemory(card, memory);
  report(sink, card, quest, questProgressOf(sink.state, card, quest));
}

/** The first quest of a card on the field that has no quest line yet: it opens as the card enters. */
function openFirstIfNew(sink: Sink, card: CardInstance): void {
  if (questMemoryOf(card) !== null) return;
  const book = questBookOf(card);
  if (book === null) return;
  writeMemory(card, { active: [], progress: {}, done: [], auras: [], waiting: [] });
  openOn(sink, card, book.first, true);
}

/**
 * Open the running card's quest `id` — a reward's next quest (R404). Counting from the moment it
 * opens: the events emitted before it in this action do not count (`QuestMemory.waiting`). Nothing
 * for a card that has left the field since its run began (R174: its quest line went with it, R78), for
 * a quest the tree does not have, or for one already opened.
 */
export function openQuest(id: string): Effect {
  return {
    kind: "openQuest",
    apply(ctx): void {
      const self = questCardOf(ctx);
      if (self === null) return;
      openOn(ctx, self, id, false);
    },
  };
}

/**
 * Hold a reward's aura on the running card (In Too Deep's L and M): it holds while the card is on the
 * field, and leaving the field ends it (R78). The card's own `aura` and `graveyardPlay` hooks read it
 * back (`heldQuestAuras`).
 */
export function holdQuestAura(rewardId: string): Effect {
  return {
    kind: "holdQuestAura",
    apply(ctx): void {
      const self = questCardOf(ctx);
      if (self === null) return;
      const memory = questMemoryOf(self);
      if (memory === null || memory.auras.includes(rewardId)) return;
      memory.auras.push(rewardId);
      writeMemory(self, memory);
    },
  };
}

/** The running card, on the field on the stay its run began with (R174), with a quest tree. */
function questCardOf(ctx: EffectContext): CardInstance | null {
  const self = selfOnItsStay(ctx);
  if (self === null || !actsOnField(ctx.state, self) || questBookOf(self) === null) return null;
  return self;
}

// ---------------------------------------------------------------------------
// Counting, as the loop dispatches each event
// ---------------------------------------------------------------------------

/** Every card with a quest tree acting on the field, in R68's order: the active side first. */
function questCardsInOrder(state: GameState): CardInstance[] {
  const sides: PlayerId[] = state.active === "p1" ? ["p1", "p2"] : ["p2", "p1"];
  return sides.flatMap((player) =>
    [...activeUnitsOf(state, player), ...backrowCardsOf(state, player)].filter((card) => questBookOf(card) !== null),
  );
}

function heroSide(id: string): PlayerId | null {
  return PLAYER_IDS.find((player) => id === `hero-${player}`) ?? null;
}

/**
 * R212: the side a card or hero stood on when an event happened, read off the events that followed
 * it — a change of control since hands back the controller before it, and a card destroyed since died
 * under the controller its `destroyed` names — and otherwise off the card where it is now: on the
 * field its controller, resolving its player, in a pile its owner. Null for a card gone with nothing
 * to say (a replaced card).
 */
function sideWhen(state: GameState, id: string, after: readonly GameEvent[]): PlayerId | null {
  const hero = heroSide(id);
  if (hero !== null) return hero;
  for (const event of after) {
    if (event.type === "controlChanged" && event.instanceId === id) return opponentOf(event.controller);
    if (event.type === "destroyed" && event.instanceId === id) return event.controller;
  }
  const card = findInstance(state, id);
  if (card !== undefined) {
    if (card.zone.z === "field") return card.controller;
    if (card.zone.z === "resolving") return card.zone.player;
    return card.owner;
  }
  // A unit token that has ceased to exist is in no pile (R11): its leaving named its owner.
  for (const event of after) {
    if ((event.type === "bounced" || event.type === "exiled") && event.instanceId === id) return event.owner;
  }
  return null;
}

/** A unit token ceases to exist rather than entering an exile pile (R11), so it is not an exiled card. */
function enteredExile(state: GameState, event: Extract<GameEvent, { type: "exiled" }>): boolean {
  const def = defOf(state, event.defId);
  return !(def.token && def.type === "Unit");
}

/** What one event adds to a quest of `goal` for a card controlled by `me`, or 0. */
function creditOf(state: GameState, goal: QuestGoal, event: GameEvent, me: PlayerId, after: () => readonly GameEvent[]): number {
  switch (goal.kind) {
    case "draws":
      return event.type === "drawn" && event.player === me ? 1 : 0;
    case "deckEmptiedByDraw":
      return event.type === "drawn" && event.player === me && event.emptied === true ? 1 : 0;
    case "enemyPermanentsDestroyed":
      return event.type === "destroyed" && (event.controller ?? event.owner) === opponentOf(me) ? 1 : 0;
    case "unspentManaAtTurnEnd":
      return event.type === "turnEnded" && event.player === me && event.unspentMana >= goal.mana ? 1 : 0;
    case "cardsExiled":
      return event.type === "exiled" && enteredExile(state, event) ? 1 : 0;
    case "damageToEnemies": {
      if (event.type !== "damage" || event.amount <= 0 || event.sourceId === null) return 0;
      const later = after();
      if (sideWhen(state, event.sourceId, later) !== me) return 0;
      return sideWhen(state, event.targetId, later) === opponentOf(me) ? event.amount : 0;
    }
    default:
      return 0;
  }
}

/** The event types a counted goal reads, so an event no quest counts costs no board read. */
const COUNTED_EVENTS: readonly GameEvent["type"][] = ["drawn", "destroyed", "turnEnded", "exiled", "damage"];

/**
 * §10.3, R404: one event reaching the resolution loop, before the traps and the triggers see it. A
 * quest a reward opened starts counting once its own opening report is dispatched; a card on the
 * field with no quest line opens its first quest; then every open, counting quest of every card on
 * the field that the event counts for moves, and says so (`questProgressed`). `after` is the events
 * that followed this one (`triggers.eventsAfterDispatched`), read only when a quest card is on the
 * field: a card that has moved since the event did not see it (R212).
 */
export function observeQuestEvent(sink: EngineSink, event: GameEvent, after: () => readonly GameEvent[]): void {
  const cards = questCardsInOrder(sink.state);
  if (cards.length === 0) return;

  if (event.type === "questProgressed") {
    const card = cards.find((candidate) => candidate.id === event.instanceId);
    const memory = card === undefined ? null : questMemoryOf(card);
    if (card !== undefined && memory !== null && memory.waiting.includes(event.quest)) {
      writeMemory(card, { ...memory, waiting: memory.waiting.filter((id) => id !== event.quest) });
    }
    return;
  }

  let events: readonly GameEvent[] | null = null;
  const later = (): readonly GameEvent[] => (events ??= after());
  let moves: LaterMoves | null = null;
  for (const card of cards) {
    moves ??= movesIn(later(), sink.state);
    // R212: a card that arrived after the event, or came back since, is on a stay that did not see it.
    if (moves.moved.has(card.id)) continue;
    openFirstIfNew(sink, card);
    if (!COUNTED_EVENTS.includes(event.type)) continue;
    const book = questBookOf(card);
    const memory = questMemoryOf(card);
    if (book === null || memory === null) continue;
    const me = moves.controllerBefore.get(card.id) ?? card.controller;
    let changed = false;
    for (const id of memory.active) {
      if (memory.waiting.includes(id)) continue;
      const quest = questDefOf(book, id);
      if (quest === null || isBoardGoal(quest.goal)) continue;
      const goal = questGoalOf(quest);
      const before = memory.progress[id] ?? 0;
      if (before >= goal) continue;
      const credit = creditOf(sink.state, quest.goal, event, me, later);
      if (credit <= 0) continue;
      const progress = Math.min(goal, before + credit);
      memory.progress[id] = progress;
      changed = true;
      report(sink, card, quest, progress);
    }
    if (changed) writeMemory(card, memory);
  }
}

// ---------------------------------------------------------------------------
// Completion, at the state check
// ---------------------------------------------------------------------------

function complete(state: GameState, card: CardInstance, quest: QuestDef, memory: QuestMemory): boolean {
  if (isBoardGoal(quest.goal)) return boardValue(state, card.controller, quest.goal) >= questGoalOf(quest);
  return (memory.progress[quest.id] ?? 0) >= questGoalOf(quest);
}

/**
 * §4.5, R404: the state check has settled the board, so every card with a quest tree on the field opens
 * its first quest if it has none yet, and every open quest whose count reached its goal or whose board
 * condition holds now is completed — moved to `done` and reported by `questCompleted`, in R68's order
 * and each card's quests in the order they opened. The card's trigger answers the report.
 */
export function noticeQuests(sink: EngineSink): void {
  for (const card of questCardsInOrder(sink.state)) {
    openFirstIfNew(sink, card);
    const book = questBookOf(card);
    const memory = questMemoryOf(card);
    if (book === null || memory === null) continue;
    const completed = memory.active.filter((id) => {
      const quest = questDefOf(book, id);
      return quest !== null && complete(sink.state, card, quest, memory);
    });
    if (completed.length === 0) continue;
    writeMemory(card, {
      ...memory,
      active: memory.active.filter((id) => !completed.includes(id)),
      done: [...memory.done, ...completed],
      waiting: memory.waiting.filter((id) => !completed.includes(id)),
    });
    for (const quest of completed) {
      sink.events.push({ type: "questCompleted", player: card.controller, instanceId: card.id, quest });
    }
  }
}

// ---------------------------------------------------------------------------
// The view (§10.8)
// ---------------------------------------------------------------------------

/**
 * R404, §10.8: the open quests with their progress and the rewards on offer, and the auras held, or
 * null for a card with no quest line. Everything here is the tree's own text and public counts, so it
 * rides on every view of the card (a face-up Field Spell both players read).
 */
export function questViewOf(state: GameState, card: CardInstance): QuestView | null {
  const book = questBookOf(card);
  const memory = questMemoryOf(card);
  if (book === null || memory === null || !actsOnField(state, card)) return null;
  const rewardText = (id: string): string => questRewardOf(book, id)?.text ?? id;
  return {
    open: memory.active.flatMap((id) => {
      const quest = questDefOf(book, id);
      if (quest === null) return [];
      return [
        {
          id,
          text: quest.text,
          progress: questProgressOf(state, card, quest),
          goal: questGoalOf(quest),
          rewards: quest.rewards.map((reward) => ({ id: reward, text: rewardText(reward) })),
        },
      ];
    }),
    auras: memory.auras.map((id) => ({ id, text: rewardText(id) })),
  };
}
