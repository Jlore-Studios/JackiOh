// #185: which tier a landing Unit slams at, for the animation queue (animations.ts `slamOf`).
//
// A landing animates over the board as it stood before the Unit arrived (BUILD M5-T4), so its
// numbers come from the newest view, where the Unit stands as it landed: its Radiant stats, its
// printed and aura Armor (R97: only what the viewer's view shows). A Unit that has already left the
// field by the end of the burst is read off its printed face instead. The Tributes are the face's
// printed "Tribute N" (unitSlam.ts). Nothing here reads a card the view hides: the queue never asks
// about a landing behind the sentinel (`landingOf`).

import type { PlayerView, UnitView } from "@jackioh/shared";

import type { LandingEvent, SlamResolver } from "./animations.ts";
import type { CardLookup } from "./catalog.ts";
import { printedTributes, slamStatsOf, slamTier, type SlamTier } from "./unitSlam.ts";

/** The Unit with this instance id on either side's field, top of its pile, in `view`. */
export function unitIn(view: PlayerView, instanceId: string): UnitView | null {
  for (const side of [view.you, view.opponent]) {
    for (const unit of side.units) if (unit !== null && unit.instanceId === instanceId) return unit;
  }
  return null;
}

/** The tier `event` lands at against `view` (the newest one) and the catalog. */
export function slamTierFor(event: LandingEvent, view: PlayerView, lookup: CardLookup | null): SlamTier {
  const unit = unitIn(view, event.instanceId);
  const info = lookup?.(event.defId, unit?.radiant ?? false);
  if (unit !== null) return slamTier(slamStatsOf(unit, info?.text));
  return slamTier({ attack: info?.attack ?? 0, health: info?.health ?? 0, armor: 0, indestructible: false, tributes: printedTributes(info?.text) });
}

/** The queue's resolver, reading the newest view and the catalog each time it is asked. */
export function createSlamResolver(view: () => PlayerView, lookup: () => CardLookup | null): SlamResolver {
  return (event) => slamTierFor(event, view(), lookup());
}
