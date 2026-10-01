// C+ #58 Fruit Basket — SPEC §8.7 row 58, BUILD M9 Classic+ row C+ 58: "Adds 3 random cards of the
// Fruit pool, the non-token Fruit cards of any set plus the five Grapes (R382), never Fruit Basket
// (R387), repeats allowed; a seeded game can hand out a Grape; a full hand burns; hidden from the
// opponent (R97); the count reads through `param()`; radiant they are Radiant".

import { defOf, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/058-fruit-basket";

const BASKET = "classicplus-058";
const FILLER = "core-005";
const GRAPES = ["classicplus-065-1", "classicplus-065-2", "classicplus-065-3", "classicplus-065-4", "classicplus-065-5"];

function basket(opts: { radiant?: boolean; seed?: string; fillers?: number } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "fruit-basket",
    p1: {
      hand: [{ def: BASKET, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)],
    },
    p2: { hand: [FILLER] },
  });
}

function added(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.filter((event): event is Extract<GameEvent, { type: "addedToHand" }> => event.type === "addedToHand" && event.player === "p1");
}

describe("C+ #58 Fruit Basket", () => {
  it("is a (1) Spell, Fruit", () => {
    expect(def.id).toBe(BASKET);
    expect(def.tags).toEqual(["Fruit"]);
  });

  describe("base", () => {
    it("adds 3 random Fruits to your hand", () => {
      const s = basket();
      s.play(BASKET);
      const fruits = added(s).map((event) => s.card(event.instanceId));
      expect(fruits).toHaveLength(3);
      for (const card of fruits) {
        expect(card.zone.z).toBe("hand");
        expect(defOf(s.state, card.defId).tags).toContain("Fruit");
        expect(card.radiant).toBe(false);
      }
    });

    it("R382 R387 over many seeds: the Fruit pool, Grapes included, never Fruit Basket", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 80; i += 1) {
        const s = basket({ seed: `basket-${i}` });
        s.play(BASKET);
        for (const event of added(s)) seen.add(event.defId);
      }
      expect(seen.has(BASKET)).toBe(false);
      const state = basket().state;
      for (const id of seen) expect(defOf(state, id).tags).toContain("Fruit");
      expect([...seen].some((id) => GRAPES.includes(id))).toBe(true);
      expect([...seen].some((id) => !GRAPES.includes(id))).toBe(true);
      const tokens = [...seen].filter((id) => defOf(state, id).token);
      expect(tokens.every((id) => GRAPES.includes(id))).toBe(true);
    });

    it("§2.4 R4 a full hand burns what does not fit", () => {
      const s = basket({ fillers: 8 });
      s.play(BASKET);
      expect(added(s)).toHaveLength(2);
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R97 the opponent sees the adds under the sentinel", () => {
      const s = basket();
      s.play(BASKET);
      const ids = added(s).map((event) => event.instanceId);
      const theirs = s.view("p2");
      const events = theirs.events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(events).toHaveLength(3);
      for (const event of events) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
      for (const id of ids) expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
    });

    it("R386 an Upgrade adds 4; a Degrade 2", () => {
      const up = basket();
      stepParam(up.card(BASKET), "fruits", 1);
      up.play(BASKET);
      expect(added(up)).toHaveLength(4);

      const down = basket();
      stepParam(down.card(BASKET), "fruits", -1);
      down.play(BASKET);
      expect(added(down)).toHaveLength(2);
    });
  });

  describe("radiant", () => {
    it("R74 the Fruits are Radiant, a Grape too, and never Fruit Basket", () => {
      const seen: { defId: string; radiant: boolean }[] = [];
      for (let i = 0; i < 60; i += 1) {
        const s = basket({ radiant: true, seed: `rbasket-${i}` });
        s.play(BASKET);
        for (const event of added(s)) seen.push({ defId: event.defId, radiant: s.card(event.instanceId).radiant });
      }
      expect(seen.every((entry) => entry.radiant)).toBe(true);
      expect(seen.some((entry) => entry.defId === BASKET)).toBe(false);
      expect(seen.some((entry) => GRAPES.includes(entry.defId))).toBe(true);
    });

    it("R60 §9.3 repeats are allowed, and the same seed adds the same Fruits", () => {
      const a = basket({ radiant: true, seed: "same" });
      const b = basket({ radiant: true, seed: "same" });
      a.play(BASKET);
      b.play(BASKET);
      expect(added(a).map((event) => event.defId)).toEqual(added(b).map((event) => event.defId));
      let repeated = false;
      for (let i = 0; i < 100 && !repeated; i += 1) {
        const s = basket({ radiant: true, seed: `rep-${i}` });
        s.play(BASKET);
        const ids = added(s).map((event) => event.defId);
        repeated = new Set(ids).size < ids.length;
      }
      expect(repeated).toBe(true);
    });
  });
});
