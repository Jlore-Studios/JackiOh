// The aura looks' names and scopes (fx/aura/looks.ts, R2787000): a card's standing look is its
// entry's `aura` field, which the aura layer draws. The predicates' boundaries are asserted, so a
// mutant accepting a wrong name, or a table missing a look, fails here.

import { describe, expect, it } from "vitest";

import {
  AURA_FIELD_TYPES,
  AURA_LOOK_NAMES,
  AURA_LOOKS,
  AURA_SCOPES,
  AURA_SIGNATURE_LOOK_NAMES,
  AURA_SIGNATURE_LOOKS,
  auraLookSpec,
  isAuraFieldType,
  isAuraLook,
  isAuraScope,
  isSignatureAuraLook,
} from "./looks.ts";

describe("aura looks", () => {
  it("accepts every shared and signature look, and nothing else", () => {
    for (const name of AURA_LOOK_NAMES) expect(isAuraLook(name)).toBe(true);
    for (const name of AURA_SIGNATURE_LOOK_NAMES) expect(isAuraLook(name)).toBe(true);
    expect(isAuraLook("ember")).toBe(true);
    expect(isAuraLook("seeingIris")).toBe(true);
    expect(isAuraLook("nope")).toBe(false);
    expect(isAuraLook("")).toBe(false);
  });

  it("keeps the shared and signature names apart", () => {
    expect(isSignatureAuraLook("seeingIris")).toBe(true);
    expect(isSignatureAuraLook("ember")).toBe(false);
    expect(isSignatureAuraLook("nope")).toBe(false);
  });

  it("accepts the five scopes and the two field types, and nothing else", () => {
    expect(AURA_SCOPES).toEqual(["tile", "zone", "lane", "side", "board"]);
    for (const scope of AURA_SCOPES) expect(isAuraScope(scope)).toBe(true);
    expect(isAuraScope("card")).toBe(false);
    expect(AURA_FIELD_TYPES).toEqual(["Field Spell", "Field Trap"]);
    expect(isAuraFieldType("Field Spell")).toBe(true);
    expect(isAuraFieldType("Field Trap")).toBe(true);
    expect(isAuraFieldType("Trap")).toBe(false);
  });

  it("draws every named look from a spec of its own", () => {
    expect(Object.keys(AURA_LOOKS).sort()).toEqual([...AURA_LOOK_NAMES].sort());
    expect(Object.keys(AURA_SIGNATURE_LOOKS).sort()).toEqual([...AURA_SIGNATURE_LOOK_NAMES].sort());
    for (const [name, spec] of Object.entries(AURA_LOOKS)) {
      const [lifeMin, lifeMax] = spec.life;
      const [sizeMin, sizeMax] = spec.size;
      expect(lifeMin, name).toBeLessThan(lifeMax);
      expect(sizeMin, name).toBeLessThanOrEqual(sizeMax);
      expect(spec.alpha, name).toBeGreaterThan(0);
    }
  });

  it("auraLookSpec reads the shared and the signature tables", () => {
    expect(auraLookSpec("ember")).toBe(AURA_LOOKS.ember);
    expect(auraLookSpec("seeingIris")).toBe(AURA_SIGNATURE_LOOKS.seeingIris);
  });
});
