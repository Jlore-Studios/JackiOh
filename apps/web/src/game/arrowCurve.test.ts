// The curve both board arrows draw (`arrowCurve.ts`): bowed upward on screen whichever way it points.

import { describe, expect, it } from "vitest";

import { arrowCurve } from "./arrowCurve.ts";

describe("arrowCurve", () => {
  it("bows the curve upward on screen whichever way the arrow points", () => {
    const rightward = arrowCurve({ x: 0, y: 100 }, { x: 1000, y: 100 }, 0.22, 90);
    const leftward = arrowCurve({ x: 1000, y: 100 }, { x: 0, y: 100 }, 0.22, 90);
    // A bend of 22% of 1000 px is 220 px, held to 90 px: the control point stands 90 px above the middle.
    for (const curve of [rightward, leftward]) {
      expect(curve.cx).toBeCloseTo(500);
      expect(curve.cy).toBeCloseTo(10);
    }
  });

  it("bends by its share of the length while that is under the most", () => {
    const curve = arrowCurve({ x: 0, y: 0 }, { x: 0, y: 200 }, 0.25, 90);
    // Straight down: 25% of 200 px is 50 px off the middle, to the side (a vertical arrow has no upper side).
    expect(curve.cx).toBeCloseTo(-50);
    expect(curve.cy).toBeCloseTo(100);
  });

  it("returns the straight length and a unit tangent at the tip", () => {
    const curve = arrowCurve({ x: 0, y: 100 }, { x: 1000, y: 100 }, 0.22, 90);
    expect(curve.length).toBe(1000);
    expect(Math.hypot(curve.ux, curve.uy)).toBeCloseTo(1);
    // The tip's tangent runs from the control point to the end, so it heads right and down.
    expect(curve.ux).toBeGreaterThan(0);
    expect(curve.uy).toBeGreaterThan(0);
  });

  it("measures an arrow whose ends meet as 1 long, so nothing divides by zero", () => {
    const curve = arrowCurve({ x: 5, y: 5 }, { x: 5, y: 5 }, 0.22, 90);
    expect(curve.length).toBe(1);
    expect(Number.isFinite(curve.ux) && Number.isFinite(curve.uy)).toBe(true);
  });
});
