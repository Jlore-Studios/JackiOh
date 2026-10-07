// `/almanac`: the public Card Almanac (R630). Every card in the catalog, tokens included, to browse
// without signing in: the deck builder's browse pane (game/deckbuilder/CardBrowser.tsx) with nothing
// that edits a deck, no collection and no ownership. Public like the Patch notes page: the catalog
// is public (§5.1) and ships in the bundle, so the page reads `@jackioh/cards/catalog.json`, as the
// landing's fan does, and asks no server and no account anything for the catalog. The card detail's
// statistics block (R654) reads the public card aggregates, like the deck builder's.
//
// The page decides nothing (CLAUDE.md rule 7): what the filter and the sort keep is filters.ts's,
// the faces and the detail view are the cards module's. It wears the deck builder's look
// (deckbuilder.css) in the tavern, with the corner's Back and the settings gear; almanac.css holds
// only what the browse pane needs without the deck editor's grid around it.

import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDef, CardDefs } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardDefsProvider, closeInspect } from "../cards/index.ts";
import CardBrowser from "../game/deckbuilder/CardBrowser.tsx";
import {
  ALMANAC_TAGS,
  DEFAULT_FILTER,
  DEFAULT_SORT,
  almanacPool,
  type CardWinRateInfo,
  type PoolFilter,
  type PoolSort,
} from "../game/deckbuilder/filters.ts";
import { getCardStats } from "../net/api.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "../game/deckbuilder/deckbuilder.css";
import "./almanac.css";

export const almanacTestid = {
  /** The page's root. */
  screen: "almanac",
} as const;

/**
 * The bundled catalog. A snapshot's `version` is what a save or a queue checks (§9.4); the almanac
 * makes neither, so nothing reads it.
 */
export const ALMANAC_CATALOG: CatalogSnapshot = {
  version: "bundled",
  // JSON widens the unions to strings; the catalog is proved against SPEC §8 in crates/cards.
  cards: catalogJson as unknown as CardDefs,
};

/**
 * The detail view's line under the glossary: the card's rarity; for a token, the rarity it prints
 * (if any) and that it never goes in a deck (L3), "Rare · Token · not deckable".
 */
export function almanacMeta(def: CardDef): string {
  if (!def.token && !def.tags.includes("Token")) return def.rarity;
  const printed = def.printedRarity === undefined ? [] : [def.printedRarity];
  return [...printed, "Token", "not deckable"].join(" · ");
}

export default function AlmanacRoute(): ReactElement {
  const [filter, setFilter] = useState<PoolFilter>(DEFAULT_FILTER);
  const [sort, setSort] = useState<PoolSort>(DEFAULT_SORT);
  const [detailCardId, setDetailCardId] = useState<string | null>(null);
  const [winRates, setWinRates] = useState<Map<string, CardWinRateInfo> | null>(null);

  useEffect(() => {
    if (sort.key === "winRate" && winRates === null) {
      let cancelled = false;
      getCardStats()
        .then((res) => {
          if (cancelled) return;
          const map = new Map<string, CardWinRateInfo>();
          for (const card of res.cards) {
            map.set(card.id, { winRate: card.winRate, hasEnoughGames: card.hasEnoughGames });
          }
          setWinRates(map);
        })
        .catch(() => {});
      return () => {
        cancelled = true;
      };
    }
  }, [sort.key, winRates]);

  const pool = useMemo(
    () => almanacPool(ALMANAC_CATALOG, filter, sort, winRates ?? undefined),
    [filter, sort, winRates],
  );
  const detailDef = detailCardId === null ? undefined : ALMANAC_CATALOG.cards[detailCardId];

  const openDetail = useCallback((cardId: string) => {
    // At most one inspect overlay at a time, as in the deck editor.
    closeInspect();
    setDetailCardId(cardId);
  }, []);

  // R279: a reference in a card's text shows the card the catalog names.
  return (
    <CardDefsProvider defs={ALMANAC_CATALOG.cards}>
      <div className="app-shell app-shell--wide tavern deckbuilder almanac" data-testid={almanacTestid.screen}>
        <header className="db-header">
          <BackLink />
          <h1 className="db-title" id="almanac-title">
            Almanac
          </h1>
        </header>
        <main className="almanac-main" aria-labelledby="almanac-title">
          <CardBrowser
            catalog={ALMANAC_CATALOG}
            pool={pool}
            filter={filter}
            onFilter={setFilter}
            sort={sort}
            onSort={setSort}
            ownedControl={false}
            tags={ALMANAC_TAGS}
            onInspect={openDetail}
            detail={
              detailCardId === null || detailDef === undefined
                ? null
                : { cardId: detailCardId, meta: <span className="db-detail-meta">{almanacMeta(detailDef)}</span> }
            }
            // R654: the almanac's detail view carries the same compact statistics block as the
            // deck builder's, reading the public card aggregates.
            showStats
            onCloseDetail={() => {
              setDetailCardId(null);
            }}
          />
        </main>
      </div>
    </CardDefsProvider>
  );
}
