import { describe, expect, it } from "vitest";

import { DAMAGE_FEEL, damageFeel, damageTier } from "./damageFeel.ts";

describe("damage feel", () => {
  it("classifies the requested boundary hits", () => {
    expect([1, 5, 8, 15, 25].map(damageTier)).toEqual(["tiny", "normal", "moderate", "big", "giga"]);
  });

  it("keeps prevented damage tiny and exposes all tuning from one config", () => {
    expect(damageTier(0)).toBe("tiny");
    expect(damageFeel(25)).toBe(DAMAGE_FEEL.giga);
    expect(DAMAGE_FEEL.moderate.crowd).toBe("ooh");
    expect(DAMAGE_FEEL.big.shakePx).toBe(6);
  });
});
