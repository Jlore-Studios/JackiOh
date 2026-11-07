// #3347: the drag ghost leans the way it moves, by its speed, up to a cap, and settles flat.

import { describe, expect, it } from "vitest";

import { DRAG_TILT_FULL_SPEED_PX_PER_S, DRAG_TILT_MAX_DEG, createTiltTracker, tiltFor } from "./tilt.ts";

describe("#3347 the drag tilt", () => {
  it("#3347 the tilt is proportional to the speed below the cap", () => {
    const half = DRAG_TILT_FULL_SPEED_PX_PER_S / 2 / 1000; // pixels in one millisecond at half the full speed
    expect(tiltFor(half, 1)).toBeCloseTo(DRAG_TILT_MAX_DEG / 2);
    expect(tiltFor(half * 2, 2)).toBeCloseTo(DRAG_TILT_MAX_DEG / 2);
    expect(tiltFor(half / 2, 1)).toBeCloseTo(DRAG_TILT_MAX_DEG / 4);
  });

  it("#3347 the tilt is clamped at DRAG_TILT_MAX_DEG either way", () => {
    expect(tiltFor(10_000, 1)).toBe(DRAG_TILT_MAX_DEG);
    expect(tiltFor(-10_000, 1)).toBe(-DRAG_TILT_MAX_DEG);
    expect(tiltFor(DRAG_TILT_FULL_SPEED_PX_PER_S / 1000, 1)).toBeCloseTo(DRAG_TILT_MAX_DEG);
  });

  it("#3347 moving right leans right (positive), moving left leans left (negative), and no move is flat", () => {
    expect(tiltFor(5, 16)).toBeGreaterThan(0);
    expect(tiltFor(-5, 16)).toBeLessThan(0);
    expect(tiltFor(0, 16)).toBe(0);
  });

  it("#3347 no time passed gives 0, never a division by zero", () => {
    expect(tiltFor(50, 0)).toBe(0);
    expect(tiltFor(50, -4)).toBe(0);
  });

  it("#3347 the first move of a tracker is 0 and each next one measures from the one before", () => {
    let t = 1000;
    const tracker = createTiltTracker(() => t);
    expect(tracker.move(100)).toBe(0);
    t += 10;
    const right = tracker.move(110);
    expect(right).toBeCloseTo(tiltFor(10, 10));
    expect(right).toBeGreaterThan(0);
    t += 10;
    expect(tracker.move(100)).toBeLessThan(0);
    t += 10;
    expect(tracker.move(100)).toBe(0);
  });

  it("#3347 a fast step caps at the maximum", () => {
    let t = 0;
    const tracker = createTiltTracker(() => t);
    tracker.move(0);
    t += 1;
    expect(tracker.move(500)).toBe(DRAG_TILT_MAX_DEG);
  });
});
