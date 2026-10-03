// Presentation tuning for match impacts. This module is deliberately client-only: it classifies
// public damage events and never changes their resolution, timing, or legality.

export type DamageTier = "tiny" | "normal" | "moderate" | "big" | "giga";

export type DamageFeel = {
  min: number;
  hitStopMs: number;
  shakePx: number;
  shakeMs: number;
  numberScale: number;
  numberLingerMs: number;
  particleScale: number;
  crowd: "none" | "ooh" | "gasp" | "roar";
};

/**
 * The one tuning surface for issue #57. Consumers use this rather than embedding thresholds or
 * timings in audio, CSS, or effects code, so a feel pass can tune every consequence together.
 */
export const DAMAGE_FEEL: Readonly<Record<DamageTier, DamageFeel>> = Object.freeze({
  tiny: { min: 0, hitStopMs: 0, shakePx: 0, shakeMs: 0, numberScale: 0.78, numberLingerMs: 300, particleScale: 0.55, crowd: "none" },
  normal: { min: 3, hitStopMs: 40, shakePx: 0, shakeMs: 0, numberScale: 1, numberLingerMs: 300, particleScale: 1, crowd: "none" },
  moderate: { min: 7, hitStopMs: 70, shakePx: 2, shakeMs: 150, numberScale: 1.18, numberLingerMs: 300, particleScale: 1.25, crowd: "ooh" },
  big: { min: 10, hitStopMs: 110, shakePx: 6, shakeMs: 250, numberScale: 1.42, numberLingerMs: 300, particleScale: 1.65, crowd: "gasp" },
  giga: { min: 20, hitStopMs: 180, shakePx: 12, shakeMs: 400, numberScale: 1.78, numberLingerMs: 1200, particleScale: 2, crowd: "roar" },
});

export const DAMAGE_TIERS_DESCENDING: readonly DamageTier[] = ["giga", "big", "moderate", "normal", "tiny"];

/** Match-wide crowd and room timing, kept beside the impact tiers it responds to. */
export const CROWD_FEEL = Object.freeze({
  reactionDebounceMs: 400,
  ambientDuckDb: 4,
  ambientFadeOutMs: 1500,
  patronMinMs: 6000,
  patronMaxMs: 15000,
  bedLoopSeconds: [47, 73] as const,
  cheerDelayMs: 180,
  applauseDelayMs: 700,
  applauseTailMs: 2500,
  reactionMs: { ooh: 700, gasp: 1100, roar: 2500, cheer: 850, applause: 2500 },
});

/** Every amount, including a prevented zero hit, has a visible tier. */
export function damageFeel(amount: number): DamageFeel {
  const safe = Number.isFinite(amount) ? amount : 0;
  for (const tier of DAMAGE_TIERS_DESCENDING) {
    const feel = DAMAGE_FEEL[tier];
    if (safe >= feel.min) return feel;
  }
  return DAMAGE_FEEL.tiny;
}

export function damageTier(amount: number): DamageTier {
  const feel = damageFeel(amount);
  return (Object.keys(DAMAGE_FEEL) as DamageTier[]).find((tier) => DAMAGE_FEEL[tier] === feel) ?? "tiny";
}
