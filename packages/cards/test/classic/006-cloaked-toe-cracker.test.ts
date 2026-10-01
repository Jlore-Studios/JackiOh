// C #6 Cloaked Toe Cracker — SPEC §8.6 row 6, BUILD M9 Classic row C 6: "Aura: your Traps and Field
// Traps in hand cost (0) while it is on the field (R65) and return to their cost when it leaves; the
// opponent's Traps and your Field Spells are untouched; the opponent's view of your changed hand costs
// shows −1 (R177); radiant 6/8: also, after you play a Trap or Field Trap (a cast included, R70), gain
// 1 mana this turn; the gain never names the face-down trap in the opponent's view (R33, R97); its
// tuned number (radiant mana) reads through `param()` (R386)".
//
// The cast case needs a card that casts a Trap: Classic+ #37 Wardrum's end-of-turn copy (the Classic+
// workstream's script), so that one test waits for integration.

import { describe, expect, it } from "vitest";
import { effectiveCost, stepParam } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/006-cloaked-toe-cracker";

const CRACKER = "classic-006";
const EXPERIMENT = "core-085"; // (2) Trap
const SHEEPISH = "core-041"; // (1) Trap
const TESLA = "classic-005"; // (2) Field Trap
const MANA_WELL = "core-006"; // (3) Field Spell
const STOCKPILE = "core-005"; // (1) Spell
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const WARDRUM = "classicplus-037"; // (5) Unit: "End of turn: Cast a copy of a random Spell, Field Spell or Trap you played this turn."

function costOf(s: Scenario, ref: string): number {
  return effectiveCost(s.state, s.card(ref));
}

