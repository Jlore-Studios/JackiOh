// R437: the brand flourish a mark plays as it lands on a card (brand.ts), in the colours of the one
// palette table the card's aura reads (cards/marks.ts).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { DEFAULT_MARK_COLOR, MARK_PALETTES } from "../cards/marks.ts";
import { ANIMATIONS, planEntries } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { brandCues } from "./brand.ts";
import { FX_BRAND_SLAM_AT, FX_BRAND_TAIL_MS, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxAnchor, FxBrandCue, FxCue } from "./types.ts";

/** fullBoardView's enemy lane-1 unit is u6. */
const TARGET = "u6";
const AT: FxAnchor = { kind: "testid", testid: testid.card(TARGET) };

const marked = (added: boolean, color = "purple", instanceId = TARGET): GameEvent => ({ type: "marked", instanceId, mark: "steal", color, added });

function plan(event: GameEvent, D = ANIMATIONS.marked.durationMs): FxCue[] {
  const view = { ...fullBoardView(), events: [event] };
  const [entry] = planEntries([event], view, false);
  if (entry === undefined) throw new Error("no entry");
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  env.memory.remember(entry.events);
  return planFx({ ...entry, durationMs: D }, view, env);
}

describe("R437 the brand", () => {
  it("R437 a mark landing brands its card with a sigil in the palette's colours, a ring and a burst at the slam", () => {
    const D = 400;
    const cues = plan(marked(true), D);
    const sigil = cues.find((cue): cue is FxBrandCue => cue.kind === "brand");
    const palette = MARK_PALETTES.purple;
    expect(sigil).toEqual({ kind: "brand", at: AT, tint: { rim: palette.rim, core: palette.core, glow: palette.glow }, delayMs: 0, durationMs: D + FX_BRAND_TAIL_MS });
    const slam = Math.round(FX_BRAND_SLAM_AT * D);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", preset: palette.preset, at: AT, delayMs: slam }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: palette.preset, at: AT, delayMs: slam }));
  });

  it("R437 another colour key brands in its own colours; an unknown key falls back to the default", () => {
    const green = plan(marked(true, "green")).find((cue): cue is FxBrandCue => cue.kind === "brand");
    expect(green?.tint.rim).toBe(MARK_PALETTES.green.rim);
    const unknown = plan(marked(true, "ultraviolet")).find((cue): cue is FxBrandCue => cue.kind === "brand");
    expect(unknown?.tint.rim).toBe(MARK_PALETTES[DEFAULT_MARK_COLOR].rim);
  });

  it("R437 a mark leaving puffs away in its colour, with no sigil", () => {
    const cues = plan(marked(false, "crimson"));
    expect(cues.some((cue) => cue.kind === "brand")).toBe(false);
    expect(cues).toEqual([expect.objectContaining({ kind: "burst", preset: MARK_PALETTES.crimson.preset, at: AT })]);
  });

  it("R437 R202 a mark on a card the viewer cannot read, or the board does not render, plans nothing", () => {
    expect(plan(marked(true, "purple", "hidden"))).toEqual([]);
    expect(plan(marked(true, "purple", "gone"))).toEqual([]);
  });

  it("R200 the brand starts inside its entry and is gone within FX_MAX_TAIL_MS of its end", () => {
    for (const D of [120, 400, 1600]) {
      for (const cue of brandCues(marked(true), AT, D, 1)) {
        expect(cue.delayMs).toBeLessThanOrEqual(D);
        const end = cue.kind === "burst" ? cue.delayMs + FX_MAX_PARTICLE_LIFE_MS : cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
        expect(end).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
      }
    }
    expect(brandCues({ type: "turnStarted", player: "p1", turn: 1 }, AT, 400, 1)).toEqual([]);
  });

  it("R437 the ANIMATIONS row names the brand recipe", () => {
    expect(ANIMATIONS.marked.fx).toEqual({ recipe: "brand" });
  });
});
