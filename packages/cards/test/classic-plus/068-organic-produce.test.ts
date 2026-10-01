// C+ #68 Organic Produce — SPEC §8.7 row 68, BUILD M9 row C+ 68: a Field Spell whose Cry adds a random
// card of the Fruit pool (R382), never Organic Produce (R387); its aura makes every Fruit-tagged card its
// controller plays Radiant at §10.5 step 3 (R213, R214), so it resolves on its Radiant face; the
// opponent's Fruits are untouched; a Fruit Trap set face-down is made Radiant unread by the opponent
// (R177); the count reads through `param()`; radiant its Cry adds 2 Fruits that cost (0).

import { describe, expect, it } from "vitest";
import {
  HAND_CAP,
  HIDDEN_ID,
  createRng,
  effectiveCost,
  query,
  stepParam,
  subsystems,
  type EngineSink,
} from "@jackioh/engine";
import { base, def, radiant } from "../../src/scripts/classic-plus/068-organic-produce";
import { scenario, type Scenario } from "../_harness";

const PRODUCE = "classicplus-068";
const FIG = "core-047"; // Fig of Life: (3) Spell, Fruit. "Heal a target 20." / Radiant 50.
const TRAP = "core-041"; // Sheepish, a Trap with no Fruit tag.
const FILLER = "core-005";

const FRUIT_POOL = query({ tags: ["Fruit"] })
  .map((card) => card.id)
  .filter((id) => id !== PRODUCE);

function added(s: Scenario, before: readonly string[]): string[] {
  return s
    .hand("p1")
    .filter((card) => !before.includes(card.id))
    .map((card) => card.defId);
}

function playProduce(s: Scenario): string[] {
  const before = s.hand("p1").map((card) => card.id);
  s.play(PRODUCE, { zone: 1 });
  return added(s, before);
}

function healHero(s: Scenario, player: "p1" | "p2"): void {
  const fig = s.hand(player).find((card) => card.defId === FIG);
  if (fig === undefined) throw new Error("no Fig of Life");
  s.play(fig, { targets: [{ pick: "hero", player }] });
}

