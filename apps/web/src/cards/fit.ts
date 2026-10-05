// Text that fits its box (docs/polish/6-cards.md, Surface B "fit.ts").
//
// Two layers. The tiers are coarse and pure: a long name or a long rules text starts from a smaller
// font (TIER_SCALE), chosen from its length alone, so the first paint is already close. Then
// `useFitText` measures the real box and shrinks the font the rest of the way by writing one inline
// custom property, `--cf-fit`, which cards.css multiplies into the font size. Everything on a face
// is sized in container units, so the factor that fits at one card size fits at every size.
//
// THE READING FLOOR (rules text only). Shrinking alone printed the densest cards at 6 px in the
// deck builder's grid. So a rules box whose fitted font is under FIT_FLOOR_PX tries, in order:
//  1. its own box from full size (`--cf-text-scale: 1` inline): the length tier's head start is a
//     first-paint guess, and a text it undersold may still fit at the floor;
//  2. the long layout (`data-long="true"` on the `.cf`: a shorter art window and a taller rules box,
//     cards.css);
//  3. the floor itself, clamped with an ellipsis (`data-clamped`, with `--cf-clamp-lines` set to
//     the lines the box holds). The detail view and the hover preview print the text whole.
// A face too small to print even full-size text at the floor (a hand card, a Discover option) is
// fitted as before: the floor is for faces that can reach it.
//
// BATCHED. A fit is a search: write `--cf-fit`, read whether the box spills, write again. Run one
// element at a time that is a forced layout per step, and 318 cards in the Almanac (two boxes each)
// spent most of a second and a half of its first paint in them. So a fit is a generator
// that yields each read as a thunk, and one scheduler runs every pending fit in rounds: all the
// writes of a round, then all its reads, so the whole page lays out once per round (about a dozen
// times) instead of once per element step. `useFitText` queues its fit from the layout effect and
// the scheduler runs on the microtask that follows the commit, before the browser paints, so no card
// is ever seen unfitted. `flushFits` runs the queue now (a test's, or anything that must read the
// result at once).

import { useLayoutEffect, type RefObject } from "react";

import { FIT_MIN, FIT_STEPS, NAME_TIER_MAX, TEXT_TIER_MAX } from "./constants.ts";

export type LengthTier = "s" | "m" | "l" | "xl" | "xxl";

function tierOf(length: number, max: { s: number; m: number; l: number; xl: number }): LengthTier {
  if (length <= max.s) return "s";
  if (length <= max.m) return "m";
  if (length <= max.l) return "l";
  if (length <= max.xl) return "xl";
  return "xxl";
}

/** ≤12 s, ≤18 m, ≤24 l, ≤30 xl, else xxl. */
export function nameTier(name: string): LengthTier {
  return tierOf(name.length, NAME_TIER_MAX);
}

/** ≤40 s, ≤90 m, ≤160 l, ≤260 xl, else xxl. Pass the base text and the radiant clause together. */
export function textTier(text: string): LengthTier {
  return tierOf(text.length, TEXT_TIER_MAX);
}

/** The one pixel of slack every measurement allows, for sub-pixel rounding. */
const SLACK_PX = 1;

const FIT_PROPERTY = "--cf-fit";
/** The length tier's font scale, which CardFace sets on the face; the fitter overrides it inline. */
const SCALE_PROPERTY = "--cf-text-scale";
const CLAMP_LINES_PROPERTY = "--cf-clamp-lines";
const CLAMPED = "data-clamped";
/** On the face (`.cf`): the rules box has taken the long layout. */
export const LONG_ATTRIBUTE = "data-long";
/** The face element a rules box asks for the long layout. */
const FACE_SELECTOR = ".cf";
/** CardFace's two halves of a rules box: the base text, and the radiant clause under its rule. */
const BASE_SELECTOR = ".cf-text-base";
const RADIANT_SELECTOR = ".cf-text-radiant";
/** cards.css's rules-box line height, for a browser that reports `normal`. */
const FALLBACK_LINE_HEIGHT = 1.18;
/** A floor comparison's allowance for the binary search's last step. */
const FLOOR_SLACK_PX = 0.05;

