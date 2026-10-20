// The raster of a shared GIF (#513, R513000), over a recording stand-in for the 2D context, since jsdom
// draws nothing: every item of a plan is drawn in order, a card the assets lack is a blank face, and the
// loop reports its frames, stops when cancelled and gives up past the byte cap. What the pixels look like
// is the component spec's (`e2e/cypress/component/replay-gif.cy.tsx`).

import { CATALOG } from "@jackioh/cards";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { lookupFromDefs } from "../../game/catalog.ts";
import { fullBoardView } from "../../test/fixtures.ts";
import { REPLAY_GIF_FPS, REPLAY_GIF_SIZES } from "../config.ts";
import { assetsFor } from "./export.ts";
import { planFrames } from "./plan.ts";
import type { GifPlan } from "./plan.ts";
import { drawFrame, rasterize } from "./raster.ts";
import type { Ctx2D, GifAssets } from "./raster.ts";

type Call = { name: string; args: unknown[] };

/** A context that records every call and assignment and answers `getImageData` with black. */
function recorder(): { ctx: Ctx2D; calls: Call[] } {
  const calls: Call[] = [];
  const gradient = { addColorStop: (...args: unknown[]) => calls.push({ name: "addColorStop", args }) };
  const ctx = new Proxy(
    {},
    {
      get(_target, name) {
        if (typeof name !== "string") return undefined;
        if (name === "getImageData") return (_x: number, _y: number, w: number, h: number) => ({ data: new Uint8ClampedArray(w * h * 4) });
        if (name === "createLinearGradient" || name === "createRadialGradient") return () => gradient;
        return (...args: unknown[]) => calls.push({ name, args });
      },
      set(_target, name, value) {
        calls.push({ name: `set ${String(name)}`, args: [value] });
        return true;
      },
    },
  ) as unknown as Ctx2D;
  return { ctx, calls };
}

const lookup = lookupFromDefs(CATALOG);
const view = fullBoardView();
const clip = [{ step: 0, turn: view.turn, view }];
const SQUARE = { ...REPLAY_GIF_SIZES.square, fps: REPLAY_GIF_FPS, catalogVersion: "v0.3.6", portraits: { p1: "vanilla", p2: "shredder" } } as const;

beforeEach(() => {
  vi.stubGlobal("Path2D", class Path2DStub {});
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the GIF raster", () => {
  it("draws every item of a frame, the names of its cards and its text", () => {
    const plan = planFrames(clip, SQUARE);
    const assets = assetsFor(plan, clip, lookup);
    const [frame] = plan.frames;
    if (frame === undefined) throw new Error("no frame");
    const { ctx, calls } = recorder();
    drawFrame(ctx, frame, assets);
    const text = calls.filter((call) => call.name === "fillText").map((call) => call.args[0]);
    for (const item of frame.items) {
      if (item.kind === "text") expect(text).toContain(item.text);
      if (item.kind === "card") expect(text, `${item.defId}'s name`).toContain(assets.faces[item.defId + (item.radiant ? "+radiant" : "")]?.name);
    }
    // A face-down card is a back and a portrait is clipped to its oval.
    expect(calls.filter((call) => call.name === "clip").length).toBeGreaterThanOrEqual(2 + frame.items.filter((item) => item.kind === "portrait").length);
    expect(calls.some((call) => call.name === "ellipse")).toBe(true);
  });

  it("a card the assets lack is a blank face with a question mark", () => {
    const plan = planFrames(clip, SQUARE);
    const [frame] = plan.frames;
    if (frame === undefined) throw new Error("no frame");
    const { ctx, calls } = recorder();
    drawFrame(ctx, frame, { faces: {}, portraits: {} });
    const text = calls.filter((call) => call.name === "fillText").map((call) => call.args[0]);
    expect(text).toContain("?");
  });

  it("rasterize reports each frame and returns a GIF", async () => {
    const plan: GifPlan = planFrames(clip, SQUARE);
    const assets: GifAssets = assetsFor(plan, clip, lookup);
    const { ctx } = recorder();
    const frames: number[] = [];
    let between = 0;
    const bytes = await rasterize(ctx, plan, assets, {
      maxBytes: REPLAY_GIF_SIZES.square.width * 1000,
      aborted: () => false,
      onFrame: (n) => frames.push(n),
      between: () => {
        between += 1;
        return Promise.resolve();
      },
    });
    expect(frames).toEqual(plan.frames.map((_, i) => i + 1));
    expect(between).toBe(plan.frames.length);
    expect(String.fromCharCode(...(bytes?.subarray(0, 6) ?? []))).toBe("GIF89a");
    expect(bytes?.at(-1)).toBe(0x3b);
  });

  it("rasterize stops with an AbortError when cancelled and gives up past the byte cap", async () => {
    const two = planFrames([...clip, { step: 1, turn: view.turn, view }], SQUARE);
    expect(two.frames.length).toBeGreaterThan(1);
    const assets = assetsFor(two, clip, lookup);
    let drawn = 0;
    await expect(
      rasterize(recorder().ctx, two, assets, { maxBytes: 1e9, onFrame: (n) => (drawn = n), aborted: () => drawn >= 1 }),
    ).rejects.toMatchObject({ name: "AbortError" });
    expect(drawn).toBe(1);

    const over = vi.fn();
    expect(await rasterize(recorder().ctx, two, assets, { maxBytes: 1, onFrame: over, aborted: () => false })).toBeNull();
    expect(over, "it gave up before reporting a frame").not.toHaveBeenCalled();
  });
});
