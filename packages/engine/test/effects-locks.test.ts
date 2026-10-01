// The Lock variants and Unlock (docs/classic-sets.md B5 E20; SPEC §3.2 Lock): a whole lane (Classic #71
// Lane Eater), the zone a permanent was just played into (Classic #84 Lockdown, Classic+ #34 Memory
// Leak), a random zone not already Locked (Classic+ #34), the firing trap's own zone (Classic+ #1 Doom
// Shroom), and Unlock (Classic+ #77 Anti-Softlock) — each played through `reduce`, replayed, and read
// from both seats. Fixtures: `fixtures/field.ts`.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { lock, unlock } from "../src/effects/counters";
import { lockLane, lockOwnZone, lockPlayedZone, lockRandomZone, unlockAll } from "../src/effects/locks";
import { legalZonesFor } from "../src/playChoices";
import { hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { findInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { cardAt, isLocked, slotsOf } from "../src/zones";
import { plain } from "./fixtures/combat";
import { actResult, banner, doom, eater, flush, leak, lockdown, notesOf, playing, unlocker } from "./fixtures/field";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

function play(
  state: GameState,
  defId: string,
  zone?: { row: "units" | "backrow"; lane: number },
  options: { radiant?: boolean } = {},
): { state: GameState; events: GameEvent[]; id: string } {
  const [card] = inHand(state, defId, "p1");
  if (card === undefined) throw new Error("no card");
  if (options.radiant === true) card.radiant = true;
  flush(state, "p1");
  const result = actResult(state, { type: "play", instanceId: card.id, ...(zone === undefined ? {} : { zone }), playerId: "p1" });
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events, id: card.id };
}

function lockedZones(state: GameState): string[] {
  return (["p1", "p2"] as const).flatMap((player) =>
    (["units", "backrow"] as const).flatMap((row) =>
      slotsOf(player, row)
        .filter((ref) => isLocked(state, ref))
        .map((ref) => `${player}:${row}:${ref.lane}`),
    ),
  );
}

describe("B5 E20 Lock a lane", () => {
  it("Lane Eater's Cry Locks the four zones of its lane, itself staying in its Locked zone", () => {
    const state = playing("lock-lane");
    const out = play(state, eater.id, { row: "units", lane: 3 });
    expect(lockedZones(out.state)).toEqual(["p1:units:3", "p1:backrow:3", "p2:units:3", "p2:backrow:3"]);
    expect(cardAt(out.state, slot("p1", "units", 3))?.id).toBe(out.id);
    expect(eventsOfType(out.events, "locked")).toHaveLength(4);
    // Both seats read the Locks (public, §10.8).
    for (const viewer of ["p1", "p2"] as const) {
      expect(viewFor(out.state, viewer).you.locks.units[2]).toBe(true);
      expect(eventsOfType(viewFor(out.state, viewer).events, "locked")).toHaveLength(4);
    }
  });

  it("the Radiant face Locks only the enemy side of the lane; a zone Locked already emits nothing more", () => {
    const state = playing("lock-lane-radiant");
    state.players.p2.locks.backrow[1] = true;
    const out = play(state, eater.id, { row: "units", lane: 2 }, { radiant: true });
    expect(lockedZones(out.state)).toEqual(["p2:units:2", "p2:backrow:2"]);
    expect(eventsOfType(out.events, "locked")).toEqual([{ type: "locked", player: "p2", row: "units", lane: 2 }]);
  });

  it("a numbered lane, and a scope narrowed to one row", () => {
    const state = playing("lock-lane-number");
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    lockLane({ lane: 5, rows: ["backrow"] }).apply(ctx);
    expect(lockedZones(state)).toEqual(["p1:backrow:5", "p2:backrow:5"]);
    lockLane({ lane: 9 }).apply(ctx);
    expect(lockedZones(state)).toEqual(["p1:backrow:5", "p2:backrow:5"]);
  });
});

describe("B5 E20 Lock the zone a permanent was just played into", () => {
  it("Lockdown Locks the zone of each permanent played, either player's, and never a Spell's", () => {
    let state = playing("lock-played");
    put(state, lockdown.id, slot("p2", "backrow", 1));
    const unit = play(state, plain.id, { row: "units", lane: 4 });
    state = unit.state;
    expect(lockedZones(state)).toEqual(["p1:units:4"]);
    expect(cardAt(state, slot("p1", "units", 4))?.id).toBe(unit.id);
    // A Locked zone takes no further play.
    const [next] = inHand(state, plain.id, "p1");
    if (next === undefined) return;
    expect(legalZonesFor(state, "p1", next)).not.toContainEqual({ row: "units", lane: 4 });
    const spell = play(state, leak.id);
    expect(lockedZones(spell.state).filter((zone) => zone.startsWith("p1"))).toEqual(["p1:units:4"]);
  });

  it("reads a `summoned` event's zone, and locks nothing for a card already gone from where it landed", () => {
    const state = playing("lock-played-events");
    const sink = sinkFor(state);
    const summoned: GameEvent = { type: "summoned", player: "p2", instanceId: "c999", defId: plain.id, row: "backrow", lane: 2 };
    lockPlayedZone({ event: summoned }).apply(makeContext(sink, null, { controller: "p1" }));
    expect(lockedZones(state)).toEqual(["p2:backrow:2"]);
    const gone: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c998", defId: plain.id, costPaid: 1 };
    lockPlayedZone({ event: gone }).apply(makeContext(sink, null, { controller: "p1" }));
    expect(lockedZones(state)).toEqual(["p2:backrow:2"]);
  });
});

describe("B5 E20 Lock a random zone", () => {
  it("Locks one of the opponent's zones not Locked already, from the match rng, and replays to the same zone", () => {
    const run = (seed: string): { locked: string[]; hash: string } => {
      const state = playing(seed);
      for (let lane = 1; lane <= 5; lane += 1) state.players.p2.locks.units[lane - 1] = true;
      state.players.p2.locks.backrow[0] = true;
      const out = play(state, leak.id);
      return { locked: lockedZones(out.state), hash: hashState(out.state) };
    };
    const live = run("lock-random");
    expect(live.locked).toHaveLength(7);
    const added = live.locked.filter((zone) => zone.startsWith("p2:backrow") && zone !== "p2:backrow:1");
    expect(added).toHaveLength(1);
    expect(run("lock-random")).toEqual(live);
  });

  it("locks nothing, and draws nothing, when every zone of the scope is Locked", () => {
    const state = playing("lock-random-none");
    for (let lane = 1; lane <= 5; lane += 1) {
      state.players.p2.locks.units[lane - 1] = true;
      state.players.p2.locks.backrow[lane - 1] = true;
    }
    const sink = sinkFor(state);
    const cursor = sink.rng.cursor;
    lockRandomZone({ side: "enemy" }).apply(makeContext(sink, null, { controller: "p1" }));
    expect(sink.rng.cursor).toBe(cursor);
    expect(sink.events).toEqual([]);
  });

  it("an occupied zone is a fair pick: a Lock evicts nothing", () => {
    const state = playing("lock-random-occupied");
    for (let lane = 1; lane <= 5; lane += 1) state.players.p2.locks.backrow[lane - 1] = true;
    for (let lane = 2; lane <= 5; lane += 1) state.players.p2.locks.units[lane - 1] = true;
    const unit = put(state, plain.id, slot("p2", "units", 1));
    lockRandomZone({ side: "enemy" }).apply(makeContext(sinkFor(state), null, { controller: "p1" }));
    expect(isLocked(state, slot("p2", "units", 1))).toBe(true);
    expect(cardAt(state, slot("p2", "units", 1))?.id).toBe(unit.id);
  });
});

describe("B5 E20 Lock the firing trap's own zone", () => {
  it("Doom Shroom fires, Locks its own zone and is consumed, the zone staying Locked behind it", () => {
    const state = playing("lock-own");
    const trap = put(state, doom.id, slot("p2", "backrow", 4));
    const out = play(state, plain.id, { row: "units", lane: 1 });
    expect(isLocked(out.state, slot("p2", "backrow", 4))).toBe(true);
    expect(cardAt(out.state, slot("p2", "backrow", 4))).toBeNull();
    const consumed = findInstance(out.state, trap.id);
    expect(consumed?.zone.z).toBe("graveyard");
    expect(notesOf(consumed)).toEqual([]);
    expect(eventsOfType(out.events, "trapFired").map((e) => e.instanceId)).toEqual([trap.id]);
  });

  it("a card off the field locks nothing", () => {
    const state = playing("lock-own-gone");
    const card = put(state, banner.id, slot("p1", "backrow", 1));
    state.players.p1.backrow[0] = null;
    card.zone = { z: "graveyard", player: "p1" };
    lockOwnZone().apply(makeContext(sinkFor(state), card, { controller: "p1" }));
    expect(lockedZones(state)).toEqual([]);
  });
});

describe("B5 E20 Unlock", () => {
  it("Unlock every zone opens each Locked zone once, in R68's walk, and the zones take plays again", () => {
    const state = playing("unlock-all");
    state.players.p1.locks.units[1] = true;
    state.players.p2.locks.backrow[4] = true;
    const out = play(state, unlocker.id);
    expect(lockedZones(out.state)).toEqual([]);
    expect(eventsOfType(out.events, "unlocked")).toEqual([
      { type: "unlocked", player: "p1", row: "units", lane: 2 },
      { type: "unlocked", player: "p2", row: "backrow", lane: 5 },
    ]);
    for (const viewer of ["p1", "p2"] as const) {
      expect(eventsOfType(viewFor(out.state, viewer).events, "unlocked")).toHaveLength(2);
    }
    const [next] = inHand(out.state, plain.id, "p1");
    if (next === undefined) return;
    expect(legalZonesFor(out.state, "p1", next)).toContainEqual({ row: "units", lane: 2 });
  });

  it("the single `unlock` opens one zone; an open zone is left as it is, with no event", () => {
    const state = playing("unlock-one");
    const sink = sinkFor(state);
    const ctx = makeContext(sink, null, { controller: "p1" });
    lock({ zone: { of: "lane", row: "units", lane: 3 } }).apply(ctx);
    unlock({ zone: { of: "lane", row: "units", lane: 3 } }).apply(ctx);
    unlock({ zone: { of: "lane", row: "units", lane: 3 } }).apply(ctx);
    unlockAll({ side: "enemy" }).apply(ctx);
    expect(lockedZones(state)).toEqual([]);
    expect(sink.events.map((e) => e.type)).toEqual(["locked", "unlocked"]);
  });
});
