// C+ #12.5 Anti-Waffle Shell — SPEC §8.7 row 12.5, BUILD M9 Classic+ row C+ 12.5: "Field Spell: its Cry
// gives every Unit you control Divine Shield (later Units get none); Aura: your Units have +2/+2 while
// it is on the field, later ones included, gone when it leaves; the aura reads through `param()`;
// radiant +4/+4".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { expectAnimated } from "../_animated";

const SHELL = "classicplus-012-5";
const MENACE = "core-019"; // (3) 9/9
const POINTMASTER = "core-020"; // (2) 7/1
const HIT_JOB = "core-016"; // (3) destroy target Unit
const FILLER = "core-005";

function shell(radiant: boolean): Scenario {
  const s = scenario({
    p1: { hand: [{ def: SHELL, radiant }, POINTMASTER, HIT_JOB, FILLER], field: [MENACE], mana: 8 },
    p2: { hand: [FILLER], field: [MENACE] },
  });
  s.play(SHELL);
  return s;
}

const hasShield = (s: Scenario, ref: string): boolean => s.stats(ref).keywords.some((k) => k.kind === "Divine Shield");

describe("C+ #12.5 Anti-Waffle Shell", () => {
  describe("base", () => {
    it("its Cry gives each Unit you control Divine Shield; the opponent's get none", () => {
      const s = shell(false);
      expect(hasShield(s, s.unit("p1", 1)?.id ?? "")).toBe(true);
      expect(hasShield(s, s.unit("p2", 1)?.id ?? "")).toBe(false);
    });

    it("Aura: your Units have +2/+2, the opponent's don't", () => {
      const s = shell(false);
      expect(s.stats(s.unit("p1", 1)?.id ?? "")).toMatchObject({ attack: 11, maxHealth: 11 });
      expect(s.stats(s.unit("p2", 1)?.id ?? "")).toMatchObject({ attack: 9, maxHealth: 9 });
    });

    it("a later Unit gets the aura but no Divine Shield", () => {
      const s = shell(false);
      s.play(POINTMASTER);
      s.expectStats(POINTMASTER, { attack: 9, health: 3 });
      expect(hasShield(s, POINTMASTER)).toBe(false);
    });

    it("the aura is gone when it leaves; the granted Divine Shield stays", () => {
      const s = shell(false);
      // Animated since patch v0.2.10 (R383), it stands in a unit zone, so a Unit's destroy takes it, and
      // a token that is a Unit ceases to exist as it leaves the field (§3.2, R11).
      const shellId = s.card(SHELL).id;
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: shellId }] });
      s.expectInZone(shellId, "gone");
      expect(s.stats(s.unit("p1", 1)?.id ?? "")).toMatchObject({ attack: 9, maxHealth: 9 });
      expect(hasShield(s, s.unit("p1", 1)?.id ?? "")).toBe(true);
    });

    it("R386 the aura reads through param(): an Upgrade makes it +3/+3", () => {
      const s = scenario({ p1: { hand: [SHELL, FILLER], field: [MENACE] }, p2: { hand: [FILLER] } });
      stepParam(s.card(SHELL), "aura", 1);
      s.play(SHELL);
      s.expectStats(MENACE, { attack: 12, maxHealth: 12 });
    });
  });

  describe("radiant", () => {
    it("+4/+4, and the Cry's Divine Shield", () => {
      const s = shell(true);
      expect(s.stats(s.unit("p1", 1)?.id ?? "")).toMatchObject({ attack: 13, maxHealth: 13 });
      expect(hasShield(s, s.unit("p1", 1)?.id ?? "")).toBe(true);
    });
  });
});

describe("C+ #12.5 Anti-Waffle Shell: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 2/4 Unit, its own aura's +2/+2 (Radiant +4/+4) on it; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classicplus-012-5", stats: { attack: 2, health: 4 } });
  });

  it("R383 radiant: a 4/8 Unit", () => {
    expectAnimated({ def: "classicplus-012-5", radiant: true, stats: { attack: 4, health: 8 } });
  });
});
