// Flicker (docs/classic-sets.md B5 E22, R444): the card leaves the field and re-enters the same zone at
// once — R78's reset, summoning sick, no Cry, no Death — and counts as summoned. A unit token comes back
// (R444, as R175 brings one back through Reborn); a Trap re-enters face-down (R33, R227); an animated card
// stays a Unit in its zone. Played through `reduce` with a pause after the flicker, a round trip, a
// replay, and both seats' views. Fixtures: `fixtures/field.ts`.

import { describe, expect, it } from "vitest";
import { animateCard } from "../src/animated";
import { flicker, flickerCard } from "../src/effects/flicker";
import { steal } from "../src/effects/steal";
import { scheduleDelayed } from "../src/modifiers";
import { hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { exitMark, leftFieldAfter } from "../src/stays";
import { settle } from "../src/triggers";
import { viewFor, HIDDEN_ID } from "../src/viewFor";
import { cardAt, homeOf, isBuried, placeOnField } from "../src/zones";
import { plain, stacker } from "./fixtures/combat";
import { act, actResult, blink, flush, playing, spatula, tesla, watcher } from "./fixtures/field";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

function byId(state: GameState, id: string): CardInstance {
  const card = findInstance(state, id);
  if (card === undefined) throw new Error(`no card ${id}`);
  return card;
}

describe("B5 E22 Flicker", () => {
  it("leaves and re-enters the same zone at once: reset, summoning sick, in Attack Position, no Cry, no Death", () => {
    const state = playing("flicker-basic");
    const unit = put(state, plain.id, slot("p1", "units", 3));
    unit.damage = 2;
    unit.buffs = { attack: 1, health: 1 };
    unit.grantedKeywords.push({ kind: "Taunt" });
    unit.counters.plague = 2;
    unit.memory.kept = true;
    unit.position = "DEF";
    unit.summonedTurn = 0;
    unit.exertion = { attacked: true, switched: false };
    const sink = sinkFor(state);
    const mark = exitMark(state);
    expect(flickerCard(sink, unit)).toBe(true);

    expect(cardAt(state, slot("p1", "units", 3))?.id).toBe(unit.id);
    expect(unit.damage).toBe(0);
    expect(unit.buffs).toEqual({ attack: 0, health: 0 });
    expect(unit.grantedKeywords).toEqual([]);
    expect(unit.counters).toEqual({});
    expect(unit.memory).toEqual({});
    expect(unit.position).toBe("ATK");
    expect(unit.summonedTurn).toBe(state.turn);
    expect(unit.exertion).toEqual({ attacked: false, switched: false });
    // It left the field (R174: what was aimed at its stay is gone), and it counts as summoned.
    expect(leftFieldAfter(state, mark, unit.id)).toBe(true);
    expect(sink.events.map((e) => e.type)).toEqual(["flickered", "summoned"]);
    expect(eventsOfType(sink.events, "flickered")).toEqual([
      { type: "flickered", player: "p1", instanceId: unit.id, defId: plain.id, row: "units", lane: 3 },
    ]);
    expect(sink.events.some((e) => e.type === "destroyed" || e.type === "enteredGraveyard")).toBe(false);
  });

  it("R444 a Flickered unit token comes back: it re-enters its zone and never ceases to exist", () => {
    const state = playing("flicker-token");
    const token = newInstance(state, "fx-token-rush", "p1", { z: "hand", player: "p1" });
    placeOnField(state, token, slot("p1", "units", 2));
    token.damage = 1;
    const sink = sinkFor(state);
    expect(flickerCard(sink, token)).toBe(true);
    expect(cardAt(state, slot("p1", "units", 2))?.id).toBe(token.id);
    expect(token.zone).toEqual({ z: "field", player: "p1", row: "units", lane: 2 });
    expect(token.damage).toBe(0);
    expect(state.players.p1.graveyard).toEqual([]);
  });

  it("counts as summoned: an enemy Tesla answers the flicker of a unit on the other side", () => {
    const state = playing("flicker-summoned");
    const zapper = put(state, tesla.id, slot("p2", "backrow", 1));
    const unit = put(state, stacker.id, slot("p1", "units", 1));
    const sink = sinkFor(state);
    flicker({ target: { of: "instance", instanceId: unit.id } }).apply(makeContext(sink, null, { controller: "p1" }));
    // Offer the flicker's events to the traps, as the resolution loop does.
    settle(sink);
    expect(eventsOfType(sink.events, "summoned").map((e) => e.instanceId)).toEqual([unit.id]);
    expect(eventsOfType(sink.events, "trapFired").map((e) => e.instanceId)).toEqual([zapper.id]);
    expect(unit.damage).toBe(4);
  });

  it("ends every delayed effect aimed at the card: what returns is a new arrival (R174)", () => {
    const state = playing("flicker-watchers");
    const unit = put(state, plain.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    scheduleDelayed(
      sink,
      "p1",
      { phase: "start", player: "p1" },
      { defId: plain.id, hook: "delayed", step: "", radiant: false, data: {} },
      unit.id,
    );
    expect(state.delayed).toHaveLength(1);
    flickerCard(sink, unit);
    expect(state.delayed).toEqual([]);
  });

  it("a stolen unit re-enters the same zone on its thief's side, still the thief's card (R640)", () => {
    const state = playing("flicker-stolen");
    const unit = put(state, plain.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    steal({ target: { of: "instance", instanceId: unit.id } }).apply(makeContext(sink, null, { controller: "p1" }));
    const at = unit.zone;
    flickerCard(sink, unit);
    expect(unit.zone).toEqual(at);
    expect(unit.controller).toBe("p1");
    expect(unit.owner).toBe("p1");
  });

  it("flickers every card a scope names, and never a card dormant under a Stack", () => {
    const state = playing("flicker-scope");
    const low = put(state, plain.id, slot("p2", "units", 1));
    const top = put(state, stacker.id, slot("p2", "units", 2));
    state.players.p2.units[1] = null;
    placeOnField(state, top, slot("p2", "units", 1), { stack: true });
    const other = put(state, plain.id, slot("p2", "units", 3));
    const sink = sinkFor(state);
    flicker({ scope: { side: "enemy" } }).apply(makeContext(sink, null, { controller: "p1" }));
    expect(eventsOfType(sink.events, "flickered").map((e) => e.instanceId)).toEqual([top.id, other.id]);
    expect(isBuried(state, low)).toBe(true);
    expect(flickerCard(sink, low)).toBe(false);
    expect(state.players.p2.units[0]?.map((c) => c.id)).toEqual([top.id, low.id]);
  });

  it("a face-down Trap re-enters face-down under a fresh id, and the other seat cannot link the two (R227, R97)", () => {
    const state = playing("flicker-trap");
    const trap = put(state, watcher.id, slot("p2", "backrow", 2));
    const oldId = trap.id;
    const sink = sinkFor(state);
    flickerCard(sink, trap);
    expect(trap.id).not.toBe(oldId);
    expect(cardAt(state, slot("p2", "backrow", 2))?.id).toBe(trap.id);
    expect(trap.faceUp).toBeUndefined();
    const summoned = eventsOfType(sink.events, "summoned");
    expect(summoned).toEqual([
      { type: "summoned", player: "p2", instanceId: trap.id, defId: watcher.id, row: "backrow", lane: 2, formerId: oldId },
    ]);
    state.applied.push({ nonce: "flicker-trap", events: sink.events });
    const theirs = viewFor(state, "p1");
    expect(eventsOfType(theirs.events, "flickered")).toMatchObject([{ instanceId: HIDDEN_ID, defId: HIDDEN_ID, lane: 2 }]);
    expect(eventsOfType(theirs.events, "summoned")).toMatchObject([{ instanceId: HIDDEN_ID, defId: HIDDEN_ID }]);
    expect(JSON.stringify(theirs.events)).not.toContain(oldId);
    const mine = viewFor(state, "p2");
    expect(eventsOfType(mine.events, "summoned")).toMatchObject([{ instanceId: trap.id, formerId: oldId }]);
  });

  it("an animated card stays a Unit in its unit zone, face-up, and its home is let go", () => {
    const state = playing("flicker-animated");
    const card = put(state, spatula.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    animateCard(sink, card);
    expect(homeOf(state, card.id)).toBeDefined();
    flickerCard(sink, card);
    expect(cardAt(state, slot("p1", "units", 1))?.id).toBe(card.id);
    expect(card.faceUp).toBe(true);
    expect(homeOf(state, card.id)).toBeUndefined();
  });
});

describe("B5 E22 Flicker inside a play that pauses (R113)", () => {
  function cast(seed: string): { state: GameState; unit: string } {
    const state = playing(seed);
    const unit = put(state, plain.id, slot("p1", "units", 2));
    unit.damage = 2;
    const [spell] = inHand(state, blink.id, "p1");
    if (spell === undefined) throw new Error("no card");
    flush(state, "p1");
    const result = actResult(state, {
      type: "play",
      instanceId: spell.id,
      targets: [{ pick: "instance", instanceId: unit.id }],
      playerId: "p1",
    });
    if (result.error !== undefined) throw new Error(result.error);
    return { state: result.state, unit: unit.id };
  }

  it("the flicker has happened when the list asks, and the answer finishes the play", () => {
    const { state, unit } = cast("flicker-pause");
    expect(state.pending?.playerId).toBe("p1");
    expect(byId(state, unit).damage).toBe(0);
    const done = act(state, {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "hero", player: "p2" }],
      playerId: "p1",
    });
    expect(done.pending).toBeNull();
    expect(cardAt(done, slot("p1", "units", 2))?.id).toBe(unit);
  });

  it("the paused state survives a JSON round trip and a replay to the same hashes", () => {
    const run = (roundTrip: boolean): { paused: string; done: string } => {
      const { state } = cast("flicker-pause-replay");
      const paused = hashState(state);
      const from = roundTrip ? (JSON.parse(JSON.stringify(state)) as GameState) : state;
      const done = act(from, {
        type: "answer",
        choiceId: from.pending?.id ?? "",
        selection: [{ pick: "hero", player: "p2" }],
        playerId: "p1",
      });
      return { paused, done: hashState(done) };
    };
    const live = run(false);
    expect(run(true)).toEqual(live);
    expect(run(false)).toEqual(live);
  });

  it("both seats read the flicker of a public unit", () => {
    const { state, unit } = cast("flicker-pause-view");
    for (const viewer of ["p1", "p2"] as const) {
      const events = viewFor(state, viewer).events;
      expect(eventsOfType(events, "flickered")).toMatchObject([{ instanceId: unit, defId: plain.id, row: "units", lane: 2 }]);
    }
  });
});
