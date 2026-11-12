// R2784000's motif names (fx/motifs/names.ts): the names a card's effects table may give a motif
// and the hooks it may give one. `aura` is a name but never a drawn motif, and waits for #2787;
// a mutant drawing it, or accepting a wrong name, fails here.

import { describe, expect, it } from "vitest";

import {
  FX_CARD_HOOKS,
  FX_DRAWN_MOTIFS,
  FX_MOTIF_NAMES,
  isCardHook,
  isDrawnMotif,
  isMotifName,
} from "./names.ts";

describe("fx motif names", () => {
  it("accepts every table name, and nothing else", () => {
    for (const name of FX_MOTIF_NAMES) expect(isMotifName(name)).toBe(true);
    expect(isMotifName("bolt")).toBe(true);
    expect(isMotifName("aura")).toBe(true);
    expect(isMotifName("nope")).toBe(false);
    expect(isMotifName("")).toBe(false);
  });

  it("draws every name but aura, which waits for #2787", () => {
    expect(FX_DRAWN_MOTIFS).toHaveLength(FX_MOTIF_NAMES.length - 1);
    expect(FX_DRAWN_MOTIFS as readonly string[]).not.toContain("aura");
    for (const name of FX_DRAWN_MOTIFS) expect(isDrawnMotif(name)).toBe(true);
    expect(isDrawnMotif("aura")).toBe(false);
    expect(isDrawnMotif("nope")).toBe(false);
  });

  it("accepts the five hooks R2797000 names, and nothing else", () => {
    expect(FX_CARD_HOOKS).toEqual(["cast", "cry", "death", "trigger", "activate"]);
    for (const hook of FX_CARD_HOOKS) expect(isCardHook(hook)).toBe(true);
    expect(isCardHook("summon")).toBe(false);
    expect(isCardHook("")).toBe(false);
  });
});
