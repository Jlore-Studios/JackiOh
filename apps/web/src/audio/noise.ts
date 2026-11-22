// The shared noise buffer (docs/polish/2-sound.md, "sfx.ts"): 1 s of mono white noise, cached
// per context (WeakMap), filled from a fixed-seed mulberry32 so every run is identical. It lives
// here, not in `sfx.ts`, so the crowd (`crowd.ts`) reads it without pulling the recipes into the
// entry chunk (#4428); `sfx.ts` re-exports it, so its Cypress and unit imports keep working.

import { createRng } from "../fx/rng.ts";

const NOISE_SEED = 0x4a41434b; // "JACK"
const noiseCache = new WeakMap<BaseAudioContext, AudioBuffer>();

/** 1 s of mono white noise, cached per context (WeakMap), filled from a fixed-seed mulberry32 so every run is identical. */
export function noiseBuffer(ctx: BaseAudioContext): AudioBuffer {
  const cached = noiseCache.get(ctx);
  if (cached !== undefined) return cached;
  const length = Math.round(ctx.sampleRate);
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const random = createRng(NOISE_SEED);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i += 1) data[i] = random() * 2 - 1;
  noiseCache.set(ctx, buffer);
  return buffer;
}
