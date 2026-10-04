// Which art windows are near the screen (CardArt `lazy`).
//
// A procedural picture is a data-URI SVG on the window's background, and the browser parses it when
// the window is first styled and laid out: about 1.3 ms a card, so a grid of 318 spent half its
// first long task on pictures nobody could see. A lazy window draws nothing until it has stayed
// within ART_NEAR_MARGIN_PX of the box that scrolls it for ART_DWELL_MS — a card flicked straight
// past never pays the parse — then draws once and keeps its picture. Until then it is its theme's
// sky as a flat gradient (art.css).
//
// One IntersectionObserver per scrolling box watches every window in it. The scrolling box is the
// nearest ancestor with `overflow-y: auto | scroll` (the viewport when there is none), because an
// observer's `rootMargin` widens its root and nothing else: a window clipped by an inner scroller
// would otherwise count as far until it was actually on screen.

import { ART_DWELL_MS, ART_NEAR_MARGIN_PX } from "../constants.ts";

type Watched = { onNear: () => void; dwell: ReturnType<typeof setTimeout> | null };
type Watch = { observer: IntersectionObserver; windows: Map<Element, Watched> };

/** One watch per scrolling box (null: the viewport), dropped when its last window leaves. */
const watches = new Map<Element | null, Watch>();
/**
 * Whether an ancestor scrolls. A grid's windows share their ancestors, so within one burst of
 * mounts each is asked once; the answers are dropped when the burst ends, because a media query
 * can move the scrolling to another box.
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

function scrollParent(element: Element): Element | null {
  for (let parent = element.parentElement; parent !== null; parent = parent.parentElement) {
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
 * ART_DWELL_MS (a window gone before then starts its count over when it comes back). Returns the
 * function that stops watching it and cancels a pending dwell.
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
