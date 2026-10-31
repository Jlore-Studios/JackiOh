// The browse pane: the filter bar, the pool grid in its frame, the empty state and a card's detail
// view. The deck editor (DeckEditor.tsx) and the Card Almanac (routes/almanac.tsx, R630) both render
// it, so the two screens share one browse UI and one look (deckbuilder.css). The pane decides
// nothing: the screen hands it the pool, the filter and the sort it holds, and the card whose detail
// is open. R1956000: the cards the screen names as the viewer's own (`own`) are drawn as their own
// copies, in the grid and in the detail view.

import type { ReactElement, ReactNode } from "react";

import type { SetName, Tag } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardDetail } from "../../cards/index.ts";
import { t } from "../../i18n/i18n.ts";
import FilterBar from "./FilterBar.tsx";
import { CUSTOM_SET } from "./loadout.ts";
import type { PoolFilter, PoolSort } from "./filters.ts";
import PoolGrid, { AsOwnCopy, type OwnCopies, type PoolGridDeckProps } from "./PoolGrid.tsx";
import { DB_EMPTY, DB_WINRATE_NOTICE, DB_WINRATE_RETRY } from "./testids.ts";
import type { CardWinRates } from "./useCardWinRates.ts";

/** The card whose detail view is open, and what the screen puts in it. */
export type BrowserDetail = {
  cardId: string;
  meta?: ReactNode;
  actions?: ReactNode;
};

export type CardBrowserProps = {
  catalog: CatalogSnapshot;
  /** The visible pool, already filtered and sorted. */
  pool: readonly string[];
  filter: PoolFilter;
  onFilter: (next: PoolFilter) => void;
  sort: PoolSort;
  onSort: (next: PoolSort) => void;
  /** FilterBar's: the "owned only" control is shown unless this is false. */
  ownedControl?: boolean;
  /** FilterBar's: true when the collection could not be read. */
  ownedUnavailable?: boolean;
  /** FilterBar's: the tag chips, the deck builder's when absent. */
  tags?: readonly Tag[];
  /** FilterBar's: the set chips, and what follows them (the deck builder's "Custom" chip and creator link). */
  sets?: readonly SetName[];
  setsAfter?: ReactNode;
  /** The grid's deck half; absent, the grid is read-only. */
  deck?: PoolGridDeckProps;
  /** R1956000: a read-only grid's cards that are the viewer's own copies, and what they own on them. */
  own?: OwnCopies;
  /** PoolGrid's `virtual`: only the rows of cards near the view are mounted (#2334, the Almanac's). */
  virtual?: boolean;
  /** Under the filters, over the pool: the deck editor's status toast. */
  status?: ReactNode;
  /** A click, right-click or long-press on a pool card. */
  onInspect: (cardId: string) => void;
  detail: BrowserDetail | null;
  onCloseDetail: () => void;
  /** SPEC §9.11, R654: render the compact card statistics block in the detail view. */
  showStats?: boolean;
  /** The screen's `useCardWinRates`: while the "Win %" sort's rates load, or after they failed, a
   *  notice over the pool says the cards are in cost order (a retry after a failure). */
  winRates?: CardWinRates;
};

export default function CardBrowser(props: CardBrowserProps): ReactElement {
  const { catalog, pool, filter, onFilter, sort, onSort, ownedControl, ownedUnavailable, tags, sets, setsAfter, showStats } = props;
  const { deck, own, virtual, status, onInspect, detail, onCloseDetail, winRates } = props;
  const detailDef = detail === null ? undefined : catalog.cards[detail.cardId];

  return (
    <section className="db-browse" aria-label={t("Browse cards")}>
      <FilterBar
        filter={filter}
        onFilter={onFilter}
        sort={sort}
        onSort={onSort}
        count={pool.length}
        ownedControl={ownedControl}
        ownedUnavailable={ownedUnavailable}
        tags={tags}
        sets={sets}
        setsAfter={setsAfter}
      />
      {status}
      {sort.key === "winRate" && winRates !== undefined && (winRates.status === "loading" || winRates.status === "failed") ? (
        <div
          className="notice"
          data-testid={DB_WINRATE_NOTICE}
          data-state={winRates.status}
          role={winRates.status === "failed" ? "alert" : "status"}
        >
          {winRates.status === "failed"
            ? t("Win rates couldn't be loaded, so the cards are in cost order.")
            : t("Loading win rates… Until they arrive, the cards are in cost order.")}
          {winRates.status === "failed" ? (
            <>
              {" "}
              <button type="button" className="link-button" data-testid={DB_WINRATE_RETRY} onClick={winRates.retry}>
                {t("Retry")}
              </button>
            </>
          ) : null}
        </div>
      ) : null}
      {/* The frame is what the pool's cards are sized against on a desktop (deckbuilder.css): its
          height is whatever the filters above leave, and two rows of cards fill it. */}
      <div className="db-pool-frame">
        {deck === undefined ? (
          <PoolGrid ids={pool} catalog={catalog} onInspect={onInspect} own={own} virtual={virtual} />
        ) : (
          <PoolGrid ids={pool} catalog={catalog} onInspect={onInspect} virtual={virtual} {...deck} />
        )}
        {pool.length === 0 ? (
          <p className="db-empty" data-testid={DB_EMPTY}>
            {t("No card matches these filters.")}
          </p>
        ) : null}
      </div>
      {/* A dialog in a portal on document.body, so where it sits in this tree changes nothing. */}
      {detail === null || detailDef === undefined ? null : (
        <AsOwnCopy own={own} cardId={detail.cardId}>
          <CardDetail
            def={detailDef}
            onClose={onCloseDetail}
            meta={detail.meta}
            actions={detail.actions}
            // R915005: a custom card is private and plays in no game the statistics count.
            showStats={showStats === true && detailDef.set !== CUSTOM_SET}
          />
        </AsOwnCopy>
      )}
    </section>
  );
}
