// Issue #3734 (R3734000): the Cosmetics tab's search, rarity chips, owned or missing view and sort, in the Cards tab's
// own sidebar or sheet (BrowsePanel.tsx) and drawn with the deck builder's chip, search and select classes
// (deckbuilder.css), so the two tabs read as one. The state is the route's (`CosmeticsBrowse`); filtering is
// cosmeticsBrowse.ts's.

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { OWNERSHIPS, toggled, type Ownership } from "../game/deckbuilder/filters.ts";
import { filterOwnershipId, filterRarityId } from "../game/deckbuilder/testids.ts";
import BrowsePanel from "./BrowsePanel.tsx";
import { collectionTestid } from "./collection.ts";
import {
  COSMETIC_RARITIES,
  COSMETICS_SORTS,
  DEFAULT_COSMETICS_BROWSE,
  cosmeticsFiltersOn,
  type CosmeticsBrowse,
  type CosmeticsSort,
} from "./cosmeticsBrowse.ts";

const OWNERSHIP_LABELS: Readonly<Record<Ownership, string>> = { all: "All", owned: "Owned", missing: "Missing" };

/** Each text is its own literal, so the translation guard sees it. */
function sortLabel(sort: CosmeticsSort): string {
  switch (sort) {
    case "registry":
      return t("Default");
    case "name":
      return t("Name");
    case "rarity":
      return t("Rarity");
  }
}

export default function CosmeticsFilters({
  browse,
  onBrowse,
}: {
  browse: CosmeticsBrowse;
  onBrowse: (next: CosmeticsBrowse) => void;
}): ReactElement {
  useLocale();
  return (
    <>
      <div className="collection-bar">
        <div className="db-filters db-filters--toolbar" role="search" aria-label={t("Search cosmetics")}>
          <div className="db-filter-row db-filter-row--top">
            <input
              type="search"
              className="db-search"
              data-testid={collectionTestid.cosmeticsSearch}
              placeholder={t("Search by name")}
              aria-label={t("Search cosmetics")}
              value={browse.search}
              onChange={(event) => {
                onBrowse({ ...browse, search: event.target.value });
              }}
            />
            <label className="db-sort">
              <span className="db-group-label">{t("Sort")}</span>
              <select
                className="db-sort-select"
                data-testid={collectionTestid.cosmeticsSort}
                value={browse.sort}
                onChange={(event) => {
                  const sort = COSMETICS_SORTS.find((candidate) => candidate === event.target.value);
                  if (sort !== undefined) onBrowse({ ...browse, sort });
                }}
              >
                {COSMETICS_SORTS.map((sort) => (
                  <option key={sort} value={sort}>
                    {sortLabel(sort)}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="db-filter-clear"
              onClick={() => {
                onBrowse(DEFAULT_COSMETICS_BROWSE);
              }}
            >
              {t("Clear filters")}
            </button>
          </div>
        </div>
      </div>
      <BrowsePanel
        active={cosmeticsFiltersOn({ ...browse, search: "" })}
        controls={
          <div className="db-filters db-filters--chips" role="group" aria-label={t("Filters")}>
            <div className="db-filter-chips">
              <div className="db-filter-row">
                <div className="db-chip-group" role="group" aria-label={t("Owned")}>
                  {OWNERSHIPS.map((view) => (
                    <button
                      key={view}
                      type="button"
                      className="db-chip button-secondary db-chip--ownership"
                      data-testid={filterOwnershipId(view)}
                      aria-pressed={browse.ownership === view}
                      onClick={() => {
                        onBrowse({ ...browse, ownership: view });
                      }}
                    >
                      {t(OWNERSHIP_LABELS[view])}
                    </button>
                  ))}
                </div>
              </div>
              <div className="db-filter-row">
                <div className="db-chip-group" role="group" aria-label={t("Rarity")}>
                  <span className="db-group-label" aria-hidden="true">
                    {t("Rarity")}
                  </span>
                  {COSMETIC_RARITIES.map((rarity) => (
                    <button
                      key={rarity}
                      type="button"
                      className="db-chip button-secondary db-chip--rarity"
                      data-testid={filterRarityId(rarity)}
                      data-rarity={rarity}
                      aria-pressed={browse.rarities.has(rarity)}
                      onClick={() => {
                        onBrowse({ ...browse, rarities: toggled(browse.rarities, rarity) });
                      }}
                    >
                      {t(rarity)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
        }
      />
    </>
  );
}
