// R4493000, R4493001 (issue #4493): the effects layer's composited memory on a phone. Every DOM cue's root is its own
// GPU layer (`.fx-dom > *` in fx.css), and so is each part it animates by transform or opacity, each held at the
// device's full pixel density; a phone's browser kills the tab past its memory, with no error to catch. So a phone's
// takeover stage merges its slices into at most FX_PHONE_STAGE_LAYERS pictures (`mergeStage`, R4493000), and the director
// holds what its mounted cues cover within FX_PHONE_LAYER_BUDGET_MB (`budgetTakeover`, R4493001), reducing a takeover's
// optional parts in a fixed order (its slices merged, its art, its stage, then its rays and wave, down to its dark and its
// sweep) and never a set piece's main beat (R3351000).
// The screens each cue covers are the tables below, at 390x844: `layerBudget.test.ts` estimates every cue from fx.css
// and holds the tables to it, so a cue that grows fails there first. Presentation only (CLAUDE.md rule 7).

import { ART_PLANES, type ArtPlane } from "../cards/art/pictures.ts";
import { testid } from "../game/contract.ts";
import { FX_BYTES_PER_MB, FX_LAYER_BYTES_PER_PX, FX_LOW_DEVICE_MEMORY_GB, FX_MOBILE_WIDTH, FX_PHONE_LAYER_BUDGET_MB, FX_PHONE_STAGE_LAYERS } from "./constants.ts";
import type { FxDomCue, FxTakeoverCue, FxTakeoverSlice, FxTakeoverStage, SetPieceShape, TakeoverLook } from "./types.ts";

/** R4493001: what the layer budget reads of the device: its window in CSS px, its pixel ratio and its memory in GB (null: unknown). */
export type FxDevice = { width: number; height: number; dpr: number; memoryGb: number | null };

/** R4493001: a phone's window: its narrower side under FX_MOBILE_WIDTH, so a phone held on its side is one too. */
export function isPhoneViewport(d: FxDevice): boolean {
  return Math.min(d.width, d.height) < FX_MOBILE_WIDTH;
}

/** R4493001: whether the director holds the device to FX_PHONE_LAYER_BUDGET_MB: a phone, or a device of FX_LOW_DEVICE_MEMORY_GB or less at any width. */
export function hasLayerBudget(d: FxDevice): boolean {
  return isPhoneViewport(d) || (d.memoryGb !== null && d.memoryGb <= FX_LOW_DEVICE_MEMORY_GB);
}

/** R4493001: FX_PHONE_LAYER_BUDGET_MB in screens' worth of `d`'s window at its pixel ratio (never below 1×); Infinity for an empty window. */
export function layerBudgetScreens(d: FxDevice): number {
  const r = Math.max(1, d.dpr || 1);
  const screen = d.width * d.height * r * r * FX_LAYER_BYTES_PER_PX;
  return screen > 0 ? (FX_PHONE_LAYER_BUDGET_MB * FX_BYTES_PER_MB) / screen : Number.POSITIVE_INFINITY;
}

/** R4493001: this browser's device: its window, its pixel ratio and `navigator.deviceMemory` where the browser reports it. */
export function readDevice(): FxDevice {
  const memory = typeof navigator === "undefined" ? undefined : (navigator as Navigator & { deviceMemory?: unknown }).deviceMemory;
  return {
    width: window.innerWidth,
    height: window.innerHeight,
    dpr: window.devicePixelRatio,
    memoryGb: typeof memory === "number" ? memory : null,
  };
}

/** R4493000: `n` items in `min(n, cap)` runs, far to near, whose lengths differ by at most one, the longer runs nearest. */
function runs(n: number, cap: number): number[] {
  const k = Math.max(1, Math.min(n, cap));
  const base = Math.floor(n / k);
  const longer = n % k;
  return Array.from({ length: k }, (_, i) => base + (i >= k - longer ? 1 : 0));
}

/** A run's pictures as one CSS `background-image`, the nearest on top (listed first); a run of none is "". */
function joined(images: readonly string[]): string {
  return images.filter((image) => image !== "").reverse().join(", ");
}

