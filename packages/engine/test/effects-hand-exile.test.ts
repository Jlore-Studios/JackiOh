// §6.3 Exile at random out of a hand — `exileRandomFromHand` (effects/handExile.ts), the verb Classic
// #15 Nose Hunter's Radiant face needs ("and a random card from their hand"). SPEC §6.3 (Exile), §3.2
// (exile is public), R11, R55, R60. The real card's test (packages/cards/test/classic/
// 015-nose-hunter.test.ts) covers the same cases again through the card.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { exileRandomFromHand } from "../src/effects/handExile";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import type { GameState } from "../src/state";
import { eventsOfType, inHand, newGame, sinkFor } from "./fixtures/harness";

function run(state: GameState, effect: Effect, controller: PlayerId = "p1"): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, null, { controller }));
  state.rngCursor = sink.rng.cursor;
  return events;
}

function handIds(state: GameState, player: PlayerId): string[] {
  return state.players[player].hand.map((card) => card.id);
}

describe("§6.3 exileRandomFromHand (C #15 Nose Hunter)", () => {
  it("R60 exiles one random card of the named player's hand, to its owner's exile, and counts it (R55)", () => {
    const state = newGame("hand-exile-one");
    const before = inHand(state, "fx-1", "p2", 3).map((card) => card.id);
    inHand(state, "fx-2", "p1", 2);

    const events = run(state, exileRandomFromHand({ player: "enemy" }));

    const exiled = state.players.p2.exile.map((card) => card.id);
    expect(exiled).toHaveLength(1);
    expect(before).toContain(exiled[0]);
    expect(handIds(state, "p2")).toHaveLength(2);
    expect(handIds(state, "p2")).not.toContain(exiled[0]);
    expect(state.players.p1.hand).toHaveLength(2);
    expect(state.counters.exiled).toBe(1);
    expect(eventsOfType(events, "exiled").map((event) => event.instanceId)).toEqual(exiled);
  });

  it("R60 a pick of N takes N different cards, or the whole hand when it holds fewer", () => {
    const state = newGame("hand-exile-many");
    inHand(state, "fx-1", "p1", 2);

    run(state, exileRandomFromHand({ count: 5 }));

    expect(state.players.p1.hand).toHaveLength(0);
    expect(state.players.p1.exile).toHaveLength(2);
    expect(new Set(state.players.p1.exile.map((card) => card.id)).size).toBe(2);
  });

  it("an empty hand fizzles: nothing moves, no event, and the rng is not drawn from", () => {
    const state = newGame("hand-exile-empty");
    state.players.p2.hand = [];
    const cursor = state.rngCursor;

    const events = run(state, exileRandomFromHand({ player: "enemy" }));

    expect(events).toEqual([]);
    expect(state.players.p2.exile).toHaveLength(0);
    expect(state.rngCursor).toBe(cursor);
  });

  it("R60 the pick is the match rng's: the same seed and cursor exile the same card", () => {
    const pick = (seed: string): string => {
      const state = newGame(seed);
      const cards = inHand(state, "fx-1", "p2", 4);
      run(state, exileRandomFromHand({ player: "enemy" }));
      return String(cards.findIndex((card) => card.zone.z === "exile"));
    };
    expect(pick("hand-exile-seed")).toBe(pick("hand-exile-seed"));
  });

  it("R11 a unit-token card in a hand ceases to exist rather than reaching the exile, and is not counted", () => {
    const state = newGame("hand-exile-token");
    const [token] = inHand(state, "fx-token-rush", "p2", 1);

    const events = run(state, exileRandomFromHand({ player: "enemy" }));

    expect(state.players.p2.hand).toHaveLength(0);
    expect(state.players.p2.exile).toHaveLength(0);
    expect(state.counters.exiled).toBe(0);
    expect(eventsOfType(events, "exiled").map((event) => event.instanceId)).toEqual([token?.id]);
  });
});
