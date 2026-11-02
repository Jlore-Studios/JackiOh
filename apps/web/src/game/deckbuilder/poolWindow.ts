// Which rows of a long pool to mount (#2334). The Almanac's pool holds about 700 cards since #2145,
// too many to mount whatever staged the mount, so a virtual grid (`PoolGrid`'s `virtual`) mounts only
// the rows near what the player sees, and leaves the rest of its height to the grid's own rows, so the
// scrollbar and every card's place are those of the whole pool. `usePoolWindow.ts` reads these as the
// pool scrolls. Nothing here is a rule (CLAUDE.md rule 7): it is where cards are drawn, never which.

/**
 * How far the window reaches past the part of the pool in view, in heights of that part, on each
 * side: the rows a scroll brings into view are mounted, and their faces drawn, before they arrive.
 */
export const POOL_OVERSCAN = 2;

/**
 * The grid's selector for a card, which is as tall as a row: the card is sized by its width, and a
 * cell placed in a row of the grid's own is as tall as that row, whatever the card is.
 */
const CARD = ".db-card";

/** What the grid's layout says about its rows, in pixels. */
export type PoolShape = {
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

/**
 * The rows to mount for `count` cards (at least one) when `band` is in view. A view off the pool is
 * held at its nearest edge, so the first row, or the last, is always mounted: Tab from the filters,
 * or Shift+Tab from what follows the pool, lands on a card.
 */
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

export function sameWindow(before: PoolWindow, after: PoolWindow): boolean {
  return (
    before.columns === after.columns &&
    before.rowHeight === after.rowHeight &&
    before.firstRow === after.firstRow &&
    before.lastRow === after.lastRow
  );
}

/**
 * The grid's row tracks for `count` cards: every row the height of a card, so that a scroll, which
 * changes only which cards are mounted, leaves the tracks as they are and the grid relays out none
 * of the cards that stay (a track sized by what it holds had every card measured again).
 */
export function poolRowTracks(count: number, mounted: PoolWindow): string {
  return `repeat(${String(Math.ceil(count / mounted.columns))}, ${String(mounted.rowHeight)}px)`;
}

/**
 * The grid and what holds it that clip it: whatever of the pool they hide is not in view, so the
 * view is what they all leave, and the page's own edges (`readBand`).
 */
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