describe("C #6 Cloaked Toe Cracker", () => {
  it("declares its one number, the Radiant face's mana (R386)", () => {
    expect(def.params).toEqual([{ key: "mana", base: 1, radiant: 1, better: "up", step: 1, min: 1 }]);
    expect(base.triggers).toBeUndefined();
    expect(radiant.triggers?.map((trigger) => trigger.on)).toEqual([["cardPlayed"]]);
  });

  describe("base", () => {
    it("is a 3/4", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [CRACKER] }, p2: { hand: [STOCKPILE] } });
      s.expectStats(CRACKER, { attack: 3, health: 4, maxHealth: 4 });
    });

    it("R65 Aura: your Traps and Field Traps in hand cost (0) while it is on the field", () => {
      const s = scenario({ p1: { hand: [EXPERIMENT, SHEEPISH, TESLA], field: [CRACKER] }, p2: { hand: [STOCKPILE] } });
      expect(costOf(s, EXPERIMENT)).toBe(0);
      expect(costOf(s, SHEEPISH)).toBe(0);
      expect(costOf(s, TESLA)).toBe(0);
      s.play(EXPERIMENT, { zone: 1 });
      s.play(TESLA, { zone: 2 });
      s.expectMana("p1", 4);
    });

    it("played from hand, the Trap costs (0) at once", () => {
      const s = scenario({ p1: { hand: [CRACKER, EXPERIMENT, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      expect(costOf(s, EXPERIMENT)).toBe(2);
      s.play(CRACKER);
      expect(costOf(s, EXPERIMENT)).toBe(0);
    });

    it("once it leaves the field your Traps return to their own cost", () => {
      const s = scenario({
        p1: { hand: [EXPERIMENT, STOCKPILE], field: [CRACKER] },
        p2: { hand: [HIT_JOB, STOCKPILE] },
        active: "p2",
      });
      expect(costOf(s, EXPERIMENT)).toBe(0);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(CRACKER).id }] });
      s.expectInZone(CRACKER, "graveyard");
      expect(costOf(s, EXPERIMENT)).toBe(2);
    });

    it("the opponent's Traps and your Field Spells and Spells are untouched", () => {
      const s = scenario({
        p1: { hand: [MANA_WELL, STOCKPILE], field: [CRACKER] },
        p2: { hand: [EXPERIMENT, STOCKPILE] },
      });
      expect(costOf(s, MANA_WELL)).toBe(3);
      expect(costOf(s, STOCKPILE)).toBe(1);
      expect(costOf(s, EXPERIMENT)).toBe(2);
    });

    it("the base face gains no mana after a Trap", () => {
      const s = scenario({ p1: { hand: [EXPERIMENT, STOCKPILE], field: [CRACKER] }, p2: { hand: [STOCKPILE] } });
      s.play(EXPERIMENT, { zone: 1 });
      s.expectMana("p1", 4);
    });

    it("R177 the opponent reads your hand as a count and no cost of it", () => {
      const s = scenario({ p1: { hand: [CRACKER, EXPERIMENT, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(CRACKER);
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 2 });
      for (const event of theirs.events) {
        if (event.type === "costChanged") expect(event.cost).toBe(-1);
      }
      expect(JSON.stringify(theirs)).not.toContain(EXPERIMENT);
    });
  });

  describe("radiant", () => {
    it("is a 6/8 with the same Aura", () => {
      const s = scenario({ p1: { hand: [EXPERIMENT], field: [{ def: CRACKER, radiant: true }] }, p2: { hand: [STOCKPILE] } });
      s.expectStats(CRACKER, { attack: 6, health: 8, maxHealth: 8 });
      expect(costOf(s, EXPERIMENT)).toBe(0);
    });

    it("after you play a Trap, gain 1 mana this turn", () => {
      const s = scenario({
        p1: { hand: [EXPERIMENT, STOCKPILE], field: [{ def: CRACKER, radiant: true }] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(EXPERIMENT, { zone: 1 });
      s.expectMana("p1", 5);
    });

    it("a Field Trap counts as a Trap", () => {
      const s = scenario({
        p1: { hand: [TESLA, SHEEPISH, STOCKPILE], field: [{ def: CRACKER, radiant: true }] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(TESLA, { zone: 1 });
      s.play(SHEEPISH, { zone: 2 });
      s.expectMana("p1", 6);
    });

    it("a Spell, a Field Spell or the opponent's Trap gains nothing", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, MANA_WELL], field: [{ def: CRACKER, radiant: true }], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [SHEEPISH, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(STOCKPILE);
      s.expectMana("p1", 3);
      s.play(MANA_WELL, { zone: 1 });
      s.expectMana("p1", 0);
      s.endTurn();
      s.play(SHEEPISH, { zone: 1 });
      s.expectMana("p2", 3);
      expect(s.view("p1").you.mana.current).toBe(0);
    });

    it("the mana is this turn's: it is gone at your next refresh", () => {
      const s = scenario({
        p1: { hand: [EXPERIMENT, STOCKPILE], field: [{ def: CRACKER, radiant: true }], library: [STOCKPILE, STOCKPILE] },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(EXPERIMENT, { zone: 1 });
      s.expectMana("p1", 5);
      s.endTurn();
      s.endTurn();
      s.expectMana("p1", 4);
    });

    it("R33 R97 the gain never names the face-down trap in the opponent's view", () => {
      const s = scenario({
        p1: { hand: [EXPERIMENT, STOCKPILE], field: [{ def: CRACKER, radiant: true }] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(EXPERIMENT, { zone: 1 });
      const trap = s.backrow("p1", 1);
      if (trap === null) throw new Error("the Trap should be set");
      const theirs = s.view("p2");
      expect(theirs.opponent.backrow[0]).toEqual({ faceDown: true, cost: 2 });
      expect(JSON.stringify(theirs)).not.toContain(`"${trap.id}"`);
      expect(JSON.stringify(theirs)).not.toContain(EXPERIMENT);
      expect(theirs.events.filter((event) => event.type === "manaChanged")).toContainEqual({
        type: "manaChanged",
        player: "p1",
        current: 5,
        max: 4,
      });
    });

    it("R70 a cast Trap counts: Wardrum's copy of the Trap you played gains mana too", () => {
      const s = scenario({
        p1: {
          hand: [SHEEPISH, STOCKPILE],
          field: [{ def: CRACKER, radiant: true }, WARDRUM],
          library: [STOCKPILE, STOCKPILE],
        },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(SHEEPISH, { zone: 1 });
      s.expectMana("p1", 5);
      s.endTurn(); // Wardrum casts a copy of the Sheepish at the end of p1's turn
      const gains = s.events.filter((event) => event.type === "manaChanged" && event.player === "p1" && event.current === 6);
      expect(gains).toHaveLength(1);
    });

    it("R386 an Upgrade of mana gains 2", () => {
      const s = scenario({
        p1: { hand: [EXPERIMENT, STOCKPILE], field: [{ def: CRACKER, radiant: true }] },
        p2: { hand: [STOCKPILE] },
      });
      stepParam(s.card(CRACKER), "mana", 1);
      s.play(EXPERIMENT, { zone: 1 });
      s.expectMana("p1", 6);
    });
  });
});
