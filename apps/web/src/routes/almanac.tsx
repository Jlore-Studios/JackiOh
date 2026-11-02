// `/almanac`: the public Card Almanac (R630), every card tokens included, without signing in: the
// deck builder's browse pane (game/deckbuilder/CardBrowser.tsx) with nothing that edits a deck. The
// catalog is public (§5.1) and ships in the bundle, so the page asks no server or account for it.
// R1956000: signed in to an active account, it also reads the collection and the session's cosmetics
// ledger (almanacOwnership.ts) and draws an owned card as the viewer's copy; signed out, none of it.
// The page decides nothing (CLAUDE.md rule 7): filters.ts filters and sorts, the cards module draws;
// almanac.css holds only what the browse pane needs without the deck editor's grid around it.

import { useCallback, useMemo, useState, type ReactElement } from "react";

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDef, CardDefs } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardDefsProvider, closeInspect } from "../cards/index.ts";
import CardBrowser from "../game/deckbuilder/CardBrowser.tsx";
import {
  ALMANAC_CHIP_TAGS,
  DEFAULT_FILTER,
  DEFAULT_SORT,
  almanacPool,
  type PoolFilter,
  type PoolSort,
} from "../game/deckbuilder/filters.ts";
import { useCardWinRates } from "../game/deckbuilder/useCardWinRates.ts";
import { useLocale, t } from "../i18n/i18n.ts";
import { useAccount } from "../net/gate.ts";
import { navigate, paths, useSearch } from "../net/navigate.ts";
import { useAlmanacOwnCopies } from "./almanacOwnership.ts";
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
  // #711: wrapped where it is built (called per render), the rarity through the dictionary's rarity words.
  if (!def.token && !def.tags.includes("Token")) return t(def.rarity);
  const printed = def.printedRarity === undefined ? [] : [t(def.printedRarity)];
  return [...printed, t("Token"), t("not deckable")].join(" · ");
}

export default function AlmanacRoute(): ReactElement {
  const locale = useLocale();
  const [filter, setFilter] = useState<PoolFilter>(DEFAULT_FILTER);
  const [sort, setSort] = useState<PoolSort>(DEFAULT_SORT);
  // `?card=<id>` opens that card's detail (`paths.almanacCard`) and the URL follows the detail, so a
  // card's page can be shared; it follows the address on every move (R2118000: the command menu opens
  // a card there).
  const linked = new URLSearchParams(useSearch()).get("card");
  const detailCardId = linked !== null && Object.hasOwn(ALMANAC_CATALOG.cards, linked) ? linked : null;
  const winRates = useCardWinRates(sort.key);
  // R1956000: an active account's own cards, as the gated screens' ownership has them (main.tsx).
  const account = useAccount();
  const own = useAlmanacOwnCopies(account.kind === "ready" && account.me.profile.status === "active" ? account.token : null);

  const pool = useMemo(
    () => almanacPool(ALMANAC_CATALOG, filter, sort, winRates.rates ?? undefined),
    [filter, sort, winRates.rates, locale],
  );
  const detailDef = detailCardId === null ? undefined : ALMANAC_CATALOG.cards[detailCardId];

  const openDetail = useCallback((cardId: string) => {
    // At most one inspect overlay at a time, as in the deck editor.
    closeInspect();
    navigate(paths.almanacCard(cardId), { replace: true });
  }, []);

  // R279: a reference in a card's text shows the card the catalog names.
  return (
    <CardDefsProvider defs={ALMANAC_CATALOG.cards}>
      <div className="app-shell app-shell--wide tavern deckbuilder almanac" data-testid={almanacTestid.screen}>
        <header className="db-header">
          <BackLink />
          <h1 className="db-title" id="almanac-title">
            {t("Almanac")}
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
            winRates={winRates}
            ownedControl={false}
            tags={ALMANAC_CHIP_TAGS}
            onInspect={openDetail}
            own={own ?? undefined}
            // #2334: about 700 cards, of which a screen shows a dozen.
            virtual
            detail={
              detailCardId === null || detailDef === undefined
                ? null
                : { cardId: detailCardId, meta: <span className="db-detail-meta">{almanacMeta(detailDef)}</span> }
            }
            // R654: the deck builder's compact statistics block, from the public card aggregates.
            showStats
            onCloseDetail={() => {
              navigate(paths.almanac, { replace: true });
            }}
          />
        </main>
      </div>
    </CardDefsProvider>
  );
}
