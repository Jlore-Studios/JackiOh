/**
 * Trauma-based board shake (docs/polish/1-animations.md, S5; B30), after Squirrel Eiserloh's GDC 2016
 * "Juicing Your Cameras With Math". Trauma in [0, 1] decays linearly and the shake is trauma², so small
 * hits barely move the board; the offsets read smooth seeded noise, not random jitter, so the motion is
 * a rumble and the same seed always shakes the same way.
 */
import {
  FX_DEFAULT_SEED,
  FX_SHAKE_FREQ_HZ,
  FX_SHAKE_MAX_DEG,
  FX_SHAKE_MAX_PX,
  FX_TRAUMA_DECAY,
} from "./constants.ts";
import { createNoise1D } from "./rng.ts";
import type { FxShakeOffset } from "./types.ts";

export type Shake = {
  add(trauma: number): void;
  step(dtMs: number): void;
  sample(nowMs: number): FxShakeOffset;
  trauma(): number;
  active(): boolean;
  reset(): void;
};

const MS_PER_SECOND = 1000;
/** Seed offsets that give the y and angle channels their own noise. */
const CHANNEL_Y = 1;
const CHANNEL_ANGLE = 2;

export function createShake(options?: {
  seed?: number;
  maxOffsetPx?: number;
  maxAngleDeg?: number;
  decayPerSecond?: number;
  frequencyHz?: number;
}): Shake {
  const seed = options?.seed ?? FX_DEFAULT_SEED;
  const maxOffsetPx = options?.maxOffsetPx ?? FX_SHAKE_MAX_PX;
  const maxAngleDeg = options?.maxAngleDeg ?? FX_SHAKE_MAX_DEG;
  const decayPerSecond = options?.decayPerSecond ?? FX_TRAUMA_DECAY;
  const frequencyHz = options?.frequencyHz ?? FX_SHAKE_FREQ_HZ;

  const noiseX = createNoise1D(seed);
  const noiseY = createNoise1D(seed + CHANNEL_Y);
  const noiseAngle = createNoise1D(seed + CHANNEL_ANGLE);
  let trauma = 0;

  return {
    add(t) {
      trauma = Math.min(1, trauma + Math.max(0, t));
    },
    step(dtMs) {
      trauma = Math.max(0, trauma - (decayPerSecond * dtMs) / MS_PER_SECOND);
    },
    sample(nowMs) {
      const shake = trauma * trauma;
      if (shake === 0) return { x: 0, y: 0, angle: 0 };
      const t = (nowMs / MS_PER_SECOND) * frequencyHz;
      // `+ 0` turns a negative zero into a plain zero, so a still board compares equal to {0, 0, 0}.
      return {
        x: maxOffsetPx * shake * noiseX(t) + 0,
        y: maxOffsetPx * shake * noiseY(t) + 0,
        angle: maxAngleDeg * shake * noiseAngle(t) + 0,
      };
    },
    trauma: () => trauma,
    active: () => trauma > 0,
    reset() {
      trauma = 0;
    },
  };
}
