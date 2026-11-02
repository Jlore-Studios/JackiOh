// #2334: a virtual grid (the Almanac's). A browser lays the grid out; here the page's layout is the
// shape and the view this file gives poolWindow.ts's reads, so the rows it mounts follow a view that
// a test moves. What a real layout says of them is e2e's (33-almanac.cy.ts). The staged grid's
// mount is PoolGrid.render.test.tsx's.

import type { CatalogSnapshot } from "@jackioh/validator";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import PoolGrid from "./PoolGrid.tsx";
import { poolWindow, type PoolBand, type PoolShape } from "./poolWindow.ts";
import { CARD_POOL, poolCardId } from "./testids.ts";

/** What the page says of the grid's layout: null where it has none. */
const layout = vi.hoisted(() => ({
  shape: null as PoolShape | null,
  band: { top: 0, bottom: 0 } as PoolBand,
}));

vi.mock("./poolWindow.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./poolWindow.ts")>()),
  clippingBoxes: () => [],
  readShape: () => layout.shape,
  readBand: () => layout.band,
}));

const CATALOG_HERE: CatalogSnapshot = { version: "test", cards: CATALOG };
const SHAPE: PoolShape = { columns: 4, rowHeight: 100, rowGap: 10, paddingTop: 10 };
/** 100 cards fill 25 rows of four. */
const IDS = Object.keys(CATALOG).slice(0, 100);
const TOP: PoolBand = { top: 0, bottom: 300 };

function noop(): void {}

function grid(ids: readonly string[]): ReactElement {
  return <PoolGrid ids={ids} catalog={CATALOG_HERE} onInspect={noop} virtual />;
}

function mounted(container: HTMLElement): string[] {
  return [...container.querySelectorAll<HTMLElement>(".db-card")].map((card) => card.dataset.card ?? "");
}

function rows(first: number, last: number): string[] {
  return IDS.slice(first * SHAPE.columns, (last + 1) * SHAPE.columns);
}

/** Scrolls the grid to show `band`, as the browser does: the grid's scroll event after the view moved. */
function scrollTo(container: HTMLElement, band: PoolBand): void {
  layout.band = band;
  fireEvent.scroll(container.querySelector(`[data-testid="${CARD_POOL}"]`) ?? container);
}

beforeEach(() => {
  layout.shape = SHAPE;
  layout.band = TOP;
});

afterEach(() => {
  cleanup();
});

