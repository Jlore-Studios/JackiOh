// R1584000: the stat showcase's one-card stage plays the board's own entries and effects.

import { describe, expect, it } from "vitest";

import { createFxMemory } from "../fx/memory.ts";
import { animationRow } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_INTERVAL_MS, SHOWCASE_LEAD_MS } from "./config.ts";
import {
  STAGE_ANIMATIONS,
  STAGE_INSTANCE_ID,
  STAGE_LANE,
  stageCues,
  stageScript,
  stageView,
  type StageAnimation,
} from "./showcaseStage.ts";

const CARD = "core-004";
const SIDES: readonly Side[] = ["you", "opponent"];

function kinds(animation: StageAnimation, side: Side, beat: number, intensity = 1): string[] {
  const script = stageScript(animation, CARD, side);
  return stageCues(script.beats[beat]!, intensity, createFxMemory()).map((cue) => cue.kind);
}

describe("R1584000 the stage's view", () => {
  it("R1584000 holds the card as a unit on its side's one lane and nothing else", () => {
    for (const side of SIDES) {
      const view = stageView(CARD, side);
      const other = side === "you" ? "opponent" : "you";
      const unit = view[side].units[STAGE_LANE];
      expect(unit?.instanceId).toBe(STAGE_INSTANCE_ID);
      expect(unit?.defId).toBe(CARD);
      expect(view[side].units).toHaveLength(1);
      expect(view[other].units).toEqual([null]);
      expect(view.pending).toBeNull();
      expect(view.result).toBeNull();
    }
  });
});

describe("R1584000 the stage's beats", () => {
  it("R1584000 each animation's beats are the board's own entries on the stage's card or zone", () => {
    for (const animation of STAGE_ANIMATIONS) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        expect(script.beats.length).toBeGreaterThan(0);
        for (const beat of script.beats) {
          expect(beat.entry.durationMs).toBe(animationRow(beat.entry.type)?.durationMs);
          const keys = [...beat.entry.frames.keys()];
          const card = testid.card(STAGE_INSTANCE_ID);
          const zone = testid.zone(beat.side, "units", STAGE_LANE);
          expect(keys.length === 1 && (keys[0] === card || keys[0] === zone), `${animation} on ${side}: ${keys.join()}`).toBe(true);
        }
        expect(script.side).toBe(side);
        expect(script.returns).toBe(animation === "death");
      }
    }
  });

  it("R1584000 the Allure stat marks the card, then takes it to the other side", () => {
    for (const side of SIDES) {
      const script = stageScript("allure", CARD, side);
      const other = side === "you" ? "opponent" : "you";
      expect(script.beats.map((beat) => beat.entry.type)).toEqual(["marked", "controlChanged"]);
      expect(script.beats.map((beat) => beat.side)).toEqual([side, other]);
    }
  });

  it("R1584000 the stage plays the board's own effects for each beat", () => {
    expect(kinds("death", "opponent", 0)).toContain("crack");
    expect(kinds("allure", "you", 0)).toContain("brand");
    expect(kinds("trap", "you", 0)).toContain("ring");
    expect(kinds("radiant", "you", 0)).toContain("sheen");
    expect(kinds("shield", "you", 0)).toContain("ring");
    for (const animation of STAGE_ANIMATIONS) expect(kinds(animation, "you", 0, 0)).toEqual([]);
  });

  it("R1584000 the way in, every beat and the way back fit in one interval", () => {
    for (const animation of STAGE_ANIMATIONS) {
      for (const side of SIDES) {
        const script = stageScript(animation, CARD, side);
        const beats = script.beats.reduce((sum, beat) => sum + beat.entry.durationMs + SHOWCASE_BEAT_GAP_MS, 0);
        const total = SHOWCASE_LEAD_MS + beats + (script.returns ? SHOWCASE_LEAD_MS : 0);
        expect(total, `${animation} takes ${String(total)} ms`).toBeLessThan(SHOWCASE_INTERVAL_MS);
      }
    }
  });
});
