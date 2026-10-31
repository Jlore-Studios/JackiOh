// The rows of a virtual pool grid to mount, kept as the pool scrolls and resizes (#2334); the
// arithmetic and the reads of the page are poolWindow.ts's. The grid's first render knows nothing of
// its layout, so it draws POOL_FIRST_CARDS (the hook says null) and the layout effect after it reads
// the page and settles the rows before the first paint. Where the page has no layout to read (a test's
// jsdom), nothing can be windowed and every card is drawn (the hook says "all"). The rows follow every scroll
// of the grid or of what holds it (one listener in the capture phase on the document, which sees them
// all), and every resize.

import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { flushSync } from "react-dom";

import {
  clippingBoxes,
  fitWindow,
  poolWindow,
  readBand,
  readShape,
  sameWindow,
  type PoolShape,
  type PoolWindow,
} from "./poolWindow.ts";

/** The rows to mount; "all" where the page has no layout, null before the first read. */
export type PoolView = PoolWindow | "all" | null;

function sameView(before: PoolView, after: PoolView): boolean {
  if (before === null || after === null || before === "all" || after === "all") return before === after;
  return sameWindow(before, after);
}

export function usePoolWindow(ref: RefObject<HTMLElement | null>, count: number): PoolView {
  const [view, setView] = useState<PoolView>(null);
  const counted = useRef(count);
  const shape = useRef<PoolShape | null>(null);
  const boxes = useRef<readonly HTMLElement[]>([]);

  // A scroll reads only where the view is, since the grid's shape only changes with its size.
  const measure = useCallback(
    (reshape: boolean): void => {
      const grid = ref.current;
      if (grid === null) return;
      if (reshape) {
        shape.current = readShape(grid);
        boxes.current = clippingBoxes(grid);
      }
      const read = shape.current;
      const next: PoolView =
        counted.current === 0
          ? null
          : read === null
            ? "all"
            : poolWindow(counted.current, read, readBand(grid, boxes.current, read.paddingTop));
      setView((now) => (sameView(now, next) ? now : next));
    },
    [ref],
  );

  useLayoutEffect(() => {
    counted.current = count;
    measure(true);
  }, [count, measure]);

  useLayoutEffect(() => {
    const grid = ref.current;
    if (grid === null) return undefined;
    const onScroll = (event: Event): void => {
      const target = event.target;
      if (target === document || (target instanceof Node && target.contains(grid))) measure(false);
    };
    // A resize changes the columns, so the rows are placed before the frame is drawn.
    const onResize = (): void => {
      flushSync(() => {
        measure(true);
      });
    };
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    window.addEventListener("resize", onResize);
    const observer = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(onResize);
    observer?.observe(grid);
    return () => {
      document.removeEventListener("scroll", onScroll, { capture: true });
      window.removeEventListener("resize", onResize);
      observer?.disconnect();
    };
  }, [ref, measure]);

  return view === null || view === "all" ? view : fitWindow(view, count);
}
