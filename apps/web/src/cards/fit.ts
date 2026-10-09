// Text that fits its box (docs/polish/6-cards.md, Surface B "fit.ts"). Coarse pure tiers start a
// long name or rules text from a smaller font (TIER_SCALE); `useFitText` then measures the real box
// and shrinks the rest of the way with the inline property `--cf-fit`, which cards.css multiplies
// into the font size. Faces are sized in container units, so a factor that fits one size fits all.
//
// THE READING FLOOR (rules text only). A rules box whose fitted font is under FIT_FLOOR_PX tries, in
// order: its own box from full size (`--cf-text-scale: 1`, since the tier's head start is a guess),
// the long layout (`data-long` on the `.cf`), then the floor itself, clamped with an ellipsis
// (`data-clamped`, `--cf-clamp-lines`). A face too small to reach the floor (a hand card) is fitted
// as before.
//
// BATCHED. A fit is a generator yielding each read as a thunk; one scheduler runs every pending fit
// in rounds (all writes, then all reads), so a page lays out once per round, not once per element
// step. `useFitText` queues from the layout effect and the queue runs on the next microtask, before
// paint; `flushFits` runs it now.
//
// SKIPPED FACES WAIT. Reading a face the browser skips (`content-visibility: auto` on a holder with
// SKIPPABLE_ATTRIBUTE) lays it out and costs more than the skip saved, so a fit first asks
// `checkVisibility({ contentVisibilityAuto: true })` (a plain call lays out) and parks if skipped,
// unless its holder's last `contentvisibilityautostatechange` said it is not. An un-skip requeues
// every parked fit, as Chrome flips a scroll's faces at once, so a face shows its tier's first guess
// for one frame. A browser without `checkVisibility` (jsdom) never parks.

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
/** Yielded instead of a read: the element is skipped, so the fit waits until it is not. */
const PARK = Symbol("park");
/** A fit in progress: its writes run as it is stepped, each `yield` hands over a read and waits for it. */
type Steps<T> = Generator<Probe | typeof PARK, T, unknown>;

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
 * Holds the text at the floor and clamps it to the lines its box holds, with an ellipsis. A clamped
 * box is as tall as its lines (cards.css); a line is kept back for the gap above a radiant clause.
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

/** On an element the browser may skip the layout of (`content-visibility: auto`), the pool grid's items. */
export const SKIPPABLE_ATTRIBUTE = "data-skippable";

/** Each skippable holder's last reported state: true while its contents are skipped. */
const holderSkipped = new WeakMap<Element, boolean>();

/**
 * True when the browser is skipping `element`'s layout for `content-visibility: auto` (see the
 * header). Style only: it never lays anything out.
 */
function skipped(element: HTMLElement): boolean {
  if (typeof element.checkVisibility !== "function") return false;
  const holder = element.closest(`[${SKIPPABLE_ATTRIBUTE}]`);
  if (holder === null) return false;
  return !element.checkVisibility({ contentVisibilityAuto: true }) && holderSkipped.get(holder) !== false;
}

/**
 * A fit, then one more read of the box it left. Reading it from the pass's own callback would force
 * a layout per element (the pass's last write is behind it); as a step of the pass it is read with
 * everyone else's. While the element is skipped it waits, parked, and asks again once resumed.
 */
function* fitThenMeasure(element: HTMLElement, options: FitOptions): Steps<string> {
  while (yield* read(() => skipped(element))) yield PARK;
  yield* fit(element, options);
  return yield* read(() => boxOf(element));
}

type Job = {
  element: HTMLElement;
  steps: Steps<string>;
  /** What the job's last read answered, which its next step receives. */
  reading: unknown;
  onDone: ((box: string) => void) | undefined;
  cancelled: boolean;
};

const queue = new Set<Job>();
let flushQueued = false;
/** Fits whose element is skipped, waiting for it to be un-skipped. */
const parked = new Set<Job>();

const STATE_CHANGE = "contentvisibilityautostatechange";

function queueFlush(): void {
  if (flushQueued) return;
  flushQueued = true;
  queueMicrotask(flushFits);
}

/**
 * A holder's contents were skipped or un-skipped. If it holds a parked fit and came near the screen,
 * every parked fit asks again, since the browser has already flipped every item this scroll
 * un-skips (see the header); those still skipped park again on their first read.
 */
function onStateChange(event: Event): void {
  const target = event.target;
  const state = (event as Event & { skipped?: boolean }).skipped;
  if (!(target instanceof Element) || typeof state !== "boolean") return;
  holderSkipped.set(target, state);
  if (state || ![...parked].some((job) => target.contains(job.element))) return;
  for (const job of parked) queue.add(job);
  parked.clear();
  queueFlush();
}

let listening = false;

/**
 * From the first fit on, the document hears every holder's state (the event does not bubble, so in
 * the capture phase), which is before the browser first decides any of them.
 */
function listen(): void {
  if (listening || typeof document === "undefined") return;
  listening = true;
  document.addEventListener(STATE_CHANGE, onStateChange, true);
}

/**
 * Runs every queued fit to the end, in rounds: each fit steps to its next read (so every write of
 * the round lands first), then all those reads run together, one layout for the lot. No fit's box
 * depends on another's, so their order changes nothing. One fit throwing ends only that fit; the
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
        else if (step.value === PARK) parked.add(job);
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
 * Queues one fitting pass for `element`, to run with the rest on the next microtask (before the
 * browser paints what the caller committed). `onDone` gets the box (`<width>x<height>`) the element
 * was left at. Returns the function that drops the pass.
 */
export function scheduleFit(element: HTMLElement, options: FitOptions, onDone?: (box: string) => void): () => void {
  const job: Job = { element, steps: fitThenMeasure(element, options), reading: undefined, onDone, cancelled: false };
  queue.add(job);
  listen();
  queueFlush();
  return () => {
    job.cancelled = true;
    queue.delete(job);
    parked.delete(job);
  };
}

/**
 * Every mounted fit's refit, for when a web font lands (fonts.css). A box's size does not change
 * when its font swaps in, so no ResizeObserver notices; this refits once the face is in.
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

/**
 * Fits `ref`'s text to its box with `--cf-fit` (FIT_MIN..1, FIT_STEPS steps), re-running on resize
 * and when `content` changes. Still overflowing at FIT_MIN, it sets `data-clamped="true"` and the
 * CSS line-clamps. With `floorPx` (the rules box) it keeps the text readable (see the header). A
 * no-op without layout (every element in jsdom); the pass is batched (`scheduleFit`).
 */
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
