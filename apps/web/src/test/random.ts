// A seeded source of numbers in [0, 1), `Math.random`'s shape, for tests that inject randomness the
// page draws from `Math.random` (the landing fan's deal, R374). mulberry32: small, and the same
// sequence for the same seed on every run.

import type { RandomSource } from "../routes/landingFan.ts";

export function seeded(seed: number): RandomSource {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
