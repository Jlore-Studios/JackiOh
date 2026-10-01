// C #2 The Trickster — SPEC §8.6 row 2, BUILD M9 Classic row C 2: "Cry: your next Trap, Field Trap
// or Field Spell costs (2) less, a player modifier with no "this turn" that waits across turns until
// the first such play consumes it; Spells and Units neither use nor consume it; floors at (0) (R65); a
// cast never uses it (R70); the opponent's view of your changed hand costs shows −1 (R177); radiant
// 4/2: the next one costs (0); its tuned number (discount) reads through `param()` (R386)".
//
// The cast case needs a card that casts a Trap: Classic+ #37 Wardrum's end-of-turn copy (its script
// is the Classic+ workstream's), so that one test waits for integration.

import { describe, expect, it } from "vitest";
import { effectiveCost, stepParam } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/002-the-trickster";

const TRICKSTER = "classic-002";
const EXPERIMENT = "core-085"; // (2) Trap
const SHEEPISH = "core-041"; // (1) Trap
const TESLA = "classic-005"; // (2) Field Trap
const MANA_WELL = "core-006"; // (3) Field Spell
const STOCKPILE = "core-005"; // (1) Spell
const VANILLA = "core-008"; // (1) Unit
const WARDRUM = "classicplus-037"; // (5) Unit: "End of turn: Cast a copy of a random Spell, Field Spell or Trap you played this turn."

function costOf(s: Scenario, ref: string): number {
  return effectiveCost(s.state, s.card(ref));
}

function modifierLabels(s: Scenario, viewer: "p1" | "p2", side: "you" | "opponent"): string[] {
  return s.view(viewer)[side].modifiers.map((mod) => mod.label);
}

