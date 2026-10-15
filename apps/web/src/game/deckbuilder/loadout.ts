// The collection as the builder reads it, and the shelf of cards the pool offers.
//
// NO RULE LIVES HERE (CLAUDE.md rule 7). Whether a deck or a trio is legal is
// `@jackioh/validator`'s alone (SPEC §9.4, R253), called from workshop.ts; this module only turns
// `GET /api/collection` into the validator's `Collection` and lists the cards a deck could hold.

import { setShips, type SetName } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { isGlitch } from "../../cards/glitch.ts";

/**
 * §12.1, R915000: a custom card's set, which never ships. The deck builder shelves the account's own
 * custom cards beside the catalog (R915005: a deck may hold them, and then plays only in practice).
 */
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
 * The cards offered in the pool: everything the profile owns that the catalog knows about, in
 * catalog order (every non-token card when `collection` is null). Tokens are left out because
 * R251 and L3 keep them out of every deck, and so are the cards of a set that has not shipped
 * (R1420) — the pool is a shelf of cards a deck can hold, and the validator still has the last word
 * on anything that reaches a deck by another route (an import). A custom card the screen put in the
 * catalog is the player's own (set `Custom`, R915005), so it is shelved though its set never ships.
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
