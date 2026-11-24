// The lines of a long game log to mount, kept as the log scrolls and resizes (#4614); the
// arithmetic and the reads of the page are logWindow.ts's. The first render knows nothing of
// its layout, so it draws every line (the hook says null) and the layout effect after it reads
// the page and settles the window before the first paint. Where the page has no layout to read
// (a test's jsdom), nothing can be windowed and every line is drawn (the hook says "all").
// The rows follow every scroll of the list or of what holds it (one listener in the capture
// phase on the document, which sees them all), every resize, and each mounted row's own size:
// rows are measured as they mount, and an unmeasured row counts as LOG_ROW_ESTIMATE_PX until it is.

import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { flushSync } from "react-dom";

import { LOG_OVERSCAN_ROWS, LOG_ROW_ESTIMATE_PX } from "./config.ts";
import {
  fitLogWindow,
  logSpacers,
  logWindow,
  readLogViewport,
  sameLogWindow,
  type LogWindow,
} from "./logWindow.ts";

/**
 * The lines to mount; "all" where the page has no layout, null before the first read. A
 * window carries its spacer heights with it, so the list draws the blank space the
 * unmounted head and tail keep from the same measure that placed the window.
 */
export type LogMount = LogWindow & { head: number; tail: number };
export type LogView = LogMount | "all" | null;

function sameView(before: LogView, after: LogView): boolean {
  if (before === null || after === null || before === "all" || after === "all") return before === after;
  return sameLogWindow(before, after) && before.head === after.head && before.tail === after.tail;
}

/**
 * The measured height of each mounted row, by line key, dividers included: a divider is its
 * line's own `li`, so a row's place is the row's height plus its divider's.
 */
function readRowHeights(list: HTMLElement, known: ReadonlyMap<string, number>): Map<string, number> | null {
  const rows = list.querySelectorAll("li[data-log-key]");
  if (rows.length === 0) return null;
  const dividers = new Map<string, number>();
  for (const divider of list.querySelectorAll("li[data-log-divider]")) {
    if (!(divider instanceof HTMLElement)) continue;
    const key = divider.dataset.logDivider;
    if (key === undefined || key === "") continue;
    const height = divider.getBoundingClientRect().height;
    if (height > 0) dividers.set(key, height);
  }
  let changed = false;
  const next = new Map(known);
  for (const row of rows) {
    if (!(row instanceof HTMLElement)) continue;
    const key = row.dataset.logKey;
    if (key === undefined || key === "") continue;
    // A row with no laid-out height (a test's jsdom) keeps its estimate: recording zero
    // would collapse the window onto the log's end.
    const height = row.getBoundingClientRect().height + (dividers.get(key) ?? 0);
    if (height > 0 && next.get(key) !== height) {
      next.set(key, height);
      changed = true;
    }
  }
  return changed ? next : null;
}

/** `keys` are the whole log's line keys in order; the held line stays mounted while it has focus. */
export function useLogWindow(
  ref: RefObject<HTMLOListElement | null>,
  keys: readonly string[],
  heldKey: string | null,
): LogView {
  const [view, setView] = useState<LogView>(null);
  const [heights, setHeights] = useState<ReadonlyMap<string, number>>(() => new Map());
  const heightsRef = useRef(heights);
  const live = useRef({ keys, heldKey });
  live.current = { keys, heldKey };

  // A read touches only refs, so the listeners below keep one measure for the log's life.
  const measure = useCallback(
    (reshape: boolean): void => {
      const list = ref.current;
      if (list === null) return;
      if (reshape) {
        const fresh = readRowHeights(list, heightsRef.current);
        if (fresh !== null) {
          heightsRef.current = fresh;
          setHeights(fresh);
        }
      }
      const at = live.current.keys;
      const held = live.current.heldKey;
      const viewport = readLogViewport(list);
      if (viewport === null) {
        setView((now) => (now === "all" ? now : "all"));
        return;
      }
      const sized = heightsRef.current;
      const heightAt = (index: number): number => {
        const key = at[index];
        return key === undefined ? LOG_ROW_ESTIMATE_PX : (sized.get(key) ?? LOG_ROW_ESTIMATE_PX);
      };
      const found = logWindow(at.length, heightAt, viewport.scrollTop, viewport.clientHeight, LOG_OVERSCAN_ROWS);
      if (held !== null) {
        const heldAt = at.indexOf(held);
        if (heldAt >= 0) {
          found.first = Math.min(found.first, heldAt);
          found.last = Math.max(found.last, heldAt);
        }
      }
      const placed = fitLogWindow(found, at.length);
      const next: LogMount = { ...placed, ...logSpacers(at.length, heightAt, placed) };
      setView((now) => (sameView(now, next) ? now : next));
    },
    [ref],
  );

  // The log's lines: length, either end (an arrival, a new game), and the held line.
  const first = keys.length === 0 ? "" : (keys[0] ?? "");
  const last = keys.length === 0 ? "" : (keys[keys.length - 1] ?? "");
  const windowKey = view === null || view === "all" ? view : `${String(view.first)}:${String(view.last)}`;

  // Measuring and observing are separate effects: no layout effect here both reads the
  // window and writes it, so a measure that moves the window cannot re-trigger itself in
  // the same commit (#4614: the combined effect cascaded into React's maximum update depth
  // under sustained arrivals, red in e2e smoke).
  useLayoutEffect(() => {
    measure(true);
    // The keys stand in through their ends and length, so new rows measure as they arrive.
  }, [ref, measure, keys.length, first, last, heldKey]);

  useLayoutEffect(() => {
    if (typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(() => {
      measure(true);
    });
    const list = ref.current;
    if (list === null) return undefined;
    observer.observe(list);
    for (const row of list.querySelectorAll("li[data-log-key]")) observer.observe(row);
    return () => {
      observer.disconnect();
    };
    // The window through its range, so rows that mount as it moves are observed; the
    // observer's own callbacks re-measure asynchronously, never in this effect.
  }, [ref, measure, keys.length, first, last, heldKey, windowKey]);

  useLayoutEffect(() => {
    const list = ref.current;
    if (list === null) return undefined;
    const onScroll = (event: Event): void => {
      const target = event.target;
      if (target === document || (target instanceof Node && target.contains(list))) measure(false);
    };
    // A resize moves every wrap, so the rows are placed before the frame is drawn.
    const onResize = (): void => {
      flushSync(() => {
        measure(true);
      });
    };
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    window.addEventListener("resize", onResize);
    return () => {
      document.removeEventListener("scroll", onScroll, { capture: true });
      window.removeEventListener("resize", onResize);
    };
  }, [ref, measure]);

  // A log that shrank since it was measured holds its window to what it keeps until the
  // next measure; the spacers settle with it.
  if (view === null || view === "all") return view;
  const held = fitLogWindow(view, keys.length);
  return { ...held, head: view.head, tail: view.tail };
}
