// C+ #12 The Mother Pancake — SPEC §8.7 row 12, BUILD M9 Classic+ row C+ 12: "Taunt; at its
// controller's end of turn adds 1 random card of exactly the eight Pancake tokens C+ #12.1–#12.8
// (repeats allowed, R60), never Mother Pancake or Mommy Barker; not at the opponent's end; a full
// hand burns it (R4); the opponent sees a card added under the sentinel (R97); the count reads through
// `param()`; radiant adds 2".

import { HAND_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const MOTHER = "classicplus-012";
const FILLER = "core-005";
const PANCAKE_TOKENS = [1, 2, 3, 4, 5, 6, 7, 8].map((k) => `classicplus-012-${k}`);

function atEndOfTurn(radiant: boolean, opts: { seed?: string; hand?: readonly string[] } = {}): { s: Scenario; added: string[] } {
  const s = scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: opts.hand ?? [FILLER], field: [{ def: MOTHER, radiant }], library: [FILLER, FILLER] },
    p2: { hand: [FILLER], library: [FILLER, FILLER] },
  });
  const before = new Set(s.hand("p1").map((card) => card.id));
  s.endTurn();
  return { s, added: s.hand("p1").filter((card) => !before.has(card.id)).map((card) => card.defId) };
}

describe("C+ #12 The Mother Pancake", () => {
  describe("base", () => {
    it("prints Taunt", () => {
      const s = scenario({ p1: { field: [MOTHER] } });
      expect(s.stats(MOTHER).keywords).toContainEqual({ kind: "Taunt" });
    });

    it("at its controller's end of turn adds one Pancake token", () => {
      const { added } = atEndOfTurn(false);
      expect(added).toHaveLength(1);
      expect(PANCAKE_TOKENS).toContain(added[0]);
    });

    it("R60 the pool is exactly the eight Pancake tokens, repeats allowed, never Mother Pancake or Mommy Barker", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 40; i += 1) {
        const { added } = atEndOfTurn(true, { seed: `mp-${i}` });
        expect(added).toHaveLength(2);
        for (const id of added) seen.add(id);
      }
      expect([...seen].sort()).toEqual([...PANCAKE_TOKENS].sort());
    });

    it("not at the opponent's end of turn", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [MOTHER], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], library: [FILLER, FILLER] },
      });
      s.endTurn(); // p2's end of turn
      expect(s.hand("p1").every((card) => !PANCAKE_TOKENS.includes(card.defId))).toBe(true);
    });

    it("R4 a full hand burns it", () => {
      const { s, added } = atEndOfTurn(false, { hand: Array.from({ length: HAND_CAP }, () => FILLER) });
      expect(added).toHaveLength(0);
      expect(s.events.some((event) => event.type === "burned" && PANCAKE_TOKENS.includes(event.defId))).toBe(true);
    });

    it("R97 the opponent sees a card added under the sentinel", () => {
      const { s } = atEndOfTurn(false);
      const shown = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(shown.length).toBeGreaterThan(0);
      for (const event of shown) if (event.type === "addedToHand") expect(PANCAKE_TOKENS).not.toContain(event.defId);
      expect(JSON.stringify(s.view("p2"))).not.toMatch(/classicplus-012-\d/);
    });

    it("R386 the count reads through param(): an Upgrade adds 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [MOTHER], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], library: [FILLER, FILLER] },
      });
      stepParam(s.card(MOTHER), "tokens", 1);
      const before = s.hand("p1").length;
      s.endTurn();
      expect(s.hand("p1")).toHaveLength(before + 2);
    });
  });

  describe("radiant", () => {
    it("16/16 Taunt, adding 2 Pancake tokens", () => {
      const { s, added } = atEndOfTurn(true);
      s.expectStats(MOTHER, { attack: 16, health: 16 });
      expect(added).toHaveLength(2);
      for (const id of added) expect(PANCAKE_TOKENS).toContain(id);
    });
  });
});
