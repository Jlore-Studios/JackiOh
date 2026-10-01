// C+ #55 Book of Greed — SPEC §8.7 row 55, BUILD M9 Classic+ row C+ 55: "Adds 3 random non-token
// Legendary or Mythic cards of any set, a token's printed rarity never counting (no Pancake, Loser or
// KY's Gift); repeats allowed; a full hand burns; hidden from the opponent (R97); the count reads
// through `param()`; radiant they are Radiant".

import { defOf, stepParam } from "@jackioh/engine";
import { fillParams } from "@jackioh/shared";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/055-book-of-greed";

const BOOK = "classicplus-055";
const FILLER = "core-005";

function book(opts: { radiant?: boolean; seed?: string; fillers?: number } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "book-of-greed",
    p1: {
      hand: [{ def: BOOK, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)],
    },
    p2: { hand: [FILLER] },
  });
}

function added(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.filter((event): event is Extract<GameEvent, { type: "addedToHand" }> => event.type === "addedToHand" && event.player === "p1");
}

describe("C+ #55 Book of Greed", () => {
  it("is a (1) Spell, Book whose text agrees with its count at every value", () => {
    expect(def.id).toBe(BOOK);
    expect(fillParams(def, "base")).toBe("Add 3 random Legendary or Mythic cards to your hand.");
    expect(fillParams(def, "base", { cards: 1 })).toBe("Add 1 random Legendary or Mythic card to your hand.");
    expect(fillParams(def, "radiant", { cards: 1 })).toBe("Add 1 random Radiant Legendary or Mythic card to your hand.");
  });

  describe("base", () => {
    it("adds 3 random Legendary or Mythic cards to your hand", () => {
      const s = book();
      s.play(BOOK);
      const cards = added(s).map((event) => s.card(event.instanceId));
      expect(cards).toHaveLength(3);
      for (const card of cards) {
        expect(["Legendary", "Mythic"]).toContain(defOf(s.state, card.defId).rarity);
        expect(card.radiant).toBe(false);
        expect(card.costOverride).toBeUndefined();
      }
    });

    it("R380 §5 over many seeds: every set, never a token whatever rarity it prints", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 60; i += 1) {
        const s = book({ seed: `greed-${i}` });
        s.play(BOOK);
        for (const event of added(s)) seen.add(event.defId);
      }
      const state = book().state;
      const defs = [...seen].map((id) => defOf(state, id));
      expect(defs.every((entry) => !entry.token && (entry.rarity === "Legendary" || entry.rarity === "Mythic"))).toBe(true);
      expect(defs.some((entry) => entry.printedRarity !== undefined)).toBe(false);
      const sets = new Set(defs.map((entry) => entry.set));
      expect([...sets].sort()).toEqual(["Classic", "Classic+", "Core"]);
    });

    it("§2.4 R4 a full hand burns what does not fit", () => {
      const s = book({ fillers: 8 });
      s.play(BOOK);
      expect(added(s)).toHaveLength(2);
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R97 the opponent sees the adds under the sentinel", () => {
      const s = book();
      s.play(BOOK);
      const ids = added(s).map((event) => event.instanceId);
      const theirs = s.view("p2");
      const events = theirs.events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(events).toHaveLength(3);
      for (const event of events) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
      for (const id of ids) expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
    });

    it("R386 an Upgrade adds 4; a Degrade 2, and never fewer than 1", () => {
      const up = book();
      stepParam(up.card(BOOK), "cards", 1);
      up.play(BOOK);
      expect(added(up)).toHaveLength(4);

      const down = book();
      stepParam(down.card(BOOK), "cards", -1);
      down.play(BOOK);
      expect(added(down)).toHaveLength(2);

      const floor = book();
      stepParam(floor.card(BOOK), "cards", -9);
      floor.play(BOOK);
      expect(added(floor)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R74 the cards are Radiant", () => {
      const s = book({ radiant: true });
      s.play(BOOK);
      const cards = added(s).map((event) => s.card(event.instanceId));
      expect(cards).toHaveLength(3);
      expect(cards.every((card) => card.radiant)).toBe(true);
      expect(cards.every((card) => ["Legendary", "Mythic"].includes(defOf(s.state, card.defId).rarity))).toBe(true);
    });

    it("R60 repeats are allowed, and the same seed adds the same cards", () => {
      const a = book({ radiant: true, seed: "same" });
      const b = book({ radiant: true, seed: "same" });
      a.play(BOOK);
      b.play(BOOK);
      expect(added(a).map((event) => event.defId)).toEqual(added(b).map((event) => event.defId));
      let repeated = false;
      for (let i = 0; i < 200 && !repeated; i += 1) {
        const s = book({ radiant: true, seed: `rep-${i}` });
        s.play(BOOK);
        const ids = added(s).map((event) => event.defId);
        repeated = new Set(ids).size < ids.length;
      }
      expect(repeated).toBe(true);
    });
  });
});
