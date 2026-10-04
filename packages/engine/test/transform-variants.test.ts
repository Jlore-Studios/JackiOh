// The Transform variants of patch v0.2.0 (docs/classic-sets.md B5 E24; §6.3 Transform, R23, R35,
// R57): the cards beneath a Stack pile become copies of its top (Classic+ #4 Juhan Biggest Bat), and
// a card becomes a random card of a pool (Classic+ #73.1 Classic Golem).

import { describe, expect, it } from "vitest";
import { defOf } from "../src/catalog";
import { transformBeneath, transformRandom } from "../src/effects";
import { isSick } from "../src/combat";
import { unitView } from "../src/layers";
import { hashState } from "../src/replay";
import { makeContext, type EngineSink } from "../src/resolve";
import type { Effect } from "../src/script";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { placeOnField, removeFromField } from "../src/zones";
import {
  act,
  body,
  classicPlusUnit,
  classicSpell,
  classicUnit,
  fuseA,
  fuseB,
  frozen,
  handCard,
  immutable,
  juhan,
  playing,
  replayed,
} from "./fixtures/generation";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";

function run(sink: EngineSink, effect: Effect, self: CardInstance | null = null): void {
  effect.apply(makeContext(sink, self, { controller: "p1" }));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

/** p1's lane 1 holding `below` bottom to top — the last one is the top of the pile. */
function pile(state: GameState, defIds: readonly string[], owner: "p1" | "p2" = "p1"): CardInstance[] {
  return defIds.map((defId) => {
    const card = newInstance(state, defId, owner, { z: "hand", player: owner });
    if (!placeOnField(state, card, { player: "p1", row: "units", lane: 1 }, { stack: true })) {
      throw new Error("could not stack");
    }
    return card;
  });
}

describe("E24 the cards beneath a Stack become copies of its top (Classic+ #4)", () => {
  it("R57 each dormant card is Replaced by a copy of the top: its face and buffs, the old card's owner, place and position", () => {
    const start = playing("transform-beneath");
    const state = start.state;
    const [bottom, middle] = pile(state, [fuseA.id, body.id], "p2");
    // The bottom card came from the opponent onto p1's lane long ago, which made p1 its current
    // owner (R659): the copy keeps that owner.
    const bottomCard = must(bottom, "the bottom card");
    expect(bottomCard.owner).toBe("p1");
    const middleCard = must(middle, "the middle card");
    middleCard.position = "DEF";
    const top = handCard(state, juhan.id);
    top.radiant = true;
    top.buffs = { attack: 1, health: 1 };
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: top.id, zone: { row: "units", lane: 1 }, playerId: "p1" });

    const lane = must(run1.state.players.p1.units[0], "the pile");
    expect(lane).toHaveLength(3);
    expect(lane[0]?.id).toBe(top.id);
    for (const copy of lane.slice(1)) {
      expect(copy.defId).toBe(juhan.id);
      expect(copy.radiant).toBe(true);
      expect(copy.buffs).toEqual({ attack: 1, health: 1 });
      expect(copy.owner).toBe(bottomCard.owner);
      expect(copy.controller).toBe("p1");
    }
    expect(lane[1]?.position).toBe("DEF");
    // The replaced cards ceased to exist: no graveyard, no Death (R35).
    expect(findInstance(run1.state, bottomCard.id)).toBeUndefined();
    expect(findInstance(run1.state, middleCard.id)).toBeUndefined();
    expect(run1.state.players.p2.graveyard).toEqual([]);
    const events = eventsOfType(run1.state.applied.at(-1)?.events ?? [], "transformed");
    expect(events.map((event) => [event.fromDefId, event.toDefId])).toEqual([
      [body.id, juhan.id],
      [fuseA.id, juhan.id],
    ]);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));

    // They stay dormant (R13) and resume in order when the top leaves.
    const after = run1.state;
    expect(unitView(after, must(lane[1], "a copy")).attack).toBeGreaterThan(0);
    removeFromField(after, must(lane[0], "the top"));
    expect(after.players.p1.units[0]?.[0]?.id).toBe(lane[1]?.id);
  });

  it("R23 an Immutable card beneath stays as it is", () => {
    const start = playing("transform-beneath-immutable");
    const state = start.state;
    const [locked, plain] = pile(state, [immutable.id, body.id]);
    const top = must(pile(state, [juhan.id])[0], "the top");
    run(sinkFor(state), transformBeneath(), top);
    const lane = must(state.players.p1.units[0], "the pile");
    expect(lane.map((card) => card.defId)).toEqual([juhan.id, juhan.id, immutable.id]);
    expect(lane[2]?.id).toBe(locked?.id);
    expect(findInstance(state, must(plain, "a card").id)).toBeUndefined();
  });

  it("E24 a card that is not the top of a unit pile changes nothing", () => {
    const start = playing("transform-beneath-not-top");
    const state = start.state;
    const [under] = pile(state, [body.id, juhan.id]);
    const sink = sinkFor(state);
    run(sink, transformBeneath(), must(under, "the dormant card"));
    const spell = handCard(state, juhan.id);
    run(sink, transformBeneath(), spell);
    expect(sink.events).toEqual([]);
  });
});

