// Issue #3734 (R3734000): the Cards tab's grid of the whole catalog, virtual as the deck builder's pool is (#2334): it
// mounts only the rows near the view and keeps the rest's height in its row tracks, so about 480 faces never
// draw at once. The arithmetic and the reads of the page are the pool's (`game/deckbuilder/poolWindow.ts`,
// `usePoolWindow.ts`); this draws one `li` for each mounted card, in its own cell. A page with no layout (a test's
// jsdom) draws every card. The card that has focus stays mounted where the grid scrolls away from it, so a page
// down on the keyboard never drops the focus to the page. What a card is, and what it does, is `render`'s.

import { useRef, useState, type ReactElement } from "react";

import { POOL_FIRST_CARDS } from "../game/deckbuilder/PoolGrid.tsx";
import { mountedAt, poolCell, poolRowTracks } from "../game/deckbuilder/poolWindow.ts";
import { usePoolWindow } from "../game/deckbuilder/usePoolWindow.ts";
import { collectionTestid } from "./collection.ts";

/** The selector of one card of the grid: its height is a row's (`readShape`). */
const CARD = ".collection-card";

export default function CardGrid({
  ids,
  render,
}: {
  /** Every card of the grid, in the order it shows them. */
  ids: readonly string[];
  /** One card, as its `li` holds it. */
  render: (id: string) => ReactElement;
}): ReactElement {
  const list = useRef<HTMLUListElement>(null);
  const view = usePoolWindow(list, ids.length, CARD);
  const [held, setHeld] = useState<string | null>(null);

  const rows = view === null || view === "all" ? null : view;
  const mounted = mountedAt(view, ids.length, held === null ? -1 : ids.indexOf(held), POOL_FIRST_CARDS);

  return (
    <ul
      ref={list}
      className={rows === null ? "collection-grid" : "collection-grid collection-grid--virtual"}
      data-testid={collectionTestid.grid}
      style={rows === null ? undefined : { gridTemplateRows: poolRowTracks(ids.length, rows) }}
      onFocus={(event) => {
        const cell = event.target instanceof Element ? event.target.closest<HTMLElement>("li[data-card]") : null;
        setHeld(cell?.dataset.card ?? null);
      }}
      onBlur={(event) => {
        // Focus moving to another card of the grid is the next card's focus to say.
        if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setHeld(null);
      }}
    >
      {mounted.map((at) => {
        const id = ids[at];
        if (id === undefined) return null;
        return (
          <li
            key={id}
            className={rows === null ? "collection-cell" : "collection-cell collection-cell--placed"}
            data-card={id}
            style={rows === null ? undefined : { gridArea: poolCell(at, rows.columns) }}
          >
            {render(id)}
          </li>
        );
      })}
    </ul>
  );
}