export type FitOptions = {
  /**
   * The rules box's reading floor in px (FIT_FLOOR_PX). Absent, the text shrinks to FIT_MIN and
   * then clamps, as a name does.
   */
  floorPx?: number;
};

function overflows(element: HTMLElement): boolean {
  return (
    element.scrollHeight > element.clientHeight + SLACK_PX || element.scrollWidth > element.clientWidth + SLACK_PX
  );
}

function setFit(element: HTMLElement, factor: number): void {
  element.style.setProperty(FIT_PROPERTY, String(Math.round(factor * 1000) / 1000));
}

function fontPx(element: HTMLElement): number {
  return parseFloat(getComputedStyle(element).fontSize);
}

/** One read of layout, which the scheduler runs with every other fit's reads of the same round. */
type Probe = () => unknown;
/** A fit in progress: its writes run as it is stepped, each `yield` hands over a read and waits for it. */
type Steps<T> = Generator<Probe, T, unknown>;

/** Asks for one read of layout and waits for its answer, which is the probe's own result. */
function* read<T>(probe: () => T): Steps<T> {
  return (yield probe) as T;
}

/**
 * Binary-searches the largest `--cf-fit` in FIT_MIN..1 that fits and leaves it set. Returns it, or
 * null when even FIT_MIN spills (FIT_MIN is left set).
 */
function* search(element: HTMLElement): Steps<number | null> {
  setFit(element, 1);
  if (!(yield* read(() => overflows(element)))) return 1;

  setFit(element, FIT_MIN);
  if (yield* read(() => overflows(element))) return null;

  let fits = FIT_MIN;
  let spills = 1;
  for (let step = 0; step < FIT_STEPS; step += 1) {
    const middle = (fits + spills) / 2;
    setFit(element, middle);
    if (yield* read(() => overflows(element))) spills = middle;
    else fits = middle;
  }
  setFit(element, fits);
  return fits;
}

/** The search found a size, and it is at least the floor. */
function* readable(element: HTMLElement, best: number | null, floorPx: number): Steps<boolean> {
  return best !== null && (yield* read(() => fontPx(element))) >= floorPx - FLOOR_SLACK_PX;
}

/** The largest font this rules box can print at all: full size, no tier scale, no shrink. */
function* fullSizePx(element: HTMLElement): Steps<number> {
  element.style.setProperty(SCALE_PROPERTY, "1");
  setFit(element, 1);
  const px = yield* read(() => fontPx(element));
  element.style.removeProperty(SCALE_PROPERTY);
  return px;
}

/**
 * Holds the text at the floor (or at full size, if the floor is above it) and clamps it to the
 * lines its box holds, with an ellipsis. A clamped box is as tall as its lines, up to the box
 * (cards.css), so nothing past the last line is painted; a line is kept back for the gap above a
 * radiant clause.
 */
function* clampAtFloor(element: HTMLElement, floorPx: number): Steps<void> {
  const current = yield* read(() => fontPx(element));
  const factor = Number(element.style.getPropertyValue(FIT_PROPERTY)) || 1;
  // Rounded up, so setFit's three decimals never land a hair under the floor.
  const atFloor = current > 0 ? Math.ceil(((factor * floorPx) / current) * 1000) / 1000 : 1;
  setFit(element, Math.min(1, atFloor));
  element.setAttribute(CLAMPED, "true");

  const { lineHeight, box, padding } = yield* read(() => {
    const style = getComputedStyle(element);
    return {
      lineHeight: parseFloat(style.lineHeight) || parseFloat(style.fontSize) * FALLBACK_LINE_HEIGHT,
      box: parseFloat(style.maxHeight) || element.clientHeight,
      padding: (parseFloat(style.paddingTop) || 0) + (parseFloat(style.paddingBottom) || 0),
    };
  });
  const inner = box - padding;
  const splitByRule = element.querySelector(RADIANT_SELECTOR) !== null && element.querySelector(BASE_SELECTOR)?.textContent !== "";
  const lines = Math.max(1, Math.floor(inner / lineHeight) - (splitByRule ? 1 : 0));
  element.style.setProperty(CLAMP_LINES_PROPERTY, String(lines));
}

