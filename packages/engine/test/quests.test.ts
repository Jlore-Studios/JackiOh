// The quests subsystem (docs/classic-sets.md B5 E33; `src/subsystems/quests.ts`; SPEC §8.6 row 90,
// §10.1, §10.6, §10.8; R404), proved through the fixture cards of `fixtures/quests.ts` so the engine
// half of Classic #90 In Too Deep stands without `packages/cards`: each goal counted and not counted,
// counting from the moment a quest opens, completion at the state check on either player's turn, the
// base face's `reward` prompt and the Radiant face's every reward and every path, a quest reached by
// two paths, a reward two quests offer, an aura held while the card stands, a pause mid-reward through
// JSON, a replay from the log, both views, and the two random picks the rewards needed
// (`effects/randomPicks.ts`). The real card's own test file proves the same with its real tree.

import type { Action, CardView, GameEvent, PlayerId } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { DECK_SIZE, HAND_CAP, HERO_HEALTH } from "../src/config";
import { unitView } from "../src/layers";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createGame, findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { fuse } from "../src/subsystems/fuse";
import { questBookOf, questMemoryOf, type QuestMemory } from "../src/subsystems/quests";
import { viewFor } from "../src/viewFor";
import { vanillaDeck } from "./fixtures/catalog";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, newGame, put, setLibrary, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import { answerKeys, must, openAs, roundTrip } from "./fixtures/promptHarness";
import {
  BOLT,
  GOAL_CARDS,
  GOALS,
  ONLY_QUEST,
  RECALL,
  TREE_BUFF,
  TREE_HEAL,
  TREE,
  TREE_PING,
  banish,
  banishAny,
  blank,
  bless,
  bolt,
  drawOne,
  drawThenRecruit,
  drawTwo,
  limiter,
  mill,
  recall,
  registerQuestFixtures,
  slay,
  tree,
  type GoalKind,
} from "./fixtures/quests";
import { cnVirus } from "./fixtures/scripts";

const RUSH_TOKEN = "fx-token-rush";

