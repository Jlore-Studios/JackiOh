// How firmly a card sits in its tier on /stats (SPEC §9.11, R3184001): the share of its Adjusted 95% range
// (R2895000) that lies inside its tier's band (R3022000). The tiers and ranges are the server's; this only
// measures one against the other (CLAUDE.md rule 7).

import type { PublicCardStat, PublicStatsTiers } from "../net/api.ts";
import type { CardTier } from "../wire/generated/CardTier.ts";

/** R3022000: the tiers that split the band into equal parts, from the top. */
const BAND_TIERS: readonly CardTier[] = ["A", "B", "C", "D"];

/** R3184001: a tier's band, low to high: S from the band's top up, F from its bottom down, A to D its quarters from the top. */
export function tierBand(tier: CardTier, bandLow: number, bandHigh: number): readonly [number, number] {
  if (tier === "S") return [bandHigh, Infinity];
  if (tier === "F") return [-Infinity, bandLow];
  const step = (bandHigh - bandLow) / BAND_TIERS.length;
  const top = bandHigh - BAND_TIERS.indexOf(tier) * step;
  return [top - step, top];
}

/**
 * R3184001: the share, 0 to 1, of the range `low` to `high` inside `tier`'s band. A range of no width is its one
 * point, the rate: wholly inside the band or wholly outside it.
 */
export function tierFit(tier: CardTier, low: number, high: number, bandLow: number, bandHigh: number): number {
  const [from, to] = tierBand(tier, bandLow, bandHigh);
  if (high <= low) return low >= from && low <= to ? 1 : 0;
  return Math.max(0, Math.min(high, to) - Math.max(low, from)) / (high - low);
}

/** R3184001: a card's tier fit, or null for a card below the floor, with no tier or no Adjusted range, or with no band. */
export function cardTierFit(card: PublicCardStat, tiers: PublicStatsTiers | undefined): number | null {
  if (!card.hasEnoughGames || !card.tier || !tiers) return null;
  if (typeof card.adjustedLow !== "number" || typeof card.adjustedHigh !== "number") return null;
  return tierFit(card.tier, card.adjustedLow, card.adjustedHigh, tiers.bandLow, tiers.bandHigh);
}