describe("#2334 a virtual grid mounts the rows near the view", () => {
  it("mounts only the rows the view and its overscan reach, each in its own cell, under tracks that hold every row", () => {
    const { container, getByTestId, queryByTestId } = render(grid(IDS));
    const reach = poolWindow(IDS.length, SHAPE, TOP);

    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
    expect(mounted(container).length).toBeLessThan(IDS.length);
    expect(queryByTestId(poolCardId(IDS[IDS.length - 1] ?? ""))).toBeNull();

    // The first row's cards have the first row's cells; the fifth card begins the second row.
    const pool = getByTestId(CARD_POOL);
    expect(pool).toHaveClass("db-pool--virtual");
    expect(pool.style.gridTemplateRows).toBe("repeat(25, 100px)");
    const cells = [...pool.querySelectorAll<HTMLElement>(".db-item")].map((item) => item.style.gridArea);
    expect(cells.slice(0, 5)).toEqual(["1 / 1 / 2 / 2", "1 / 2 / 2 / 3", "1 / 3 / 2 / 4", "1 / 4 / 2 / 5", "2 / 1 / 3 / 2"]);
    for (const item of pool.querySelectorAll(".db-item")) expect(item).toHaveClass("db-item--placed");
  });

  it("mounts the rows a scroll brings near and drops those it leaves, and leaves the tracks as they are", () => {
    const { container, getByTestId } = render(grid(IDS));
    const tracks = getByTestId(CARD_POOL).style.gridTemplateRows;

    const band = { top: 1100, bottom: 1400 };
    act(() => {
      scrollTo(container, band);
    });
    const reach = poolWindow(IDS.length, SHAPE, band);
    expect(reach.firstRow).toBeGreaterThan(0);
    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
    expect(getByTestId(CARD_POOL).style.gridTemplateRows).toBe(tracks);

    // The cards in view are among them; the first card, far above, is not.
    const inView = rows(Math.floor(band.top / 110), Math.floor(band.bottom / 110));
    for (const id of inView) expect(getByTestId(poolCardId(id))).toBeTruthy();
    expect(container.querySelector(`[data-card="${IDS[0] ?? ""}"]`)).toBeNull();

    act(() => {
      scrollTo(container, TOP);
    });
    expect(container.querySelector(`[data-card="${IDS[0] ?? ""}"]`)).not.toBeNull();
  });

  it("mounts the last row for a view past the pool's end, and the first for one above its start", () => {
    const { container } = render(grid(IDS));
    act(() => {
      scrollTo(container, { top: 90_000, bottom: 90_500 });
    });
    expect(mounted(container)).toContain(IDS[IDS.length - 1]);
    act(() => {
      scrollTo(container, { top: -9000, bottom: -8500 });
    });
    expect(mounted(container)).toContain(IDS[0]);
  });

  it("keeps the card that has focus mounted, in its own cell, when the pool scrolls away from it, and drops it when the focus leaves", () => {
    const { container, getByTestId, queryByTestId } = render(grid(IDS));
    const first = getByTestId(poolCardId(IDS[0] ?? ""));
    act(() => {
      first.focus();
    });
    expect(first).toHaveFocus();

    act(() => {
      scrollTo(container, { top: 6000, bottom: 6300 });
    });
    const kept = getByTestId(poolCardId(IDS[0] ?? ""));
    expect(kept).toBe(first);
    expect(kept).toHaveFocus();
    expect(kept.closest<HTMLElement>(".db-item")?.style.gridArea).toBe("1 / 1 / 2 / 2");
    // In the grid's order: before the cards that came near.
    expect(mounted(container)[0]).toBe(IDS[0]);
    expect(mounted(container).length).toBeGreaterThan(1);

    act(() => {
      first.blur();
    });
    expect(queryByTestId(poolCardId(IDS[0] ?? ""))).toBeNull();
  });

  it("keeps a card's focus when it moves to the card's own button", () => {
    const { container, getByTestId } = render(
      <PoolGrid
        ids={IDS}
        catalog={CATALOG_HERE}
        onInspect={noop}
        virtual
        deckName="Deck"
        collection={null}
        inDeck={new Set()}
        holders={new Map()}
        refusedCardId={null}
        onAdd={noop}
        onDragStart={noop}
        onDragEnd={noop}
      />,
    );
    const id = IDS[0] ?? "";
    act(() => {
      getByTestId(poolCardId(id)).focus();
    });
    act(() => {
      scrollTo(container, { top: 6000, bottom: 6300 });
    });
    const add = getByTestId(`db-add-${id}`);
    act(() => {
      add.focus();
    });
    expect(add).toHaveFocus();
    expect(add.isConnected).toBe(true);
  });

  it("draws every card where the page has no layout to read, as the jsdom of every other test has none", () => {
    layout.shape = null;
    const { container } = render(grid(IDS));
    expect(mounted(container)).toEqual(IDS);
    expect(container.querySelector(".db-pool--virtual")).toBeNull();
  });

  it("holds its rows to a pool that shrinks, and mounts the rows of one that grows back", () => {
    const { container, rerender } = render(grid(IDS));
    const few = IDS.slice(0, 6);
    rerender(grid(few));
    expect(mounted(container)).toEqual(few);
    expect(container.querySelector<HTMLElement>(`[data-testid="${CARD_POOL}"]`)?.style.gridTemplateRows).toBe("repeat(2, 100px)");

    rerender(grid(IDS));
    const reach = poolWindow(IDS.length, SHAPE, TOP);
    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
  });
});
