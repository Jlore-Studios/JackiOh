// A seeded source of numbers in [0, 1), `Math.random`'s shape, for tests that inject randomness the
// page draws from `Math.random` (the landing fan's deal, R374). mulberry32: small, and the same
// sequence for the same seed on every run.

import { createRng } from "../fx/rng.ts";
import type { RandomSource } from "../routes/landingFan.ts";

export function seeded(seed: number): RandomSource {
  return createRng(seed);
}
