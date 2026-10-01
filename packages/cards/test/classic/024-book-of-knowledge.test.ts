// C #24 Book of Knowledge — SPEC §8.6 row 24, BUILD M9 Classic row C 24: "Draw 3: three draws, each
// with its own cast-on-draw chain (R58), fatigue from an empty deck, a full hand burning (R317), a
// draw limit stopping the rest (§2.4); the drawn cards are never named in the opponent's view;
// radiant: draw 6; its tuned number (draw) reads through `param()` (R386)".
//
// The draw-limit case puts C #4 Palantir in the opponent's backrow ("Aura: Your opponent can't draw
// more than 1 card each turn", B5 E3, R457).

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/024-book-of-knowledge";

const BOOK = "classic-024";
const PALANTIR = "classic-004"; // Field Spell; Aura: Your opponent can't draw more than 1 card each turn.
const PALANTIR_PASS = "mode:pass"; // its "you may Tribute this to steal it", declined
const VIRUS = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
const FILLER = "core-005";
// Library cards nobody else holds, so the opponent's view can be searched for their ids.
const A = "core-019";
const B = "core-011";
const C = "core-001";
const D = "core-020";
const E = "core-012";
const F = "core-045";
const G = "core-032";

function handDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.hand(player).map((card) => card.defId);
}

function count(s: Scenario, type: string): number {
  return s.events.filter((event) => event.type === type).length;
}

describe("C #24 Book of Knowledge", () => {
  it("runs one script on both faces", () => {
    expect(def.id).toBe(BOOK);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("draws 3, from the top, one draw each", () => {
      const s = scenario({ p1: { hand: [BOOK], library: [A, B, C, D] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A, B, C]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([D]);
      expect(count(s, "drawn")).toBe(3);
    });

    it("R58 each draw has its own cast-on-draw chain: a CN-Virus casts and its draw repeats", () => {
      const s = scenario({ p1: { hand: [BOOK], library: [VIRUS, A, B, C, D] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A, B, C]);
      s.expectHealth("p1", 29);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([D]);
    });

    it("§2.4 an empty deck fatigues: one card, then fatigue 1 and 2", () => {
      const s = scenario({ p1: { hand: [BOOK], library: [A] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A]);
      expect(count(s, "fatigue")).toBe(2);
      s.expectHealth("p1", 27);
    });

    it("R4 R317 a full hand burns the overflow into the graveyard, which both players read", () => {
      const s = scenario({
        p1: { hand: [BOOK, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], library: [A, B, C] },
        p2: { hand: [FILLER] },
      });

      s.play(BOOK);

      expect(s.hand("p1")).toHaveLength(10);
      expect(handDefs(s).at(-1)).toBe(A);
      expect(count(s, "burned")).toBe(2);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual(expect.arrayContaining([B, C]));
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).toContain(B);
      expect(theirs).toContain(C);
      expect(theirs).not.toContain(A);
    });

    it("§2.4 B5 E3 a draw limit stops the rest: under the opponent's Palantir only the first draw happens", () => {
      const s = scenario({ p1: { hand: [BOOK], library: [A, B, C] }, p2: { hand: [FILLER], backrow: [{ def: PALANTIR, faceUp: true }] } });

      // Palantir's other line answers a Book: its controller may Tribute it to steal the play. Declined.
      s.play(BOOK);
      expect(s.state.pending?.playerId).toBe("p2");
      s.answer(PALANTIR_PASS);

      expect(handDefs(s)).toEqual([A]);
      expect(count(s, "drawLimited")).toBe(2);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([B, C]);
    });

    it("R97 the drawn cards are never named in the opponent's view", () => {
      const s = scenario({ p1: { hand: [BOOK], library: [A, B, C] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      const theirs = JSON.stringify(s.view("p2"));
      for (const id of [A, B, C]) expect(theirs).not.toContain(id);
      expect(s.view("p2").opponent.hand).toEqual({ count: 3 });
      const mine = JSON.stringify(s.view("p1"));
      for (const id of [A, B, C]) expect(mine).toContain(id);
    });

    it("R386 an Upgrade makes it draw 4, a Degrade 2", () => {
      const up = scenario({ p1: { hand: [BOOK], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(up.card(BOOK), "draw", 1);
      up.play(BOOK);
      expect(handDefs(up)).toEqual([A, B, C, D]);

      const down = scenario({ p1: { hand: [BOOK], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(down.card(BOOK), "draw", -1);
      down.play(BOOK);
      expect(handDefs(down)).toEqual([A, B]);
    });
  });

  describe("radiant", () => {
    it("draws 6", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }], library: [A, B, C, D, E, F, G] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A, B, C, D, E, F]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([G]);
    });

    it("§2.4 six draws from a deck of four: four cards, then fatigue 1 and 2", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }], library: [A, B, C, D] }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A, B, C, D]);
      s.expectHealth("p1", 27);
    });

    it("R386 an Upgrade steps the Radiant 6 to 7", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }], library: [A, B, C, D, E, F, G] }, p2: { hand: [FILLER] } });
      stepParam(s.card(BOOK), "draw", 1);

      s.play(BOOK);

      expect(handDefs(s)).toEqual([A, B, C, D, E, F, G]);
    });
  });
});
