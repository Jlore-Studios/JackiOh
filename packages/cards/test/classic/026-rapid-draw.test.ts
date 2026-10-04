// C #26 Rapid Draw — SPEC §8.6 row 26, BUILD M9 Classic row C 26: "Draw 4 (§2.4: burns, fatigue, a
// draw limit), then discard 4 cards at random (R641; 4 or fewer in hand → all of them), with no prompt;
// radiant: draw 5, discard 4; its tuned numbers (draw, discard) read through `param()` (R386)".
//
// The draw-limit case puts C #4 Palantir in the opponent's backrow ("Aura: Your opponent can't draw
// more than 1 card each turn", B5 E3, R457).

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/026-rapid-draw";

const RAPID = "classic-026";
const PALANTIR = "classic-004"; // Field Spell; Aura: Your opponent can't draw more than 1 card each turn.
const FILLER = "core-005"; // Stockpile
const VIRUS = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
// Library cards nobody else holds, so a view can be searched for their ids.
const A = "core-019";
const B = "core-011";
const C = "core-001";
const D = "core-020";
const E = "core-012";
const F = "core-045";

function handDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.hand(player).map((card) => card.defId);
}

function graveDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.pile(player, "graveyard").map((card) => card.defId);
}

function discardedDefs(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "discarded" ? [event.defId] : []));
}

describe("C #26 Rapid Draw", () => {
  it("runs one script on both faces, with no resume step (R641: random, no prompt)", () => {
    expect(def.id).toBe(RAPID);
    expect(radiant).toBe(base);
    expect(base.resume).toBeUndefined();
  });

  describe("base", () => {
    it("draws 4, then R641 discards 4 at random with no prompt", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER, FILLER], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(2);
      const discarded = discardedDefs(s);
      expect(discarded).toHaveLength(4);
      for (const defId of discarded) expect([FILLER, A, B, C, D]).toContain(defId);
      expect(graveDefs(s)).toEqual(expect.arrayContaining([...discarded, RAPID]));
    });

    it("R641 the random discards come from the match rng: the same game discards the same cards", () => {
      const opts = { p1: { hand: [RAPID, FILLER, FILLER], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } };
      const first = scenario(opts);
      first.play(RAPID);
      const second = scenario(opts);
      second.play(RAPID);
      expect(discardedDefs(first)).toEqual(discardedDefs(second));
    });

    it("R641 with 4 or fewer cards in hand it discards all of them and asks nothing", () => {
      const s = scenario({ p1: { hand: [RAPID], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      expect(handDefs(s)).toEqual([]);
      expect(discardedDefs(s)).toEqual([A, B, C, D]);
    });

    it("R58 each draw has its own cast-on-draw chain, and the discard then reads the hand the draws left", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER], library: [VIRUS, A, B, C, D, E] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      // The CN-Virus cast itself in the first draw's chain, which then drew A.
      s.expectHealth("p1", 29);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([E]);
      // Four drawn into a five-card hand, then four random discards: one card left, no prompt.
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(1);
      const discarded = discardedDefs(s);
      expect(discarded).toHaveLength(4);
      for (const defId of discarded) expect([FILLER, A, B, C, D]).toContain(defId);
    });

    it("§2.4 fatigue: a deck of 2 draws 2 and fatigues twice, and the 2 cards in hand are discarded", () => {
      const s = scenario({ p1: { hand: [RAPID], library: [A, B] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      s.expectHealth("p1", 27);
      expect(discardedDefs(s)).toEqual([A, B]);
      expect(s.state.pending).toBeNull();
    });

    it("R4 R317 a full hand burns the overflow, then the discard takes 4 at random", () => {
      const s = scenario({
        p1: { hand: [RAPID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], library: [A, B, C, D] },
        p2: { hand: [FILLER] },
      });

      s.play(RAPID);

      // Three burned on the draw, four discarded at random: six cards left, no prompt.
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(6);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(3);
      expect(graveDefs(s)).toEqual(expect.arrayContaining([B, C, D]));
      expect(discardedDefs(s)).toHaveLength(4);
    });

    it("§2.4 B5 E3 a draw limit stops the rest: under the opponent's Palantir it draws 1, and the discard takes it", () => {
      const s = scenario({ p1: { hand: [RAPID], library: [A, B, C, D] }, p2: { hand: [FILLER], backrow: [{ def: PALANTIR, faceUp: true }] } });

      s.play(RAPID);

      expect(s.events.filter((event) => event.type === "drawLimited")).toHaveLength(3);
      expect(discardedDefs(s)).toEqual([A]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([B, C, D]);
    });

    it("R97 the card left in hand stays hidden from the opponent, while the discards are public", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER], library: [A, B, C, D] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      // Four drawn, four discarded at random: the one card left is hidden, the discards are not.
      const [left] = s.hand("p1");
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(left?.id ?? "no-card-in-hand");
      for (const card of s.pile("p1", "graveyard")) expect(theirs).toContain(card.id);
      const mine = JSON.stringify(s.view("p1"));
      for (const id of [A, B, C, D, FILLER]) expect(mine).toContain(id);
    });

    it("R386 an Upgrade draws 5; a Degrade of discard takes 5, an Upgrade of it 3", () => {
      const drawUp = scenario({ p1: { hand: [RAPID], library: [A, B, C, D, E, F] }, p2: { hand: [FILLER] } });
      stepParam(drawUp.card(RAPID), "draw", 1);
      drawUp.play(RAPID);
      expect(drawUp.state.pending).toBeNull();
      expect(drawUp.hand("p1")).toHaveLength(1);
      expect(discardedDefs(drawUp)).toHaveLength(4);

      const discardUp = scenario({ p1: { hand: [RAPID], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(discardUp.card(RAPID), "discard", -1);
      discardUp.play(RAPID);
      expect(discardedDefs(discardUp)).toHaveLength(3);

      const discardDown = scenario({ p1: { hand: [RAPID, FILLER, FILLER], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(discardDown.card(RAPID), "discard", 1);
      discardDown.play(RAPID);
      expect(discardedDefs(discardDown)).toHaveLength(5);
    });
  });

  describe("radiant", () => {
    it("draws 5, then discards 4 at random", () => {
      const s = scenario({ p1: { hand: [{ def: RAPID, radiant: true }], library: [A, B, C, D, E, F] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(1);
      expect(discardedDefs(s)).toHaveLength(4);
    });

    it("R641 a deck of 3 leaves 3 in hand, all discarded without a prompt", () => {
      const s = scenario({ p1: { hand: [{ def: RAPID, radiant: true }], library: [A, B, C] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      expect(discardedDefs(s)).toEqual([A, B, C]);
    });
  });
});