/**
 * R4493000: `stage` drawn in at most `cap` layers: adjacent slices (R3350000) or planes (R3345000) merge into one picture each,
 * the nearest of a run on top, so no picture is lost and the camera flies through `cap` depths. A merged slice stands at
 * its run's farthest depth and plane; merged planes keep their four places, a run's first holding its pictures and the
 * rest none. A stage within the cap is returned as it is.
 */
export function mergeStage(stage: FxTakeoverStage, cap: number): FxTakeoverStage {
  if (stage.slices !== undefined) {
    const slices = stage.slices;
    if (slices.length <= cap) return stage;
    let start = 0;
    const merged: FxTakeoverSlice[] = runs(slices.length, cap).map((length) => {
      const run = slices.slice(start, start + length);
      start += length;
      const first = run[0]!;
      return { image: joined(run.map((slice) => slice.image)), plane: first.plane, depth: first.depth };
    });
    return { ...stage, slices: merged };
  }
  const planes = stage.planes;
  if (planes.length <= cap) return stage;
  const out: string[] = [];
  let start = 0;
  for (const length of runs(planes.length, cap)) {
    out.push(joined(planes.slice(start, start + length)));
    for (let i = 1; i < length; i += 1) out.push("");
    start += length;
  }
  return { ...stage, planes: out };
}

/** R4493001: a takeover's forms, fullest first; each drops what the one before it kept (`takeoverForm`). */
export const TAKEOVER_FULL = 0;
/** R4493001: its stage merged into FX_PHONE_STAGE_LAYERS pictures (R4493000), as a phone always draws it. */
export const TAKEOVER_MERGED = 1;
/** R4493001: merged, and without its look's own art and its picture (R3400000, R3407000). */
export const TAKEOVER_ARTLESS = 2;
/** R4493001: no stage either: the dark, the sweep, the rays and the wave, as a takeover with no set piece plays (R3345000). */
export const TAKEOVER_BARE = 3;
/** R4493001: the dark and the sweep alone, no rays and no wave: the least a takeover mounts, a main beat still (R3351000). */
export const TAKEOVER_DARK = 4;

/** R4493001: `cue` at the form `level` names, each level dropping what the one before it kept. */
export function takeoverForm(cue: FxTakeoverCue, level: number): FxTakeoverCue {
  if (level <= TAKEOVER_FULL) return cue;
  let form: FxTakeoverCue = { ...cue, stage: cue.stage === null ? null : mergeStage(cue.stage, FX_PHONE_STAGE_LAYERS) };
  if (level >= TAKEOVER_ARTLESS) {
    const { art: _art, ...rest } = form;
    form = { ...rest, artless: true };
  }
  if (level >= TAKEOVER_BARE) form = { ...form, stage: null };
  if (level >= TAKEOVER_DARK) form = { ...form, sweepOnly: true };
  return form;
}

/**
 * R4493001: the form of `cue` the device's budget holds while the mounted and coming DOM cues cover `held` screens: the
 * cue itself on a device with no budget; else the fullest form, from merged on a phone (full elsewhere) to bare, that
 * keeps `held` and it within the budget; failing every one, its dark and its sweep, which always mount (a main beat,
 * R3351000).
 */
export function budgetTakeover(cue: FxTakeoverCue, held: number, d: FxDevice): FxTakeoverCue {
  if (!hasLayerBudget(d)) return cue;
  const budget = layerBudgetScreens(d);
  for (let level = isPhoneViewport(d) ? TAKEOVER_MERGED : TAKEOVER_FULL; level <= TAKEOVER_BARE; level += 1) {
    const form = takeoverForm(cue, level);
    if (held + cueScreens(form) <= budget) return form;
  }
  return takeoverForm(cue, TAKEOVER_DARK);
}

/* The screens each cue covers at 390x844: its promoted elements' boxes at their largest scale, summed (layerBudget.test.ts). */

