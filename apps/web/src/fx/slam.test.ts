// #185 Unit Slam in the effects layer: the board under a landing Unit, by its tier (R701).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import type { AnimationEntry, EntrySlam } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { UNIT_SLAM, type SlamTier } from "../game/damageFeel.ts";
import { baseView } from "../test/fixtures.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "./constants.ts";
import { planFx, slamLandMs, traumaForShakePx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCue, FxPlanEnv } from "./types.ts";

const D = 250;
const EVENT: GameEvent = { type: "summoned", player: "p2", instanceId: "slammer", defId: "core-004", row: "units", lane: 3 };
const ZONE = testid.zone("opponent", "units", 3);

function slamOf(tier: SlamTier): EntrySlam {
  const feel = UNIT_SLAM[tier];
  return { instanceId: "slammer", side: "opponent", zone: ZONE, tier, hitStopMs: feel.hitStopMs, shakePx: feel.shakePx, shakeMs: feel.shakeMs, anticipationMs: feel.anticipationMs };
}

function cuesFor(slam: EntrySlam | undefined): { cues: FxCue[]; entry: AnimationEntry } {
  const view = baseView();
  const wait = slam?.anticipationMs ?? 0;
  const entry: AnimationEntry = { events: [EVENT], type: "summoned", durationMs: D + wait, frames: new Map(), view, ...(slam === undefined ? {} : { slam }) };
  const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => ({ rarity: "Common", attack: 3, health: 4, type: "Unit" }), memory: createFxMemory() };
  return { cues: planFx(entry, view, env), entry };
}

const dust = (cues: readonly FxCue[]): number =>
  cues.reduce((total, cue) => total + (cue.kind === "burst" && cue.preset === "dust" ? cue.count : 0), 0);

describe("R701 the board under a landing Unit, by its tier", () => {
  it("R701 a Tiny Unit throws up nothing, and each tier up throws up more dust", () => {
    const tiers: SlamTier[] = ["tiny", "small", "medium", "large", "huge"];
    const amounts = tiers.map((tier) => dust(cuesFor(slamOf(tier)).cues));
    expect(amounts[0]).toBe(0);
    for (let i = 1; i < amounts.length; i += 1) expect(amounts[i], tiers[i]).toBeGreaterThan(amounts[i - 1] ?? 0);
  });

  it("R701 only Huge and MASSIVE shake the board, by UNIT_SLAM's shakePx, on the landing beat after the anticipation", () => {
    for (const tier of ["tiny", "small", "medium", "large"] as const) {
      expect(cuesFor(slamOf(tier)).cues.some((cue) => cue.kind === "shake"), tier).toBe(false);
    }
    for (const tier of ["huge", "massive"] as const) {
      const { cues, entry } = cuesFor(slamOf(tier));
      const shake = cues.find((cue) => cue.kind === "shake");
      expect(shake?.kind === "shake" ? shake.trauma : 0, tier).toBeCloseTo(traumaForShakePx(UNIT_SLAM[tier].shakePx) * FX_INTENSITY_SCALE.normal);
      const land = UNIT_SLAM[tier].anticipationMs + Math.round(FX_SLAM_AT * D);
      expect(slamLandMs(entry, entry.durationMs)).toBe(land);
      expect(shake?.delayMs).toBe(land);
    }
  });

  it("R701 a MASSIVE Unit sends a shockwave across the board; nothing smaller does", () => {
    const rings = (tier: SlamTier) => cuesFor(slamOf(tier)).cues.filter((cue) => cue.kind === "ring" && cue.at.kind !== "testid").length;
    expect(rings("massive")).toBe(1);
    expect(rings("huge")).toBe(0);
  });

  it("R701 a shake the action's cap took away is not played", () => {
    const capped = { ...slamOf("massive"), shakePx: 0 };
    expect(cuesFor(capped).cues.some((cue) => cue.kind === "shake")).toBe(false);
  });

  it("R701 without a slam (no resolver), a summon keeps its plain dust ring", () => {
    expect(dust(cuesFor(undefined).cues)).toBeGreaterThan(0);
  });
});
