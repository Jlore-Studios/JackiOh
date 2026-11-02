// Small cue builders shared by the recipes outside `cues.ts` (cardFx.ts, brand.ts, chaos.ts and others).
// They build plain data exactly as `cues.ts` builds its own, so a cue from here is bounded by R200 the same way.

import { FX_MAX_TAIL_MS, FX_RAYS_TAIL_MS, FX_RING_MS } from "./constants.ts";
import type { FxAnchor, FxBurstCue, FxCue, FxPoint, FxPreset, FxRayTone, FxRingCue, FxRaysCue, FxShakeCue, FxSpread } from "./types.ts";

/** A burst's base particle count at intensity "normal" and the power it is thrown with (rule 9). */
export type BurstTuning = { readonly count: number; readonly power: number };

export function tid(testid: string, at?: FxPoint): FxAnchor {
  return at === undefined ? { kind: "testid", testid } : { kind: "testid", testid, at: { x: at.x, y: at.y } };
}

export function frac(fraction: number, durationMs: number): number {
  return Math.round(fraction * durationMs);
}

/** `count` scales with the intensity, never below 1; power does not (as in cues.ts). */
export function tunedBurst(
  intensity: number,
  preset: FxPreset,
  at: FxAnchor,
  spread: FxSpread,
  delayMs: number,
  tuning: BurstTuning,
): FxBurstCue {
  return { kind: "burst", preset, at, delayMs, count: Math.max(1, Math.round(tuning.count * intensity)), spread, power: tuning.power };
}

export function ringCue(D: number, preset: FxPreset, at: FxAnchor, delayMs: number): FxRingCue {
  return { kind: "ring", preset, at, delayMs, durationMs: Math.min(FX_RING_MS, D - delayMs + FX_MAX_TAIL_MS) };
}

export function raysCue(D: number, tone: FxRayTone, at: FxAnchor, delayMs: number): FxRaysCue {
  return { kind: "rays", tone, at, delayMs, durationMs: D - delayMs + FX_RAYS_TAIL_MS };
}

export function shakeCues(intensity: number, base: number, delayMs: number): FxCue[] {
  const trauma = Math.min(1, base * intensity);
  if (!(trauma > 0)) return [];
  const cue: FxShakeCue = { kind: "shake", trauma, delayMs };
  return [cue];
}