/**
 * R4493001: each takeover look's own layers: `sweep`, the root, the dark and the sweep (what its darkest form keeps);
 * `burst`, its rays and its wave; `art`, its art layer's (`.takeover-art`), or 0.
 */
export const FX_TAKEOVER_LOOK_SCREENS: Readonly<Record<TakeoverLook, { sweep: number; burst: number; art: number }>> = {
  prismatic: { sweep: 2.78, burst: 5.5, art: 0 },
  void: { sweep: 4.86, burst: 5.5, art: 0 },
  pawn: { sweep: 2.66, burst: 5.5, art: 1 },
  genie: { sweep: 2.56, burst: 5.5, art: 1 },
  ai: { sweep: 2, burst: 5.5, art: 0.46 },
  rotunda: { sweep: 2.66, burst: 5.5, art: 1 },
  crystalForge: { sweep: 2.66, burst: 5.5, art: 1 },
  summit: { sweep: 2, burst: 5.5, art: 0.13 },
  canvas: { sweep: 2, burst: 5.5, art: 0.09 },
  napkin: { sweep: 2, burst: 5.5, art: 0.37 },
  abyss: { sweep: 3.03, burst: 5.5, art: 0 },
  freshJack: { sweep: 2.37, burst: 5.5, art: 0 },
  ikwzki: { sweep: 2.56, burst: 5.5, art: 0 },
  shockwave: { sweep: 2.23, burst: 5.5, art: 0 },
  alteran: { sweep: 2.3, burst: 5.5, art: 0 },
  edge: { sweep: 2.3, burst: 5.5, art: 0 },
  redNarcor: { sweep: 2.66, burst: 5.5, art: 0 },
  ascent: { sweep: 2, burst: 5.5, art: 0.67 },
  workshop: { sweep: 2, burst: 5.5, art: 0.79 },
  brush: { sweep: 2, burst: 5.5, art: 0.43 },
  mine: { sweep: 2, burst: 5.5, art: 1.05 },
  quarry: { sweep: 2, burst: 5.5, art: 0.39 },
  pavilion: { sweep: 2, burst: 5.5, art: 1.11 },
  virus: { sweep: 2, burst: 5.5, art: 1.02 },
  dojo: { sweep: 2, burst: 5.5, art: 1.26 },
  studyHall: { sweep: 2, burst: 5.5, art: 0.28 },
  caneRack: { sweep: 2, burst: 5.5, art: 0.22 },
  warRoom: { sweep: 2, burst: 5.5, art: 1.64 },
  fortress: { sweep: 2, burst: 5.5, art: 0.1 },
  zealot: { sweep: 2.66, burst: 5.5, art: 1 },
  portal: { sweep: 2.66, burst: 5.5, art: 1 },
  stair: { sweep: 2.66, burst: 5.5, art: 1 },
  vineyard: { sweep: 2.66, burst: 5.5, art: 1 },
  vault: { sweep: 2, burst: 5.5, art: 0.79 },
  rod: { sweep: 2, burst: 5.5, art: 0.14 },
  wire: { sweep: 2, burst: 5.5, art: 1.31 },
  whiteDwarf: { sweep: 2, burst: 5.5, art: 2.19 },
  forge: { sweep: 2, burst: 5.5, art: 0.83 },
};

/** R4493001: a takeover's picture over the dark (R3407000, `img.fx-takeover-art`): the window. */
export const FX_TAKEOVER_PICTURE_SCREENS = 1;
/** R4493001: a takeover's stage itself (R3345000, `.fx-takeover-stage`): the window. */
export const FX_TAKEOVER_STAGE_SCREENS = 1;
/** R4493001: how much bigger than the window a slice of depth 1 flies (fx.css's `fx-takeover-fly`, R3350000). */
export const FX_TAKEOVER_FLY_REACH = 0.6;
/** R4493001: each depth plane's largest scale (fx.css's `fx-takeover-dolly-*`, R3345000); a plane covers the window at rest. */
export const FX_TAKEOVER_DOLLY_SCALES: Readonly<Record<ArtPlane, number>> = { far: 1.04, middle: 1.1, near: 1.25, nearest: 1.6 };

