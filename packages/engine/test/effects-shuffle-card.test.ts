// §6.3 Shuffle of an existing card into its owner's library — `shuffleCardInto`
// (effects/shuffleCard.ts), the verb Classic #30 Recycle needs ("Shuffle your graveyard into your
// deck"). SPEC §6.3, R80, R97, R311, R316. The real card's test (packages/cards/test/classic/
// 030-recycle.test.ts) covers the same cases again through the card.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { LIBRARY_CAP } from "../src/config";
import { shuffleCardInto } from "../src/effects/shuffleCard";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { eventsOfType, newGame, setLibrary, sinkFor } from "./fixtures/harness";

function run(state: GameState, effect: Effect, controller: PlayerId = "p1"): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, null, { controller }));
  state.rngCursor = sink.rng.cursor;
  return events;
}

function inGraveyard(state: GameState, defId: string, player: PlayerId): CardInstance {
  const card = newInstance(state, defId, player, { z: "graveyard", player });
  state.players[player].graveyard.push(card);
  return card;
}

describe("§6.3 shuffleCardInto (C #30 Recycle)", () => {
  it("moves a graveyard card into its owner's library, keeping its id and its costMod (R78)", () => {
    const state = newGame("shuffle-card-one");
    setLibrary(state, "p1", ["fx-1", "fx-2", "fx-3"]);
    const card = inGraveyard(state, "fx-4", "p1");
    card.costMod = -1;

    const events = run(state, shuffleCardInto({ instanceId: card.id }));

    expect(state.players.p1.graveyard).toHaveLength(0);
    expect(state.players.p1.library).toHaveLength(4);
    const moved = state.players.p1.library.find((c) => c.id === card.id);
    expect(moved?.zone).toEqual({ z: "library", player: "p1" });
    expect(moved?.costMod).toBe(-1);
    expect(eventsOfType(events, "shuffledIn").map((event) => event.instanceId)).toEqual([card.id]);
  });

  it("goes to its owner's library, whoever runs the effect", () => {
    const state = newGame("shuffle-card-owner");
    setLibrary(state, "p2", ["fx-1"]);
    const card = inGraveyard(state, "fx-4", "p2");

    run(state, shuffleCardInto({ instanceId: card.id }), "p1");

    expect(state.players.p2.library.map((c) => c.id)).toContain(card.id);
  });

  it("R60 the position is the match rng's: the same seed puts it in the same place", () => {
    const at = (seed: string): number => {
      const state = newGame(seed);
      setLibrary(state, "p1", ["fx-1", "fx-2", "fx-3", "fx-5", "fx-6"]);
      const card = inGraveyard(state, "fx-4", "p1");
      run(state, shuffleCardInto({ instanceId: card.id }));
      return state.players.p1.library.findIndex((c) => c.id === card.id);
    };
    expect(at("shuffle-card-seed")).toBe(at("shuffle-card-seed"));
  });

  it("R80 R316 a full library turns it away: it stays in the graveyard where it was, with no second move", () => {
    const state = newGame("shuffle-card-full");
    setLibrary(state, "p1", Array.from({ length: LIBRARY_CAP }, () => "fx-1"));
    const first = inGraveyard(state, "fx-4", "p1");
    const second = inGraveyard(state, "fx-5", "p1");

    const events = run(state, shuffleCardInto({ instanceId: first.id }));

    expect(state.players.p1.library).toHaveLength(LIBRARY_CAP);
    expect(state.players.p1.graveyard.map((c) => c.id)).toEqual([first.id, second.id]);
    expect(eventsOfType(events, "enteredGraveyard")).toEqual([]);
    expect(eventsOfType(events, "shuffledIn")).toEqual([]);
    expect(eventsOfType(events, "libraryOverflow")).toEqual([
      { type: "libraryOverflow", player: "p1", instanceId: first.id, defId: "fx-4", outcome: "graveyard" },
    ]);
  });

  it("R316 a Radiant card turned away is reported Radiant", () => {
    const state = newGame("shuffle-card-full-radiant");
    setLibrary(state, "p1", Array.from({ length: LIBRARY_CAP }, () => "fx-1"));
    const card = inGraveyard(state, "fx-4", "p1");
    card.radiant = true;

    const events = run(state, shuffleCardInto({ instanceId: card.id }));

    expect(eventsOfType(events, "libraryOverflow")[0]?.radiant).toBe(true);
  });

  it("a card that no longer exists is skipped: nothing moves and the rng is not drawn from", () => {
    const state = newGame("shuffle-card-gone");
    setLibrary(state, "p1", ["fx-1"]);
    const cursor = state.rngCursor;

    const events = run(state, shuffleCardInto({ instanceId: "no-such-card" }));

    expect(events).toEqual([]);
    expect(state.players.p1.library).toHaveLength(1);
    expect(state.rngCursor).toBe(cursor);
  });
});