/**
 * One fitting pass. Without layout (jsdom, or a rules box the small-card container query hides)
 * it only drops a stale `data-clamped` and `data-long`, which are absent in jsdom, so there it
 * changes nothing.
 */
function* fit(element: HTMLElement, options: FitOptions): Steps<void> {
  const face = options.floorPx === undefined ? null : element.closest<HTMLElement>(FACE_SELECTOR);
  if (yield* read(() => element.clientWidth === 0 && element.clientHeight === 0)) {
    if (element.hasAttribute(CLAMPED)) element.removeAttribute(CLAMPED);
    if (face?.hasAttribute(LONG_ATTRIBUTE) === true) face.removeAttribute(LONG_ATTRIBUTE);
    element.style.removeProperty(SCALE_PROPERTY);
    return;
  }

  element.removeAttribute(CLAMPED);
  element.style.removeProperty(CLAMP_LINES_PROPERTY);
  element.style.removeProperty(SCALE_PROPERTY);
  face?.removeAttribute(LONG_ATTRIBUTE);

  const { floorPx } = options;
  // No floor, or a face too small to reach it: shrink to fit, and clamp only past FIT_MIN.
  if (floorPx === undefined || (yield* fullSizePx(element)) < floorPx - FLOOR_SLACK_PX) {
    // Even the smallest font spills: the CSS line-clamps with an ellipsis instead.
    if ((yield* search(element)) === null) element.setAttribute(CLAMPED, "true");
    return;
  }

  if (yield* readable(element, yield* search(element), floorPx)) return;

  // 1. The tier's head start undersold it: its own box, from full size.
  element.style.setProperty(SCALE_PROPERTY, "1");
  if (yield* readable(element, yield* search(element), floorPx)) return;

  // 2. The long layout gives the text more room.
  if (face !== null) {
    face.setAttribute(LONG_ATTRIBUTE, "true");
    if (yield* readable(element, yield* search(element), floorPx)) return;
  }

  // 3. The floor, clamped.
  yield* clampAtFloor(element, floorPx);
}

/** The box a pass settled at, as `useFitText` compares it with the next notice of a resize. */
function boxOf(element: HTMLElement): string {
  return `${element.clientWidth}x${element.clientHeight}`;
}

/**
 * A fit, then one more read of the box it left. Reading it from the pass's own callback would force
 * a layout per element (the pass's last write is behind it); as a step of the pass it is read with
 * everyone else's.
 */
function* fitThenMeasure(element: HTMLElement, options: FitOptions): Steps<string> {
  yield* fit(element, options);
  return yield* read(() => boxOf(element));
}

type Job = {
  steps: Steps<string>;
  /** What the job's last read answered, which its next step receives. */
  reading: unknown;
  onDone: ((box: string) => void) | undefined;
  cancelled: boolean;
};

const queue = new Set<Job>();
let flushQueued = false;

/**
 * Runs every queued fit to the end, in rounds. A round steps each fit up to its next read (so every
 * write of the round lands first), then runs all of those reads together, which is one layout for
 * the lot. A fit's boxes never depend on another's (each face sizes its text from its own
 * container), so the order they run in changes nothing. One fit throwing ends only that fit; the
 * first error is rethrown once the rest are done.
 */