/** R4493001: each set piece shape's layers (R2800000, R3338000), its Terminus beats included. */
export const FX_SET_PIECE_LAYER_SCREENS: Readonly<Record<SetPieceShape, number>> = {
  vortex: 3.87,
  fold: 4.15,
  rift: 2.1,
  lamp: 2.65,
  chalk: 2.41,
  bend: 1.14,
  freeze: 2.44,
  abyss: 3,
  crunch: 3.88,
  doom: 2.03,
};

/** R4493001: every other DOM cue's layers, its largest case's on a card (or on the board, where it always lands there). */
export const FX_CUE_LAYER_SCREENS: Readonly<Record<Exclude<FxDomCue["kind"], "takeover" | "setPiece">, number>> = {
  splat: 0.03,
  rays: 0.67,
  sheen: 0.04,
  ghost: 0.02,
  callout: 0.01,
  arrows: 0.04,
  banner: 0.05,
  result: 1.27,
  fracture: 0.2,
  walls: 1.47,
  brand: 0.09,
  shield: 0.02,
  chaos: 0.15,
  roll: 0.08,
  fog: 5.59,
  zone: 0.05,
  launch: 0.07,
  ship: 0.05,
  rift: 0.17,
  anomalyTitle: 0.5,
  radar: 2.68,
  module: 0.01,
  weld: 0.01,
  cinematic: 5.64,
  doctrine: 0.17,
  wreck: 0.06,
  scorch: 0.03,
  figure: 0.37,
};

/**
 * R4493001: the layers of the cues that grow with the box they land on, landed on the whole board (`testid.board`): a fan
 * (2.8 times the board's longer side, capped at 100vmax), a sheen across it and a figure over it. Every other anchor is a
 * card's, a tile's, a hero's or a pile's, which FX_CUE_LAYER_SCREENS counts.
 */
export const FX_BOARD_CUE_LAYER_SCREENS: Readonly<Partial<Record<keyof typeof FX_CUE_LAYER_SCREENS, number>>> = {
  rays: 2.82,
  sheen: 1.14,
  figure: 0.81,
};

/** Whether `cue` lands on the whole board. */
function onBoard(cue: FxDomCue): boolean {
  return "at" in cue && cue.at.kind === "testid" && cue.at.testid === testid.board;
}

/** R4493001: a takeover stage's layers: the stage, and each slice or picture at its largest scale. */
function stageScreens(stage: FxTakeoverStage): number {
  const pictures =
    stage.slices !== undefined
      ? stage.slices.reduce((sum, slice) => sum + (1 + slice.depth * FX_TAKEOVER_FLY_REACH) ** 2, 0)
      : stage.planes.reduce((sum, image, i) => {
          const plane = ART_PLANES[i];
          return image === "" || plane === undefined ? sum : sum + FX_TAKEOVER_DOLLY_SCALES[plane] ** 2;
        }, 0);
  return FX_TAKEOVER_STAGE_SCREENS + pictures;
}

/** R4493001: the screens' worth of layers `cue` holds while it is mounted, at 390x844 (the tables above). */
export function cueScreens(cue: FxDomCue): number {
  switch (cue.kind) {
    case "takeover": {
      const look = FX_TAKEOVER_LOOK_SCREENS[cue.look];
      return (
        look.sweep +
        (cue.sweepOnly === true ? 0 : look.burst) +
        (cue.artless === true ? 0 : look.art) +
        (cue.art === undefined ? 0 : FX_TAKEOVER_PICTURE_SCREENS) +
        (cue.stage === null ? 0 : stageScreens(cue.stage))
      );
    }
    case "setPiece":
      return FX_SET_PIECE_LAYER_SCREENS[cue.shape];
    default:
      return (onBoard(cue) ? FX_BOARD_CUE_LAYER_SCREENS[cue.kind] : undefined) ?? FX_CUE_LAYER_SCREENS[cue.kind];
  }
}
