// C+ #13 Mommy Barker — SPEC §8.7 row 13, BUILD M9 Classic+ row C+ 13: "Death adds 1 random Pancake
// token (C+ #12.1–#12.8) to your hand, hidden from the opponent (R97); exile or a bounce adds nothing; a
// full hand burns it; the count reads through `param()`; radiant 4/4 with Reborn, adding a token on
// both deaths (R8)".

import { HAND_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const MOMMY = "classicplus-013";
const MENACE = "core-019"; // 9/9: Mommy dies attacking it.
const FLOOD = "core-017"; // (4) Spell: bounce all Units.
const COLLATERAL = "core-034"; // (4) Spell: exile target permanent and a random card of the opponent's deck.
const FILLER = "core-005";
const PANCAKE_TOKENS = [1, 2, 3, 4, 5, 6, 7, 8].map((k) => `classicplus-012-${k}`);

function pancakesIn(s: Scenario): string[] {
  return s.hand("p1").map((card) => card.defId).filter((id) => PANCAKE_TOKENS.includes(id));
}

function diesAttacking(radiant: boolean, hand: readonly string[] = [FILLER]): Scenario {
  const s = scenario({
    p1: { hand, field: [{ def: MOMMY, radiant }], library: [FILLER] },
    p2: { hand: [FILLER], field: [MENACE] },
  });
  s.attack(MOMMY, MENACE);
  return s;
}

describe("C+ #13 Mommy Barker", () => {
  describe("base", () => {
    it("Death adds one random Pancake token to your hand", () => {
      const s = diesAttacking(false);
      s.expectInZone(MOMMY, "graveyard");
      expect(pancakesIn(s)).toHaveLength(1);
    });

    it("R97 the token is hidden from the opponent", () => {
      const s = diesAttacking(false);
      expect(JSON.stringify(s.view("p2"))).not.toMatch(/classicplus-012-\d/);
    });

    it("a bounce adds nothing", () => {
      const s = scenario({ p1: { hand: [FLOOD, FILLER], field: [MOMMY], library: [FILLER] }, p2: { hand: [FILLER] } });
      s.play(FLOOD);
      s.expectInZone(MOMMY, "hand");
      expect(pancakesIn(s)).toHaveLength(0);
    });

    it("exile adds nothing: it never dies", () => {
      const s = scenario({ p1: { hand: [COLLATERAL, FILLER], field: [MOMMY], library: [FILLER], mana: 8 }, p2: { hand: [FILLER], library: [FILLER] } });
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(MOMMY).id }] });
      s.expectInZone(MOMMY, "exile");
      expect(pancakesIn(s)).toHaveLength(0);
    });

    it("a full hand burns it", () => {
      const s = diesAttacking(false, Array.from({ length: HAND_CAP }, () => FILLER));
      expect(pancakesIn(s)).toHaveLength(0);
      expect(s.events.some((event) => event.type === "burned" && PANCAKE_TOKENS.includes(event.defId))).toBe(true);
    });

    it("R386 the count reads through param(): an Upgrade adds 2", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [MOMMY], library: [FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });
      stepParam(s.card(MOMMY), "tokens", 1);
      s.attack(MOMMY, MENACE);
      expect(pancakesIn(s)).toHaveLength(2);
    });
  });

  describe("radiant", () => {
    it("4/4 with Reborn; a token on each of both deaths (R8)", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MOMMY, radiant: true }], library: [FILLER] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.expectStats(MOMMY, { attack: 4, health: 4 });
      expect(s.stats(MOMMY).keywords).toContainEqual({ kind: "Reborn" });
      s.attack(MOMMY, MENACE); // first death: Reborn brings it back
      expect(pancakesIn(s)).toHaveLength(1);
      expect(s.unit("p1", 1)?.defId).toBe(MOMMY);
      s.endTurn().endTurn();
      s.attack(s.unit("p1", 1)?.id ?? MOMMY, MENACE); // second death
      expect(pancakesIn(s)).toHaveLength(2);
      // §5.1: added on their base faces.
      expect(s.hand("p1").filter((card) => PANCAKE_TOKENS.includes(card.defId)).every((card) => !card.radiant)).toBe(true);
    });
  });
});
