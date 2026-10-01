// The engine half of patch v0.2.0's Core patches and cosmetics (docs/classic-sets.md B0, issue #40):
//
//   R429  §10.5 step 4 counts a card's plays on its instance when its script asks
//         (`StaticFlags.countsPlays`, `timesPlayed`), which #31 KY's Math Equation reads;
//   R433  `createGame` learns the seats that were dealt a deck, whose library lists as unknown;
//   R434  once the game is over, each view carries the other player's hand;
//   R437  a delayed effect aimed at a card marks it in both views while it waits (`marks.ts`).
//
// R426 (#32 Prem Panther) rides `Script.afterAttack`, proved in after-attack.test.ts. Every behaviour
// runs through fixture scripts (`fixtures/corePatches.ts`); the real cards prove it again in
// packages/cards. Pauses are JSON round-tripped mid-way, and the games that can be folded
// from `(seed, decks, log)` are folded and compared (§9.3).

import type { Action, ActionInput, CardView, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE } from "../src/config";
import { damage } from "../src/effects";
import { marksOn } from "../src/marks";
import { leftFieldSinceResolved } from "../src/query";
import { exitMark } from "../src/stays";
import { moveToZone } from "../src/zones";
import { ownLibraryView } from "../src/ownLibrary";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { castCard, makeContext } from "../src/resolve";
import { registerScripts, registeredScripts } from "../src/scripts";
import { stateCheck } from "../src/stateCheck";
import { cloneState, createGame, newInstance, type CardInstance, type GameState } from "../src/state";
import { timesPlayedOf } from "../src/timesPlayed";
import { viewFor, HIDDEN_ID } from "../src/viewFor";
import { bigBody, plain } from "./fixtures/combat";
import { vanillaDeck } from "./fixtures/catalog";
import {
  CORE_PATCH_SCRIPTS,
  TEST_MARK,
  corePatchCatalog,
  counted,
  marker,
  quietTrap,
  uncounted,
} from "./fixtures/corePatches";
import { eventsOfType, inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";

function register(): void {
  registerCatalog({ ...registeredCatalog(), ...corePatchCatalog() });
  registerScripts({ ...registeredScripts(), ...CORE_PATCH_SCRIPTS });
}

/** A main-phase board on p1's turn, both libraries the harness's vanilla decks. */
function board(seed: string): GameState {
  const state = newGame(seed);
  register();
  state.turn = 5;
  state.active = "p1";
  state.phase = "main";
  return state;
}

let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `core-patches-${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

// ---------------------------------------------------------------------------------------------
// R429: the times a card has been played
// ---------------------------------------------------------------------------------------------

describe("R429 §10.5 step 4 counts the plays of a card that asks", () => {
  it("R429 a card that counts its plays has one after its first, the play under way included, and keeps it in every zone", () => {
    const state = board("r429-count");
    put(state, plain.id, slot("p1", "units", 1));
    const card = must(inHand(state, counted.id, "p1")[0], "the counted Spell");
    state.players.p1.mana.current = 4;

    const played = act(state, { type: "play", instanceId: card.id, playerId: "p1" }).state;
    const landed = must(played.players.p1.graveyard.find((held) => held.id === card.id), "the Spell in the graveyard");
    expect(timesPlayedOf(landed)).toBe(1);

    // R78's reset leaves it: moved back into the hand and played again, it counts 2.
    const again = cloneState(played);
    const back = must(again.players.p1.graveyard.pop(), "the Spell");
    back.zone = { z: "hand", player: "p1" };
    again.players.p1.hand.push(back);
    const twice = act(again, { type: "play", instanceId: back.id, playerId: "p1" }).state;
    expect(timesPlayedOf(must(twice.players.p1.graveyard.find((held) => held.id === card.id), "the Spell"))).toBe(2);
  });

  it("R429 a card that does not ask carries no count at all, so a game without one hashes as before", () => {
    const state = board("r429-none");
    put(state, plain.id, slot("p1", "units", 1));
    const card = must(inHand(state, uncounted.id, "p1")[0], "the plain Spell");
    state.players.p1.mana.current = 4;

    const played = act(state, { type: "play", instanceId: card.id, playerId: "p1" }).state;
    const landed = must(played.players.p1.graveyard.find((held) => held.id === card.id), "the Spell");
    expect(landed).not.toHaveProperty("timesPlayed");
  });

  it("R429, R70 a cast is a play: casting the card counts it", () => {
    const state = board("r429-cast");
    const card = newInstance(state, counted.id, "p1", { z: "resolving", player: "p1" });
    state.players.p1.resolving.push(card);
    const sink = sinkFor(state);

    castCard(sink, card);

    expect(timesPlayedOf(card)).toBe(1);
    expect(eventsOfType(sink.events, "cardPlayed").map((event) => event.instanceId)).toEqual([card.id]);
  });

  it("R429, R57 a new instance of the same card starts its own count", () => {
    const state = board("r429-copy");
    const card = newInstance(state, counted.id, "p1", { z: "hand", player: "p1" });
    card.timesPlayed = 3;
    const copy = newInstance(state, counted.id, "p1", { z: "hand", player: "p1" });
    expect(timesPlayedOf(card)).toBe(3);
    expect(timesPlayedOf(copy)).toBe(0);
  });
});

describe("R427, R174 a resolved play's card that something answering the play has since taken off the field", () => {
  it("R427 `leftFieldSinceResolved` is false while the card stands, and for one that left before the play resolved; true once it leaves after", () => {
    const state = board("r427-left");
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const resolvedNow = (): Extract<GameEvent, { type: "cardResolved" }> => ({
      type: "cardResolved",
      player: "p1",
      instanceId: unit.id,
      defId: plain.id,
      permanent: true,
      costPaid: 1,
      exitsFrom: exitMark(state),
    });

    // Standing: nothing has taken it.
    const event = resolvedNow();
    expect(leftFieldSinceResolved(state, event)).toBe(false);

    // Taken off after the play resolved — what a trap answering the play does (R174).
    moveToZone(state, unit, "graveyard");
    expect(leftFieldSinceResolved(state, event)).toBe(true);

    // A play whose card had already left as it resolved (its own resolution took it, R427): its
    // event's mark is after the departure, so nothing has taken it since.
    expect(leftFieldSinceResolved(state, { ...resolvedNow(), permanent: false })).toBe(false);
  });
});

// ---------------------------------------------------------------------------------------------
// R433: a dealt deck
// ---------------------------------------------------------------------------------------------

/** Both mulligans answered keeping everything, in seat order. */
function keepAll(state: GameState, log: Action[]): GameState {
  let next = state;
  for (const player of ["p1", "p2"] as PlayerId[]) {
    nonce += 1;
    const action = { type: "mulligan", keep: next.players[player].hand.map((card) => card.id), playerId: player, nonce: `r433-${nonce}` } as Action;
    const result = reduce(next, action);
    if (result.error !== undefined) throw new Error(result.error);
    log.push(action);
    next = result.state;
  }
  return next;
}

describe("R433 a dealt deck lists only the cards its owner has been shown", () => {
  const decks: [string[], string[]] = [vanillaDeck(DECK_SIZE, 1), vanillaDeck(DECK_SIZE, 21)];

  it("R433 a dealt seat's starting library is every card unknown; a built seat's is listed in full (R310)", () => {
    newGame("r433-setup");
    const state = createGame({ seed: "r433", decks, dealt: ["p1"] });
    expect(ownLibraryView(state, "p1")).toEqual({ cards: [], unknown: DECK_SIZE });
    expect(ownLibraryView(state, "p2").unknown).toBe(0);
    expect(ownLibraryView(state, "p2").cards.reduce((sum, entry) => sum + entry.count, 0)).toBe(DECK_SIZE);
    // The view carries it so: the list, and nothing else of the library (§9.1).
    expect(viewFor(state, "p1").you.ownLibrary).toEqual({ cards: [], unknown: DECK_SIZE });
    expect(viewFor(state, "p1").opponent.ownLibrary).toBeUndefined();
  });

  it("R433, R311 what goes in openly afterwards is listed: a mulligan's returns are known, the rest stays unknown", () => {
    newGame("r433-setup");
    let state = beginGame(createGame({ seed: "r433-mulligan", decks, dealt: ["p1"] })).state;
    const hand = state.players.p1.hand.map((card) => card.id);
    const returned = must(hand[0], "a card to return");
    state = act(state, { type: "mulligan", keep: hand.slice(1), playerId: "p1" }).state;
    state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((card) => card.id), playerId: "p2" }).state;

    // The one returned card went in openly (R311) and is listed while it is there; every other card
    // of the library, the dealt ones, stays unknown.
    const list = ownLibraryView(state, "p1");
    const known = list.cards.reduce((sum, entry) => sum + entry.count, 0);
    const stillThere = state.players.p1.library.some((card) => card.id === returned);
    expect(known).toBe(stillThere ? 1 : 0);
    expect(list.unknown).toBe(state.players.p1.library.length - known);
    expect(list.unknown).toBeGreaterThan(0);
  });

  it("R433 no seat dealt, or `dealt` omitted: the game is created exactly as before", () => {
    newGame("r433-setup");
    const plainGame = createGame({ seed: "r433-same", decks });
    const noneDealt = createGame({ seed: "r433-same", decks, dealt: [] });
    expect(hashState(noneDealt)).toBe(hashState(plainGame));
    expect(hashState(createGame({ seed: "r433-same", decks, dealt: ["p2"] }))).not.toBe(hashState(plainGame));
  });

  it("R433, §9.3 a fold given the same dealt seats reproduces the game; one without them does not", () => {
    newGame("r433-setup");
    const log: Action[] = [];
    const live = keepAll(beginGame(createGame({ seed: "r433-fold", decks, dealt: ["p1", "p2"] })).state, log);

    const replayed = fold({ seed: "r433-fold", decks, log, dealt: ["p1", "p2"] });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(live));
    expect(viewFor(replayed.state, "p1")).toEqual(viewFor(live, "p1"));

    const forgotten = fold({ seed: "r433-fold", decks, log });
    expect(hashState(forgotten.state)).not.toBe(hashState(live));
  });
});

// ---------------------------------------------------------------------------------------------
// R434: the game's end reveals both hands
// ---------------------------------------------------------------------------------------------

describe("R434 once the game is over, each view carries the other player's hand", () => {
  it("R434 before the end the opponent's hand is a count; after it, the cards as they stand — and nothing else is revealed", () => {
    const state = board("r434");
    const theirs = inHand(state, plain.id, "p2", 2);
    inHand(state, uncounted.id, "p1");
    put(state, bigBody.id, slot("p1", "units", 1));
    put(state, plain.id, slot("p2", "units", 1));

    expect(viewFor(state, "p1").opponent.hand).toEqual({ count: 2 });

    // p2 concedes: the game is decided (§2.5).
    const over = act(state, { type: "concede", playerId: "p2" }).state;
    expect(over.result).not.toBeNull();

    const p1View = viewFor(over, "p1");
    const hand = p1View.opponent.hand as CardView[];
    expect(Array.isArray(hand)).toBe(true);
    expect(hand.map((card) => card.instanceId)).toEqual(theirs.map((card) => card.id));
    // As the owner sees them: a Unit's stats ride with it (R243).
    expect(hand[0]).toMatchObject({ defId: plain.id, attack: 3, health: 3 });
    // And for the other seat, symmetrically.
    expect((viewFor(over, "p2").opponent.hand as CardView[]).map((card) => card.defId)).toEqual([uncounted.id]);
    // The libraries stay a count, and the opponent's list is never sent (§9.1).
    expect(p1View.opponent.libraryCount).toBe(over.players.p2.library.length);
    expect(p1View.opponent.ownLibrary).toBeUndefined();
  });

  it("R434, R97 an event that named a hand card while it was hidden stays redacted after the end", () => {
    const state = board("r434-events");
    put(state, bigBody.id, slot("p1", "units", 1));
    put(state, plain.id, slot("p2", "units", 1));
    inHand(state, uncounted.id, "p1");
    // p1 ends the turn; p2's draw names a card p1 may not read.
    const turned = act(state, { type: "endTurn", playerId: "p1" }).state;
    const drawEvent = must(
      viewFor(turned, "p1").events.find((event) => event.type === "drawn" && event.player === "p2"),
      "p2's draw",
    );
    expect(drawEvent).toMatchObject({ instanceId: HIDDEN_ID });

    const over = act(turned, { type: "concede", playerId: "p1" }).state;
    const after = viewFor(over, "p1");
    expect(Array.isArray(after.opponent.hand)).toBe(true);
    const stillHidden = after.events.filter((event) => event.type === "drawn" && event.player === "p2");
    expect(stillHidden.length).toBeGreaterThan(0);
    for (const event of stillHidden) expect(event).toMatchObject({ instanceId: HIDDEN_ID });
  });
});

// ---------------------------------------------------------------------------------------------
// R437: a marked card shows its mark
// ---------------------------------------------------------------------------------------------

function unitViewOf(state: GameState, viewer: PlayerId, side: "you" | "opponent", lane: number): CardView {
  return must(viewFor(state, viewer)[side].units[lane - 1], `${viewer}'s ${side} lane ${lane}`);
}

