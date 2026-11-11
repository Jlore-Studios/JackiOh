// R3351000: the living game's shared shed level (issue #3351, part of #3330).

import { describe, expect, it } from "vitest";

import {
  resetShedForTests,
  setShedLevel,
  setShedPointer,
  SHED_AURAS,
  SHED_FACES,
  SHED_FULL,
  SHED_MAX,
  SHED_MOTES,
  shedAuraScale,
  shedEmissionScale,
  shedLevel,
  shedPointer,
  shedStillsFaces,
  subscribeShed,
} from "./shed.ts";

describe("R3351000 the shared shed level", () => {
  it("R3351000 the stages run full, motes, far auras, faces, and the level starts full", () => {
    resetShedForTests();
    expect(SHED_FULL).toBe(0);
    expect(SHED_MOTES).toBe(1);
    expect(SHED_AURAS).toBe(2);
    expect(SHED_FACES).toBe(SHED_MAX);
    expect(shedLevel()).toBe(SHED_FULL);
    resetShedForTests();
  });

  it("R3351000 the level clamps to 0..SHED_MAX and tells its listeners only when it changes", () => {
    resetShedForTests();
    let calls = 0;
    const stop = subscribeShed(() => {
      calls += 1;
    });
    setShedLevel(1);
    expect(shedLevel()).toBe(1);
    expect(calls).toBe(1);
    setShedLevel(1);
    expect(calls).toBe(1);
    setShedLevel(99);
    expect(shedLevel()).toBe(SHED_MAX);
    setShedLevel(-4);
    expect(shedLevel()).toBe(SHED_FULL);
    stop();
    setShedLevel(2);
    expect(calls).toBe(3);
    resetShedForTests();
  });

  it("R3351000 motes shed first, then far auras, then the faces past the pointer's own, never backwards", () => {
    expect(shedEmissionScale(SHED_FULL)).toBe(1);
    expect(shedAuraScale(SHED_FULL)).toBe(1);
    expect(shedStillsFaces(SHED_FULL)).toBe(false);
    // Stage 1 halves mote emission while auras and faces stay whole.
    expect(shedEmissionScale(SHED_MOTES)).toBeLessThan(1);
    expect(shedAuraScale(SHED_MOTES)).toBe(1);
    expect(shedStillsFaces(SHED_MOTES)).toBe(false);
    // Stage 2 halves aura intensity too while the faces still move.
    expect(shedEmissionScale(SHED_AURAS)).toBeLessThan(1);
    expect(shedAuraScale(SHED_AURAS)).toBeLessThan(1);
    expect(shedStillsFaces(SHED_AURAS)).toBe(false);
    // Stage 3 stills the faces past the pointer's own and sheds nothing back.
    expect(shedStillsFaces(SHED_FACES)).toBe(true);
    expect(shedEmissionScale(SHED_MAX)).toBeLessThan(1);
    expect(shedAuraScale(SHED_MAX)).toBeLessThan(1);
  });

  it("R3351000 the pointer starts unknown and only a move tells the listeners", () => {
    resetShedForTests();
    expect(shedPointer()).toBeNull();
    let calls = 0;
    const stop = subscribeShed(() => {
      calls += 1;
    });
    setShedPointer(null);
    expect(calls).toBe(0);
    setShedPointer({ x: 10, y: 20 });
    expect(shedPointer()).toEqual({ x: 10, y: 20 });
    expect(calls).toBe(1);
    setShedPointer({ x: 10, y: 20 });
    expect(calls).toBe(1);
    setShedPointer(null);
    expect(shedPointer()).toBeNull();
    stop();
    resetShedForTests();
  });
});