describe("C+ #68 Organic Produce", () => {
  it("is a (4) Field Spell, Fruit; both faces flag Fruit plays Radiant", () => {
    expect(def).toMatchObject({ cost: 4, type: "Field Spell", tags: ["Fruit"] });
    expect(base.staticFlags?.radiantPlaysTagged).toEqual(["Fruit"]);
    expect(radiant.staticFlags?.radiantPlaysTagged).toEqual(["Fruit"]);
  });

  describe("base", () => {
    it("R382 its Cry adds one random card of the Fruit pool, which holds the five Grapes", () => {
      expect(FRUIT_POOL).toEqual(expect.arrayContaining(["classicplus-065-1", "classicplus-065-5", FIG]));
      const s = scenario({ p1: { hand: [PRODUCE, FILLER] }, p2: { hand: [FILLER] } });
      const fruit = playProduce(s);
      expect(fruit).toHaveLength(1);
      expect(FRUIT_POOL).toContain(fruit[0]);
    });

    it("R387 never Organic Produce itself, whatever the seed, and every pick is from the Fruit pool", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 40; n += 1) {
        const s = scenario({ seed: `produce-${n}`, p1: { hand: [PRODUCE, FILLER] }, p2: { hand: [FILLER] } });
        for (const id of playProduce(s)) seen.add(id);
      }
      expect(seen.has(PRODUCE)).toBe(false);
      expect([...seen].every((id) => FRUIT_POOL.includes(id))).toBe(true);
      expect(seen.size).toBeGreaterThan(3);
    });

    it("§2.4 a full hand burns the Fruit it adds", () => {
      const s = scenario({ p1: { hand: [PRODUCE, ...Array.from({ length: HAND_CAP }, () => FILLER)] }, p2: { hand: [FILLER] } });
      const graveyard = s.pile("p1", "graveyard").length;
      playProduce(s);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.pile("p1", "graveyard").length).toBe(graveyard + 1);
      s.expectEvents("burned");
    });

    it("R386 the count reads through param(): an Upgrade of `fruits` adds two", () => {
      const s = scenario({ p1: { hand: [PRODUCE, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(PRODUCE), "fruits", 1);
      expect(playProduce(s)).toHaveLength(2);
    });

    it("R213 R214 a Fruit you play while it stands is made Radiant as it is played and resolves on its Radiant face", () => {
      const s = scenario({ p1: { hand: [FIG, FILLER], backrow: [PRODUCE], health: 10, mana: 10 }, p2: { hand: [FILLER] } });
      healHero(s, "p1");
      // Fig of Life's Radiant face heals 50, not 20.
      s.expectHealth("p1", 60);
      expect(s.card(FIG).radiant).toBe(true);
      s.expectEvents("radiantSet", "cardPlayed");
    });

    it("R213 a non-Fruit you play is untouched, and so are the opponent's Fruits", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], backrow: [PRODUCE] },
        p2: { hand: [FIG, FILLER], health: 10, mana: 10 },
      });
      healHero(s, "p2");
      s.expectHealth("p2", 30);
      expect(s.card(FIG).radiant).toBe(false);
      const own = scenario({ p1: { hand: [FILLER, FILLER], backrow: [PRODUCE] }, p2: { hand: [FILLER] } });
      own.play(own.hand("p1")[0]!);
      expect(own.events.some((event) => event.type === "radiantSet")).toBe(false);
    });

    it("R119 it never catches its own play, but a second Organic Produce is a Fruit: Radiant, its Cry adds 2 that cost (0)", () => {
      const s = scenario({ p1: { hand: [PRODUCE, PRODUCE, FILLER], mana: 8 }, p2: { hand: [FILLER] } });
      const [first, second] = s.hand("p1");
      s.play(first!, { zone: 1 });
      expect(s.card(first!).radiant).toBe(false);
      const before = s.hand("p1").map((card) => card.id);
      s.play(second!, { zone: 2 });
      expect(s.card(second!).radiant).toBe(true);
      const fruits = s.hand("p1").filter((card) => !before.includes(card.id));
      expect(fruits).toHaveLength(2);
      expect(fruits.map((card) => effectiveCost(s.state, card))).toEqual([0, 0]);
    });

    it("R177 a Fruit Trap set face-down is made Radiant, and the opponent reads only that some card changed", () => {
      const s = scenario({ p1: { hand: [TRAP, FIG, FILLER], backrow: [PRODUCE], mana: 10 }, p2: { hand: [FILLER] } });
      // A Trap that carries the Fruit tag: Sheepish with a Fig of Life fused into it, kept in hand (R470).
      const state = s.state;
      const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
      const fused = subsystems.fuse(sink, { ingredients: [s.card(FIG)], into: s.card(TRAP), handPrice: "fused" });
      state.rngCursor = sink.rng.cursor;
      if (fused === null) throw new Error("the fusion");
      expect(s.card(fused.id).zone.z).toBe("hand");
      // The fused card carries Fig of Life's declared target too (R102).
      s.play(fused.id, { zone: 3, targets: [{ pick: "hero", player: "p1" }] });
      const set = s.backrow("p1", 3);
      // R227: a card set face-down takes a fresh id.
      expect(set?.defId).toBe(fused.defId);
      expect(set?.radiant).toBe(true);
      expect(set?.faceUp).not.toBe(true);
      const theirs = s.view("p2").events.filter((event) => event.type === "radiantSet");
      expect(theirs).toHaveLength(1);
      expect(theirs[0]).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
      const mine = s.view("p1").events.filter((event) => event.type === "radiantSet");
      expect(mine[0]).toMatchObject({ defId: fused.defId });
      expect(mine[0]).not.toMatchObject({ instanceId: HIDDEN_ID });
    });
  });

  describe("radiant", () => {
    it("its Cry adds 2 random Fruits that cost (0)", () => {
      const s = scenario({ p1: { hand: [{ def: PRODUCE, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      const before = s.hand("p1").map((card) => card.id);
      s.play(PRODUCE, { zone: 1 });
      const fruits = s.hand("p1").filter((card) => !before.includes(card.id));
      expect(fruits).toHaveLength(2);
      expect(fruits.every((card) => FRUIT_POOL.includes(card.defId))).toBe(true);
      expect(fruits.map((card) => effectiveCost(s.state, card))).toEqual([0, 0]);
    });

    it("R213 its aura is the same: a Fruit you play resolves Radiant", () => {
      const s = scenario({
        p1: { hand: [FIG, FILLER], backrow: [{ def: PRODUCE, radiant: true }], health: 10, mana: 10 },
        p2: { hand: [FILLER] },
      });
      healHero(s, "p1");
      s.expectHealth("p1", 60);
    });

    it("R386 an Upgrade of `fruits` adds three", () => {
      const s = scenario({ p1: { hand: [{ def: PRODUCE, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(PRODUCE), "fruits", 1);
      const before = s.hand("p1").map((card) => card.id);
      s.play(PRODUCE, { zone: 1 });
      expect(s.hand("p1").filter((card) => !before.includes(card.id))).toHaveLength(3);
    });
  });
});
