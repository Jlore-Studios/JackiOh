// R3184000: how firmly a card sits in its tier on /stats (SPEC §9.11, issue #3184): the share of its Adjusted 95%
// range that falls inside its tier's stretch of rates. The stretches come from the server's band (`tiers.bandLow`
// to `tiers.bandHigh`, R3022000): S is the band's top and up, F its bottom and down, and A, B, C and D split the
// band into quarters from the top.

import type { CardTier } from "../wire/generated/CardTier.ts";

/** R3022000: the tiers inside the band, from the top; each holds an equal share of it. */
const BAND_TIERS: readonly CardTier[] = ["A", "B", "C", "D"];

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

/** The band the tiers split, as fractions: `PublicStatsTiers`' `bandLow` and `bandHigh`. */
export type TierBand = { bandLow: number; bandHigh: number };

/** A tier's stretch of rates, as fractions; S's has no top and F's no bottom. */
export function tierBounds(tier: CardTier, band: TierBand): { low: number; high: number } {
  if (tier === "S") return { low: band.bandHigh, high: Number.POSITIVE_INFINITY };
  if (tier === "F") return { low: Number.NEGATIVE_INFINITY, high: band.bandLow };
  const step = (band.bandHigh - band.bandLow) / BAND_TIERS.length;
  const from = BAND_TIERS.indexOf(tier);
  return { low: band.bandHigh - (from + 1) * step, high: band.bandHigh - from * step };
}

/**
 * The share of the range `low` to `high` inside `tier`'s stretch, to a whole percent: the overlap of the two over
 * the range's width. A range of no width is the rate itself, all inside (100) or all outside (0). Null when there is
 * no tier, no range or no band.
 */
export function tierFit(
  tier: CardTier | null | undefined,
  low: number | null | undefined,
  high: number | null | undefined,
  band: TierBand | null | undefined,
): number | null {
  if (!tier || typeof low !== "number" || typeof high !== "number" || !band) return null;
  const bounds = tierBounds(tier, band);
  const width = high - low;
  if (width <= 0) return low >= bounds.low && low <= bounds.high ? PERCENT : 0;
  const overlap = Math.max(0, Math.min(high, bounds.high) - Math.max(low, bounds.low));
  return Math.round((overlap / width) * PERCENT);
}
