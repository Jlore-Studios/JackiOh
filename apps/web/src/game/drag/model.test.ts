// The pure drag model beside its integration suites (`./model.ts`, S9, R384, R510, R658).
// The lifts and drops themselves are proved through `<Game/>` in `test/ux/drag-model.test.ts`
// (B34/B35), `test/ux/drag-layer.test.tsx`, `test/ux/drag-continue.test.tsx` and
// `game/activate.test.ts`; here the module's own constants and null edges are pinned, so a
// changed threshold or a lifted nothing fails colocated with the source.

import { describe, expect, it } from "vitest";

import { baseView } from "../../test/fixtures.ts";
import { IDLE } from "../actions.ts";
import { DRAG_THRESHOLD_PX, planDrag } from "./model.ts";

describe("DRAG_THRESHOLD_PX", () => {
  it("keeps the press a click below eight CSS px", () => {
    expect(DRAG_THRESHOLD_PX).toBe(8);
  });
});

describe("planDrag null edges", () => {
  it("lifts nothing from an empty legal list", () => {
    expect(planDrag(baseView(), [], IDLE, { on: "hand", instanceId: "h1" })).toBeNull();
  });

  it("lifts nothing from a hero, a zone or a switch control", () => {
    const view = baseView();
    expect(planDrag(view, [], IDLE, { on: "hero", side: "you" })).toBeNull();
    expect(planDrag(view, [], IDLE, { on: "zone", side: "you", row: "units", lane: 1 })).toBeNull();
  });
});

describe("planDrag switch control", () => {
  it("lifts nothing from a switch control", () => {
    expect(
      planDrag(baseView(), [], IDLE, { on: "switch", instanceId: "u2" } as never),
    ).toBeNull();
  });
});
