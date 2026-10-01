// C+ #6 Wrong-House Attacker — SPEC §8.7 row 6, BUILD M9 Classic+ row C+ 6: "Rush, Lifesteal,
// Poisonous: attacks a unit on its summon turn but not the hero, its hit destroys any unit it damages
// and heals your hero by the amount dealt; radiant 2/2 with Reborn, returning once at 1 health without
// Reborn". Keywords only: both faces are catalog data, applied by the layers (§10.4).

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/006-wrong-house-attacker";

const WRONG = "classicplus-006";
const MENACE = "core-019"; // (3) 9/9 Taunt.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: WRONG, radiant: radiantFace }, HIT_JOB, HIT_JOB, FILLER], library: [STOCKPILE, STOCKPILE], health: 20, mana: 8 },
    p2: { hand: [FILLER], library: [STOCKPILE], field: [MENACE] },
  });
}

describe("C+ #6 Wrong-House Attacker", () => {
  it("is a (1) 1/1 Human Unit with Rush, Lifesteal, Poisonous (Radiant 2/2 plus Reborn); no script on either face", () => {
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([1, 1, 1, 2, 2]);
    expect(def.base.keywords.map((k) => k.kind)).toEqual(["Rush", "Lifesteal", "Poisonous"]);
    expect(def.radiant.keywords.map((k) => k.kind)).toEqual(["Rush", "Lifesteal", "Poisonous", "Reborn"]);
    expect(base).toEqual({});
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("Rush: on its summon turn it may attack a unit but not the hero", () => {
      const s = setup();
      s.play(WRONG, { zone: 1 });

      expect(() => s.attack(WRONG, "hero")).toThrow();
      s.attack(WRONG, MENACE);
      expect(s.events.some((event) => event.type === "attackDeclared")).toBe(true);
    });

    it("Poisonous and Lifesteal: its hit destroys the 9/9 it damages and heals your hero by the 1 dealt", () => {
      const s = setup();
      const menace = s.card(MENACE);
      s.play(WRONG, { zone: 1 });

      s.attack(WRONG, menace);

      s.expectInZone(menace, "graveyard");
      s.expectInZone(WRONG, "graveyard");
      s.expectHealth("p1", 21);
    });
  });

  describe("radiant", () => {
    it("R64 Reborn: it returns once at 1 health without Reborn, then dies for good", () => {
      const s = setup(true);
      const wrong = s.card(WRONG);
      s.play(wrong, { zone: 1 });
      s.expectStats(wrong, { attack: 2, health: 2 });

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: wrong.id }] });

      const back = s.unit("p1", 1);
      expect(back?.defId).toBe(WRONG);
      s.expectStats(back!, { attack: 2, health: 1 });
      expect(s.stats(back!).keywords.map((k) => k.kind)).not.toContain("Reborn");
      expect(s.stats(back!).keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Rush", "Lifesteal", "Poisonous"]));

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: back!.id }] });

      s.expectInZone(wrong, "graveyard");
      expect(s.unit("p1", 1)).toBeNull();
    });
  });
});
