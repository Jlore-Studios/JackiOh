// The shared cue builders (fx/build.ts): plain data every recipe reads, bounded by R200. Each
// builder's arithmetic is asserted, so a mutant moving a count, a bound or a delay fails here.

import { describe, expect, it } from "vitest";

import { FX_MAX_TAIL_MS, FX_RAYS_TAIL_MS, FX_RING_MS } from "./constants.ts";
import { frac, ghostCue, ringCue, raysCue, shakeCues, tid, tunedBurst } from "./build.ts";

const AT = { kind: "testid", testid: "fx-board" } as const;

describe("fx cue builders", () => {
  it("tid anchors to a testid, and to a point when one is given", () => {
    expect(tid("fx-board")).toEqual({ kind: "testid", testid: "fx-board" });
    expect(tid("fx-board", { x: 3, y: 4 })).toEqual({ kind: "testid", testid: "fx-board", at: { x: 3, y: 4 } });
  });

  it("frac rounds a fraction of a duration to a whole millisecond", () => {
    expect(frac(0.5, 1000)).toBe(500);
    expect(frac(0.55, 1000)).toBe(550);
    expect(frac(1 / 3, 900)).toBe(300);
  });

  it("tunedBurst scales the count with the intensity but never below one, and passes power through", () => {
    const burst = tunedBurst(1, "spark", AT, "area", 40, { count: 10, power: 2 });
    expect(burst).toMatchObject({ kind: "burst", preset: "spark", delayMs: 40, count: 10, power: 2 });

    expect(tunedBurst(0.5, "spark", AT, "area", 0, { count: 10, power: 2 }).count).toBe(5);
    // Zero intensity still draws one particle; the power is never intensity's.
    expect(tunedBurst(0, "spark", AT, "area", 0, { count: 10, power: 2 }).count).toBe(1);
    expect(tunedBurst(0, "spark", AT, "area", 0, { count: 10, power: 2 }).power).toBe(2);
  });

  it("ringCue caps the ring at FX_RING_MS within the entry's tail", () => {
    expect(ringCue(2000, "holy", AT, 100).durationMs).toBe(
      Math.min(FX_RING_MS, 2000 - 100 + FX_MAX_TAIL_MS),
    );
    // A short entry still caps at the ring's own length: the tail cannot stretch it.
    expect(ringCue(100, "holy", AT, 90).durationMs).toBe(FX_RING_MS);
  });

  it("raysCue runs to the entry's end plus the rays tail", () => {
    expect(raysCue(2000, "holy", AT, 100).durationMs).toBe(2000 - 100 + FX_RAYS_TAIL_MS);
  });

  it("shakeCues shakes nothing at zero trauma, and clamps the trauma at one", () => {
    expect(shakeCues(0, 3, 50)).toEqual([]);
    expect(shakeCues(1, 0, 50)).toEqual([]);
    const [cue] = shakeCues(1, 0.5, 50);
    expect(cue).toMatchObject({ kind: "shake", trauma: 0.5, delayMs: 50 });
    const [clamped] = shakeCues(4, 2, 0);
    expect(clamped).toMatchObject({ trauma: 1 });
  });

  it("ghostCue carries the flight from one anchor to another over the entry", () => {
    expect(ghostCue(900, AT, tid("fx-hand"))).toEqual({
      kind: "ghost",
      from: AT,
      to: { kind: "testid", testid: "fx-hand" },
      delayMs: 0,
      durationMs: 900,
    });
  });
});
