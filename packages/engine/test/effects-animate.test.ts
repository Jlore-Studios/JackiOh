// The `animate` verb (docs/classic-sets.md B3.1 rules 2 and 4, R383, R445): what an Animated trap's list
// ends with. The whole system — firing, turn start, cleanup, home zones — is `animated.test.ts`; this is
// the verb's own surface: its target, its position, and every card it leaves alone.

import { describe, expect, it } from "vitest";
import { animate } from "../src/effects/animate";
import { makeContext } from "../src/resolve";
import { newInstance } from "../src/state";
import { cardAt, placeOnField } from "../src/zones";
import { plain } from "./fixtures/combat";
import { banner, cover, golem, playing, springer, tower } from "./fixtures/field";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";

describe("animate (B3.1, R383)", () => {
  it("R383 animates the card running the script, into its lane's unit zone, in Attack Position by default", () => {
    const state = playing("animate-self");
    const card = put(state, springer.id, slot("p2", "backrow", 3));
    const sink = sinkFor(state);
    animate().apply(makeContext(sink, card, { controller: "p2" }));
    expect(cardAt(state, slot("p2", "units", 3))?.id).toBe(card.id);
    expect(card.position).toBe("ATK");
    expect(eventsOfType(sink.events, "animated")).toHaveLength(1);
    expect(eventsOfType(sink.events, "summoned")).toEqual([]);
  });

  it("R383 animates a named card in the position the text gives, else the leftmost open unit zone", () => {
    const state = playing("animate-named");
    const card = put(state, golem.id, slot("p1", "backrow", 2));
    put(state, plain.id, slot("p1", "units", 2));
    const sink = sinkFor(state);
    animate({ target: { of: "instance", instanceId: card.id }, position: "DEF" }).apply(makeContext(sink, null, { controller: "p1" }));
    expect(cardAt(state, slot("p1", "units", 1))?.id).toBe(card.id);
    expect(card.position).toBe("DEF");
    expect(eventsOfType(sink.events, "animated")).toMatchObject([{ backrowLane: 2, unitLane: 1 }]);
  });

  it("R383 leaves alone a card in a hand, one dormant under a backrow pile, and a Unit a carrier holds", () => {
    const state = playing("animate-refusals");
    const held = newInstance(state, golem.id, "p1", { z: "hand", player: "p1" });
    state.players.p1.hand.push(held);
    const buried = put(state, golem.id, slot("p1", "backrow", 1));
    const top = newInstance(state, cover.id, "p1", { z: "hand", player: "p1" });
    placeOnField(state, top, slot("p1", "backrow", 1), { stack: true });
    put(state, tower.id, slot("p1", "backrow", 2));
    const rider = newInstance(state, plain.id, "p1", { z: "hand", player: "p1" });
    placeOnField(state, rider, slot("p1", "backrow", 2));
    const sink = sinkFor(state);
    for (const card of [held, buried, rider]) {
      animate({ target: { of: "instance", instanceId: card.id } }).apply(makeContext(sink, null, { controller: "p1" }));
    }
    expect(sink.events).toEqual([]);
    expect(held.zone.z).toBe("hand");
    expect(cardAt(state, slot("p1", "backrow", 1))?.id).toBe(top.id);
  });

  it("R383 a card that is a Unit already does not move or change position", () => {
    const state = playing("animate-already");
    const card = put(state, springer.id, slot("p2", "backrow", 4));
    const sink = sinkFor(state);
    animate({ position: "DEF" }).apply(makeContext(sink, card, { controller: "p2" }));
    animate({ position: "ATK" }).apply(makeContext(sink, card, { controller: "p2" }));
    expect(cardAt(state, slot("p2", "units", 4))?.id).toBe(card.id);
    expect(card.position).toBe("DEF");
    expect(eventsOfType(sink.events, "animated")).toHaveLength(1);
  });

  it("R383 with every unit zone taken the card stays in the backrow, and nothing is reported", () => {
    const state = playing("animate-full");
    for (let lane = 1; lane <= 5; lane += 1) put(state, plain.id, slot("p1", "units", lane));
    const card = put(state, banner.id, slot("p1", "backrow", 5));
    const sink = sinkFor(state);
    animate().apply(makeContext(sink, card, { controller: "p1" }));
    expect(cardAt(state, slot("p1", "backrow", 5))?.id).toBe(card.id);
    expect(sink.events).toEqual([]);
  });
});
