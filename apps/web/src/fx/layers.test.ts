// R4493000, R4493001 (issue #4493): the phone's merged stage and the layer budget's order (layers.ts).

import { afterEach, describe, expect, it } from "vitest";

import { dioramaOf } from "../cards/art/dioramas/index.ts";
import { ART_PLANES } from "../cards/art/pictures.ts";
import { FX_LOW_DEVICE_MEMORY_GB, FX_MOBILE_WIDTH, FX_PHONE_STAGE_LAYERS } from "./constants.ts";
import {
  budgetTakeover,
  cueScreens,
  hasLayerBudget,
  isPhoneViewport,
  layerBudgetScreens,
  mergeStage,
  readDevice,
  takeoverForm,
  TAKEOVER_ARTLESS,
  TAKEOVER_BARE,
  TAKEOVER_DARK,
  TAKEOVER_FULL,
  TAKEOVER_MERGED,
  type FxDevice,
} from "./layers.ts";
import type { FxTakeoverCue, FxTakeoverStage } from "./types.ts";

const PHONE: FxDevice = { width: 390, height: 844, dpr: 3, memoryGb: null };
const DESKTOP: FxDevice = { width: 1280, height: 800, dpr: 2, memoryGb: 8 };
const LOW_DESKTOP: FxDevice = { width: 1280, height: 800, dpr: 1, memoryGb: FX_LOW_DEVICE_MEMORY_GB };

function voidStage(): FxTakeoverStage {
  const diorama = dioramaOf("core-100");
  if (diorama === null) throw new Error("Ceaseless Void has no diorama");
  return {
    planes: [],
    name: "Ceaseless Void",
    face: "script",
    slices: diorama.slices.map((slice, i) => ({ image: `url(s${String(i)})`, plane: slice.plane, depth: slice.depth })),
  };
}

const cue = (stage: FxTakeoverStage | null, art?: string): FxTakeoverCue => ({
  kind: "takeover",
  stage,
  look: "freshJack",
  tint: { rim: "#2dd4bf", core: "#fdf4ff", glow: "#e879f9" },
  at: { kind: "testid", testid: "card-u1" },
  landMs: 1200,
  delayMs: 0,
  durationMs: 2400,
  ...(art === undefined ? {} : { art }),
});

afterEach(() => {
  Reflect.deleteProperty(navigator, "deviceMemory");
});

describe("R4493000 a phone's takeover stage merges its slices", () => {
  it("R4493000 a phone stage merges Ceaseless Void's seven slices into three pictures, nearest first, none lost", () => {
    const stage = voidStage();
    const slices = stage.slices ?? [];
    expect(slices).toHaveLength(7);
    const merged = mergeStage(stage, FX_PHONE_STAGE_LAYERS);
    // Far, middle and near: runs of two, two and three, the longest nearest, each the nearest of its run on top.
    expect(merged.slices?.map((slice) => slice.image)).toEqual(["url(s1), url(s0)", "url(s3), url(s2)", "url(s6), url(s5), url(s4)"]);
    // Each stands where its run's farthest slice stood.
    expect(merged.slices?.map((slice) => [slice.plane, slice.depth])).toEqual([0, 2, 4].map((i) => [slices[i]?.plane, slices[i]?.depth]));
    const shown = (merged.slices ?? []).flatMap((slice) => slice.image.split(", "));
    expect([...shown].sort()).toEqual(slices.map((slice) => slice.image).sort());
    // The rest of the stage is the card's as it was.
    expect({ ...merged, slices: undefined }).toEqual({ ...stage, slices: undefined });
    // Eight slices run two, three and three.
    const eight = { ...stage, slices: [...slices, { image: "url(s7)", plane: "nearest" as const, depth: 1 }] };
    expect(mergeStage(eight, FX_PHONE_STAGE_LAYERS).slices?.map((slice) => slice.image.split(", ").length)).toEqual([2, 3, 3]);
  });

  it("R4493000 a four-plane stage merges its near and nearest planes", () => {
    const stage: FxTakeoverStage = { planes: ART_PLANES.map((plane) => `url(${plane})`), name: "Mythic", face: "script" };
    // The places keep their planes' dollies; the nearest's picture rides over the near one's, and its place stays empty.
    expect(mergeStage(stage, FX_PHONE_STAGE_LAYERS).planes).toEqual(["url(far)", "url(middle)", "url(nearest), url(near)", ""]);
  });

  it("R4493000 a stage within the cap is unchanged", () => {
    const stage = voidStage();
    const three = { ...stage, slices: (stage.slices ?? []).slice(0, 3) };
    expect(mergeStage(three, FX_PHONE_STAGE_LAYERS)).toBe(three);
    const planes: FxTakeoverStage = { planes: ["url(a)", "url(b)"], name: "Mythic", face: "script" };
    expect(mergeStage(planes, FX_PHONE_STAGE_LAYERS)).toBe(planes);
    expect(mergeStage(stage, stage.slices?.length ?? 0)).toBe(stage);
  });
});

