// C #26 Rapid Draw — SPEC §8.6 row 26, BUILD M9 Classic row C 26: "Draw 4 (§2.4: burns, fatigue, a
// draw limit), then discard 4 cards of your choice (R16; 4 or fewer in hand → all of them), a prompt
// over your own hand whose options the opponent's view never names; radiant: draw 5, discard 4; its
// tuned numbers (draw, discard) read through `param()` (R386)".
//
// The draw-limit case puts C #4 Palantir in the opponent's backrow ("Aura: Your opponent can't draw
// more than 1 card each turn", B5 E3, R457).

import { reduce, stepParam, type GameState } from "@jackioh/engine";
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

/** The hand card ids of the given defs, in the order named. */
function idsOf(s: Scenario, defs: readonly string[]): string[] {
  const hand = s.hand("p1");
  return defs.map((defId) => {
    const card = hand.find((c) => c.defId === defId);
    if (card === undefined) throw new Error(`${defId} is not in p1's hand`);
    return card.id;
  });
}

describe("C #26 Rapid Draw", () => {
  it("runs one script on both faces, and its discard answers into the resume step", () => {
    expect(def.id).toBe(RAPID);
    expect(radiant).toBe(base);
    expect(base.resume?.discard).toBeTypeOf("function");
  });

  describe("base", () => {
    it("draws 4, then R16 asks for exactly 4 cards of your own hand and discards those", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER, FILLER], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(handDefs(s)).toEqual([FILLER, FILLER, A, B, C, D]);
      const pending = s.state.pending;
      expect(pending?.kind).toBe("hand");
      expect(pending?.playerId).toBe("p1");
      expect(pending?.min).toBe(4);
      expect(pending?.max).toBe(4);
      expect(pending?.options).toHaveLength(6);

      s.answer(idsOf(s, [A, C, FILLER, D]));

      expect(handDefs(s)).toEqual([FILLER, B]);
      expect([...discardedDefs(s)].sort()).toEqual([A, C, FILLER, D].sort());
      expect(graveDefs(s)).toEqual(expect.arrayContaining([A, C, FILLER, D, RAPID]));
      expect(s.state.pending).toBeNull();
    });

    it("R16 with 4 or fewer cards in hand it discards all of them and asks nothing", () => {
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
      expect(handDefs(s)).toEqual([FILLER, A, B, C, D]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([E]);
      expect(s.state.pending?.options).toHaveLength(5);
      expect(s.state.pending?.min).toBe(4);
    });

    it("§2.4 fatigue: a deck of 2 draws 2 and fatigues twice, and the 2 cards in hand are discarded", () => {
      const s = scenario({ p1: { hand: [RAPID], library: [A, B] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      s.expectHealth("p1", 27);
      expect(discardedDefs(s)).toEqual([A, B]);
      expect(s.state.pending).toBeNull();
    });

    it("R4 R317 a full hand burns the overflow, then the discard asks for 4 of the 10", () => {
      const s = scenario({
        p1: { hand: [RAPID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], library: [A, B, C, D] },
        p2: { hand: [FILLER] },
      });

      s.play(RAPID);

      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(3);
      expect(graveDefs(s)).toEqual(expect.arrayContaining([B, C, D]));
      expect(s.state.pending?.min).toBe(4);
      s.answer(s.hand("p1").slice(0, 4).map((card) => card.id));
      expect(s.hand("p1")).toHaveLength(6);
    });

    it("§2.4 B5 E3 a draw limit stops the rest: under the opponent's Palantir it draws 1, and the discard takes it", () => {
      const s = scenario({ p1: { hand: [RAPID], library: [A, B, C, D] }, p2: { hand: [FILLER], backrow: [{ def: PALANTIR, faceUp: true }] } });

      s.play(RAPID);

      expect(s.events.filter((event) => event.type === "drawLimited")).toHaveLength(3);
      expect(discardedDefs(s)).toEqual([A]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([B, C, D]);
    });

    it("R97 R177 the prompt is over your own hand, and the opponent's view never names its options", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER], library: [A, B, C, D] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending?.options.map((option) => option.selection)).toEqual(
        s.hand("p1").map((card) => ({ pick: "instance", instanceId: card.id })),
      );
      const theirs = JSON.stringify(s.view("p2"));
      for (const id of [A, B, C, D]) expect(theirs).not.toContain(id);
      const mine = JSON.stringify(s.view("p1"));
      for (const id of [A, B, C, D]) expect(mine).toContain(id);
    });

    it("R113 the paused discard survives a JSON round trip and resumes through reduce", () => {
      const s = scenario({ p1: { hand: [RAPID, FILLER], library: [A, B, C, D] }, p2: { hand: [FILLER] } });
      s.play(RAPID);
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);

      const choiceId = revived.pending?.id ?? "";
      const picks = idsOf(s, [A, B, C, D]).map((instanceId) => ({ pick: "instance" as const, instanceId }));
      const result = reduce(revived, { type: "answer", playerId: "p1", choiceId, selection: picks, nonce: "rt-026" });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(result.events.filter((event) => event.type === "discarded").map((event) => (event.type === "discarded" ? event.defId : ""))).toEqual([A, B, C, D]);
    });

    it("R386 an Upgrade draws 5; a Degrade of discard asks for 5, an Upgrade of it for 3", () => {
      const drawUp = scenario({ p1: { hand: [RAPID], library: [A, B, C, D, E, F] }, p2: { hand: [FILLER] } });
      stepParam(drawUp.card(RAPID), "draw", 1);
      drawUp.play(RAPID);
      expect(handDefs(drawUp)).toEqual([A, B, C, D, E]);
      expect(drawUp.state.pending?.min).toBe(4);

      const discardUp = scenario({ p1: { hand: [RAPID], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(discardUp.card(RAPID), "discard", -1);
      discardUp.play(RAPID);
      expect(discardUp.state.pending?.min).toBe(3);

      const discardDown = scenario({ p1: { hand: [RAPID, FILLER, FILLER], library: [A, B, C, D, E] }, p2: { hand: [FILLER] } });
      stepParam(discardDown.card(RAPID), "discard", 1);
      discardDown.play(RAPID);
      expect(discardDown.state.pending?.min).toBe(5);
    });
  });

  describe("radiant", () => {
    it("draws 5, then asks for 4", () => {
      const s = scenario({ p1: { hand: [{ def: RAPID, radiant: true }], library: [A, B, C, D, E, F] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(handDefs(s)).toEqual([A, B, C, D, E]);
      expect(s.state.pending?.min).toBe(4);
      s.answer(idsOf(s, [A, B, C, D]));
      expect(handDefs(s)).toEqual([E]);
    });

    it("R16 a deck of 3 leaves 3 in hand, all discarded without a prompt", () => {
      const s = scenario({ p1: { hand: [{ def: RAPID, radiant: true }], library: [A, B, C] }, p2: { hand: [FILLER] } });

      s.play(RAPID);

      expect(s.state.pending).toBeNull();
      expect(discardedDefs(s)).toEqual([A, B, C]);
    });
  });
});
