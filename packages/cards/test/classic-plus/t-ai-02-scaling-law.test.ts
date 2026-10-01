// T-AI-2 Scaling Law — SPEC §8.7 row T-AI-2, BUILD M9 row T-AI-2: a 2/2 with +1/+1 for each AI generated
// card its controller has played this game — a layer-2 set-stat over the per-game count of AI-tagged
// plays (casts included, R70; the opponent's don't count; the count outlives the cards) — counting
// itself once played, so 3/3 as the game's first; an AI Slop fusion counts once; radiant 4/4 with
// +2/+2 for each.

import { describe, expect, it } from "vitest";
import {
  applyEffects,
  createRng,
  makeContext,
  playedThisGameWithTag,
  settle,
  subsystems,
  type EngineSink,
} from "@jackioh/engine";
import { castNew } from "@jackioh/engine/effects";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-02-scaling-law";
import { scenario, type Scenario } from "../_harness";

const SCALING = "classicplus-t-ai-02";
const HALLUCINATION = "classicplus-t-ai-03"; // (0) Spell, AI.
const ASSISTANT = "classicplus-t-ai-01"; // (1) Unit, AI.
const FILLER = "core-005";

function sinkOf(s: Scenario): EngineSink {
  return { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
}

describe("T-AI-2 Scaling Law", () => {
  it("is a (2) 2/2 Unit, AI, Token (Radiant 4/4) with one set-stat hook on both faces", () => {
    expect(def).toMatchObject({ cost: 2, type: "Unit", tags: ["AI", "Token"], token: true });
    expect(def.base).toMatchObject({ attack: 2, health: 2 });
    expect(def.radiant).toMatchObject({ attack: 4, health: 4 });
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§10.4 it counts itself once played: 3/3 as the game's first AI card", () => {
      const s = scenario({ p1: { hand: [SCALING, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SCALING, { zone: 1 });
      expect(playedThisGameWithTag(s.state, "p1", "AI")).toBe(1);
      s.expectStats(s.unit("p1", 1)!, { attack: 3, health: 3, maxHealth: 3 });
    });

    it("+1/+1 for each AI generated card played after it, and the count outlives the cards", () => {
      const s = scenario({ p1: { hand: [SCALING, HALLUCINATION, HALLUCINATION, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SCALING, { zone: 1 });
      const unit = s.unit("p1", 1)!;
      s.play(HALLUCINATION);
      s.expectStats(unit, { attack: 4, health: 4 });
      s.play(HALLUCINATION);
      s.expectStats(unit, { attack: 5, health: 5 });
      expect(s.pile("p1", "graveyard").filter((card) => card.defId === HALLUCINATION)).toHaveLength(2);
    });

    it("counts the AI cards played before it arrived", () => {
      const s = scenario({ p1: { hand: [HALLUCINATION, ASSISTANT, SCALING, FILLER], mana: 10 }, p2: { hand: [FILLER] } });
      s.play(HALLUCINATION);
      s.play(ASSISTANT, { zone: 2 });
      s.play(SCALING, { zone: 1 });
      s.expectStats(s.unit("p1", 1)!, { attack: 5, health: 5 });
    });

    it("R70 a cast AI card counts", () => {
      const s = scenario({ p1: { hand: [SCALING, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SCALING, { zone: 1 });
      const sink = sinkOf(s);
      applyEffects([castNew({ def: HALLUCINATION })], makeContext(sink, null, { controller: "p1" }));
      settle(sink);
      s.state.rngCursor = sink.rng.cursor;
      s.expectStats(s.unit("p1", 1)!, { attack: 4, health: 4 });
    });

    it("the opponent's AI cards don't count", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [SCALING] },
        p2: { hand: [HALLUCINATION, HALLUCINATION, FILLER] },
      });
      const unit = s.unit("p1", 1)!;
      s.expectStats(unit, { attack: 2, health: 2 });
      s.play(HALLUCINATION);
      s.expectStats(unit, { attack: 2, health: 2 });
      expect(playedThisGameWithTag(s.state, "p2", "AI")).toBe(1);
    });

    it("an AI Slop fusion of two AI cards counts once (§10.5 step 4 counts each tag of a play once)", () => {
      const s = scenario({ p1: { hand: [SCALING, HALLUCINATION, HALLUCINATION, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SCALING, { zone: 1 });
      const [into, other] = s.hand("p1").filter((card) => card.defId === HALLUCINATION);
      const sink = sinkOf(s);
      const fused = subsystems.fuse(sink, { ingredients: [other!], into: into!, handPrice: "fused" });
      s.state.rngCursor = sink.rng.cursor;
      if (fused === null) throw new Error("the fusion");
      s.play(fused.id);
      expect(playedThisGameWithTag(s.state, "p1", "AI")).toBe(2);
      s.expectStats(s.unit("p1", 1)!, { attack: 4, health: 4 });
    });
  });

  describe("radiant", () => {
    it("a 4/4 with +2/+2 for each: 6/6 as the game's first, 8/8 after one more", () => {
      const s = scenario({ p1: { hand: [{ def: SCALING, radiant: true }, HALLUCINATION, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SCALING, { zone: 1 });
      const unit = s.unit("p1", 1)!;
      s.expectStats(unit, { attack: 6, health: 6 });
      s.play(HALLUCINATION);
      s.expectStats(unit, { attack: 8, health: 8 });
    });
  });
});
