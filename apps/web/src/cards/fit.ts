// Fits card text in its box (docs/polish/6-cards.md, Surface B): tiers start smaller and `useFitText` sets `--cf-fit`, which cards.css applies.
// Rules text retries full size, then `data-long`, before clamping at FIT_FLOOR_PX.
// Fits batch layout reads and park while `content-visibility: auto` skips a holder.

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

export function nameTier(name: string): LengthTier {
  return tierOf(name.length, NAME_TIER_MAX);
}

export function textTier(text: string): LengthTier {
  return tierOf(text.length, TEXT_TIER_MAX);
}

/** The one pixel of slack every measurement allows, for sub-pixel rounding. */
const SLACK_PX = 1;

const FIT_PROPERTY = "--cf-fit";
/** CardFace's tier scale, overridden inline while fitting. */
const SCALE_PROPERTY = "--cf-text-scale";
const CLAMP_LINES_PROPERTY = "--cf-clamp-lines";
const CLAMPED = "data-clamped";
/** On the face (`.cf`): the rules box has taken the long layout. */
export const LONG_ATTRIBUTE = "data-long";
/** The face element a rules box asks for the long layout. */
const FACE_SELECTOR = ".cf";
/** CardFace's base text and radiant-clause halves of a rules box. */
const BASE_SELECTOR = ".cf-text-base";
const RADIANT_SELECTOR = ".cf-text-radiant";
/** cards.css's rules-box line height, for a browser that reports `normal`. */
const FALLBACK_LINE_HEIGHT = 1.18;
/** A floor comparison's allowance for the binary search's last step. */
const FLOOR_SLACK_PX = 0.05;

export type FitOptions = {
  /** Rules-text floor in px; without it, text shrinks to FIT_MIN then clamps. */
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
/** A fit in progress: each `yield` hands over a read and waits for it. */
type Steps<T> = Generator<Probe | typeof PARK, T, unknown>;

/** Asks for one layout read and returns the probe's result. */
function* read<T>(probe: () => T): Steps<T> {
  return (yield probe) as T;
}

/** Finds the largest fitting `--cf-fit`; leaves FIT_MIN set when it still spills. */
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

/** The rules box's font at full size, without tier scaling or fitting. */
function* fullSizePx(element: HTMLElement): Steps<number> {
  element.style.setProperty(SCALE_PROPERTY, "1");
  setFit(element, 1);
  const px = yield* read(() => fontPx(element));
  element.style.removeProperty(SCALE_PROPERTY);
  return px;
}

/** Clamps floor-size text with room for the gap above a radiant clause. */
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

/** One fitting pass; without layout it only clears stale layout attributes. */
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
  // No reachable floor: shrink to fit, then clamp only past FIT_MIN.
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

/** A pass's settled box, used to detect a resize. */
function boxOf(element: HTMLElement): string {
  return `${element.clientWidth}x${element.clientHeight}`;
}

/** Marks a pool-grid holder whose layout the browser may skip. */
export const SKIPPABLE_ATTRIBUTE = "data-skippable";

/** Each skippable holder's last reported state: true while its contents are skipped. */
const holderSkipped = new WeakMap<Element, boolean>();

/** Whether `content-visibility: auto` skips this element without forcing layout. */
function skipped(element: HTMLElement): boolean {
  if (typeof element.checkVisibility !== "function") return false;
  const holder = element.closest(`[${SKIPPABLE_ATTRIBUTE}]`);
  if (holder === null) return false;
  return !element.checkVisibility({ contentVisibilityAuto: true }) && holderSkipped.get(holder) !== false;
}

/** Measures with the fit's reads, avoiding one layout per element; skipped elements park. */
function* fitThenMeasure(element: HTMLElement, options: FitOptions): Steps<string> {
  while (yield* read(() => skipped(element))) yield PARK;
  yield* fit(element, options);
  return yield* read(() => boxOf(element));
}

type Job = {
  element: HTMLElement;
  steps: Steps<string>;
  /** The last read's answer, passed to the next step. */
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

/** Requeues parked fits when their holder is un-skipped; still-skipped fits park again. */
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

/** Listen in capture because this non-bubbling event must reach every holder. */
function listen(): void {
  if (listening || typeof document === "undefined") return;
  listening = true;
  document.addEventListener(STATE_CHANGE, onStateChange, true);
}

/** Batches each round's writes before reads, then rethrows the first error after remaining fits. */
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

/** Queues a fitting pass before paint; returns its cancellation function. */
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

/** Refits mounted faces when web fonts load, which ResizeObserver cannot detect. */
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

/** Fits text on resize or content changes; overflowing text clamps and rules text preserves its floor. */
export function useFitText(ref: RefObject<HTMLElement | null>, content: string, options: FitOptions = {}): void {
  const { floorPx } = options;
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return undefined;

    const fitOptions: FitOptions = floorPx === undefined ? {} : { floorPx };
    // The container sizes this box; its first pass establishes the comparison baseline.
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
