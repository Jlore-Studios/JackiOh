// #2334: the Almanac's grid is virtual, so its first paint is the rows near the view and not the
// 700 cards of the pool: the pool is still the filters', counted whole, and a scroll mounts the cards
// it brings near. almanac.test.tsx has no layout, so it draws every card; here the layout is mocked.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_FILTER, DEFAULT_SORT, almanacPool } from "../game/deckbuilder/filters.ts";
import { poolWindow, type PoolBand, type PoolShape } from "../game/deckbuilder/poolWindow.ts";
import { CARD_POOL, DB_RESULT_COUNT, DB_SEARCH, poolCardId } from "../game/deckbuilder/testids.ts";
import { at } from "../test/stubServer.ts";
import AlmanacRoute, { ALMANAC_CATALOG } from "./almanac.tsx";

/** What the page says of the grid's layout. */
const layout = vi.hoisted(() => ({
  shape: { columns: 6, rowHeight: 250, rowGap: 10, paddingTop: 10 } as PoolShape,
  band: { top: 0, bottom: 520 } as PoolBand,
}));

vi.mock("../game/deckbuilder/poolWindow.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../game/deckbuilder/poolWindow.ts")>()),
  clippingBoxes: () => [],
  readShape: () => layout.shape,
  readBand: () => layout.band,
}));

const POOL = almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, DEFAULT_SORT);

/** Grids of 700 cards take a while to open under jsdom. */
const SLOW_GRID = { timeout: 30_000 } as const;

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
  layout.band = { top: 0, bottom: 520 };
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

/** The ids the grid has mounted, in its order. */
function mounted(): string[] {
  return [...screen.getByTestId(CARD_POOL).querySelectorAll<HTMLElement>(".db-card")].map((card) => card.dataset.card ?? "");
}

describe("#2334 the Almanac's grid mounts only the rows near the view", SLOW_GRID, () => {
  it("#2334 draws the rows in view and a result count of the whole pool", () => {
    render(<AlmanacRoute />);
    const rows = poolWindow(POOL.length, layout.shape, layout.band);

    expect(POOL.length).toBeGreaterThan(300);
    expect(mounted()).toEqual(POOL.slice(0, (rows.lastRow + 1) * layout.shape.columns));
    expect(mounted().length).toBeLessThan(POOL.length / 4);
    expect(screen.getByTestId(DB_RESULT_COUNT)).toHaveAttribute("data-count", String(POOL.length));
  });

  it("#2334 mounts the cards a scroll brings near, and a search draws only the cards it keeps", async () => {
    render(<AlmanacRoute />);
    const far = POOL[POOL.length - 1] ?? "";
    expect(screen.queryByTestId(poolCardId(far))).toBeNull();

    act(() => {
      layout.band = { top: 100_000, bottom: 100_520 };
      fireEvent.scroll(screen.getByTestId(CARD_POOL));
    });
    expect(screen.getByTestId(poolCardId(far))).toBeInTheDocument();
    expect(mounted()).not.toContain(POOL[0]);

    await userEvent.type(screen.getByTestId(DB_SEARCH), "bigot");
    const kept = almanacPool(ALMANAC_CATALOG, { ...DEFAULT_FILTER, search: "bigot" }, DEFAULT_SORT);
    expect(kept.length).toBeGreaterThan(0);
    expect(screen.getByTestId(DB_RESULT_COUNT)).toHaveAttribute("data-count", String(kept.length));
    expect(mounted()).toEqual(kept);
  });
});
