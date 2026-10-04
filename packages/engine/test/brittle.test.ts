// Brittle X (docs/classic-sets.md B3.3; R385, R438, R440, R441, R638): where the count lives, when it
// starts, when it ticks (on the field only), what a crumble does, what a Vanilla does to it, what the
// views show, and a crumble whose Death asks something pausing the settle after the tick (R113), with
// the paused game surviving a JSON round trip and resuming identically.
//
// `turn.ts` runs `brittleTick` as a stage of the start of a turn (the activate-and-turn workstream
// wires it); these tests drive the stage directly, setting `state.turn` and `state.active` as a start
// of turn would, and settle after it as the stage does.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { activeBrittleCount, brittleTick, gainBrittleCount, giveBrittleCount } from "../src/brittle";
import { BRITTLE_FIRST_TICK_TURNS } from "../src/config";
import { reveal } from "../src/effects/reveal";
import { vanilla } from "../src/effects/transform";
import { unitView } from "../src/layers";
import { makeContext } from "../src/resolve";
import { beginGame, reduce } from "../src/reduce";
import { stateCheck } from "../src/stateCheck";
import { cloneState, newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { placeOnField, removeFromField } from "../src/zones";
import { indestructible, plain, stacker } from "./fixtures/combat";
import { eventsOfType, inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { asker, brittleTrap, brittleUnit, instanceGame } from "./fixtures/instanceData";

/** A game at `turn`, `active`'s, with nothing else set up. */
function at(turn: number, active: PlayerId = "p1", seed = "brittle"): GameState {
  const state = instanceGame(seed);
  state.turn = turn;
  state.active = active;
  state.phase = "main";
  return state;
}

/** Run the tick for `player` at `turn`, as the start of their turn does, and settle after it. */
function tickAt(state: GameState, turn: number, player: PlayerId, doSettle = true): GameEvent[] {
  state.turn = turn;
  state.active = player;
  const sink = sinkFor(state);
  brittleTick(sink, player);
  if (doSettle) settle(sink);
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

function given(state: GameState, card: CardInstance, count: number, turn: number): void {
  const now = state.turn;
  state.turn = turn;
  giveBrittleCount(state, card, count);
  state.turn = now;
}

describe("B3.3 where a Brittle count lives and when it starts (R385)", () => {
  it("R385 a printed Brittle starts as its card enters the field, and not before", () => {
    const state = at(4);
    const [held] = inHand(state, brittleUnit.id, "p1");
    if (held === undefined) throw new Error("no card");
    expect(held.brittle).toBeUndefined();
    expect(activeBrittleCount(held)).toBeNull();

    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    expect(unit.brittle).toEqual({ count: 2, since: 4, printed: true });
    const radiant = put(state, brittleUnit.id, slot("p1", "units", 2), { radiant: true });
    expect(radiant.brittle?.count).toBe(4);
    // R659: a Field Trap set face-down has entered the field but is unrevealed, so it starts
    // no count — no Brittle while unrevealed (Classic+ #74).
    const trap = put(state, brittleTrap.id, slot("p1", "backrow", 1));
    expect(trap.brittle).toBeUndefined();
    // The count starts when the card reveals.
    reveal().apply(makeContext(sinkFor(state), trap, {}));
    expect(trap.brittle).toEqual({ count: 3, since: 4, printed: true });
  });

  it("R385 a card that already has a count keeps it as it enters the field, so a count is kept in every zone", () => {
    const state = at(6);
    const card = newInstance(state, brittleUnit.id, "p1", { z: "hand", player: "p1" });
    given(state, card, 5, 3);
    expect(placeOnField(state, card, slot("p1", "units", 1))).toBe(true);
    // R638: the count is kept, and its turn cycle starts on the field, at this arrival.
    expect(card.brittle).toEqual({ count: 5, since: 6 });
  });

  it("R638 a move from one field zone to another is no arrival: the count's cycle is not restarted", () => {
    const state = at(8);
    const unit = put(state, plain.id, slot("p1", "units", 1));
    given(state, unit, 3, 5);
    expect(removeFromField(state, unit)).toBe(true);
    expect(placeOnField(state, unit, slot("p1", "units", 2))).toBe(true);
    expect(unit.brittle).toEqual({ count: 3, since: 5 });
  });

  it("R385 a count ticks first at its controller's start of turn t + 2 or later, then every one of theirs", () => {
    expect(BRITTLE_FIRST_TICK_TURNS).toBe(2);
    const state = at(5);
    const mine = put(state, plain.id, slot("p1", "units", 1));
    // Given on p1's turn 5: it lives through the rest of 5 and p2's turn 6, ticks at 7, crumbles at 9.
    given(state, mine, 2, 5);
    tickAt(state, 5, "p1");
    expect(mine.brittle?.count).toBe(2);
    tickAt(state, 7, "p1");
    expect(mine.brittle?.count).toBe(1);
    tickAt(state, 9, "p1");
    expect(mine.zone.z).toBe("graveyard");
  });

  it("R385 a count given on the other player's turn waits a whole turn cycle of its own", () => {
    const state = at(6, "p2");
    const mine = put(state, plain.id, slot("p1", "units", 1));
    given(state, mine, 2, 6);
    // p1's turn 7 is only one player-turn later: no tick yet. Turn 9 is the first.
    tickAt(state, 7, "p1");
    expect(mine.brittle?.count).toBe(2);
    tickAt(state, 9, "p1");
    expect(mine.brittle?.count).toBe(1);
  });

  it("R385 the other player's start of turn never ticks a count", () => {
    const state = at(3);
    const mine = put(state, plain.id, slot("p1", "units", 1));
    given(state, mine, 1, 1);
    tickAt(state, 4, "p2");
    expect(mine.brittle?.count).toBe(1);
    expect(mine.zone.z).toBe("field");
  });

  it("R638 a count in a hand or a deck holds: no tick, no crumble and no event, however long it waits", () => {
    const state = at(9);
    const [handCard] = inHand(state, plain.id, "p1");
    const [deckCard] = setLibrary(state, "p1", [body(), plain.id]);
    if (handCard === undefined || deckCard === undefined) throw new Error("no card");
    given(state, handCard, 1, 3);
    given(state, deckCard, 2, 3);

    for (const turn of [9, 11, 13]) expect(tickAt(state, turn, "p1")).toEqual([]);
    expect(handCard.zone).toEqual({ z: "hand", player: "p1" });
    expect(handCard.brittle).toEqual({ count: 1, since: 3 });
    expect(deckCard.zone.z).toBe("library");
    expect(deckCard.brittle).toEqual({ count: 2, since: 3 });
  });

  it("R638 a count held in a hand starts its cycle as the card enters the field: first tick at t + 2 of the arrival", () => {
    const state = at(5);
    const card = newInstance(state, plain.id, "p1", { z: "hand", player: "p1" });
    given(state, card, 2, 1);
    state.turn = 9;
    expect(placeOnField(state, card, slot("p1", "units", 1))).toBe(true);
    expect(card.brittle).toEqual({ count: 2, since: 9 });

    // Held since turn 1, yet not due at 9: the cycle is the field's.
    tickAt(state, 9, "p1");
    expect(card.brittle?.count).toBe(2);
    tickAt(state, 11, "p1");
    expect(card.brittle?.count).toBe(1);
    tickAt(state, 13, "p1");
    expect(card.zone.z).toBe("graveyard");
  });

  it("R638 a card that leaves the field keeps the count it has, and it holds there until the card is back", () => {
    const state = at(6);
    const unit = put(state, plain.id, slot("p1", "units", 1));
    given(state, unit, 3, 2);
    tickAt(state, 6, "p1");
    expect(unit.brittle?.count).toBe(2);

    expect(removeFromField(state, unit)).toBe(true);
    unit.zone = { z: "hand", player: "p1" };
    state.players.p1.hand.push(unit);
    tickAt(state, 8, "p1");
    tickAt(state, 10, "p1");
    expect(unit.brittle?.count).toBe(2);
    expect(unit.zone.z).toBe("hand");
  });
});

describe("B3.3 a crumble (R385)", () => {
  it("R385 on the field a count at 0 is an ordinary destroy: the state check collects it and it dies", () => {
    const state = at(7);
    const unit = put(state, plain.id, slot("p1", "units", 2));
    given(state, unit, 1, 5);
    const events = tickAt(state, 7, "p1");
    expect(unit.zone.z).toBe("graveyard");
    expect(events.filter((e) => e.type === "counterChanged" || e.type === "crumbled" || e.type === "destroyed").map((e) => e.type)).toEqual([
      "counterChanged",
      "crumbled",
      "destroyed",
    ]);
    expect(eventsOfType(events, "counterChanged")[0]).toEqual({ type: "counterChanged", instanceId: unit.id, counter: "brittle", value: 0 });
    expect(eventsOfType(events, "crumbled")[0]).toEqual({ type: "crumbled", instanceId: unit.id, defId: plain.id, owner: "p1", zone: "field" });
    // R441: the count that crumbled it is spent, and went with R78's reset.
    expect(unit.brittle).toBeUndefined();
  });

  it("R385 Indestructible ignores the crumble; the count stays 0 and crumbles it again at each tick", () => {
    const state = at(7);
    const unit = put(state, indestructible.id, slot("p1", "units", 1));
    given(state, unit, 1, 5);
    const first = tickAt(state, 7, "p1");
    expect(unit.zone.z).toBe("field");
    expect(unit.brittle?.count).toBe(0);
    expect(eventsOfType(first, "crumbled")).toHaveLength(1);
    expect(eventsOfType(first, "destroyed")).toHaveLength(0);

    const second = tickAt(state, 9, "p1");
    expect(unit.zone.z).toBe("field");
    expect(unit.brittle?.count).toBe(0);
    expect(eventsOfType(second, "crumbled")).toHaveLength(1);
    // Nothing more to take off: no second counterChanged.
    expect(eventsOfType(second, "counterChanged")).toHaveLength(0);
  });

  it("R385 a count ticks at its controller's start of turn on the field: a stolen card ticks on the thief's", () => {
    const state = at(7);
    const stolen = put(state, plain.id, slot("p1", "units", 3));
    stolen.owner = "p2";
    given(state, stolen, 3, 5);
    tickAt(state, 8, "p2");
    expect(stolen.brittle?.count).toBe(3);
    tickAt(state, 9, "p1");
    expect(stolen.brittle?.count).toBe(2);
  });

  it("R385 a card dormant under a Stack is not on the field and does not tick (R13)", () => {
    const state = at(7);
    const under = put(state, plain.id, slot("p1", "units", 1));
    given(state, under, 1, 5);
    const top = newInstance(state, stacker.id, "p1", { z: "hand", player: "p1" });
    expect(placeOnField(state, top, slot("p1", "units", 1), { stack: true })).toBe(true);
    const events = tickAt(state, 7, "p1");
    expect(under.brittle?.count).toBe(1);
    expect(eventsOfType(events, "crumbled")).toHaveLength(0);
  });

  it("R385 a face-down trap's count ticks silently: a cue would tell the other player a hidden card is Brittle (R440)", () => {
    const state = at(7);
    const trap = put(state, brittleTrap.id, slot("p1", "backrow", 2));
    given(state, trap, 2, 5);
    const events = tickAt(state, 7, "p1");
    expect(trap.brittle?.count).toBe(1);
    expect(events).toEqual([]);
  });
});

describe("B3.3 rule 5: Vanilla and the count (R385, R441)", () => {
  it("R385 a Vanilla switches a printed count off while it lasts and keeps a given one", () => {
    const state = at(7);
    const printed = put(state, brittleUnit.id, slot("p1", "units", 1));
    const givenTo = put(state, plain.id, slot("p1", "units", 2));
    given(state, givenTo, 2, 5);
    printed.brittle = { count: 2, since: 5, printed: true };
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    vanilla({ instanceId: printed.id }).apply(ctx);
    vanilla({ instanceId: givenTo.id }).apply(ctx);

    expect(activeBrittleCount(printed)).toBeNull();
    expect(unitView(state, printed).keywords.some((k) => k.kind === "Brittle")).toBe(false);
    expect(activeBrittleCount(givenTo)).toBe(2);
    expect(unitView(state, givenTo).keywords).toContainEqual({ kind: "Brittle", n: 2 });

    tickAt(state, 7, "p1");
    expect(printed.brittle?.count).toBe(2);
    expect(givenTo.brittle?.count).toBe(1);
  });

  it("R385 the count in force is the unit's Brittle keyword, a given one on a card that prints none included", () => {
    const state = at(7);
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    expect(unitView(state, unit).keywords).toEqual([{ kind: "Brittle", n: 2 }]);
    unit.brittle = { count: 1, since: 7, printed: true };
    expect(unitView(state, unit).keywords).toEqual([{ kind: "Brittle", n: 1 }]);
    const other = put(state, plain.id, slot("p1", "units", 2));
    given(state, other, 3, 7);
    expect(unitView(state, other).keywords).toEqual([{ kind: "Brittle", n: 3 }]);
  });

  it("R441 a crumbled card that comes back to the field starts its printed Brittle afresh", () => {
    const state = at(7);
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    unit.brittle = { count: 1, since: 5, printed: true };
    tickAt(state, 7, "p1");
    expect(unit.zone.z).toBe("graveyard");
    expect(unit.brittle).toBeUndefined();
    state.players.p1.graveyard = state.players.p1.graveyard.filter((card) => card.id !== unit.id);
    expect(placeOnField(state, unit, slot("p1", "units", 1))).toBe(true);
    expect(unit.brittle).toEqual({ count: 2, since: 7, printed: true });
  });

  it("R441 gain +N on a card with no count starts one now at N more than it prints; a Vanilla keeps it", () => {
    const state = at(4);
    const [card] = inHand(state, brittleUnit.id, "p1");
    const [bare] = inHand(state, plain.id, "p1");
    if (card === undefined || bare === undefined) throw new Error("no card");
    gainBrittleCount(state, card, 1);
    gainBrittleCount(state, bare, 2);
    expect(card.brittle).toEqual({ count: 3, since: 4 });
    expect(bare.brittle).toEqual({ count: 2, since: 4 });
    gainBrittleCount(state, card, 2);
    expect(card.brittle).toEqual({ count: 5, since: 4 });
  });
});

describe("B3.3 rule 6: who sees the count (R385)", () => {
  it("R385 the count is public on the field, its owner's alone in a hand, and a face-down trap's to its controller", () => {
    const state = at(3);
    const unit = put(state, plain.id, slot("p1", "units", 1));
    given(state, unit, 2, 3);
    const [held] = inHand(state, plain.id, "p1");
    if (held === undefined) throw new Error("no card");
    given(state, held, 2, 3);
    const trap = put(state, brittleTrap.id, slot("p1", "backrow", 1));

    const mine = viewFor(state, "p1");
    const theirs = viewFor(state, "p2");
    expect(mine.you.units[0]?.brittle).toBe(2);
    expect(theirs.opponent.units[0]?.brittle).toBe(2);
    const hand = mine.you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand.find((card) => card.instanceId === held.id)?.brittle).toBe(2);
    expect(theirs.opponent.hand).toEqual({ count: 1 });
    const own = mine.you.backrow[0];
    expect(own !== null && own?.faceDown === false ? own.brittle : undefined).toBe(trap.brittle?.count);
    expect(theirs.opponent.backrow[0]).toEqual({ faceDown: true, cost: 2 });
    expect(JSON.stringify(theirs.opponent.backrow)).not.toContain("brittle");
  });
});

// ---------------------------------------------------------------------------
// A crumble whose Death asks (R113, §9.3)
// ---------------------------------------------------------------------------

let nonce = 0;
function act(state: GameState, body: ActionInput, fixed?: string): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: fixed ?? `br${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** p1's main phase on turn 1, past the mulligans. */
function playing(seed: string): GameState {
  let state = beginGame(instanceGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  return state;
}

describe("B3.3 a crumble that pauses the settle after the tick (R113, §9.3)", () => {
  it("R385 a crumbled unit's Death prompt parks, the paused game survives JSON and resumes to the same end", () => {
    const state = playing("brittle-pause");
    const unit = put(state, asker.id, slot("p1", "units", 4));
    unit.brittle = { count: 1, since: state.turn - BRITTLE_FIRST_TICK_TURNS };
    const other = put(state, plain.id, slot("p1", "units", 5));
    other.brittle = { count: 1, since: state.turn - BRITTLE_FIRST_TICK_TURNS };

    const sink = sinkFor(state);
    brittleTick(sink, "p1");
    settle(sink);
    state.rngCursor = sink.rng.cursor;
    expect(eventsOfType(sink.events, "crumbled").map((e) => e.instanceId)).toEqual([unit.id, other.id]);
    // Both crumbled together in one state check (R59), and the asker's Death is asking.
    expect(unit.zone.z).toBe("graveyard");
    expect(other.zone.z).toBe("graveyard");
    const pending = state.pending;
    expect(pending?.playerId).toBe("p1");
    if (pending === null) throw new Error("expected the Death to ask");

    const copy = JSON.parse(JSON.stringify(state)) as GameState;
    expect(copy).toEqual(cloneState(state));
    const answer = { type: "answer" as const, choiceId: pending.id, selection: [{ pick: "mode" as const, option: "two" }], playerId: "p1" as const };
    const live = act(state, answer, "answer");
    const replayed = act(copy, answer, "answer");
    expect(replayed).toEqual(live);
    expect(live.pending).toBeNull();
    expect(live.work).toEqual([]);
    expect(live.players.p2.hero.health).toBe(state.players.p2.hero.health - 2);
  });

  it("R385 nothing asks, nothing parks: the tick and its check finish in one go", () => {
    const state = playing("brittle-quiet");
    const unit = put(state, plain.id, slot("p1", "units", 4));
    unit.brittle = { count: 1, since: state.turn - BRITTLE_FIRST_TICK_TURNS };
    const sink = sinkFor(state);
    brittleTick(sink, "p1");
    stateCheck(sink);
    expect(state.pending).toBeNull();
    expect(state.work).toEqual([]);
    expect(unit.zone.z).toBe("graveyard");
  });
});

/** A second vanilla fixture unit's id, for a deck that is not all one card. */
function body(): string {
  return "fx-2";
}
