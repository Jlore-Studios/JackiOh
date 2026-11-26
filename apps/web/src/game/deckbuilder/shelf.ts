// R630's shelf, the cards the Card Almanac shows, kept apart from `filters.ts` (which imports the
// client's aliases) so the build's static `/almanac` page (`apps/web/static-pages.ts`) lists exactly
// the cards the page does, from one rule.

import { SHIPPED_SETS, type SetName } from "../../wire/catalog.ts";
import { GLITCH_DEF_ID } from "../../wire/engineConfig.ts";

/**
 * R5131000: the sets the Almanac shows, in its order: the sets that ship (R1380, R1420), then the Infinite Assault's
 * (§8.13), a single-player set whose cards its enemies play. Boss and Boss-X, the other single-player sets, stay off.
 * The deck builder's sets are still `SHIPPED_SETS` alone, since no deck holds a single-player card (R1846001).
 */
export const ALMANAC_SETS: readonly SetName[] = [...SHIPPED_SETS, "Infinite Assault"];

/** R1420, R5131000: whether the Almanac shows a set's cards. */
export function almanacShows(set: SetName): boolean {
  return ALMANAC_SETS.includes(set);
}

/** Every catalog id but Glitch's (R674) and those of a set the Almanac does not show (R1420, R5131000), in catalog order. */
export function almanacShelf(cards: Readonly<Record<string, { readonly set: SetName }>>): string[] {
  return Object.keys(cards).filter((id) => {
    const def = cards[id];
    return def !== undefined && id !== GLITCH_DEF_ID && almanacShows(def.set);
  });
}
