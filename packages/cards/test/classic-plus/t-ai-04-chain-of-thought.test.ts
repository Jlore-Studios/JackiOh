// T-AI-4 Chain of Thought — SPEC §8.7 row T-AI-4, BUILD M9 Classic+ row T-AI-4: "Draws 1; if the card
// that draw put in your hand costs (1) or less (its current cost; an X-cost card counts 0, R65) it draws
// again, at most `CHAIN_OF_THOUGHT_REPEATS` (4) more times, 5 draws in all; a cast-on-draw card (never in
// hand, R58), a burned card or a fatigue draw ends the chain; radiant the threshold is (2)".

import { CHAIN_OF_THOUGHT_REPEATS } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type PileSetup, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-04-chain-of-thought";

const CHAIN = "classicplus-t-ai-04";
const TIMMY = "core-011"; // (1) Unit
const RAPID = "core-010"; // (0) Spell
const D_FENDER = "core-001"; // (2) Unit
const MENACE = "core-019"; // (3) Unit
const ADAPTIVE_UI = "core-074"; // (X) Spell
const HINDER = "core-021"; // (0) Spell, cast on draw; the base face discards 1 at random with no prompt (R431, R682)
const VANILLA = "core-008"; // (1) Unit, the spare in hand
const PALANTIR = "classic-004"; // (1) Field Spell: "Aura: Your opponent can't draw more than 1 card each turn."

function chain(library: readonly PileSetup[], opts: { radiantFace?: boolean; hand?: readonly PileSetup[] } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: CHAIN, radiant: opts.radiantFace === true }, ...(opts.hand ?? [VANILLA])], library },
    p2: { hand: [VANILLA], library: [VANILLA] },
  });
}

function drawn(s: Scenario): number {
  return s.events.filter((event) => event.type === "drawn" && event.player === "p1").length;
}

describe("T-AI-4 Chain of Thought", () => {
  it("is a (1) AI Spell token; the chain's bound is CHAIN_OF_THOUGHT_REPEATS (4)", () => {
    expect(def.cost).toBe(1);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(CHAIN_OF_THOUGHT_REPEATS).toBe(4);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("draws 1, and again while the card drawn costs (1) or less: 5 draws in all at most", () => {
      const s = chain([TIMMY, RAPID, TIMMY, RAPID, TIMMY, TIMMY, TIMMY]).play(CHAIN);
      expect(drawn(s)).toBe(CHAIN_OF_THOUGHT_REPEATS + 1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("a card that costs more is still drawn, and ends the chain", () => {
      const s = chain([TIMMY, D_FENDER, TIMMY]).play(CHAIN);
      expect(drawn(s)).toBe(2);
      s.expectInZone(D_FENDER, "hand");
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TIMMY]);
    });

    it("a first card that costs more ends it at once", () => {
      const s = chain([MENACE, TIMMY]).play(CHAIN);
      expect(drawn(s)).toBe(1);
    });

    it("R65 an X-cost card counts 0", () => {
      const s = chain([ADAPTIVE_UI, TIMMY, D_FENDER]).play(CHAIN);
      expect(drawn(s)).toBe(3);
    });

    it("R65 its current cost: a (2) card that costs (1) less goes on, a (1) card that costs (1) more stops", () => {
      const cheaper = chain([{ def: D_FENDER, costMod: -1 }, TIMMY, D_FENDER]).play(CHAIN);
      expect(drawn(cheaper)).toBe(3);
      const dearer = chain([{ def: TIMMY, costMod: 1 }, TIMMY]).play(CHAIN);
      expect(drawn(dearer)).toBe(1);
    });

    it("R596 a card cast on draw ends the chain, though its cast's own repeat brings a (1) card", () => {
      const s = chain([{ def: HINDER, radiant: true }, TIMMY, TIMMY, TIMMY]).play(CHAIN);
      s.expectInZone(HINDER, "graveyard");
      // The cast's repeat drew the first Timmy; the chain itself drew nothing more.
      expect(s.hand("p1").filter((card) => card.defId === TIMMY)).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("§2.4 R4 a burned card ends the chain", () => {
      const s = chain([TIMMY, TIMMY, TIMMY], { hand: Array.from({ length: 9 }, () => VANILLA) }).play(CHAIN);
      // 9 left in hand after the play: the first Timmy fits, the second burns, and the chain ends.
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(1);
    });

    it("§2.4 a fatigue draw ends the chain after one hit", () => {
      const s = chain([]).play(CHAIN);
      expect(s.events.filter((event) => event.type === "fatigue")).toHaveLength(1);
      s.expectHealth("p1", 29);
    });

    it("R457 a draw the draw limit stops ends the chain: it is tried once and not again", () => {
      const s = scenario({
        p1: { hand: [CHAIN, VANILLA], library: [TIMMY, TIMMY, TIMMY] },
        p2: { hand: [VANILLA], backrow: [{ def: PALANTIR, faceUp: true }], library: [VANILLA] },
      }).play(CHAIN);
      expect(drawn(s)).toBe(1);
      expect(s.events.filter((event) => event.type === "drawLimited")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("R97 the opponent sees the draws under the sentinel", () => {
      const s = chain([TIMMY, D_FENDER]).play(CHAIN);
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(s.card(D_FENDER).id);
      expect(s.view("p2").opponent.hand).toEqual({ count: 3 });
    });

    it("R682, R431 Hinder's random discard asks nothing mid-draw: no prompt opens and the chain has ended", () => {
      const s = chain([TIMMY, HINDER, TIMMY, TIMMY]).play(CHAIN);
      // R682: "Discard 1" names no "of your choice", so the discard is random and no
      // hand prompt pauses the draw; the cast-on-draw card still ends the chain (R58, R596).
      expect(s.state.pending).toBeNull();

      // Timmy, then Hinder cast (discarding one card at random), whose repeat drew the
      // second Timmy; the third stays. The chain itself drew nothing more.
      expect(drawn(s)).toBe(3);
      s.expectInZone(HINDER, "graveyard");
      s.expectInZone(CHAIN, "graveyard");
      expect(s.pile("p1", "graveyard")).toHaveLength(3);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TIMMY]);
      expect(s.hand("p1")).toHaveLength(2);
      expect(s.hand("p1").map((card) => card.defId)).toContain(TIMMY);
    });
  });

  describe("radiant", () => {
    it("the threshold is (2): a (2) card goes on, a (3) card stops", () => {
      const s = chain([D_FENDER, TIMMY, MENACE, TIMMY], { radiantFace: true }).play(CHAIN);
      expect(drawn(s)).toBe(3);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TIMMY]);
    });

    it("still 5 draws at most", () => {
      const s = chain([D_FENDER, D_FENDER, D_FENDER, D_FENDER, D_FENDER, D_FENDER], { radiantFace: true }).play(CHAIN);
      expect(drawn(s)).toBe(CHAIN_OF_THOUGHT_REPEATS + 1);
    });

    it("R596 a card cast on draw still ends it", () => {
      const s = chain([{ def: HINDER, radiant: true }, D_FENDER, D_FENDER], { radiantFace: true }).play(CHAIN);
      expect(s.pile("p1", "library")).toHaveLength(1);
    });
  });
});
