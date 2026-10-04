// The browse pane: the filter bar, the pool grid in its frame, the empty state and a card's detail
// view. The deck editor (DeckEditor.tsx) and the Card Almanac (routes/almanac.tsx, R630) both render
// it, so the two screens share one browse UI and one look (deckbuilder.css).
//
// The pane decides nothing. The screen hands it the pool (`visiblePool` or `almanacPool`), the
// filter and the sort it holds, and the card whose detail is open. What only a deck has comes in
// through props: the "owned only" control, the grid's deck half (the "+", the drag, where each
// card already is), a status line under the filters, and the detail view's meta and actions.
// Without the grid's deck half the grid is read-only (PoolGrid.tsx).

import type { ReactElement, ReactNode } from "react";

import type { Tag } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardDetail } from "../../cards/index.ts";
import FilterBar from "./FilterBar.tsx";
import type { PoolFilter, PoolSort } from "./filters.ts";
import PoolGrid, { type PoolGridDeckProps } from "./PoolGrid.tsx";
import { DB_EMPTY } from "./testids.ts";

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
  /** The grid's deck half; absent, the grid is read-only. */
  deck?: PoolGridDeckProps;
  /** Under the filters, over the pool: the deck editor's status toast. */
  status?: ReactNode;
  /** A click, right-click or long-press on a pool card. */
  onInspect: (cardId: string) => void;
  /** The open detail view, or null when none is. */
  detail: BrowserDetail | null;
  onCloseDetail: () => void;
  /** SPEC §9.11, R654: render the compact card statistics block in the detail view (deckbuilder and almanac). */
  showStats?: boolean;
};

export default function CardBrowser(props: CardBrowserProps): ReactElement {
  const { catalog, pool, filter, onFilter, sort, onSort, ownedControl, ownedUnavailable, tags, showStats } = props;
  const { deck, status, onInspect, detail, onCloseDetail } = props;
  const detailDef = detail === null ? undefined : catalog.cards[detail.cardId];

  return (
    <section className="db-browse" aria-label="Browse cards">
      <FilterBar
        filter={filter}
        onFilter={onFilter}
        sort={sort}
        onSort={onSort}
        count={pool.length}
        ownedControl={ownedControl}
        ownedUnavailable={ownedUnavailable}
        tags={tags}
      />
      {status}
      {/* The frame is what the pool's cards are sized against on a desktop (deckbuilder.css): its
          height is whatever the filters above leave, and two rows of cards fill it. */}
      <div className="db-pool-frame">
        {deck === undefined ? (
          <PoolGrid ids={pool} catalog={catalog} onInspect={onInspect} />
        ) : (
          <PoolGrid ids={pool} catalog={catalog} onInspect={onInspect} {...deck} />
        )}
        {pool.length === 0 ? (
          <p className="db-empty" data-testid={DB_EMPTY}>
            No card matches these filters.
          </p>
        ) : null}
      </div>
      {/* A dialog in a portal on document.body, so where it sits in this tree changes nothing. */}
      {detail === null || detailDef === undefined ? null : (
        <CardDetail def={detailDef} onClose={onCloseDetail} meta={detail.meta} actions={detail.actions} showStats={showStats} />
      )}
    </section>
  );
}
