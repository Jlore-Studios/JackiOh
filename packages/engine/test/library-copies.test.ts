// T-AI-3 Hallucination's verb, `addLibraryCopies` (SPEC §8.7 row T-AI-3; R57, R60, R97, R129, R385):
// copies of different random cards of a deck into the caster's hand, Brittle given to each that lands.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HAND_CAP } from "../src/config";
import { addLibraryCopies } from "../src/effects";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import type { GameState } from "../src/state";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { eventsOfType, inHand, newGame, setLibrary, sinkFor } from "./fixtures/harness";

function game(seed: string): GameState {
  const state = newGame(seed);
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

function run(state: GameState, effect: Effect): GameEvent[] {
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, null, { controller: "p1" }));
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

describe("addLibraryCopies (T-AI-3)", () => {
  it("R57 a copy is a new card the caster owns, with its source's definition, radiant flag, statsOverride and tuning; the source stays", () => {
    const state = game("copies-carry");
    const [source] = setLibrary(state, "p2", ["fx-5"]);
    if (source === undefined) throw new Error("a deck card");
    source.radiant = true;
    source.statsOverride = { attack: 2, health: 3 };
    source.tuning = { attack: 1 };
    source.costMod = 1;
    run(state, addLibraryCopies({ of: "enemy", count: 1, brittle: 2 }));
    const [copy] = state.players.p1.hand.slice(-1);
    expect(copy).toMatchObject({ defId: "fx-5", owner: "p1", controller: "p1", radiant: true, costMod: 0 });
    expect(copy?.id).not.toBe(source.id);
    expect(copy?.statsOverride).toEqual({ attack: 2, health: 3 });
    expect(copy?.tuning).toEqual({ attack: 1 });
    expect(copy?.brittle).toEqual({ count: 2, since: 3 });
    expect(state.players.p2.library.map((card) => card.id)).toEqual([source.id]);
  });

  it("R586 R60 the copies are of N different cards and reach the hand in the order drawn, never the deck's", () => {
    const orders = new Set<string>();
    for (let seed = 1; seed <= 12; seed += 1) {
      const state = game(`copies-order-${seed}`);
      const deck = setLibrary(state, "p2", ["fx-21", "fx-22", "fx-23", "fx-24", "fx-25", "fx-26"]);
      const before = state.players.p1.hand.length;
      run(state, addLibraryCopies({ of: "enemy", count: 2 }));
      const copies = state.players.p1.hand.slice(before);
      expect(copies).toHaveLength(2);
      expect(new Set(copies.map((card) => card.defId)).size).toBe(2);
      const positions = copies.map((copy) => deck.findIndex((card) => card.defId === copy.defId));
      orders.add(positions[0]! < positions[1]! ? "deck order" : "reversed");
    }
    // Not the deck's order: it would tell the caster where the sources lay.
    expect(orders).toEqual(new Set(["deck order", "reversed"]));
  });

  it("R129 an empty deck gives nothing and draws nothing; a deck of no more than N cards gives each, with no draw", () => {
    const empty = game("copies-empty");
    setLibrary(empty, "p2", []);
    const cursor = empty.rngCursor;
    const before = empty.players.p1.hand.length;
    expect(run(empty, addLibraryCopies({ of: "enemy", count: 2, brittle: 2 }))).toEqual([]);
    expect(empty.players.p1.hand).toHaveLength(before);
    expect(empty.rngCursor).toBe(cursor);

    const short = game("copies-short");
    setLibrary(short, "p2", ["fx-30"]);
    const at = short.rngCursor;
    run(short, addLibraryCopies({ of: "enemy", count: 2 }));
    expect(short.players.p1.hand.at(-1)?.defId).toBe("fx-30");
    expect(short.rngCursor).toBe(at);
  });

  it("R586 a deck of no more than N cards is copied whole in definition order, so the hand never shows the deck's order", () => {
    const state = game("copies-whole");
    setLibrary(state, "p2", ["fx-35", "fx-31"]);
    const before = state.players.p1.hand.length;
    const cursor = state.rngCursor;
    run(state, addLibraryCopies({ of: "enemy", count: 2 }));
    expect(state.players.p1.hand.slice(before).map((card) => card.defId)).toEqual(["fx-31", "fx-35"]);
    expect(state.rngCursor).toBe(cursor);
  });

  it("R586 §2.4 a copy the hand cap burns goes to the caster's graveyard and takes no Brittle", () => {
    const state = game("copies-burn");
    inHand(state, "fx-1", "p1", HAND_CAP - state.players.p1.hand.length);
    setLibrary(state, "p2", ["fx-31"]);
    const events = run(state, addLibraryCopies({ of: "enemy", count: 1, brittle: 2 }));
    const burned = state.players.p1.graveyard.at(-1);
    expect(burned?.defId).toBe("fx-31");
    expect(burned?.brittle).toBeUndefined();
    expect(eventsOfType(events, "burned")).toHaveLength(1);
    expect(eventsOfType(events, "counterChanged")).toEqual([]);
  });

  it("R97 the other player reads only that a card reached the hand; their deck is untouched", () => {
    const state = game("copies-view");
    const deck = setLibrary(state, "p2", ["fx-32", "fx-33", "fx-34"]);
    const events = run(state, addLibraryCopies({ of: "enemy", count: 1, brittle: 2 }));
    state.applied = [{ nonce: "lc", events }];
    const theirs = viewFor(state, "p2").events;
    for (const event of eventsOfType(theirs, "addedToHand")) {
      expect([event.instanceId, event.defId]).toEqual([HIDDEN_ID, HIDDEN_ID]);
    }
    expect(eventsOfType(theirs, "addedToHand")).toHaveLength(1);
    expect(JSON.stringify(theirs)).not.toMatch(/fx-3[234]/);
    expect(state.players.p2.library.map((card) => card.id)).toEqual(deck.map((card) => card.id));
    const mine = eventsOfType(viewFor(state, "p1").events, "addedToHand");
    expect(mine[0]?.defId).toMatch(/fx-3[234]/);
  });
});