describe("C #2 The Trickster", () => {
  it("declares its one number, discount (R386)", () => {
    expect(def.params).toEqual([{ key: "discount", base: 2, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("is a 2/1", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [TRICKSTER] }, p2: { hand: [STOCKPILE] } });
      s.expectStats(TRICKSTER, { attack: 2, health: 1, maxHealth: 1 });
    });

    it("Cry: your next Trap costs (2) less", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, EXPERIMENT, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      expect(costOf(s, EXPERIMENT)).toBe(2);
      s.play(TRICKSTER);
      expect(costOf(s, EXPERIMENT)).toBe(0);
      s.play(EXPERIMENT, { zone: 1 });
      // 4 − 1 (Trickster) − 0.
      s.expectMana("p1", 3);
    });

    it("a Field Trap counts as a Trap, and a Field Spell is reached too", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, TESLA, MANA_WELL, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(TRICKSTER);
      expect(costOf(s, TESLA)).toBe(0);
      expect(costOf(s, MANA_WELL)).toBe(1);
    });

    it("the first such play consumes it: the next one pays full price", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, MANA_WELL, SHEEPISH, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(TRICKSTER);
      s.play(MANA_WELL, { zone: 1 });
      s.expectMana("p1", 2); // 4 − 1 − 1
      expect(costOf(s, SHEEPISH)).toBe(1);
      expect(modifierLabels(s, "p1", "you")).toEqual([]);
    });

    it("the first such play consumes it even when that card already costs (0): under Cloaked Toe Cracker", () => {
      const s = scenario({
        p1: { hand: [TRICKSTER, EXPERIMENT, MANA_WELL, STOCKPILE], field: ["classic-006"] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(TRICKSTER);
      expect(costOf(s, EXPERIMENT)).toBe(0);
      s.play(EXPERIMENT, { zone: 1 });
      expect(modifierLabels(s, "p1", "you")).toEqual([]);
      expect(costOf(s, MANA_WELL)).toBe(3);
    });

    it("R65 the price floors at (0): a (1) Cost Trap costs (0), not less", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, SHEEPISH, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(TRICKSTER);
      expect(costOf(s, SHEEPISH)).toBe(0);
      s.play(SHEEPISH, { zone: 1 });
      s.expectMana("p1", 3);
    });

    it("Spells and Units neither use nor consume it", () => {
      const s = scenario({
        p1: { hand: [TRICKSTER, STOCKPILE, VANILLA, EXPERIMENT], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(TRICKSTER);
      expect(costOf(s, STOCKPILE)).toBe(1);
      expect(costOf(s, VANILLA)).toBe(1);
      s.play(STOCKPILE);
      s.play(VANILLA);
      expect(costOf(s, EXPERIMENT)).toBe(0);
      expect(modifierLabels(s, "p1", "you")).toEqual(["Your next Trap or Field Spell costs (2) less"]);
    });

    it("no \"this turn\": it waits across turns until used", () => {
      const s = scenario({
        p1: { hand: [TRICKSTER, MANA_WELL, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(TRICKSTER);
      s.endTurn();
      expect(s.state.active).toBe("p2");
      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(costOf(s, MANA_WELL)).toBe(1);
      s.play(MANA_WELL, { zone: 1 });
      expect(modifierLabels(s, "p1", "you")).toEqual([]);
    });

    it("it is yours: the opponent's Traps are untouched", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, STOCKPILE] }, p2: { hand: [EXPERIMENT, STOCKPILE] } });
      s.play(TRICKSTER);
      expect(costOf(s, EXPERIMENT)).toBe(2);
    });

    it("R177 the opponent reads your hand as a count and no cost of it, only the public badge", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, EXPERIMENT, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(TRICKSTER);
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 2 });
      for (const event of theirs.events) {
        if (event.type === "costChanged") expect(event.cost).toBe(-1);
      }
      expect(JSON.stringify(theirs)).not.toContain(EXPERIMENT);
      expect(modifierLabels(s, "p2", "opponent")).toEqual(["Your next Trap or Field Spell costs (2) less"]);
      // The change is real in your own view: the Trap reads (0) there.
      const yourHand = s.view("p1").you.hand;
      if (!Array.isArray(yourHand)) throw new Error("your own hand travels in full (§10.8)");
      expect(yourHand.find((card) => card.defId === EXPERIMENT)?.cost).toBe(0);
    });

    it("R70 a cast never uses it: a Trap Wardrum casts leaves the discount waiting", () => {
      const s = scenario({
        p1: { hand: [SHEEPISH, TRICKSTER, MANA_WELL, STOCKPILE], field: [WARDRUM], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(SHEEPISH, { zone: 1 }); // before the Trickster: full price
      s.play(TRICKSTER);
      s.endTurn(); // Wardrum casts a copy of the Sheepish
      // The played Sheepish and the cast copy: two plays of it, the second paying nothing (R70).
      const plays = s.events.filter((event) => event.type === "cardPlayed" && event.defId === SHEEPISH);
      expect(plays).toHaveLength(2);
      expect(modifierLabels(s, "p1", "you")).toEqual(["Your next Trap or Field Spell costs (2) less"]);
    });

    it("R386 an Upgrade of discount makes it (3) less", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, MANA_WELL, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      stepParam(s.card(TRICKSTER), "discount", 1);
      s.play(TRICKSTER);
      expect(costOf(s, MANA_WELL)).toBe(0);
      expect(modifierLabels(s, "p1", "you")).toEqual(["Your next Trap or Field Spell costs (3) less"]);
    });

    it("R386 a Degrade of discount makes it (1) less", () => {
      const s = scenario({ p1: { hand: [TRICKSTER, MANA_WELL, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      stepParam(s.card(TRICKSTER), "discount", -1);
      s.play(TRICKSTER);
      expect(costOf(s, MANA_WELL)).toBe(2);
    });
  });

  describe("radiant", () => {
    it("is a 4/2", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [{ def: TRICKSTER, radiant: true }] }, p2: { hand: [STOCKPILE] } });
      s.expectStats(TRICKSTER, { attack: 4, health: 2, maxHealth: 2 });
    });

    it("Cry: your next Trap, Field Trap or Field Spell costs (0)", () => {
      const s = scenario({
        p1: { hand: [{ def: TRICKSTER, radiant: true }, MANA_WELL, TESLA, EXPERIMENT, STOCKPILE] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(TRICKSTER);
      expect(costOf(s, MANA_WELL)).toBe(0);
      expect(costOf(s, TESLA)).toBe(0);
      expect(costOf(s, EXPERIMENT)).toBe(0);
      expect(modifierLabels(s, "p1", "you")).toEqual(["Your next Trap or Field Spell costs (0)"]);
      s.play(MANA_WELL, { zone: 1 });
      s.expectMana("p1", 3);
      // Consumed by that play.
      expect(costOf(s, EXPERIMENT)).toBe(2);
    });

    it("Spells and Units neither use nor consume it, and it waits across turns", () => {
      const s = scenario({
        p1: { hand: [{ def: TRICKSTER, radiant: true }, STOCKPILE, MANA_WELL], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(TRICKSTER);
      expect(costOf(s, STOCKPILE)).toBe(1);
      s.play(STOCKPILE);
      s.endTurn();
      s.endTurn();
      expect(costOf(s, MANA_WELL)).toBe(0);
    });
  });
});
