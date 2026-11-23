// #3609: a fresh view shares every unchanged card with the one before it, so only what changed is drawn again.
// A view comes from JSON.parse, so a clone is how a test hands one in: it equals the old view and shares nothing.

import type { CardView, GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { card, fullBoardView, withEvents } from "../test/fixtures.ts";
import { shareEqual, shareView } from "./shareView.ts";

/** What the wasm module and a socket hand the client: the same data, as new objects. */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function handOf(view: PlayerView): CardView[] {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the fixture's own hand is cards");
  return hand;
}

function unitOf(view: PlayerView, lane: number): UnitView {
  const found = view.you.units[lane];
  if (found === null || found === undefined) throw new Error(`the fixture has a unit in lane ${lane + 1}`);
  return found;
}

const MANA: GameEvent = { type: "manaChanged", player: "p1", current: 1, max: 2 };

describe("#3609 shareEqual", () => {
  it("returns next when it is prev, and a primitive as it is", () => {
    const same = { a: 1 };
    expect(shareEqual(same, same)).toBe(same);
    expect(shareEqual(1, 2)).toBe(2);
    expect(shareEqual("a", "a")).toBe("a");
    expect(shareEqual(Number.NaN, Number.NaN)).toBeNaN();
    expect(shareEqual(null, null)).toBeNull();
  });

  it("returns prev for an equal clone, however deep", () => {
    const prev = { list: [{ id: 1, tags: ["a", "b"] }, null], nested: { deeper: { n: 1 } } };
    expect(shareEqual(prev, clone(prev))).toBe(prev);
  });

  it("shares nothing when the shapes differ, and the result still equals next", () => {
    const prev = { a: { n: 1 }, b: 2 };
    const swapped = { a: [1], b: 2 };
    expect(shareEqual(prev, swapped)).toEqual(swapped);
    const grown = { a: { n: 1 }, b: 2, c: 3 };
    const widened = shareEqual(prev, grown);
    expect(widened).toEqual(grown);
    expect(widened).not.toBe(prev);
    expect(widened.a).toBe(prev.a);
    const shrunk = { a: { n: 1 } };
    const narrowed = shareEqual(prev, shrunk);
    expect(narrowed).toEqual(shrunk);
    expect(narrowed).not.toBe(prev);
    expect(narrowed.a).toBe(prev.a);
    expect(shareEqual({ a: 1 }, { b: 1 })).toEqual({ b: 1 });
  });

  it("keeps a key whose value is undefined distinct from an absent key", () => {
    const withKey = { a: 1, b: undefined };
    const without = { a: 1 };
    expect(shareEqual(without, withKey)).toEqual(withKey);
    expect(Object.keys(shareEqual(without, withKey))).toEqual(["a", "b"]);
    expect(Object.keys(shareEqual(withKey, without))).toEqual(["a"]);
  });

  it("does not turn an own __proto__ key into a prototype", () => {
    const hostile = JSON.parse('{"__proto__":{"polluted":true},"a":1}') as Record<string, unknown>;
    const shared = shareEqual({ a: 0 }, hostile);
    expect(Object.getPrototypeOf(shared)).toBe(Object.prototype);
    expect(Object.hasOwn(shared, "__proto__")).toBe(true);
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });
});

describe("#3609 shareView", () => {
  it("returns an equal clone's seats and cards as the previous view's objects", () => {
    const prev = fullBoardView({ trades: { cost: 0, left: 1 } });
    const next = clone(prev);
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    expect(shared.you).toBe(prev.you);
    expect(shared.opponent).toBe(prev.opponent);
    expect(shared.trades).toBe(prev.trades);
    expect(shared.you.units).toBe(prev.you.units);
    expect(shared.you.units[1]).toBe(prev.you.units[1]);
    expect(handOf(shared)[0]).toBe(handOf(prev)[0]);
    expect(shared.opponent.units[0]).toBe(prev.opponent.units[0]);
    expect(shared.opponent.backrow[2]).toBe(prev.opponent.backrow[2]);
  });

  it("replaces only the path to a changed leaf", () => {
    const prev = fullBoardView();
    const next = clone(prev);
    unitOf(next, 1).attack += 1;
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    expect(shared.you).not.toBe(prev.you);
    expect(shared.you.units).not.toBe(prev.you.units);
    expect(shared.you.units[1]).not.toBe(prev.you.units[1]);
    expect(shared.you.units[1]?.attack).toBe((prev.you.units[1]?.attack ?? 0) + 1);
    for (const lane of [0, 2, 3, 4]) expect(shared.you.units[lane]).toBe(prev.you.units[lane]);
    expect(shared.you.hand).toBe(prev.you.hand);
    expect(shared.you.backrow).toBe(prev.you.backrow);
    expect(shared.you.hero).toBe(prev.you.hero);
    expect(shared.opponent).toBe(prev.opponent);
  });

  it("keeps the later hand cards the previous view's objects when the first is played", () => {
    const prev = fullBoardView();
    const next = clone(prev);
    next.you.hand = handOf(next).slice(1);
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    const before = handOf(prev);
    const after = handOf(shared);
    expect(after).toHaveLength(before.length - 1);
    after.forEach((entry, index) => expect(entry).toBe(before[index + 1]));
  });

  it("pairs a card with itself by instance id when the order changes", () => {
    const prev = fullBoardView();
    const next = clone(prev);
    next.you.hand = handOf(next).reverse();
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    const before = handOf(prev);
    const after = handOf(shared);
    expect(after).not.toBe(before);
    expect(after.map((entry) => entry.instanceId)).toEqual([...before].reverse().map((entry) => entry.instanceId));
    after.forEach((entry, index) => expect(entry).toBe(before[before.length - 1 - index]));
  });

  it("shares a unit that changes lane, and shares nothing for a card that is new", () => {
    const prev = fullBoardView();
    const next = clone(prev);
    const moved = next.you.units[0] ?? null;
    next.you.units[0] = null;
    next.you.units[2] = moved;
    const fresh = card({ instanceId: "fresh1", defId: "core-002" });
    next.you.hand = [...handOf(next), fresh];
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    expect(shared.you.units[2]).toBe(prev.you.units[0]);
    expect(shared.you.units[3]).toBe(prev.you.units[3]);
    expect(handOf(shared).at(-1)).toBe(fresh);
  });

  it("returns next itself for no previous view and for a different viewer", () => {
    const next = fullBoardView();
    expect(shareView(null, next)).toBe(next);
    const prev = clone(next);
    prev.viewer = "p2";
    const shared = shareView(prev, next);
    expect(shared).toBe(next);
    expect(shared.you).toBe(next.you);
    expect(shared.you.units[0]).not.toBe(prev.you.units[0]);
  });

  it("returns next itself when it is the previous view, which has nothing to share with itself", () => {
    const view = fullBoardView({ trades: { cost: 0, left: 1 } });
    expect(shareView(view, view)).toBe(view);
  });

  it("gives a new root for a new turn, keeps the seats, and never shares the events", () => {
    const prev = withEvents(fullBoardView(), [MANA]);
    const next = { ...clone(prev), turn: prev.turn + 1 };
    const shared = shareView(prev, next);
    expect(shared).toEqual(next);
    expect(shared).not.toBe(next);
    expect(shared).not.toBe(prev);
    expect(shared.turn).toBe(prev.turn + 1);
    expect(shared.you).toBe(prev.you);
    expect(shared.opponent).toBe(prev.opponent);
    expect(shared.events).toBe(next.events);
    expect(shared.events).toEqual(prev.events);
    expect(shared.events).not.toBe(prev.events);
    expect(shared.events[0]).toBe(next.events[0]);
    expect(shared.events[0]).not.toBe(prev.events[0]);
  });

  it("shares neither the pending prompt nor the definitions", () => {
    const prev = fullBoardView({ defs: { x: { id: "x" } as never } });
    const next = clone(prev);
    const shared = shareView(prev, next);
    expect(shared.defs).toBe(next.defs);
    expect(shared.defs).not.toBe(prev.defs);
    expect(shared.pending).toBe(next.pending);
  });

  it("leaves an absent trades key absent, and shares one that is there", () => {
    const prev = fullBoardView({ trades: { cost: 0, left: 1 } });
    const next = clone(prev);
    delete next.trades;
    const without = shareView(prev, next);
    expect(without).toEqual(next);
    expect(Object.hasOwn(without, "trades")).toBe(false);
    const spent = { ...clone(prev), trades: { cost: 0, left: 0 } };
    const changed = shareView(prev, spent);
    expect(changed.trades).toEqual({ cost: 0, left: 0 });
    expect(changed.trades).not.toBe(prev.trades);
    expect(changed.you).toBe(prev.you);
  });

  it("does not change the previous view, which a component may still hold", () => {
    const prev = fullBoardView();
    const before = clone(prev);
    const next = clone(prev);
    unitOf(next, 0).health = 99;
    shareView(prev, next);
    expect(prev).toEqual(before);
  });
});
