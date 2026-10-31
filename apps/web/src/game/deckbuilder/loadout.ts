// Collection adapter and deck pool shelf (SPEC §9.4, R253; CLAUDE.md rule 7).

import { setShips, type SetName } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { isGlitch } from "../../cards/glitch.ts";

/** §12.1, R915000: custom card set (R915005: valid only in practice). */
export const CUSTOM_SET: SetName = "Custom";

/** `GET /api/collection`'s entries, as the validator wants them. */
export function collectionFrom(
  entries: readonly { cardId: string; quantity: number }[],
): Collection {
  const owned: Record<string, number> = {};
  for (const entry of entries) owned[entry.cardId] = entry.quantity;
  return owned;
}

/**
 * Pool cards: owned non-token catalog cards in catalog order. Excludes tokens (R251, L3),
 * unshipped sets (R1420), but includes player custom cards (R915005).
 */
export function poolFrom(catalog: CatalogSnapshot, collection: Collection | null): readonly string[] {
  const ids = Object.keys(catalog.cards).filter((id) => {
    const def = catalog.cards[id];
    if (def === undefined) return false;
    if (def.token || def.tags.includes("Token")) return false;
    // R674: Glitch is never in a pool, a token or not.
    if (isGlitch(id)) return false;
    // R1420: nor is a card of a set that has not shipped yet; R915005: the player's own custom card is.
    if (!setShips(def.set) && def.set !== CUSTOM_SET) return false;
    if (collection === null) return true;
    return (collection[id] ?? 0) > 0;
  });
  return ids.sort((a, b) => {
    const left = catalog.cards[a];
    const right = catalog.cards[b];
    const byIndex = indexOrder(left?.index) - indexOrder(right?.index);
    if (byIndex !== 0) return byIndex;
    return a.localeCompare(b);
  });
}

/** §5: `index` is "43" or "51.1"; sort by the printed number so the pool reads like SPEC §8. */
function indexOrder(index: string | undefined): number {
  if (index === undefined) return Number.POSITIVE_INFINITY;
  const parsed = Number.parseFloat(index);
  return Number.isNaN(parsed) ? Number.POSITIVE_INFINITY : parsed;
}
