// Unit Slam (#185): how hard a Unit lands, from its size. Presentation only (CLAUDE.md rule 7): it
// reads the public numbers of a Unit the viewer can see and never changes resolution or timing.
// Its numbers are in damageFeel.ts, beside #57's (CLAUDE.md rule 9).

import { hasKeyword, type UnitView } from "@jackioh/shared";

import { SLAM_TIERS_ASCENDING, SLAM_WEIGHTS, UNIT_SLAM, type SlamTier } from "./damageFeel.ts";

export type { SlamTier } from "./damageFeel.ts";

/** The numbers a landing Unit's size is read from, as the viewer's `UnitView` shows them. */
export type SlamStats = {
  attack: number;
  health: number;
  /** Printed plus aura Armor, as the view shows it; Defense Position's +1 is not counted. */
  armor: number;
  indestructible: boolean;
  /** The Tributes the card requires. */
  tributes: number;
};

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
  return SLAM_TIERS_ASCENDING.find((tier) => total <= UNIT_SLAM[tier].maxTotal) ?? "massive";
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
