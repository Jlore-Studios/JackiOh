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

/** Match-wide crowd timing, kept beside the impact tiers it responds to. */
export const CROWD_FEEL = Object.freeze({
  reactionDebounceMs: 400,
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

/** #185 (R700–R702): how hard a Unit lands, by the size tier `unitSlam.ts` reads off its stats. */
export type SlamTier = "tiny" | "small" | "medium" | "large" | "huge" | "massive";

/** What the board does under a landing Unit, from a few grains shifting to a crater. */
export type SlamBoard = "none" | "grains" | "puff" | "cloud" | "cracks" | "crater";

export type SlamFeel = {
  /** The tier's highest total stats; MASSIVE has no ceiling. */
  maxTotal: number;
  hitStopMs: number;
  shakePx: number;
  shakeMs: number;
  /** The beat the Unit hangs before it lands, played inside its animation entry (never under Reduce motion). */
  anticipationMs: number;
  board: SlamBoard;
  /** Which cards wobble: none, the landing side's, the whole board's, or every card on screen. */
  wobble: "none" | "side" | "board" | "all";
  /** A shockwave ring and a vignette flash on the landing beat. */
  shockwave: boolean;
  /** The summon thud's weight, 0 (a soft tap) to 1 (a deep boom). */
  thud: number;
  /** An impact under the thud: Huge's crack, MASSIVE's boom. */
  impact: "none" | "big" | "giga";
  crowd: "none" | "murmur" | "anticipation" | "excited";
};

/**
 * The one tuning surface for issue #185, beside #57's. The weights: total = Attack + Health
 * + 2 × Armor + 4 if Indestructible + 4 × each Tribute the card requires.
 */
export const SLAM_WEIGHTS = Object.freeze({ armor: 2, indestructible: 4, tribute: 4 });

export const UNIT_SLAM: Readonly<Record<SlamTier, SlamFeel>> = Object.freeze({
  tiny: { maxTotal: 5, hitStopMs: 0, shakePx: 0, shakeMs: 0, anticipationMs: 0, board: "none", wobble: "none", shockwave: false, thud: 0, impact: "none", crowd: "none" },
  small: { maxTotal: 11, hitStopMs: 0, shakePx: 0, shakeMs: 0, anticipationMs: 0, board: "grains", wobble: "none", shockwave: false, thud: 0.25, impact: "none", crowd: "none" },
  medium: { maxTotal: 17, hitStopMs: 0, shakePx: 0, shakeMs: 0, anticipationMs: 0, board: "puff", wobble: "none", shockwave: false, thud: 0.5, impact: "none", crowd: "none" },
  large: { maxTotal: 27, hitStopMs: 40, shakePx: 0, shakeMs: 0, anticipationMs: 0, board: "cloud", wobble: "side", shockwave: false, thud: 0.75, impact: "none", crowd: "murmur" },
  huge: { maxTotal: 39, hitStopMs: 90, shakePx: 6, shakeMs: 250, anticipationMs: 300, board: "cracks", wobble: "board", shockwave: false, thud: 1, impact: "big", crowd: "anticipation" },
  massive: { maxTotal: Number.POSITIVE_INFINITY, hitStopMs: 160, shakePx: 12, shakeMs: 400, anticipationMs: 500, board: "crater", wobble: "all", shockwave: true, thud: 1, impact: "giga", crowd: "excited" },
});

export const SLAM_TIERS_ASCENDING: readonly SlamTier[] = ["tiny", "small", "medium", "large", "huge", "massive"];

/**
 * Several Units landing in one action play one after another (each is its own animation entry) and
 * share these totals, so a board flood never stacks into a long freeze or a violent shake.
 */
export const SLAM_BURST = Object.freeze({ hitStopMs: 200, shakePx: 12, anticipationMs: 500 });

/** #185: where in its landing motion a Unit hits the table (jk-summon-scale's 55% keyframe). */
export const SLAM_LAND_AT = 0.55;

/** #185: a slam's pitch varies by up to this much either way. */
export const SLAM_PITCH_SPREAD = 0.05;
