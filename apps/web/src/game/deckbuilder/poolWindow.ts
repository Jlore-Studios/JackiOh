// Virtualized pool row mounting (#2145, #2334, CLAUDE.md rule 7).
// Calculates visible row window and grid tracks as pool scrolls without affecting rules.

/** Multiplier for mounting rows past visible band on each side. */
export const POOL_OVERSCAN = 2;

/**
 * The grid's selector for a card, which is as tall as a row: the card is sized by its width, and a
 * cell placed in a row of the grid's own is as tall as that row, whatever the card is.
 */
const CARD = ".db-card";

/** What the grid's layout says about its rows, in pixels. */
export type PoolShape = {
  /** Cards to a row. */
  columns: number;
  rowHeight: number;
  rowGap: number;
  /** Between the grid's border and its first row. */
  paddingTop: number;
};

/** The part of the pool in view, in pixels from the top of the first row (negative above it). */
export type PoolBand = { top: number; bottom: number };

/** The mounted rows, `firstRow` to `lastRow` inclusive, each `columns` cards wide. */
export type PoolWindow = { columns: number; rowHeight: number; firstRow: number; lastRow: number };

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}

/** Rows to mount for card count within visible band; clamps to edges for focus navigation. */
export function poolWindow(count: number, shape: PoolShape, band: PoolBand): PoolWindow {
  const { columns, rowHeight, rowGap } = shape;
  const rows = Math.ceil(count / columns);
  const pitch = rowHeight + rowGap;
  const end = rows * pitch - rowGap;
  const reach = Math.max(band.bottom - band.top, 0) * POOL_OVERSCAN;
  return {
    columns,
    rowHeight,
    firstRow: clamp(Math.floor((clamp(band.top, 0, end) - reach) / pitch), 0, rows - 1),
    lastRow: clamp(Math.floor((clamp(band.bottom, 0, end) + reach) / pitch), 0, rows - 1),
  };
}

/** `mounted` held to the rows `count` cards fill, for a pool that shrank since it was made. */
export function fitWindow(mounted: PoolWindow, count: number): PoolWindow {
  const lastRow = Math.max(Math.ceil(count / mounted.columns) - 1, 0);
  return { ...mounted, firstRow: Math.min(mounted.firstRow, lastRow), lastRow: Math.min(mounted.lastRow, lastRow) };
}

/** The same rows. */
export function sameWindow(before: PoolWindow, after: PoolWindow): boolean {
  return (
    before.columns === after.columns &&
    before.rowHeight === after.rowHeight &&
    before.firstRow === after.firstRow &&
    before.lastRow === after.lastRow
  );
}

/** Grid row track definitions sized by card height to stabilize layout during scroll. */
export function poolRowTracks(count: number, mounted: PoolWindow): string {
  return `repeat(${String(Math.ceil(count / mounted.columns))}, ${String(mounted.rowHeight)}px)`;
}

/** Clipped intersection between grid and containing viewport elements. */
export function clippingBoxes(grid: HTMLElement): HTMLElement[] {
  const boxes: HTMLElement[] = [];
  for (let box: HTMLElement | null = grid; box !== null && box !== document.body; box = box.parentElement) {
    if (getComputedStyle(box).overflowY !== "visible") boxes.push(box);
  }
  return boxes;
}

/** The grid's rows as it lays them out; null where it lays out none (no layout engine, no card yet). */
export function readShape(grid: HTMLElement): PoolShape | null {
  const style = getComputedStyle(grid);
  // The resolved tracks of the grid are lengths in pixels, one for each column it fits.
  const columns = style
    .getPropertyValue("grid-template-columns")
    .split(" ")
    .filter((track) => track.endsWith("px")).length;
  const card = grid.querySelector(CARD);
  if (columns === 0 || card === null) return null;
  const rowHeight = card.getBoundingClientRect().height;
  if (!(rowHeight > 0)) return null;
  return {
    columns,
    rowHeight,
    rowGap: Number.parseFloat(style.getPropertyValue("row-gap")) || 0,
    paddingTop: Number.parseFloat(style.getPropertyValue("padding-top")) || 0,
  };
}

/** The part of the grid that neither the page's edge nor a box that clips it hides. */
export function readBand(grid: HTMLElement, boxes: readonly HTMLElement[], paddingTop: number): PoolBand {
  // Where the first row starts on the page: under the grid's border and padding, scrolled by the grid if it is the scroller.
  const first = grid.getBoundingClientRect().top + grid.clientTop + paddingTop - grid.scrollTop;
  let top = 0;
  let bottom = window.innerHeight;
  for (const box of boxes) {
    const edge = box.getBoundingClientRect().top + box.clientTop;
    top = Math.max(top, edge);
    bottom = Math.min(bottom, edge + box.clientHeight);
  }
  return { top: top - first, bottom: Math.max(bottom, top) - first };
}
