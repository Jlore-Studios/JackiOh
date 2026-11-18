// `/almanac`: the public Card Almanac (R630), every card tokens included, without signing in: the
// deck builder's browse pane (game/deckbuilder/CardBrowser.tsx) with nothing that edits a deck. The
// catalog is public (§5.1) and ships in the bundle, so the page asks no server or account for it.
// R1956000: signed in to an active account, it also reads the collection and the session's cosmetics
// ledger (almanacOwnership.ts) and draws an owned card as the viewer's copy; signed out, none of it.
// The page decides nothing (CLAUDE.md rule 7): filters.ts filters and sorts, the cards module draws;
// almanac.css holds only what the browse pane needs without the deck editor's grid around it.
// R3838000: its "Show card text" button prints every face's words, no full art (cardText.tsx); R3838001: every card
// with words opens a hover preview with its whole text beside it, and R3838002 a reference's tooltip does too
// (`AlwaysFullText`).

import { useCallback, useMemo, useState, type ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { AlwaysFullText, CardDefsProvider, closeInspect } from "../cards/index.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
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
import { CardTextButton, CardTextFaces, useCardText } from "./cardText.tsx";
import { BackLink } from "./nav.tsx";
import { PAGE_TITLE_ID } from "./PageTitle.tsx";

import "../auth/tavern.css";
import "../game/deckbuilder/deckbuilder.css";
import "./almanac.css";

export const almanacTestid = {
  /** The page's root. */
  screen: "almanac",
} as const;

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
  // #4180: the module's catalog (`game/currentCatalog.ts`); the page sits behind `EngineGate`.
  const catalog = currentCatalog();
  const detailCardId = linked !== null && Object.hasOwn(catalog.cards, linked) ? linked : null;
  const winRates = useCardWinRates(sort.key);
  // R1956000: an active account's own cards, as the gated screens' ownership has them (main.tsx).
  const account = useAccount();
  const own = useAlmanacOwnCopies(account.kind === "ready" && account.me.profile.status === "active" ? account.token : null);
  const cardText = useCardText();

  const pool = useMemo(
    () => almanacPool(catalog, filter, sort, winRates.rates ?? undefined),
    [catalog, filter, sort, winRates.rates, locale],
  );
  const detailDef = detailCardId === null ? undefined : catalog.cards[detailCardId];

  const openDetail = useCallback((cardId: string) => {
    // At most one inspect overlay at a time, as in the deck editor.
    closeInspect();
    navigate(paths.almanacCard(cardId), { replace: true });
  }, []);

  // R279: a reference in a card's text shows the card the catalog names.
  return (
    <CardDefsProvider defs={catalog.cards}>
      <CardTextFaces shown={cardText.shown}>
        <AlwaysFullText>
          <div className="app-shell app-shell--wide tavern deckbuilder almanac" data-testid={almanacTestid.screen}>
            <header className="db-header">
              <BackLink title={t("Almanac")}>
                <CardTextButton shown={cardText.shown} onToggle={cardText.toggle} />
              </BackLink>
            </header>
            <main className="almanac-main" aria-labelledby={PAGE_TITLE_ID}>
              <CardBrowser
                catalog={catalog}
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
        </AlwaysFullText>
      </CardTextFaces>
    </CardDefsProvider>
  );
}
