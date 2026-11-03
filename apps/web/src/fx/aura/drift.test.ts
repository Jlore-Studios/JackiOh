// R2787000: the auras' motes: each aura keeps exactly its budget inside its box, a begun aura's burst
// leaves the box's centre, an ended aura's fade leaves nothing drawn, and one seed gives the same motes.

import { describe, expect, it } from "vitest";

import { AURA_FADE_OUT_MS, FX_MAX_DT_MS } from "../constants.ts";
import { FLAVOR_LOOKS } from "../flavors.ts";
import { createRng } from "../rng.ts";
import type { FxBox } from "../types.ts";
import { createAuraDrift, type AuraDriftItem } from "./drift.ts";
import { AURA_LOOK_NAMES } from "./looks.ts";
import type { AuraPhase } from "./plan.ts";

const BOX: FxBox = { x: 100, y: 50, width: 240, height: 120 };

function item(key: string, budget: number, phase: AuraPhase = "standing", box: FxBox = BOX): AuraDriftItem {
  return { key, box, look: "ember", colors: FLAVOR_LOOKS.Fire.colors, budget, phase };
}

/** A 2D context that counts the arcs it is asked to draw and keeps the state it is left in. */
function countingContext() {
  const calls = { arc: 0, fill: 0 };
  const ctx = {
    globalAlpha: 1,
    globalCompositeOperation: "source-over",
    fillStyle: "#000000",
    beginPath: (): void => undefined,
    arc: (): void => {
      calls.arc += 1;
    },
    fill: (): void => {
      calls.fill += 1;
    },
  };
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls, state: ctx };
}

function stepFor(drift: ReturnType<typeof createAuraDrift>, ms: number): void {
  for (let t = 0; t < ms; t += FX_MAX_DT_MS) drift.step(Math.min(FX_MAX_DT_MS, ms - t));
}

describe("R2787000 the auras' motes", () => {
  it("R2787000 each aura keeps exactly its budget, trimmed and topped up, and a gone aura's motes go with it", () => {
    const drift = createAuraDrift({ rng: createRng(3) });
    drift.sync([item("a", 5), item("b", 3)]);
    expect(drift.count()).toBe(8);
    expect(drift.count("a")).toBe(5);
    expect(drift.size()).toBe(2);
    drift.sync([item("a", 2), item("b", 6)]);
    expect([drift.count("a"), drift.count("b")]).toEqual([2, 6]);
    drift.sync([item("b", 6)]);
    expect(drift.size()).toBe(1);
    expect(drift.count("a")).toBe(0);
    expect(drift.count()).toBe(6);
    stepFor(drift, 10_000);
    expect(drift.count()).toBe(6);
    drift.sync([item("b", 0)]);
    expect(drift.count()).toBe(0);
    drift.clear();
    expect(drift.size()).toBe(0);
  });

  it("R2787000 a begun aura's motes burst from its box's centre, a standing one's are spread through it", () => {
    const drift = createAuraDrift({ rng: createRng(5) });
    drift.sync([item("begun", 12, "begin"), item("standing", 12)]);
    const centre = { x: BOX.x + BOX.width / 2, y: BOX.y + BOX.height / 2 };
    for (const mote of drift.motes("begun")) {
      expect(mote).toMatchObject({ x: centre.x, y: centre.y, age: 0 });
    }
    expect(drift.motes("standing").some((mote) => mote.x !== centre.x || mote.y !== centre.y)).toBe(true);
    drift.step(FX_MAX_DT_MS);
    const moved = drift.motes("begun").map((mote) => Math.hypot(mote.x - centre.x, mote.y - centre.y));
    expect(Math.min(...moved)).toBeGreaterThan(0);
  });

  it("R2787000 an ended aura's motes fade to nothing after AURA_FADE_OUT_MS, and the context is left as it was", () => {
    const drift = createAuraDrift({ rng: createRng(7) });
    drift.sync([item("a", 16)]);
    stepFor(drift, 400);
    const before = countingContext();
    drift.draw(before.ctx);
    expect(before.calls.arc).toBeGreaterThan(0);
    expect(before.state.globalAlpha).toBe(1);
    expect(before.state.globalCompositeOperation).toBe("source-over");
    drift.sync([item("a", 16, "end")]);
    stepFor(drift, AURA_FADE_OUT_MS / 2);
    const fading = countingContext();
    drift.draw(fading.ctx);
    expect(fading.calls.arc).toBeGreaterThan(0);
    stepFor(drift, AURA_FADE_OUT_MS / 2);
    const after = countingContext();
    drift.draw(after.ctx);
    expect(after.calls.arc).toBe(0);
  });

  it("R2787000 every look's motes stay inside their box, and one seed gives the same motes", () => {
    for (const look of AURA_LOOK_NAMES) {
      const drift = createAuraDrift({ rng: createRng(11) });
      drift.sync([{ ...item("a", 24, "begin"), look }]);
      stepFor(drift, 5_000);
      for (const mote of drift.motes()) {
        expect(mote.x, look).toBeGreaterThanOrEqual(BOX.x);
        expect(mote.x, look).toBeLessThan(BOX.x + BOX.width);
        expect(mote.y, look).toBeGreaterThanOrEqual(BOX.y);
        expect(mote.y, look).toBeLessThan(BOX.y + BOX.height);
        expect(mote.age, look).toBeLessThan(mote.life);
      }
    }
    const run = (): unknown => {
      const drift = createAuraDrift({ rng: createRng(13) });
      drift.sync([item("a", 10, "begin"), item("b", 10)]);
      stepFor(drift, 1_234);
      return drift.motes();
    };
    expect(run()).toEqual(run());
  });

  it("R2787000 a moved box takes its motes with it", () => {
    const drift = createAuraDrift({ rng: createRng(17) });
    drift.sync([item("a", 10)]);
    const moved: FxBox = { x: 600, y: 400, width: 80, height: 40 };
    drift.sync([item("a", 10, "standing", moved)]);
    drift.step(FX_MAX_DT_MS);
    for (const mote of drift.motes()) {
      expect(mote.x).toBeGreaterThanOrEqual(moved.x);
      expect(mote.x).toBeLessThan(moved.x + moved.width);
      expect(mote.y).toBeGreaterThanOrEqual(moved.y);
      expect(mote.y).toBeLessThan(moved.y + moved.height);
    }
  });
});
