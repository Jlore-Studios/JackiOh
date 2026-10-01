// Brittle X's verbs (docs/classic-sets.md B3.3 rule 4; R385, R440, R441): "Give Brittle N" sets the
// count and starts it now, "gain +N Brittle" adds to it, on a named card anywhere or over a card scope,
// and what each reports to whom.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { gainBrittle, giveBrittle } from "../src/effects";
import { makeContext, type HookOptions } from "../src/resolve";
import type { Effect } from "../src/script";
import type { CardInstance, GameState } from "../src/state";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { plain } from "./fixtures/combat";
import { inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { brittleTrap, brittleUnit, instanceGame } from "./fixtures/instanceData";

function game(): GameState {
  const state = instanceGame("brittle-verbs");
  state.turn = 5;
  state.active = "p1";
  state.phase = "main";
  return state;
}

function run(state: GameState, effect: Effect, options: HookOptions & { self?: CardInstance | null } = {}): GameEvent[] {
  const { self = null, ...hook } = options;
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, self, { controller: "p1", ...hook }));
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

describe("B3.3 rule 4: give and gain (R385)", () => {
  it("R385 Give Brittle N sets the count to N from now, whatever the card had", () => {
    const state = game();
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    expect(unit.brittle).toEqual({ count: 2, since: 5, printed: true });
    state.turn = 7;
    const events = run(state, giveBrittle({ instanceId: unit.id, n: 5 }));
    expect(unit.brittle).toEqual({ count: 5, since: 7 });
    expect(events).toEqual([{ type: "counterChanged", instanceId: unit.id, counter: "brittle", value: 5 }]);
  });

  it("R385 gain +N adds to the count in force and keeps when it started", () => {
    const state = game();
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    state.turn = 8;
    run(state, gainBrittle({ target: { of: "self" }, n: 1 }), { self: unit });
    expect(unit.brittle).toEqual({ count: 3, since: 5, printed: true });
  });

  it("R385 a named card in a hand or a deck takes a count too, reported to whoever may read it (R97)", () => {
    const state = game();
    const [held] = inHand(state, plain.id, "p1");
    const [deck] = setLibrary(state, "p1", [plain.id]);
    if (held === undefined || deck === undefined) throw new Error("no card");
    const events = run(state, giveBrittle({ target: { of: "chosen" }, n: 2 }), {
      targets: [{ pick: "instance", instanceId: held.id }],
    });
    run(state, giveBrittle({ instanceId: deck.id, n: 2 }));
    expect(held.brittle).toEqual({ count: 2, since: 5 });
    expect(deck.brittle).toEqual({ count: 2, since: 5 });
    state.applied = [{ nonce: "t", events }];
    expect(viewFor(state, "p1").events).toEqual([{ type: "counterChanged", instanceId: held.id, counter: "brittle", value: 2 }]);
    // The other player may not read the card, so neither its id nor its count (R385).
    expect(viewFor(state, "p2").events).toEqual([{ type: "counterChanged", instanceId: HIDDEN_ID, counter: "brittle", value: -1 }]);
  });

  it("R440 over a scope, a card of a hidden pile takes its count silently; a public one is reported", () => {
    const state = game();
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const trap = put(state, brittleTrap.id, slot("p1", "backrow", 1));
    const hand = inHand(state, plain.id, "p1", 3);
    const events = run(state, giveBrittle({ scope: { zones: ["field", "hand"] }, n: 2 }));
    for (const card of [unit, trap, ...hand]) expect(card.brittle).toEqual({ count: 2, since: 5 });
    expect(events).toEqual([{ type: "counterChanged", instanceId: unit.id, counter: "brittle", value: 2 }]);
  });

  it("R385 a card that has ceased to exist takes nothing", () => {
    const state = game();
    const [held] = inHand(state, plain.id, "p1");
    if (held === undefined) throw new Error("no card");
    state.players.p1.hand = [];
    held.zone = { z: "gone", player: "p1" };
    expect(run(state, giveBrittle({ target: { of: "chosen" }, n: 2 }), { targets: [{ pick: "instance", instanceId: held.id }] })).toEqual([]);
    expect(held.brittle).toBeUndefined();
  });
});