/** p1's main phase on turn 3 with 4 mana each side, the quest fixtures registered. */
function questBoard(seed: string): GameState {
  const state = newGame(seed);
  registerQuestFixtures();
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  state.players.p1.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  state.players.p2.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  // R345: each turn ends when the test ends it, never on its own (R82).
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

type Step = { state: GameState; events: GameEvent[] };

let nonce = 0;
function act(state: GameState, body: Record<string, unknown>, log?: Action[]): Step {
  nonce += 1;
  const action = { ...body, nonce: `qt${nonce}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(`${action.type} refused: ${result.error}`);
  log?.push(action);
  return { state: result.state, events: result.events };
}

/** Play a card from its owner's hand, with declared targets. */
function play(state: GameState, card: CardInstance, targets: string[] = [], log?: Action[]): Step {
  return act(
    state,
    {
      type: "play",
      playerId: card.owner,
      instanceId: card.id,
      targets: targets.map((id) => (id.startsWith("hero-") ? { pick: "hero", player: id.slice(5) } : { pick: "instance", instanceId: id })),
    },
    log,
  );
}

function hand(state: GameState, defId: string, player: PlayerId = "p1"): CardInstance {
  return must(inHand(state, defId, player)[0], defId);
}

/** A quest card played by p1 into the backrow: its first quest opens as it enters. */
function playQuestCard(state: GameState, defId: string, player: PlayerId = "p1"): Step & { card: CardInstance } {
  const card = hand(state, defId, player);
  const step = play(state, card);
  return { ...step, card: must(findInstance(step.state, card.id), "the quest card") };
}

function memoryOf(state: GameState, card: CardInstance): QuestMemory {
  return must(questMemoryOf(must(findInstance(state, card.id), "the card")), "a quest line");
}

function progressOf(state: GameState, card: CardInstance, quest = ONLY_QUEST): number {
  return memoryOf(state, card).progress[quest] ?? 0;
}

function goal(state: GameState, kind: GoalKind, player: PlayerId = "p1"): Step & { card: CardInstance } {
  return playQuestCard(state, GOAL_CARDS[kind].id, player);
}

function completed(events: readonly GameEvent[], card: CardInstance): string[] {
  return eventsOfType(events, "questCompleted").filter((e) => e.instanceId === card.id).map((e) => e.quest);
}

/** The card view of a face-up backrow card as `viewer` is shown it. */
function shownTo(state: GameState, viewer: PlayerId, card: CardInstance): CardView | undefined {
  const view = viewFor(state, viewer);
  const side = view.you.player === card.controller ? view.you : view.opponent;
  for (const entry of side.backrow) {
    if (entry !== null && !entry.faceDown && entry.instanceId === card.id) return entry;
  }
  return undefined;
}

function endTurn(state: GameState, player: PlayerId = state.active): Step {
  return act(state, { type: "endTurn", playerId: player });
}

describe("E33 quests: the first quest opens as the card enters (R404)", () => {
  it("R404 a played quest card opens its first quest, reported at 0, and counts from then on", () => {
    const state = questBoard("q-open");
    inHand(state, plain.id, "p1", 1);
    const { state: after, events, card } = goal(state, "draws");
    expect(memoryOf(after, card)).toEqual({ active: [ONLY_QUEST], progress: { [ONLY_QUEST]: 0 }, done: [], auras: [], waiting: [] });
    expect(eventsOfType(events, "questProgressed")).toEqual([
      { type: "questProgressed", player: "p1", instanceId: card.id, quest: ONLY_QUEST, progress: 0, goal: GOALS.draws.count },
    ]);
    // The play's own events came before the opening: the quest has counted nothing.
    expect(progressOf(after, card)).toBe(0);
  });

  it("R404 a card placed on the field without a play opens at the first event or check after it, counting what follows", () => {
    const state = questBoard("q-placed");
    const card = put(state, GOAL_CARDS.draws.id, slot("p1", "backrow", 1));
    expect(questMemoryOf(card)).toBeNull();
    const { state: after, events } = play(state, hand(state, drawTwo.id));
    expect(completed(events, card)).toEqual([ONLY_QUEST]);
    expect(memoryOf(after, card).done).toEqual([ONLY_QUEST]);
  });

  it("R212 a quest card recruited after two draws in the same list does not count them", () => {
    const state = questBoard("q-recruit");
    setLibrary(state, "p1", [plain.id, plain.id, GOAL_CARDS.draws.id, plain.id]);
    const { state: after } = play(state, hand(state, drawThenRecruit.id));
    const card = must(after.players.p1.backrow.find((c) => c?.defId === GOAL_CARDS.draws.id), "the recruited card");
    expect(memoryOf(after, card).active).toEqual([ONLY_QUEST]);
    expect(progressOf(after, card)).toBe(0);
  });
});

describe("E33 quests: each goal counted, and not counted", () => {
  it("R541 draws: your draws count, a burned one and a cast-on-draw one included; the opponent's do not", () => {
    let state = questBoard("q-draws");
    setLibrary(state, "p1", [cnVirus.id, plain.id, plain.id, plain.id]);
    let { state: s1, card } = goal(state, "draws");
    // One draw that casts the CN-Virus and draws on (R58): two draws.
    const step = play(s1, hand(s1, drawOne.id));
    expect(eventsOfType(step.events, "drawn").filter((e) => e.player === "p1")).toHaveLength(2);
    expect(completed(step.events, card)).toEqual([ONLY_QUEST]);

    // A burned draw: the hand is full.
    state = questBoard("q-draws-burn");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    ({ state: s1, card } = goal(state, "draws"));
    inHand(s1, plain.id, "p1", HAND_CAP - 1);
    const burned = play(s1, hand(s1, drawOne.id));
    expect(eventsOfType(burned.events, "burned")).toHaveLength(0);
    inHand(burned.state, plain.id, "p1", 1);
    const full = play(burned.state, hand(burned.state, drawOne.id));
    // The hand held 10 when the draw came: the card burned, and it was a draw all the same.
    expect(eventsOfType(full.events, "burned")).toHaveLength(1);
    expect(completed(full.events, card)).toEqual([ONLY_QUEST]);

    // The opponent's draws: p2's turn starts with a draw, which counts for nothing of p1's.
    state = questBoard("q-draws-theirs");
    ({ state: s1, card } = goal(state, "draws"));
    inHand(s1, plain.id, "p2", 1);
    const theirs = endTurn(s1);
    expect(eventsOfType(theirs.events, "drawn").some((e) => e.player === "p2")).toBe(true);
    expect(progressOf(theirs.state, card)).toBe(0);
  });

  it("R541 draws: a draw a limit stopped never happened, and a fatigue draw takes no card", () => {
    let state = questBoard("q-draws-limit");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    put(state, limiter.id, slot("p1", "backrow", 2));
    let { state: s1, card } = goal(state, "draws");
    const limited = play(s1, hand(s1, drawTwo.id));
    expect(eventsOfType(limited.events, "drawLimited")).toHaveLength(1);
    expect(progressOf(limited.state, card)).toBe(1);

    state = questBoard("q-draws-fatigue");
    setLibrary(state, "p1", []);
    ({ state: s1, card } = goal(state, "draws"));
    const tired = play(s1, hand(s1, drawTwo.id));
    expect(eventsOfType(tired.events, "fatigue")).toHaveLength(2);
    expect(progressOf(tired.state, card)).toBe(0);
  });

  it("R404 deckEmptiedByDraw: the draw that takes your deck's last card; a mill that empties it does not; an empty deck at the opening completes it at once", () => {
    let state = questBoard("q-deck");
    setLibrary(state, "p1", [plain.id, plain.id]);
    let { state: s1, card } = goal(state, "deckEmptiedByDraw");
    const one = play(s1, hand(s1, drawOne.id));
    expect(eventsOfType(one.events, "drawn")[0]?.emptied).toBeUndefined();
    expect(progressOf(one.state, card)).toBe(0);
    const last = play(one.state, hand(one.state, drawOne.id));
    expect(eventsOfType(last.events, "drawn")[0]?.emptied).toBe(true);
    expect(completed(last.events, card)).toEqual([ONLY_QUEST]);

    state = questBoard("q-deck-mill");
    setLibrary(state, "p1", [plain.id]);
    ({ state: s1, card } = goal(state, "deckEmptiedByDraw"));
    const milled = play(s1, hand(s1, mill.id));
    expect(milled.state.players.p1.library).toHaveLength(0);
    expect(progressOf(milled.state, card)).toBe(0);
    // ... and the fatigue draw after it takes no card either.
    const tired = play(milled.state, hand(milled.state, drawOne.id));
    expect(memoryOf(tired.state, card).done).toEqual([]);

    state = questBoard("q-deck-empty");
    setLibrary(state, "p1", []);
    const opened = goal(state, "deckEmptiedByDraw");
    expect(eventsOfType(opened.events, "questProgressed")[0]?.progress).toBe(1);
    expect(completed(opened.events, opened.card)).toEqual([ONLY_QUEST]);
  });

  it("R404 enemyPermanentsDestroyed: by anything, counted on the side it died on (`destroyed.controller`)", () => {
    const state = questBoard("q-kills");
    const theirs = put(state, plain.id, slot("p2", "units", 1));
    const mine = put(state, plain.id, slot("p1", "units", 1));
    // A unit p2 owns that p1 controls (stolen), and one p1 owns that p2 controls.
    const stolen = put(state, plain.id, slot("p1", "units", 2));
    stolen.owner = "p2";
    const lent = put(state, plain.id, slot("p2", "units", 2));
    lent.owner = "p1";
    const token = put(state, RUSH_TOKEN, slot("p2", "units", 3));
    const { state: s1, card } = goal(state, "enemyPermanentsDestroyed");

    const own = play(s1, hand(s1, slay.id), [mine.id]);
    expect(progressOf(own.state, card)).toBe(0);
    const steal = play(own.state, hand(own.state, slay.id), [stolen.id]);
    const died = eventsOfType(steal.events, "destroyed")[0];
    expect(died).toMatchObject({ owner: "p2", controller: "p1" });
    expect(progressOf(steal.state, card)).toBe(0);
    const lentStep = play(steal.state, hand(steal.state, slay.id), [lent.id]);
    expect(eventsOfType(lentStep.events, "destroyed")[0]).toMatchObject({ owner: "p1", controller: "p2" });
    expect(progressOf(lentStep.state, card)).toBe(1);
    // A token is a permanent too, and an ordinary death names its owner's side.
    const tok = play(lentStep.state, hand(lentStep.state, slay.id), [token.id]);
    expect(eventsOfType(tok.events, "destroyed")[0]?.controller).toBe("p2");
    expect(completed(tok.events, card)).toEqual([ONLY_QUEST]);
    expect(findInstance(tok.state, theirs.id)?.zone.z).toBe("field");
  });

  it("R404 unspentManaAtTurnEnd: your turn's end with enough mana left; not less, and not the opponent's", () => {
    let state = questBoard("q-mana");
    inHand(state, plain.id, "p1", 1);
    inHand(state, plain.id, "p2", 1);
    let { state: s1, card } = goal(state, "unspentManaAtTurnEnd");
    // 4 mana, 1 paid: 3 left.
    expect(s1.players.p1.mana.current).toBe(GOALS.unspentManaAtTurnEnd.mana);
    const ended = endTurn(s1);
    expect(completed(ended.events, card)).toEqual([ONLY_QUEST]);

    state = questBoard("q-mana-short");
    inHand(state, plain.id, "p1", 1);
    inHand(state, plain.id, "p2", 1);
    state.players.p1.mana.current = GOALS.unspentManaAtTurnEnd.mana;
    ({ state: s1, card } = goal(state, "unspentManaAtTurnEnd"));
    const short = endTurn(s1);
    expect(progressOf(short.state, card)).toBe(0);
    // p2's turn ends with 4 unspent: not p1's turn.
    expect(short.state.active).toBe("p2");
    short.state.players.p2.mana.current = 4;
    const theirs = endTurn(short.state);
    expect(eventsOfType(theirs.events, "turnEnded").find((e) => e.player === "p2")?.unspentMana).toBeGreaterThanOrEqual(3);
    expect(progressOf(theirs.state, card)).toBe(0);
  });

  it("R404 damageToEnemies: damage your cards deal to the enemy hero and enemy units; never to your side, never theirs", () => {
    const state = questBoard("q-damage");
    const target = put(state, plain.id, slot("p2", "units", 1));
    const mine = put(state, plain.id, slot("p1", "units", 1));
    const { state: s1, card } = goal(state, "damageToEnemies");
    const self = play(s1, hand(s1, bolt.id), ["hero-p1"]);
    const ownUnit = play(self.state, hand(self.state, bolt.id), [mine.id]);
    expect(progressOf(ownUnit.state, card)).toBe(0);
    const unit = play(ownUnit.state, hand(ownUnit.state, bolt.id), [target.id]);
    expect(progressOf(unit.state, card)).toBe(BOLT);
    const hero = play(unit.state, hand(unit.state, bolt.id), ["hero-p2"]);
    // 3 + 3 = 6, shown capped at the goal of 5.
    expect(progressOf(hero.state, card)).toBe(GOALS.damageToEnemies.amount);
    expect(completed(hero.events, card)).toEqual([ONLY_QUEST]);
  });

  it("R404 damageToEnemies: a unit token that dies in the combat still dealt its damage; the opponent's hits count for nothing", () => {
    const state = questBoard("q-damage-token");
    const token = put(state, RUSH_TOKEN, slot("p1", "units", 1));
    const wall = put(state, plain.id, slot("p2", "units", 1));
    wall.damage = 0;
    const { state: s1, card } = goal(state, "damageToEnemies");
    // The 3/3 Rush Token and the 3/3 trade: both die in the check after the combat, before dispatch.
    const fight = act(s1, { type: "attack", playerId: "p1", attackerId: token.id, targetId: wall.id });
    expect(findInstance(fight.state, token.id)).toBeUndefined();
    expect(progressOf(fight.state, card)).toBe(3);

    // The opponent's bolt on p1's hero, on the opponent's turn.
    inHand(fight.state, plain.id, "p1", 1);
    const turn = endTurn(fight.state);
    const theirs = play(turn.state, hand(turn.state, bolt.id, "p2"), ["hero-p1"]);
    expect(eventsOfType(theirs.events, "damage").some((e) => e.targetId === "hero-p1")).toBe(true);
    expect(progressOf(theirs.state, card)).toBe(3);
  });

  it("R404 cardsExiled: either player's card entering an exile pile; a unit token ceases to exist instead (R11)", () => {
    const state = questBoard("q-exile");
    const theirs = put(state, plain.id, slot("p2", "units", 1));
    const mine = put(state, plain.id, slot("p1", "units", 1));
    const token = put(state, RUSH_TOKEN, slot("p2", "units", 2));
    const { state: s1, card } = goal(state, "cardsExiled");
    const tok = play(s1, hand(s1, banish.id), [token.id]);
    expect(eventsOfType(tok.events, "exiled")).toHaveLength(1);
    expect(progressOf(tok.state, card)).toBe(0);
    const one = play(tok.state, hand(tok.state, banish.id), [theirs.id]);
    expect(progressOf(one.state, card)).toBe(1);
    const two = play(one.state, hand(one.state, banish.id), [mine.id]);
    expect(completed(two.events, card)).toEqual([ONLY_QUEST]);
  });

  it("R404 permanentsControlled: read off the board at the state check, this card included; the opponent's do not count", () => {
    const state = questBoard("q-board");
    put(state, plain.id, slot("p2", "units", 1));
    put(state, plain.id, slot("p2", "units", 2));
    put(state, plain.id, slot("p1", "units", 1));
    const { state: s1, events, card } = goal(state, "permanentsControlled");
    expect(completed(events, card)).toEqual([]);
    expect(shownTo(s1, "p1", card)?.quest?.open[0]).toMatchObject({ progress: 2, goal: 3 });
    const unit = hand(s1, plain.id);
    const placed = act(s1, { type: "play", playerId: "p1", instanceId: unit.id });
    expect(completed(placed.events, card)).toEqual([ONLY_QUEST]);
  });

  it("R404 unitTotals: your Units' total attack and total health at once, through the layers", () => {
    const state = questBoard("q-stats");
    put(state, plain.id, slot("p1", "units", 1));
    put(state, plain.id, slot("p2", "units", 1));
    const { state: s1, events, card } = goal(state, "unitTotals");
    expect(completed(events, card)).toEqual([]);
    const unit = hand(s1, plain.id);
    const placed = act(s1, { type: "play", playerId: "p1", instanceId: unit.id });
    expect(completed(placed.events, card)).toEqual([ONLY_QUEST]);
  });

  it("R404 unitsInGraveyard: Units in your graveyard, by type; a Spell there is not one", () => {
    const state = questBoard("q-grave");
    const a = put(state, plain.id, slot("p1", "units", 1));
    const { state: s1, card } = goal(state, "unitsInGraveyard");
    const first = play(s1, hand(s1, slay.id), [a.id]);
    // One Unit and a Slay (a Spell) lie there now.
    expect(first.state.players.p1.graveyard).toHaveLength(2);
    expect(memoryOf(first.state, card).done).toEqual([]);
    const b = put(first.state, plain.id, slot("p1", "units", 2));
    const second = play(first.state, hand(first.state, slay.id), [b.id]);
    expect(completed(second.events, card)).toEqual([ONLY_QUEST]);
  });
});

describe("E33 quests: the tree, base face (a reward prompt)", () => {
  function treeAfterFirstQuest(seed: string): { state: GameState; card: CardInstance } {
    const state = questBoard(seed);
    setLibrary(state, "p1", [plain.id, plain.id, plain.id, plain.id, plain.id]);
    const { state: s1, card } = playQuestCard(state, tree.id);
    const { state: s2 } = play(s1, hand(s1, drawTwo.id));
    return { state: s2, card };
  }

  it("R404 a completed quest is a `reward` prompt for its controller, its options the rewards on offer", () => {
    const { state, card } = treeAfterFirstQuest("t-prompt");
    const pending = openAs(state, "reward", "p1");
    expect(pending.options.map((o) => [o.key, o.label])).toEqual([
      ["mode:heal", "Heal your hero 2"],
      ["mode:ask", "Deal 1 damage to a target"],
    ]);
    expect(memoryOf(state, card)).toMatchObject({ active: [], done: ["draws"] });
    // The other seat sees that a prompt is open, never the options.
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

    const health = state.players.p1.hero.health;
    expect(answerKeys(state, "mode:heal").error).toBeNull();
    expect(state.players.p1.hero.health).toBe(health + TREE_HEAL);
    expect(memoryOf(state, card)).toMatchObject({ active: ["kill"], done: ["draws"], waiting: [] });
  });

  it("R404 a reward that asks a question pauses, survives a JSON round trip, and opens its quest after the answer", () => {
    const { state, card } = treeAfterFirstQuest("t-pause");
    const foe = put(state, plain.id, slot("p2", "units", 1));
    answerKeys(state, "mode:ask");
    const aim = openAs(state, "target", "p1");
    expect(aim.options.map((o) => o.key)).toContain(`instance:${foe.id}`);
    // Paused mid-reward: the quest it leads to is not open yet.
    expect(memoryOf(state, card).active).toEqual([]);

    const copy = roundTrip(state);
    expect(hashState(copy)).toBe(hashState(state));
    answerKeys(state, `instance:${foe.id}`);
    answerKeys(copy, `instance:${foe.id}`);
    expect(hashState(copy)).toBe(hashState(state));
    expect(findInstance(state, foe.id)?.damage).toBe(TREE_PING);
    expect(memoryOf(state, card).active).toEqual(["board"]);
  });

  it("R404 the next quest counts only what comes after it opens: a reward's own draws come first", () => {
    const state = questBoard("t-waiting");
    setLibrary(state, "p1", Array.from({ length: 8 }, () => plain.id));
    put(state, plain.id, slot("p1", "units", 1));
    const { state: s1, card } = playQuestCard(state, tree.id);
    // Quest 1 (draw 2) → "ask" → quest "board" (3 permanents: the tree, the unit, one more).
    let s = play(s1, hand(s1, drawTwo.id)).state;
    answerKeys(s, "mode:ask");
    answerKeys(s, "hero:p2");
    expect(memoryOf(s, card).active).toEqual(["board"]);
    s = play(s, hand(s, plain.id)).state;
    // "board" completes: choose "hand" (draw 2) → "fresh" (draw 2), which the reward's draws do not feed.
    openAs(s, "reward", "p1");
    answerKeys(s, "mode:hand");
    expect(memoryOf(s, card)).toMatchObject({ active: ["fresh"], waiting: [] });
    expect(progressOf(s, card, "fresh")).toBe(0);
    const more = play(s, hand(s, drawTwo.id));
    expect(completed(more.events, card)).toEqual(["fresh"]);
  });

  it("R404 completion is noticed on the other player's turn, and the prompt is the card's controller's", () => {
    const { state } = treeAfterFirstQuest("t-their-turn");
    answerKeys(state, "mode:heal");
    inHand(state, plain.id, "p1", 1);
    const theirUnit = put(state, plain.id, slot("p2", "units", 1));
    const turn = endTurn(state);
    expect(turn.state.active).toBe("p2");
    // p2 destroys its own unit: an enemy permanent of p1's, destroyed by anything.
    const killed = play(turn.state, hand(turn.state, slay.id, "p2"), [theirUnit.id]);
    const pending = openAs(killed.state, "reward", "p1");
    expect(pending.options.map((o) => o.key)).toEqual(["mode:both"]);
    expect(killed.state.active).toBe("p2");
  });

  it("R404 an aura reward holds while the card stands, and leaving the field ends it and resets the line (R78)", () => {
    const state = questBoard("t-aura");
    inHand(state, plain.id, "p2", 1);
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const card = put(state, tree.id, slot("p1", "backrow", 1));
    card.memory.quest = { active: ["mana"], progress: { mana: 0 }, done: ["draws", "kill"], auras: [], waiting: [] };
    const ended = endTurn(state);
    // The end of p1's turn with 4 unspent completes "mana": its one reward is still a prompt.
    openAs(ended.state, "reward", "p1");
    answerKeys(ended.state, "mode:aura");
    const s = ended.state;
    expect(memoryOf(s, card)).toMatchObject({ active: [], auras: ["aura"] });
    expect(hasKeyword(unitView(s, must(findInstance(s, unit.id), "unit")).keywords, "Indestructible")).toBe(true);
    // Both views show the quest line.
    for (const viewer of ["p1", "p2"] as const) {
      expect(shownTo(s, viewer, card)).toMatchObject({ quest: { open: [], auras: [{ id: "aura", text: "Aura: your Units have Indestructible" }] } });
    }

    // p2 exiles the tree on its turn: the aura goes with it, and the line resets.
    const gone = play(s, hand(s, banishAny.id, "p2"), [card.id]);
    expect(findInstance(gone.state, card.id)?.zone.z).toBe("exile");
    expect(questMemoryOf(must(findInstance(gone.state, card.id), "tree"))).toBeNull();
    expect(hasKeyword(unitView(gone.state, must(findInstance(gone.state, unit.id), "unit")).keywords, "Indestructible")).toBe(false);
  });
});

describe("E33 quests: a fused quest card carries its tree (R102)", () => {
  it("R102 a Fuse that keeps the quest card keeps its line, and the fusion carries the tree: it counts on and asks its reward", () => {
    const state = questBoard("t-fuse-kept");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    const card = put(state, tree.id, slot("p1", "backrow", 1));
    card.memory.quest = { active: ["draws"], progress: { draws: 1 }, done: [], auras: [], waiting: [] };
    const food = put(state, blank.id, slot("p1", "backrow", 2));

    const fused = must(fuse(sinkFor(state), { ingredients: [food], target: card }), "a fusion");
    expect([fused.id, fused.defId === tree.id]).toEqual([card.id, false]);
    expect(questBookOf(fused)?.quests.map((quest) => quest.id)).toEqual(TREE.quests.map((quest) => quest.id));
    expect(memoryOf(state, card)).toMatchObject({ active: ["draws"], progress: { draws: 1 } });

    const drawn = play(state, hand(state, drawOne.id));
    expect(completed(drawn.events, card)).toEqual(["draws"]);
    expect(openAs(drawn.state, "reward", "p1").options.map((o) => o.key)).toEqual(["mode:heal", "mode:ask"]);
    expect(answerKeys(drawn.state, "mode:heal").error).toBeNull();
    expect(memoryOf(drawn.state, card)).toMatchObject({ active: ["kill"], done: ["draws"] });
  });

  it("R102 a quest card fused onto another permanent gives the fusion its tree: the first quest opens there and counts", () => {
    const state = questBoard("t-fuse-ingredient");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    const card = put(state, tree.id, slot("p1", "backrow", 1));
    const kept = put(state, blank.id, slot("p1", "backrow", 2));

    const fused = must(fuse(sinkFor(state), { ingredients: [card], target: kept }), "a fusion");
    expect(questBookOf(fused)?.first).toBe(TREE.first);
    const drawn = play(state, hand(state, drawTwo.id));
    expect(completed(drawn.events, kept)).toEqual(["draws"]);
    openAs(drawn.state, "reward", "p1");
  });
});

describe("E33 quests: the tree, Radiant face (every reward, every path)", () => {
  it("R404 a completed quest grants every reward it offers, then opens every quest they lead to", () => {
    const state = questBoard("t-radiant");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    const card = hand(state, tree.id);
    card.radiant = true;
    const s1 = play(state, card).state;
    const health = s1.players.p1.hero.health;
    const s = play(s1, hand(s1, drawTwo.id)).state;
    // "heal" healed at once; "ask" asks its target with no reward prompt.
    expect(s.players.p1.hero.health).toBe(health + TREE_HEAL);
    openAs(s, "target", "p1");
    expect(memoryOf(s, card).active).toEqual([]);
    answerKeys(s, "hero:p2");
    expect(s.players.p2.hero.health).toBe(HERO_HEALTH - TREE_PING);
    expect(memoryOf(s, card).active).toEqual(["kill", "board"]);
  });

  it("R404 a quest reached by two paths opens once, and a reward two completed quests offer is granted by each", () => {
    const state = questBoard("t-two-paths");
    const unit = put(state, plain.id, slot("p1", "units", 1));
    put(state, plain.id, slot("p1", "units", 2));
    const foe = put(state, plain.id, slot("p2", "units", 1));
    const card = put(state, tree.id, slot("p1", "backrow", 1), { radiant: true });
    // Both paths of quest 1 are open, as the Radiant face leaves them.
    card.memory.quest = { active: ["kill", "board"], progress: { kill: 0 }, done: ["draws"], auras: [], waiting: [] };
    // The first check of the play finds 3 permanents ("board"), and the death it collects then
    // completes "kill": "both" granted by each, "mana" opened once, "hand" drawing 2 and opening "fresh".
    setLibrary(state, "p1", [plain.id, plain.id, plain.id, plain.id]);
    const step = play(state, hand(state, slay.id), [foe.id]);
    expect(completed(step.events, card)).toEqual(["board", "kill"]);
    expect(eventsOfType(step.events, "buffed")).toHaveLength(2);
    expect(eventsOfType(step.events, "buffed").every((e) => e.attack === TREE_BUFF && e.health === TREE_BUFF)).toBe(true);
    const memory = memoryOf(step.state, card);
    expect(memory.active.filter((id) => id === "mana")).toHaveLength(1);
    expect(memory.active).toEqual(["mana", "fresh"]);
    expect(findInstance(step.state, unit.id)).toBeDefined();
  });
});

describe("E33 quests: the view, hidden information and replay", () => {
  it("R404 both views carry the open quests, their progress and the rewards on offer; the events name the public card", () => {
    const state = questBoard("t-view");
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    const { state: s1, card } = playQuestCard(state, tree.id);
    const s = play(s1, hand(s1, drawOne.id)).state;
    for (const viewer of ["p1", "p2"] as const) {
      const view = viewFor(s, viewer);
      expect(shownTo(s, viewer, card)?.quest).toEqual({
        open: [
          {
            id: "draws",
            text: "Draw 2 cards",
            progress: 1,
            goal: 2,
            rewards: [
              { id: "heal", text: "Heal your hero 2" },
              { id: "ask", text: "Deal 1 damage to a target" },
            ],
          },
        ],
        auras: [],
      });
      for (const event of eventsOfType(view.events, "questProgressed")) expect(event.instanceId).toBe(card.id);
    }
  });

  it("R404 a game through a quest, its reward and the next quest folds from its log to the same hash (§9.2)", () => {
    setupCatalog();
    registerQuestFixtures();
    const decks: [string[], string[]] = [
      [...vanillaDeck(DECK_SIZE - 2, 1), tree.id, drawTwo.id],
      vanillaDeck(DECK_SIZE, 21),
    ];
    const seed = "t-replay";
    const log: Action[] = [];
    let state = beginGame(createGame({ seed, decks })).state;
    for (const player of ["p1", "p2"] as const) {
      state = act(state, { type: "mulligan", playerId: player, keep: state.players[player].hand.map((c) => c.id) }, log).state;
    }
    const treeCard = must(state.players.p1.hand.find((c) => c.defId === tree.id), "the tree in hand");
    state = play(state, treeCard, [], log).state;
    state = act(state, { type: "endTurn", playerId: "p1" }, log).state;
    state = act(state, { type: "endTurn", playerId: "p2" }, log).state;
    // p1's second turn drew a card: 1 of 2. Draw Two finishes it.
    expect(progressOf(state, treeCard, "draws")).toBe(1);
    const twice = must(state.players.p1.hand.find((c) => c.defId === drawTwo.id), "Draw Two in hand");
    state = play(state, twice, [], log).state;
    const pending = openAs(state, "reward", "p1");
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [{ pick: "mode", option: "heal" }] }, log).state;
    expect(memoryOf(state, treeCard).active).toEqual(["kill"]);

    const folded = fold({ seed, decks, log });
    expect(folded.errors).toEqual([]);
    expect(hashState(folded.state)).toBe(hashState(state));
  });
});

describe("E33 the random picks In Too Deep's rewards need (`effects/randomPicks.ts`, R60)", () => {
  it("R60 buffRandomUnit: one of your Units, drawn from the match rng; none, nothing", () => {
    const state = questBoard("rp-bless");
    const none = play(state, hand(state, bless.id));
    expect(eventsOfType(none.events, "buffed")).toEqual([]);
    const a = put(none.state, plain.id, slot("p1", "units", 1));
    const b = put(none.state, plain.id, slot("p1", "units", 2));
    put(none.state, plain.id, slot("p2", "units", 1));
    const one = play(none.state, hand(none.state, bless.id));
    const buffed = eventsOfType(one.events, "buffed");
    expect(buffed).toHaveLength(1);
    expect([a.id, b.id]).toContain(buffed[0]?.instanceId);
  });

  it("R60 returnRandomFromGraveyard: N different cards, fewer if fewer lie there, a full hand burning them back once each", () => {
    const state = questBoard("rp-recall");
    for (let i = 0; i < 3; i += 1) {
      const card = newInstance(state, plain.id, "p1", { z: "graveyard", player: "p1" });
      state.players.p1.graveyard.push(card);
    }
    const back = play(state, hand(state, recall.id));
    const added = eventsOfType(back.events, "addedToHand").filter((e) => e.defId === plain.id);
    expect(added).toHaveLength(RECALL);
    expect(new Set(added.map((e) => e.instanceId)).size).toBe(RECALL);

    const full = questBoard("rp-recall-full");
    for (let i = 0; i < 3; i += 1) {
      full.players.p1.graveyard.push(newInstance(full, plain.id, "p1", { z: "graveyard", player: "p1" }));
    }
    inHand(full, plain.id, "p1", HAND_CAP);
    const burned = play(full, hand(full, recall.id));
    const burns = eventsOfType(burned.events, "burned");
    expect(burns).toHaveLength(RECALL);
    expect(new Set(burns.map((e) => e.instanceId)).size).toBe(RECALL);
  });
});
