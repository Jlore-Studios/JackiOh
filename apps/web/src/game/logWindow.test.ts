// #4614: which lines of a long game log a virtual log mounts. The arithmetic is checked with
// numbers; the reads of the page, which a real layout answers, with the boxes a test gives
// them (a laid-out log is e2e's).

import { afterEach, describe, expect, it, vi } from "vitest";

import { LOG_OVERSCAN_ROWS, LOG_ROW_ESTIMATE_PX } from "./config.ts";
import {
  fitLogWindow,
  logSpacers,
  logWindow,
  readLogViewport,
  sameLogWindow,
} from "./logWindow.ts";

/** Every row the estimate tall: 100 lines fill 2400 px. */
const COUNT = 100;
const uniform = (_index: number): number => LOG_ROW_ESTIMATE_PX;
/** The viewport a phone's log popover shows: 400 px of lines. */
const VIEWPORT = 400;

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe("#4614 the rows a virtual log mounts", () => {
  it("mounts the rows in view and the overscan around them, and no others", () => {
    const top = 500;
    const mounted = logWindow(COUNT, uniform, top, VIEWPORT, LOG_OVERSCAN_ROWS);

    const firstVisible = Math.floor(top / LOG_ROW_ESTIMATE_PX);
    const lastVisible = Math.floor((top + VIEWPORT - 1) / LOG_ROW_ESTIMATE_PX);
    expect(mounted.first).toBe(firstVisible - LOG_OVERSCAN_ROWS);
    expect(mounted.last).toBe(lastVisible + LOG_OVERSCAN_ROWS);
    expect(mounted.first).toBeLessThanOrEqual(firstVisible);
    expect(mounted.last).toBeGreaterThanOrEqual(lastVisible);
    expect(mounted.last - mounted.first + 1).toBeLessThan(COUNT);
  });

  it("mounts the first rows for a view above the log, and the last for one below it", () => {
    const above = logWindow(COUNT, uniform, -5000, VIEWPORT, LOG_OVERSCAN_ROWS);
    expect(above).toEqual({ first: 0, last: LOG_OVERSCAN_ROWS });

    const below = logWindow(COUNT, uniform, 90_000, VIEWPORT, LOG_OVERSCAN_ROWS);
    expect(below).toEqual({ first: COUNT - 1 - LOG_OVERSCAN_ROWS, last: COUNT - 1 });
  });

  it("keeps every row of a log the view and its overscan cover", () => {
    expect(logWindow(COUNT, uniform, 0, 100_000, LOG_OVERSCAN_ROWS)).toEqual({ first: 0, last: COUNT - 1 });
  });

  it("holds a window to the lines a smaller log keeps", () => {
    const wide = logWindow(COUNT, uniform, 1000, VIEWPORT, LOG_OVERSCAN_ROWS);
    expect(fitLogWindow(wide, COUNT)).toEqual(wide);
    // 10 lines: a window made for 100 is held to them.
    expect(fitLogWindow(wide, 10)).toEqual({ first: 9, last: 9 });
    expect(fitLogWindow(wide, 0)).toEqual({ first: 0, last: -1 });
  });

  it("tells one window from another by its rows", () => {
    const mounted = logWindow(COUNT, uniform, 0, VIEWPORT, LOG_OVERSCAN_ROWS);
    expect(sameLogWindow(mounted, { ...mounted })).toBe(true);
    expect(sameLogWindow(mounted, { ...mounted, last: mounted.last + 1 })).toBe(false);
    expect(sameLogWindow(mounted, { ...mounted, first: mounted.first + 1 })).toBe(false);
  });

  it("tenfold lines keep the mounted count bounded", () => {
    const hundred = logWindow(100, uniform, 1200, VIEWPORT, LOG_OVERSCAN_ROWS);
    const thousand = logWindow(1000, uniform, 1200, VIEWPORT, LOG_OVERSCAN_ROWS);
    const hundredCount = hundred.last - hundred.first + 1;
    const thousandCount = thousand.last - thousand.first + 1;
    expect(hundredCount).toBeLessThanOrEqual(60);
    expect(thousandCount).toBe(hundredCount);
  });

  it("measures variable rows by the heights it is given, estimate then measured", () => {
    // Wrapped lines run tall, turn lines short; the estimate stands in until measured.
    const heights = new Map<number, number>([[3, 60]]);
    const heightAt = (index: number): number => heights.get(index) ?? LOG_ROW_ESTIMATE_PX;
    const before = logWindow(COUNT, uniform, 0, VIEWPORT, LOG_OVERSCAN_ROWS);
    const after = logWindow(COUNT, heightAt, 0, VIEWPORT, LOG_OVERSCAN_ROWS);
    expect(after.last).toBeLessThanOrEqual(before.last);
    expect(after.first).toBe(before.first);
  });
});

describe("#4614 the blank space the unmounted head and tail keep", () => {
  it("head and tail spacers sum to the whole height with the mounted rows", () => {
    const mounted = logWindow(COUNT, uniform, 500, VIEWPORT, LOG_OVERSCAN_ROWS);
    const { head, tail } = logSpacers(COUNT, uniform, mounted);
    let mountedHeight = 0;
    for (let index = mounted.first; index <= mounted.last; index += 1) mountedHeight += uniform(index);
    expect(head + mountedHeight + tail).toBe(COUNT * LOG_ROW_ESTIMATE_PX);
    expect(head).toBe(mounted.first * LOG_ROW_ESTIMATE_PX);
  });

  it("spacers follow measured heights, not the estimate", () => {
    const heightAt = (index: number): number => (index % 2 === 0 ? 20 : 40);
    const mounted = { first: 10, last: 19 };
    const { head, tail } = logSpacers(100, heightAt, mounted);
    expect(head).toBe(5 * 20 + 5 * 40);
    expect(tail).toBe(40 * 20 + 40 * 40);
  });

  it("an empty window keeps no space", () => {
    expect(logSpacers(0, uniform, { first: 0, last: -1 })).toEqual({ head: 0, tail: 0 });
  });
});

describe("#4614 the reads of the page that place the view", () => {
  it("reads no viewport where the page has no layout", () => {
    const list = document.createElement("ol");
    document.body.append(list);
    expect(readLogViewport(list)).toBeNull();
  });

  it("reads the scroll and the height where the page lays the list out", () => {
    const list = document.createElement("ol");
    document.body.append(list);
    Object.defineProperty(list, "clientHeight", { value: 400, configurable: true });
    Object.defineProperty(list, "scrollTop", { value: 120, configurable: true });
    expect(readLogViewport(list)).toEqual({ scrollTop: 120, clientHeight: 400 });
  });
});
