// Unit Slam (#185): how hard a Unit lands, from its size. Presentation only (CLAUDE.md rule 7): it
// reads the public numbers of a Unit the viewer can see and never changes resolution or timing.

import { hasKeyword, type UnitView } from "@jackioh/shared";

export type SlamTier = "tiny" | "small" | "medium" | "large" | "huge" | "massive";

/** The numbers a landing Unit's size is read from, as the viewer's `UnitView` shows them. */
export type SlamStats = {
  attack: number;
  health: number;
  /** Printed plus aura Armor; Defense Position's +1 is not counted. */
  armor: number;
  indestructible: boolean;
  /** The Tributes the card requires. */
  tributes: number;
};

/** Weights of the total stats; every number is a named constant (CLAUDE.md rule 9). */
export const SLAM_WEIGHTS = Object.freeze({ armor: 2, indestructible: 4, tribute: 4 });

/** Each tier's highest total; MASSIVE takes everything above Huge. */
export const SLAM_TIER_MAX: Readonly<Record<Exclude<SlamTier, "massive">, number>> = Object.freeze({
  tiny: 5,
  small: 11,
  medium: 17,
  large: 27,
  huge: 39,
});

export const SLAM_TIERS_ASCENDING: readonly SlamTier[] = ["tiny", "small", "medium", "large", "huge", "massive"];

export function slamTotal(stats: SlamStats): number {
  const n = (value: number) => (Number.isFinite(value) ? Math.max(0, value) : 0);
  return (
    n(stats.attack) +
    n(stats.health) +
    SLAM_WEIGHTS.armor * n(stats.armor) +
    (stats.indestructible ? SLAM_WEIGHTS.indestructible : 0) +
    SLAM_WEIGHTS.tribute * n(stats.tributes)
  );
}

export function slamTierOfTotal(total: number): SlamTier {
  for (const tier of SLAM_TIERS_ASCENDING) {
    if (tier === "massive" || total <= SLAM_TIER_MAX[tier]) return tier;
  }
  return "massive";
}

export function slamTier(stats: SlamStats): SlamTier {
  return slamTierOfTotal(slamTotal(stats));
}

/**
 * The card's printed "Tribute N". The view carries no Tribute cost, so the client reads it off the
 * face's text the way the card prints it ("Tribute 1, Indestructible", "Taunt, Tribute 3"). A Cry
 * that tributes ("Tribute one of your other Units") is an effect, not a cost, and has no number.
 */
const TRIBUTE_COST = /\bTribute (\d+)\b/;

export function printedTributes(text: string | undefined): number {
  const match = text === undefined ? null : TRIBUTE_COST.exec(text);
  return match === null ? 0 : Number(match[1]);
}

/** A landing Unit's numbers, read from its view and the text of the face it landed on. */
export function slamStatsOf(unit: Pick<UnitView, "attack" | "health" | "armor" | "keywords">, faceText?: string): SlamStats {
  return {
    attack: unit.attack,
    health: unit.health,
    armor: unit.armor,
    indestructible: hasKeyword(unit.keywords, "Indestructible"),
    tributes: printedTributes(faceText),
  };
}