describe("R437 a delayed effect aimed at a card marks it in both views while it waits", () => {
  function marked(seed: string): { state: GameState; target: CardInstance; events: GameEvent[] } {
    const state = board(seed);
    put(state, bigBody.id, slot("p1", "units", 5));
    const target = put(state, bigBody.id, slot("p2", "units", 1));
    put(state, plain.id, slot("p2", "units", 2));
    inHand(state, plain.id, "p2");
    const card = must(inHand(state, marker.id, "p1")[0], "the marker");
    inHand(state, uncounted.id, "p1");
    state.players.p1.mana.current = 4;
    const played = act(state, {
      type: "play",
      instanceId: card.id,
      targets: [{ pick: "instance", instanceId: target.id }],
      playerId: "p1",
    });
    return { state: played.state, target, events: played.events };
  }

  it("R437 the play marks its target, with a `marked` event, and both seats' views carry the mark", () => {
    const { state, target, events } = marked("r437-mark");
    expect(eventsOfType(events, "marked")).toEqual([
      { type: "marked", instanceId: target.id, mark: TEST_MARK.mark, color: TEST_MARK.color, added: true },
    ]);
    expect(marksOn(state, target.id)).toEqual([TEST_MARK]);
    expect(unitViewOf(state, "p1", "opponent", 1).marks).toEqual([TEST_MARK]);
    expect(unitViewOf(state, "p2", "you", 1).marks).toEqual([TEST_MARK]);
    // No other card carries one, and an unmarked card carries no key at all.
    expect(unitViewOf(state, "p2", "you", 2)).not.toHaveProperty("marks");
    // Both seats read the event (the target is public).
    for (const viewer of ["p1", "p2"] as PlayerId[]) {
      expect(viewFor(state, viewer).events.filter((event) => event.type === "marked")).toEqual(eventsOfType(events, "marked"));
    }
  });

  it("R437 the mark survives a round trip and goes when the effect resolves, with a `marked` removal", () => {
    const first = marked("r437-resolve");
    const thawed = JSON.parse(JSON.stringify(first.state)) as GameState;
    expect(thawed).toEqual(first.state);

    let state = act(thawed, { type: "endTurn", playerId: "p1" }).state;
    expect(unitViewOf(state, "p1", "opponent", 1).marks).toEqual([TEST_MARK]);
    const back = act(state, { type: "endTurn", playerId: "p2" });
    state = back.state;

    // The hit landed at p1's start of turn, and the mark is gone from both views.
    expect(state.active).toBe("p1");
    expect(eventsOfType(back.events, "damage").some((event) => event.targetId === first.target.id)).toBe(true);
    expect(eventsOfType(back.events, "marked")).toEqual([
      { type: "marked", instanceId: first.target.id, mark: TEST_MARK.mark, color: TEST_MARK.color, added: false },
    ]);
    expect(unitViewOf(state, "p1", "opponent", 1)).not.toHaveProperty("marks");
    expect(state.marks).toBeUndefined();
  });

  it("R437, R174 the mark goes the moment its card leaves the field, and the effect with it", () => {
    const { state, target } = marked("r437-leave");
    const sink = sinkFor(state);
    const ctx = makeContext(sink, null, { controller: "p1" });
    damage({ to: { of: "instance", instanceId: target.id }, amount: 20 }).apply(ctx);
    stateCheck(sink);
    // The next time the loop collects events, the removal is said.
    const after = act(state, { type: "endTurn", playerId: "p1" });
    expect(eventsOfType(after.events, "marked")).toEqual([
      { type: "marked", instanceId: target.id, mark: TEST_MARK.mark, color: TEST_MARK.color, added: false },
    ]);
    expect(after.state.delayed).toHaveLength(0);
    expect(after.state.marks).toBeUndefined();
    // The view drops it at once, before any sweep: the effect it belonged to is gone.
    expect(marksOn(state, target.id)).toEqual([]);
  });

  it("R437, R33 a face-down target: its controller reads the mark on the card, the other player on its back, and the event names the sentinel", () => {
    const state = board("r437-face-down");
    put(state, bigBody.id, slot("p1", "units", 5));
    put(state, plain.id, slot("p2", "units", 1));
    inHand(state, plain.id, "p2");
    const trap = put(state, quietTrap.id, slot("p2", "backrow", 1));
    const card = must(inHand(state, marker.id, "p1")[0], "the marker");
    inHand(state, uncounted.id, "p1");
    state.players.p1.mana.current = 4;

    const played = act(state, {
      type: "play",
      instanceId: card.id,
      targets: [{ pick: "instance", instanceId: trap.id }],
      playerId: "p1",
    }).state;

    // p1 may not read the trap (R33): its back carries the mark, and nothing names it.
    const back = viewFor(played, "p1").opponent.backrow[0];
    expect(back).toEqual({ faceDown: true, cost: 1, marks: [TEST_MARK] });
    const p1Event = must(viewFor(played, "p1").events.find((event) => event.type === "marked"), "p1's marked event");
    expect(p1Event).toMatchObject({ instanceId: HIDDEN_ID, mark: TEST_MARK.mark, color: TEST_MARK.color, added: true });
    // p2 reads its own trap, mark and all, and the event names it.
    expect(viewFor(played, "p2").you.backrow[0]).toMatchObject({ instanceId: trap.id, marks: [TEST_MARK] });
    expect(viewFor(played, "p2").events.find((event) => event.type === "marked")).toMatchObject({ instanceId: trap.id });
  });
});
