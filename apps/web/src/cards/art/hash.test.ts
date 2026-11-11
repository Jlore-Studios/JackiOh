// The procedural art's number helpers (cards/art/hash.ts): exact integer arithmetic or plain
// rounding, never Math.random or the clock, so one id draws one picture everywhere. A mutant
// changing the hash, the rounding or the formatting fails here.

import { describe, expect, it } from "vitest";

import { fmt, hashId, round2 } from "./hash.ts";

describe("procedural art numbers", () => {
  it("hashId is FNV-1a over the id's UTF-8 bytes, stable for every id", () => {
    expect(hashId("")).toBe(0x811c9dc5);
    expect(hashId("core-004")).toBe(hashId("core-004"));
    expect(hashId("core-004")).not.toBe(hashId("core-005"));
    expect(Number.isInteger(hashId("core-004"))).toBe(true);
    expect(hashId("core-004")).toBeGreaterThanOrEqual(0);
    expect(hashId("core-004")).toBeLessThan(2 ** 32);
  });

  it("round2 rounds to hundredths and never returns -0", () => {
    expect(round2(1.234)).toBe(1.23);
    expect(round2(1.236)).toBe(1.24);
    expect(round2(-0.001)).toBe(0);
    expect(Object.is(round2(-0.001), -0)).toBe(false);
    expect(round2(2)).toBe(2);
  });

  it("fmt prints the rounded number with no exponent and no -0", () => {
    expect(fmt(2)).toBe("2");
    expect(fmt(1.5)).toBe("1.5");
    expect(fmt(1.25)).toBe("1.25");
    expect(fmt(1.2)).toBe("1.2");
    expect(fmt(0)).toBe("0");
    expect(fmt(-0.001)).toBe("0");
    expect(fmt(0.1 + 0.2)).toBe("0.3");
  });
});
