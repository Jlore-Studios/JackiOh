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
 * The one tuning surface for damage feel. Consumers use this rather than embedding thresholds or
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
  patronMinMs: 6000,
  patronMaxMs: 15000,
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

/** R700–R702: how hard a Unit lands, by the size tier `unitSlam.ts` reads off its stats. */
export type SlamTier = "tiny" | "small" | "medium" | "large" | "huge" | "massive";

/** What the board does under a landing Unit. */
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
 * The one tuning surface for Unit slams, beside `DAMAGE_FEEL`. The weights: total = Attack + Health
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

/** R3346000 (#3346): how heavy a play lands, by the mana it cost (`cardPlayed.costPaid`). */
export type PlayTier = "light" | "medium" | "heavy" | "massive";

/** The colour of a play's impact: a body landing, a charge and release, or the ground settling. */
export type PlayType = "unit" | "spell" | "field";

export type PlayFeel = {
  /** The tier's lowest mana cost paid. */
  min: number;
  /** The impact's weight, 0 (a flick, a tap, a hum) to 1 (a thunderclap, a slam, a rumble). */
  weight: number;
};

/**
 * R3346000: the one tuning surface for a play's weight, beside `UNIT_SLAM` (which sizes the
 * landing thud by the Unit's stats; this sizes the impact by what the card cost). The impact, the
 * board shake and the phone's buzz all key on `playTier`. Client-only: it reads a public
 * `costPaid` and changes no resolution, timing or legality.
 */
export const PLAY_IMPACT_TIERS: Readonly<Record<PlayTier, PlayFeel>> = Object.freeze({
  light: { min: 0, weight: 0.2 },
  medium: { min: 3, weight: 0.5 },
  heavy: { min: 6, weight: 0.8 },
  massive: { min: 9, weight: 1 },
});

export const PLAY_TIERS_ASCENDING: readonly PlayTier[] = ["light", "medium", "heavy", "massive"];

/** A play's tier from the mana it cost; a cost that is not a finite, non-negative number is light. */
export function playTier(costPaid: number): PlayTier {
  const safe = Number.isFinite(costPaid) ? costPaid : 0;
  for (let i = PLAY_TIERS_ASCENDING.length - 1; i > 0; i--) {
    const tier = PLAY_TIERS_ASCENDING[i]!;
    if (safe >= PLAY_IMPACT_TIERS[tier].min) return tier;
  }
  return "light";
}

/**
 * Several Units landing in one action play one after another (each is its own animation entry) and
 * share these totals, so a board flood never stacks into a long freeze or a violent shake.
 */
export const SLAM_BURST = Object.freeze({ hitStopMs: 200, shakePx: 12, anticipationMs: 500 });

/** Where in its landing motion a Unit hits the table (jk-summon-scale's 55% keyframe). */
export const SLAM_LAND_AT = 0.55;

/** A slam's pitch varies by up to this much either way. */
export const SLAM_PITCH_SPREAD = 0.05;

/**
 * R1363 (MN05): whether a hit's target's Armor took half or more of it — `absorbed` of the whole
 * `absorbed + amount` the hit came in with (R1360). The sound's clank and the effects layer's shield
 * flash both key on this test; a hit with no Armor part never passes it, and one Armor took whole
 * is `damageAbsorbed`.
 */
export function armorTookHalf(absorbed: number | undefined, amount: number): boolean {
  const took = absorbed ?? 0;
  return took > 0 && took * 2 >= took + amount;
}
