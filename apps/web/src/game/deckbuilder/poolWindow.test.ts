// #2334: which rows of a long pool a virtual grid mounts. The arithmetic is checked with numbers; the
// reads of the page, which a real layout answers, with the boxes a test gives them (the grid mounted
// in a browser is e2e's: 33-almanac.cy.ts).

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  POOL_OVERSCAN,
  clippingBoxes,
  fitWindow,
  poolRowTracks,
  poolWindow,
  readBand,
  readShape,
  sameWindow,
  type PoolShape,
} from "./poolWindow.ts";

const SHAPE: PoolShape = { columns: 4, rowHeight: 100, rowGap: 10, paddingTop: 10 };
const PITCH = SHAPE.rowHeight + SHAPE.rowGap;
/** 100 cards fill 25 rows. */
const COUNT = 100;
const LAST_ROW = 24;

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe("#2334 the rows a virtual pool mounts", () => {
  it("mounts the rows in view and the overscan around them, and no others", () => {
    const band = { top: 1000, bottom: 1300 };
    const reach = (band.bottom - band.top) * POOL_OVERSCAN;
    const mounted = poolWindow(COUNT, SHAPE, band);

    expect(mounted.firstRow).toBe(Math.floor((band.top - reach) / PITCH));
    expect(mounted.lastRow).toBe(Math.floor((band.bottom + reach) / PITCH));
    expect(mounted.firstRow).toBeLessThanOrEqual(Math.floor(band.top / PITCH));
    expect(mounted.lastRow).toBeGreaterThanOrEqual(Math.floor(band.bottom / PITCH));
    expect(mounted.lastRow - mounted.firstRow + 1).toBeLessThan(LAST_ROW);
    expect(mounted).toMatchObject({ columns: SHAPE.columns, rowHeight: SHAPE.rowHeight });
  });

  it("mounts the first row for a view above the pool, and the last for one below it", () => {
    const above = poolWindow(COUNT, SHAPE, { top: -5000, bottom: -4500 });
    expect(above.firstRow).toBe(0);
    expect(above.lastRow).toBeGreaterThanOrEqual(0);
    expect(above.lastRow).toBeLessThan(LAST_ROW);

    const below = poolWindow(COUNT, SHAPE, { top: 90_000, bottom: 90_500 });
    expect(below.lastRow).toBe(LAST_ROW);
    expect(below.firstRow).toBeGreaterThan(0);
  });

  it("keeps every row of a pool that the view and its overscan cover, and of one that fills a single row", () => {
    expect(poolWindow(COUNT, SHAPE, { top: 0, bottom: 100_000 })).toMatchObject({ firstRow: 0, lastRow: LAST_ROW });
    expect(poolWindow(3, SHAPE, { top: 0, bottom: 300 })).toMatchObject({ firstRow: 0, lastRow: 0 });
    // A view with no height still lands on a row.
    expect(poolWindow(COUNT, SHAPE, { top: 500, bottom: 500 }).firstRow).toBe(Math.floor(500 / PITCH));
  });

  it("holds a window to the rows a smaller pool fills", () => {
    const wide = poolWindow(COUNT, SHAPE, { top: 2000, bottom: 2300 });
    expect(fitWindow(wide, COUNT)).toEqual(wide);
    // 10 cards fill 3 rows: a window made for 100 is held to them.
    expect(fitWindow(wide, 10)).toMatchObject({ firstRow: 2, lastRow: 2 });
    expect(fitWindow(wide, 0)).toMatchObject({ firstRow: 0, lastRow: 0 });
  });

  it("gives every row the height of a card, so a scroll leaves the tracks as they are", () => {
    const a = poolWindow(COUNT, SHAPE, { top: 0, bottom: 300 });
    const b = poolWindow(COUNT, SHAPE, { top: 1500, bottom: 1800 });
    expect(poolRowTracks(COUNT, a)).toBe("repeat(25, 100px)");
    expect(poolRowTracks(COUNT, b)).toBe(poolRowTracks(COUNT, a));
    expect(poolRowTracks(101, a)).toBe("repeat(26, 100px)");
  });

  it("tells one window from another by its rows and its columns", () => {
    const a = poolWindow(COUNT, SHAPE, { top: 0, bottom: 300 });
    expect(sameWindow(a, { ...a })).toBe(true);
    expect(sameWindow(a, { ...a, lastRow: a.lastRow + 1 })).toBe(false);
    expect(sameWindow(a, { ...a, columns: 5 })).toBe(false);
  });
});

/** A box with the geometry of a laid out one, which jsdom does not have. */
function lay(
  element: HTMLElement,
  box: { top: number; clientTop?: number; clientHeight: number; scrollTop?: number },
): void {
  vi.spyOn(element, "getBoundingClientRect").mockReturnValue({ top: box.top } as DOMRect);
  Object.defineProperty(element, "clientTop", { value: box.clientTop ?? 0, configurable: true });
  Object.defineProperty(element, "clientHeight", { value: box.clientHeight, configurable: true });
  Object.defineProperty(element, "scrollTop", { value: box.scrollTop ?? 0, configurable: true });
}

describe("#2334 the reads of the page that place the view", () => {
  it("reads no shape where the page has no layout", () => {
    const grid = document.createElement("section");
    grid.innerHTML = '<div class="db-item"><button class="db-card"></button></div>';
    document.body.append(grid);
    expect(readShape(grid)).toBeNull();
  });

  it("reads the boxes that clip the pool, and not the page's own", () => {
    const region = document.createElement("main");
    region.style.overflowY = "auto";
    const frame = document.createElement("div");
    const grid = document.createElement("section");
    grid.style.overflowY = "auto";
    frame.append(grid);
    region.append(frame);
    document.body.append(region);
    expect(clippingBoxes(grid)).toEqual([grid, region]);

    grid.style.overflowY = "visible";
    region.style.overflowY = "visible";
    expect(clippingBoxes(grid)).toEqual([]);
  });

  it("reads the view from a grid that scrolls itself", () => {
    const grid = document.createElement("section");
    document.body.append(grid);
    vi.stubGlobal("innerHeight", 768);
    // The grid's box starts 100 px down, with a 1 px border, 10 px of padding and 500 px scrolled.
    lay(grid, { top: 100, clientTop: 1, clientHeight: 400, scrollTop: 500 });
    // The first row is at 100 + 1 + 10 - 500 = -389 on the page, and the box shows 101 to 501.
    expect(readBand(grid, [grid], 10)).toEqual({ top: 101 + 389, bottom: 501 + 389 });
  });

  it("reads the view from a grid that the page's region scrolls, and cuts it at the page's foot", () => {
    const region = document.createElement("main");
    const grid = document.createElement("section");
    region.append(grid);
    document.body.append(region);
    vi.stubGlobal("innerHeight", 600);
    lay(region, { top: 50, clientHeight: 800 });
    // The region shows 50 to 850, the page 0 to 600: the grid, which clips nothing itself, starts at 300.
    lay(grid, { top: 300, clientHeight: 5000 });
    expect(readBand(grid, [region], 0)).toEqual({ top: 50 - 300, bottom: 600 - 300 });
  });

  it("reads an empty view, not a negative one, from a pool scrolled out of its region", () => {
    const grid = document.createElement("section");
    document.body.append(grid);
    vi.stubGlobal("innerHeight", 600);
    lay(grid, { top: 700, clientHeight: 300 });
    // Its box is below the page: the view is nothing, at the box's top.
    const band = readBand(grid, [grid], 0);
    expect(band.bottom - band.top).toBe(0);
  });
});