describe("E24 a card becomes a random card of a pool (Classic+ #73.1)", () => {
  it("R35 on the field only a card of its row: a Unit becomes a random Classic or Classic+ Unit, never a Spell of the pool", () => {
    const start = playing("transform-random");
    const state = start.state;
    const golem = put(state, fuseB.id, slot("p1", "units", 2));
    golem.damage = 1;
    const sink = sinkFor(state);
    run(sink, transformRandom({ instanceId: golem.id, query: { set: ["Classic", "Classic+"] } }), golem);

    const now = must(state.players.p1.units[1]?.[0], "the new Unit");
    expect([classicUnit.id, classicPlusUnit.id]).toContain(now.defId);
    expect(now.defId).not.toBe(classicSpell.id);
    expect(now.id).not.toBe(golem.id);
    expect(now.damage).toBe(0);
    expect(now.radiant).toBe(false);
    expect(isSick(state, now)).toBe(true);
    expect(golem.zone).toEqual({ z: "gone", player: "p1" });
    expect(eventsOfType(sink.events, "transformed")).toHaveLength(1);
  });

  it("R424 readyToAttack: the new Unit is not summoning sick this turn; `radiant: \"keep\"` keeps the old face", () => {
    const start = playing("transform-random-ready");
    const state = start.state;
    const golem = put(state, fuseB.id, slot("p1", "units", 3), { radiant: true });
    golem.summonedTurn = state.turn;
    run(
      sinkFor(state),
      transformRandom({ instanceId: golem.id, query: { set: "Classic+" }, radiant: "keep", readyToAttack: true }),
      golem,
    );
    const now = must(state.players.p1.units[2]?.[0], "the new Unit");
    expect(now.defId).toBe(classicPlusUnit.id);
    expect(now.radiant).toBe(true);
    expect(isSick(state, now)).toBe(false);
    expect(now.exertion).toEqual({ attacked: false, switched: false });
  });

  it("R129 a refusal draws nothing: an Immutable card, an empty pool, a pool with nothing of its row", () => {
    const start = playing("transform-random-refused");
    const state = start.state;
    const locked = put(state, immutable.id, slot("p1", "units", 1));
    const plain = put(state, body.id, slot("p1", "units", 2));
    const sink = sinkFor(state);
    const cursor = sink.rng.cursor;
    run(sink, transformRandom({ instanceId: locked.id, query: { set: "Classic" } }));
    run(sink, transformRandom({ instanceId: plain.id, query: { tags: ["Pancake"] } }));
    run(sink, transformRandom({ instanceId: plain.id, query: { defId: [classicSpell.id] } }));
    expect(sink.rng.cursor).toBe(cursor);
    expect(sink.events).toEqual([]);
    expect(state.players.p1.units[0]?.[0]?.id).toBe(locked.id);
    expect(state.players.p1.units[1]?.[0]?.id).toBe(plain.id);
  });

  it("B4.1 the running card never becomes itself, and a card in hand is replaced there, hidden from the other player", () => {
    const start = playing("transform-random-hand");
    const state = start.state;
    const inHand = handCard(state, body.id);
    const sink = sinkFor(state);
    // The pool names the running card's own definition; it is excluded (R387).
    run(
      sink,
      transformRandom({ instanceId: inHand.id, query: { defId: [body.id, classicUnit.id] } }),
      must(put(state, body.id, slot("p1", "units", 4)), "the running card"),
    );
    const replaced = must(state.players.p1.hand.at(-1), "the new hand card");
    expect(replaced.defId).toBe(classicUnit.id);
    expect(defOf(state, replaced.defId).set).toBe("Classic");
    state.applied.push({ nonce: "transform-view", events: sink.events });
    const theirs = eventsOfType(viewFor(state, "p2").events, "transformed").at(-1);
    expect(theirs).toMatchObject({ instanceId: HIDDEN_ID, fromDefId: HIDDEN_ID, toDefId: HIDDEN_ID });
    const mine = eventsOfType(viewFor(state, "p1").events, "transformed").at(-1);
    expect(mine).toMatchObject({ toDefId: classicUnit.id });
  });
});