describe("R4493001 the layer budget", () => {
  it("R4493001 the budget applies on a phone either way up, on low deviceMemory at any width, not on desktop", () => {
    expect(isPhoneViewport(PHONE)).toBe(true);
    expect(isPhoneViewport({ ...PHONE, width: 844, height: 390 })).toBe(true);
    expect(isPhoneViewport({ ...DESKTOP, width: FX_MOBILE_WIDTH, height: FX_MOBILE_WIDTH })).toBe(false);
    expect(hasLayerBudget(PHONE)).toBe(true);
    expect(hasLayerBudget({ ...PHONE, width: 844, height: 390 })).toBe(true);
    expect(hasLayerBudget(LOW_DESKTOP)).toBe(true);
    expect(hasLayerBudget({ ...LOW_DESKTOP, memoryGb: FX_LOW_DEVICE_MEMORY_GB * 2 })).toBe(false);
    expect(hasLayerBudget(DESKTOP)).toBe(false);
    expect(hasLayerBudget({ ...DESKTOP, memoryGb: null })).toBe(false);
    // The budget is its megabytes over the window's pixels at its ratio, never under 1x.
    expect(layerBudgetScreens(PHONE)).toBeCloseTo((200 * 1_048_576) / (390 * 844 * 9 * 4), 9);
    expect(layerBudgetScreens({ ...PHONE, dpr: 0.5 })).toBe(layerBudgetScreens({ ...PHONE, dpr: 1 }));
    expect(layerBudgetScreens({ ...PHONE, width: 0 })).toBe(Number.POSITIVE_INFINITY);
    // The browser's device: its window, its ratio and its memory where it reports one.
    Object.defineProperty(navigator, "deviceMemory", { value: 2, configurable: true });
    expect(readDevice()).toEqual({ width: window.innerWidth, height: window.innerHeight, dpr: window.devicePixelRatio, memoryGb: 2 });
    Reflect.deleteProperty(navigator, "deviceMemory");
    expect(readDevice().memoryGb).toBeNull();
  });

  it("R4493001 the budget reduces a takeover in order: merged, artless, bare, its dark and its sweep", () => {
    const full = cue(voidStage(), "data:,");
    const budget = layerBudgetScreens(PHONE);
    const merged = takeoverForm(full, TAKEOVER_MERGED);
    const artless = takeoverForm(full, TAKEOVER_ARTLESS);
    const bare = takeoverForm(full, TAKEOVER_BARE);
    const dark = takeoverForm(full, TAKEOVER_DARK);
    expect(takeoverForm(full, TAKEOVER_FULL)).toBe(full);
    expect(merged.stage?.slices).toHaveLength(FX_PHONE_STAGE_LAYERS);
    expect(merged.art).toBe("data:,");
    expect(artless.stage?.slices).toHaveLength(FX_PHONE_STAGE_LAYERS);
    expect(artless.art).toBeUndefined();
    expect(artless.artless).toBe(true);
    expect(bare.stage).toBeNull();
    expect(bare.artless).toBe(true);
    expect(bare.sweepOnly).toBeUndefined();
    expect(dark.stage).toBeNull();
    expect(dark.artless).toBe(true);
    expect(dark.sweepOnly).toBe(true);
    // Each form holds less than the one before it.
    expect(cueScreens(merged)).toBeLessThan(cueScreens(full));
    expect(cueScreens(artless)).toBeLessThan(cueScreens(merged));
    expect(cueScreens(bare)).toBeLessThan(cueScreens(artless));
    expect(cueScreens(dark)).toBeLessThan(cueScreens(bare));
    // A phone always merges; then each step goes only when what is held leaves no room for the one before it.
    expect(budgetTakeover(full, 0, PHONE)).toEqual(merged);
    expect(budgetTakeover(full, budget - cueScreens(merged), PHONE)).toEqual(merged);
    expect(budgetTakeover(full, budget - cueScreens(merged) + 0.01, PHONE)).toEqual(artless);
    expect(budgetTakeover(full, budget - cueScreens(artless), PHONE)).toEqual(artless);
    expect(budgetTakeover(full, budget - cueScreens(artless) + 0.01, PHONE)).toEqual(bare);
    expect(budgetTakeover(full, budget - cueScreens(bare), PHONE)).toEqual(bare);
    expect(budgetTakeover(full, budget - cueScreens(bare) + 0.01, PHONE)).toEqual(dark);
    // Never less than its dark and its sweep: the takeover always mounts, however full the budget is.
    const over = budgetTakeover(full, budget * 10, PHONE);
    expect(over).toEqual(dark);
    expect(over.kind).toBe("takeover");
    // A takeover with no set piece has no stage to merge or drop: its art goes, then its rays and wave.
    const plain = cue(null, "data:,");
    expect(budgetTakeover(plain, budget - cueScreens(takeoverForm(plain, TAKEOVER_ARTLESS)), PHONE)).toEqual(takeoverForm(plain, TAKEOVER_ARTLESS));
    expect(budgetTakeover(plain, budget, PHONE)).toEqual(takeoverForm(plain, TAKEOVER_DARK));
  });

  it("R4493001 a low-memory desktop keeps every slice until the budget is passed", () => {
    const full = cue(voidStage());
    const budget = layerBudgetScreens(LOW_DESKTOP);
    expect(budgetTakeover(full, 0, LOW_DESKTOP)).toBe(full);
    expect(budgetTakeover(full, budget - cueScreens(full), LOW_DESKTOP)).toBe(full);
    const merged = budgetTakeover(full, budget - cueScreens(full) + 0.01, LOW_DESKTOP);
    expect(merged.stage?.slices).toHaveLength(FX_PHONE_STAGE_LAYERS);
  });

  it("R4493001 desktop gets its cue unchanged", () => {
    const full = cue(voidStage(), "data:,");
    expect(budgetTakeover(full, 0, DESKTOP)).toBe(full);
    expect(budgetTakeover(full, 1e6, DESKTOP)).toBe(full);
    expect(budgetTakeover(full, 1e6, { ...DESKTOP, memoryGb: null })).toBe(full);
  });
});
