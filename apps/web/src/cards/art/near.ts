// Lazy art waits for ART_DWELL_MS inside ART_NEAR_MARGIN_PX to avoid off-screen SVG parsing.
// The nearest `overflow-y` scroller must be the observer root, or inner-scroller art appears far.

import { ART_DWELL_MS, ART_NEAR_MARGIN_PX } from "../constants.ts";
import { SKIPPABLE_ATTRIBUTE } from "../fit.ts";

type Watched = { onNear: () => void; dwell: ReturnType<typeof setTimeout> | null };
type Watch = { observer: IntersectionObserver; windows: Map<Element, Watched> };

const watches = new Map<Element | null, Watch>();
/** Cache ancestors for one mount burst; media queries can change the scroller. */
let scrolls = new WeakMap<Element, boolean>();
let forgetting = false;

function scrolling(element: Element): boolean {
  const known = scrolls.get(element);
  if (known !== undefined) return known;
  const overflow = getComputedStyle(element).overflowY;
  const answer = overflow === "auto" || overflow === "scroll";
  scrolls.set(element, answer);
  if (!forgetting) {
    forgetting = true;
    queueMicrotask(() => {
      scrolls = new WeakMap();
      forgetting = false;
    });
  }
  return answer;
}

/** Start above a skipped holder: reading a skipped descendant's style forces layout. */
function scrollParent(element: Element): Element | null {
  const start = element.closest(`[${SKIPPABLE_ATTRIBUTE}]`) ?? element;
  for (let parent = start.parentElement; parent !== null; parent = parent.parentElement) {
    if (scrolling(parent)) return parent;
  }
  return null;
}

export function canWatchArt(): boolean {
  return typeof IntersectionObserver !== "undefined";
}

/** Calls `onNear` once after ART_DWELL_MS; rewatching cancels the previous dwell. */
export function whenNear(element: Element, onNear: () => void): () => void {
  const root = scrollParent(element);
  let watch = watches.get(root);
  if (watch === undefined) {
    const windows = new Map<Element, Watched>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const watched = windows.get(entry.target);
          if (watched === undefined) continue;
          if (entry.isIntersecting) {
            // Preserve the first dwell timeout.
            watched.dwell ??= setTimeout(() => {
              leave(root, entry.target);
              watched.onNear();
            }, ART_DWELL_MS);
          } else if (watched.dwell !== null) {
            clearTimeout(watched.dwell);
            watched.dwell = null;
          }
        }
      },
      { root, rootMargin: `${String(ART_NEAR_MARGIN_PX)}px 0px` },
    );
    watch = { observer, windows };
    watches.set(root, watch);
  }
  // A replaced watch cannot keep its pending dwell.
  const previous = watch.windows.get(element);
  if (previous !== undefined && previous.dwell !== null) clearTimeout(previous.dwell);
  watch.windows.set(element, { onNear, dwell: null });
  watch.observer.observe(element);
  return () => {
    leave(root, element);
  };
}

function leave(root: Element | null, element: Element): void {
  const watch = watches.get(root);
  if (watch === undefined) return;
  const watched = watch.windows.get(element);
  if (watched !== undefined && watched.dwell !== null) clearTimeout(watched.dwell);
  watch.observer.unobserve(element);
  watch.windows.delete(element);
  if (watch.windows.size === 0) {
    watch.observer.disconnect();
    watches.delete(root);
  }
}
