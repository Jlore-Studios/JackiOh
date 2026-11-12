// Issue #3734 (R3734000): the Collection's virtual grid. A browser lays the grid out; here the page's layout is the
// shape and the view this file gives poolWindow.ts's reads, as PoolGrid.virtual.test.tsx does for the pool, so the
// rows it mounts follow a view that a test moves. What a real layout says of them is e2e's (collection-screen.cy.tsx).

import { act, cleanup, fireEvent, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { poolWindow, type PoolBand, type PoolShape } from "../game/deckbuilder/poolWindow.ts";
import CardGrid from "./CardGrid.tsx";
import { collectionTestid } from "./collection.ts";

/** What the page says of the grid's layout: null where it has none. */
const layout = vi.hoisted(() => ({
  shape: null as PoolShape | null,
  band: { top: 0, bottom: 0 } as PoolBand,
}));

vi.mock("../game/deckbuilder/poolWindow.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../game/deckbuilder/poolWindow.ts")>()),
  clippingBoxes: () => [],
  readShape: () => layout.shape,
  readBand: () => layout.band,
}));

const SHAPE: PoolShape = { columns: 4, rowHeight: 100, rowGap: 10, paddingTop: 10 };
/** 300 cards fill 75 rows of four. */
const IDS = Array.from({ length: 300 }, (_, index) => `c-${String(index)}`);
const TOP: PoolBand = { top: 0, bottom: 300 };

function Card({ id }: { id: string }): ReactElement {
  return (
    <button type="button" className="collection-card" data-testid={`card-${id}`}>
      {id}
    </button>
  );
}

function grid(ids: readonly string[]): ReactElement {
  return <CardGrid ids={ids} render={(id) => <Card id={id} />} />;
}

function mounted(container: HTMLElement): string[] {
  return [...container.querySelectorAll<HTMLElement>("li[data-card]")].map((cell) => cell.dataset.card ?? "");
}

function rows(first: number, last: number): string[] {
  return IDS.slice(first * SHAPE.columns, (last + 1) * SHAPE.columns);
}

function scrollTo(container: HTMLElement, band: PoolBand): void {
  layout.band = band;
  fireEvent.scroll(container.querySelector(`[data-testid="${collectionTestid.grid}"]`) ?? container);
}

beforeEach(() => {
  layout.shape = SHAPE;
  layout.band = TOP;
});

afterEach(() => {
  cleanup();
});

describe("R3734000 the Collection's grid mounts the rows near the view", () => {
  it("R3734000 300 cards mount only the rows the view and its overscan reach, each in its own cell, under tracks that hold every row", () => {
    const { container, getByTestId, queryByTestId } = render(grid(IDS));
    const reach = poolWindow(IDS.length, SHAPE, TOP);

    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
    expect(mounted(container).length).toBeLessThan(IDS.length / 4);
    expect(queryByTestId(`card-${IDS[IDS.length - 1] ?? ""}`)).toBeNull();

    const list = getByTestId(collectionTestid.grid);
    expect(list).toHaveClass("collection-grid--virtual");
    expect(list.style.gridTemplateRows).toBe("repeat(75, 100px)");
    const cells = [...list.querySelectorAll<HTMLElement>("li")].map((cell) => cell.style.gridArea);
    expect(cells.slice(0, 5)).toEqual(["1 / 1 / 2 / 2", "1 / 2 / 2 / 3", "1 / 3 / 2 / 4", "1 / 4 / 2 / 5", "2 / 1 / 3 / 2"]);
    for (const cell of list.querySelectorAll("li")) expect(cell).toHaveClass("collection-cell--placed");
  });

  it("R3734000 a scroll mounts the rows it brings near and drops those it leaves, and leaves the tracks as they are", () => {
    const { container, getByTestId } = render(grid(IDS));
    const tracks = getByTestId(collectionTestid.grid).style.gridTemplateRows;

    const band = { top: 3000, bottom: 3300 };
    act(() => {
      scrollTo(container, band);
    });
    const reach = poolWindow(IDS.length, SHAPE, band);
    expect(reach.firstRow).toBeGreaterThan(0);
    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
    expect(getByTestId(collectionTestid.grid).style.gridTemplateRows).toBe(tracks);
    expect(container.querySelector(`[data-card="${IDS[0] ?? ""}"]`)).toBeNull();

    act(() => {
      scrollTo(container, TOP);
    });
    expect(container.querySelector(`[data-card="${IDS[0] ?? ""}"]`)).not.toBeNull();
  });

  it("R3734000 the card that has focus stays mounted, in its own cell, when the grid scrolls away from it, and is dropped when the focus leaves", () => {
    const { container, getByTestId, queryByTestId } = render(grid(IDS));
    const first = getByTestId(`card-${IDS[0] ?? ""}`);
    act(() => {
      first.focus();
    });
    act(() => {
      scrollTo(container, { top: 6000, bottom: 6300 });
    });
    const kept = getByTestId(`card-${IDS[0] ?? ""}`);
    expect(kept).toBe(first);
    expect(kept).toHaveFocus();
    expect(kept.closest<HTMLElement>("li")?.style.gridArea).toBe("1 / 1 / 2 / 2");
    expect(mounted(container)[0]).toBe(IDS[0]);

    act(() => {
      first.blur();
    });
    expect(queryByTestId(`card-${IDS[0] ?? ""}`)).toBeNull();
  });

  it("R3734000 it draws every card where the page has no layout to read, as the jsdom of every other test has none", () => {
    layout.shape = null;
    const { container, getByTestId } = render(grid(IDS));
    expect(mounted(container)).toEqual(IDS);
    expect(getByTestId(collectionTestid.grid)).not.toHaveClass("collection-grid--virtual");
  });

  it("R3734000 it holds its rows to a grid that shrinks, and mounts the rows of one that grows back", () => {
    const { container, rerender, getByTestId } = render(grid(IDS));
    const few = IDS.slice(0, 6);
    rerender(grid(few));
    expect(mounted(container)).toEqual(few);
    expect(getByTestId(collectionTestid.grid).style.gridTemplateRows).toBe("repeat(2, 100px)");

    rerender(grid(IDS));
    const reach = poolWindow(IDS.length, SHAPE, TOP);
    expect(mounted(container)).toEqual(rows(reach.firstRow, reach.lastRow));
  });
});