export function flushFits(): void {
  flushQueued = false;
  let live = [...queue];
  queue.clear();
  let failure: unknown;
  let failed = false;
  const fail = (error: unknown): void => {
    if (!failed) failure = error;
    failed = true;
  };

  while (live.length > 0) {
    const reading: Array<[Job, Probe]> = [];
    for (const job of live) {
      if (job.cancelled) continue;
      try {
        const step = job.steps.next(job.reading);
        if (step.done === true) job.onDone?.(step.value);
        else reading.push([job, step.value]);
      } catch (error) {
        fail(error);
      }
    }
    live = [];
    for (const [job, probe] of reading) {
      try {
        job.reading = probe();
        live.push(job);
      } catch (error) {
        fail(error);
      }
    }
  }
  if (failed) throw failure;
}

/**
 * Queues one fitting pass for `element`, to run with every other queued pass on the next microtask
 * (which is before the browser paints what the caller just committed). `onDone` runs when the pass
 * has finished, with the box (`<width>x<height>`) the element was left at. Returns the function that drops it, for a pass the element no longer needs.
 */
export function scheduleFit(element: HTMLElement, options: FitOptions, onDone?: (box: string) => void): () => void {
  const job: Job = { steps: fitThenMeasure(element, options), reading: undefined, onDone, cancelled: false };
  queue.add(job);
  if (!flushQueued) {
    flushQueued = true;
    queueMicrotask(flushFits);
  }
  return () => {
    job.cancelled = true;
    queue.delete(job);
  };
}

/**
 * Binary-searches the inline custom property `--cf-fit` (FIT_MIN..1, FIT_STEPS steps) on `ref`
 * until scrollHeight ≤ clientHeight + 1 and scrollWidth ≤ clientWidth + 1. Re-runs on resize
 * (ResizeObserver when present) and when `content` changes. If FIT_MIN still overflows it sets
 * `data-clamped="true"` and the CSS line-clamps with an ellipsis. With `floorPx` (the rules box)
 * it also keeps the text readable: the long layout first, then a clamp at the floor (see the
 * header). A no-op when the element has no layout (clientWidth and clientHeight both 0, which is
 * every element in jsdom). The pass runs batched with the rest of the page's, on the microtask
 * after the commit (`scheduleFit`).
 */
/**
 * Every mounted fit's refit, for when a web font lands (fonts.css). The face is measured, and a
 * box's size does not change when its font swaps in, so no ResizeObserver notices: the metric-matched
 * fallback keeps the drift to a fraction of a pixel, and this refits exactly once the face is in.
 */
const refitOnFonts = new Set<() => void>();
let fontsWatched = false;

function watchFonts(): void {
  if (fontsWatched || typeof document === "undefined") return;
  const fonts = (document as Document & { fonts?: FontFaceSet }).fonts;
  if (fonts === undefined || typeof fonts.addEventListener !== "function") return;
  fontsWatched = true;
  fonts.addEventListener("loadingdone", () => {
    for (const refit of [...refitOnFonts]) refit();
  });
}

export function useFitText(ref: RefObject<HTMLElement | null>, content: string, options: FitOptions = {}): void {
  const { floorPx } = options;
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return undefined;

    const fitOptions: FitOptions = floorPx === undefined ? {} : { floorPx };
    // The observed box is sized by its container, never by its font. The long layout does resize
    // it, but a refit lands on the same layout again, so the size it settles at is stable. Unknown
    // until the first pass has run: a notice before that has nothing to compare with, and that
    // pass measures the box as it is anyway.
    let last: string | undefined;
    const queueFit = (): (() => void) =>
      scheduleFit(element, fitOptions, (box) => {
        last = box;
      });
    let cancel = queueFit();
    const refit = (): void => {
      cancel();
      last = undefined;
      cancel = queueFit();
    };
    watchFonts();
    refitOnFonts.add(refit);

    if (typeof ResizeObserver === "undefined") {
      return () => {
        refitOnFonts.delete(refit);
        cancel();
      };
    }
    const observer = new ResizeObserver(() => {
      if (last === undefined || boxOf(element) === last) return;
      refit();
    });
    observer.observe(element);
    return () => {
      refitOnFonts.delete(refit);
      observer.disconnect();
      cancel();
    };
  }, [ref, content, floorPx]);
}
