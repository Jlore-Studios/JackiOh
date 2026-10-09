// Which art windows are near the screen (CardArt `lazy`).
//
// The browser parses a procedural picture's data-URI SVG when its window is first styled, about 1.3 ms
// a card. A lazy window draws nothing until it has stayed within ART_NEAR_MARGIN_PX of the box that
// scrolls it for ART_DWELL_MS, then draws once and keeps its picture. Until then it is its theme's
// sky as a flat gradient (art.css).
//
// One IntersectionObserver per scrolling box watches every window in it. The scrolling box is the
// nearest ancestor with `overflow-y: auto | scroll` (the viewport when there is none), because an
// observer's `rootMargin` widens its root and nothing else: a window clipped by an inner scroller
// would otherwise count as far until it was actually on screen.

import { ART_DWELL_MS, ART_NEAR_MARGIN_PX } from "../constants.ts";
import { SKIPPABLE_ATTRIBUTE } from "../fit.ts";

type Watched = { onNear: () => void; dwell: ReturnType<typeof setTimeout> | null };
type Watch = { observer: IntersectionObserver; windows: Map<Element, Watched> };

/** One watch per scrolling box (null: the viewport), dropped when its last window leaves. */
const watches = new Map<Element | null, Watch>();
/**
 * Whether an ancestor scrolls, asked once per burst of mounts (a grid's windows share ancestors);
 * the answers are dropped when the burst ends, because a media query can move the scrolling.
 */
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

/**
 * The walk starts above a skippable holder (fit.ts SKIPPABLE_ATTRIBUTE): nothing inside one scrolls,
 * and asking the style of an element the browser is skipping lays it out on its own.
 */
function scrollParent(element: Element): Element | null {
  const start = element.closest(`[${SKIPPABLE_ATTRIBUTE}]`) ?? element;
  for (let parent = start.parentElement; parent !== null; parent = parent.parentElement) {
    if (scrolling(parent)) return parent;
  }
  return null;
}

/** Whether windows can be watched at all. Without it (jsdom, an old browser) every window is near. */
export function canWatchArt(): boolean {
  return typeof IntersectionObserver !== "undefined";
}

/**
 * Calls `onNear` once, when `element` has stayed within ART_NEAR_MARGIN_PX of its scrolling box for
 * ART_DWELL_MS (a window gone before then starts its count over). Returns the function that stops
 * watching it and cancels a pending dwell; watching an element again replaces the first watch.
 */
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
            // A window already counting keeps the earlier count.
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
  // Watching an element already watched replaces it; its pending dwell must not outlive it.
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
