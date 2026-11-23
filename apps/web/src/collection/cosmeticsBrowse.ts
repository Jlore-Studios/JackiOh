// Issue #3734 (R3734000): the Cosmetics tab's search, rarity chips, owned or missing view and sort over the account's
// tiles (`accountTiles`). Ownership is the tile's own, `useOwnsAccountCosmetic`'s answer (CLAUDE.md rule 7): the view
// only hides tiles. Pure; the route holds the state, so it survives a switch of tab.

import { COSMETIC_RARITIES, type CosmeticRarity } from "../wire/generated/cosmetics.ts";
import { DEFAULT_LOCALE, getLocale, t } from "../i18n/i18n.ts";
import type { Ownership } from "../game/deckbuilder/filters.ts";
import type { AccountTile } from "./collection.ts";

/** "registry" is the order the registry lists them in, each kind's default first. */
export type CosmeticsSort = "registry" | "name" | "rarity";

export const COSMETICS_SORTS: readonly CosmeticsSort[] = ["registry", "name", "rarity"];

export type CosmeticsBrowse = {
  search: string;
  rarities: ReadonlySet<CosmeticRarity>;
  ownership: Ownership;
  sort: CosmeticsSort;
};

export const DEFAULT_COSMETICS_BROWSE: CosmeticsBrowse = {
  search: "",
  rarities: new Set<CosmeticRarity>(),
  ownership: "all",
  sort: "registry",
};

export { COSMETIC_RARITIES };

/** Whether `browse` is hiding nothing: the tab's Filters button counts what it is not. */
export function cosmeticsFiltersOn(browse: CosmeticsBrowse): number {
  return (browse.search.trim() === "" ? 0 : 1) + browse.rarities.size + (browse.ownership === "all" ? 0 : 1);
}

/**
 * The tiles `browse` keeps, in its sort's order: the name contains each word of the search (case-insensitively, in the
 * reader's language), the rarity is one of the chips on (none on is every one), and the tile is owned or missing as the
 * view says. The sort is stable, so equals keep the registry's order.
 */
export function browseTiles(tiles: readonly AccountTile[], browse: CosmeticsBrowse): AccountTile[] {
  const words = browse.search.trim().toLowerCase().split(/\s+/).filter((word) => word.length > 0);
  const locale = getLocale();
  const kept = tiles.filter(({ cosmetic, owned }) => {
    if (browse.rarities.size > 0 && !browse.rarities.has(cosmetic.rarity)) return false;
    if (browse.ownership !== "all" && owned !== (browse.ownership === "owned")) return false;
    const name = t(cosmetic.name).toLowerCase();
    return words.every((word) => name.includes(word));
  });
  if (browse.sort === "registry") return kept;
  const order = (tile: AccountTile): number => COSMETIC_RARITIES.indexOf(tile.cosmetic.rarity);
  return kept
    .map((tile, at) => ({ tile, at }))
    .sort((left, right) => {
      const byKey =
        browse.sort === "rarity"
          ? order(left.tile) - order(right.tile)
          : t(left.tile.cosmetic.name).localeCompare(t(right.tile.cosmetic.name), locale === DEFAULT_LOCALE ? undefined : locale, {
              sensitivity: "base",
            });
      return byKey !== 0 ? byKey : left.at - right.at;
    })
    .map(({ tile }) => tile);
}
