// R630's shelf, the cards the Card Almanac shows, kept apart from `filters.ts` (which imports the
// client's aliases) so the build's static `/almanac` page (`apps/web/static-pages.ts`) lists exactly
// the cards the page does, from one rule.

import { setShips, type SetName } from "../../wire/catalog.ts";
import { GLITCH_DEF_ID } from "../../wire/engineConfig.ts";

/** Every catalog id but Glitch's (R674) and those of a set not shipped yet (R1420), in catalog order. */
export function almanacShelf(cards: Readonly<Record<string, { readonly set: SetName }>>): string[] {
  return Object.keys(cards).filter((id) => {
    const def = cards[id];
    return def !== undefined && id !== GLITCH_DEF_ID && setShips(def.set);
  });
}
