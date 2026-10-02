// C #36 Burn — SPEC §8.6 row 36, BUILD M9 Classic row C 36: "A target, Unit or hero: deal 2, then draw
// 1 if your current mana as it resolves is 4 or more; `conditionMet` answers in hand whether your mana
// after paying its cost now would be 4 or more (R195), so a surcharge (C #77) moves it; radiant: deal
// 4, and draw if your max mana is 4 or more (§2.3); its name is a rules word, and "burned" elsewhere
// is no reference to it (R381); its tuned numbers (damage, threshold, draw) read through `param()`
// (R386)".
//
// The `conditionMet` proofs (R195) are in `../condition-active.test.ts`, with the other cards'.

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/036-burn";

const BURN = "classic-036";
const MENACE = "core-019";
const ANCHOR = "core-010";
const A = "core-020";
const B = "core-001";

const AT_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

function handDefs(s: Scenario): string[] {
  return s.hand("p1").map((card) => card.defId);
}

describe("C #36 Burn", () => {
  it("declares one target, any unit or hero on either side, and a script per face", () => {
    expect(def.id).toBe(BURN);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant.targets).toEqual(base.targets);
  });

  it("R381 its name is a rules word: it is the one card named Burn, and no entry's refs name it", () => {
    expect(Object.values(CATALOG).filter((card) => card.name === "Burn").map((card) => card.id)).toEqual([BURN]);
    expect(Object.values(CATALOG).filter((card) => (card.refs ?? []).includes(BURN))).toEqual([]);
  });

  describe("base", () => {
    it("deals 2 to a Unit, then with 4 mana left draws 1", () => {
      const s = scenario({ p1: { hand: [BURN, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR], field: [MENACE] } });
      const menace = s.card(MENACE);

      s.play(BURN, { targets: [{ pick: "instance", instanceId: menace.id }] });

      s.expectStats(menace, { health: 7 });
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      s.expectEvents("damage", "drawn");
    });

    it("deals 2 to a hero, either side", () => {
      const s = scenario({ p1: { hand: [BURN, BURN, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR] } });
      const [first, second] = s.hand("p1");
      if (first === undefined || second === undefined) throw new Error("two Burns");

      s.play(first, { targets: AT_HERO });
      s.play(second, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p2", 28).expectHealth("p1", 28);
    });

    it("with 3 mana left it draws nothing", () => {
      const s = scenario({ p1: { hand: [BURN, ANCHOR], library: [A], mana: 3 }, p2: { hand: [ANCHOR] } });

      s.play(BURN, { targets: AT_HERO });

      s.expectHealth("p2", 28);
      expect(handDefs(s)).toEqual([ANCHOR]);
    });

    it("mana left is after paying: a Burn made to cost (1) leaves 3 of 4, and draws nothing", () => {
      const s = scenario({ p1: { hand: [{ def: BURN, costMod: 1 }, ANCHOR], library: [A] }, p2: { hand: [ANCHOR] } });

      s.play(BURN, { targets: AT_HERO });

      s.expectMana("p1", 3);
      expect(handDefs(s)).toEqual([ANCHOR]);
    });

    it("§2.3 temporary mana above max counts as mana left", () => {
      const s = scenario({ turn: 3, p1: { hand: [BURN, ANCHOR], library: [A], mana: 5 }, p2: { hand: [ANCHOR] } });

      s.play(BURN, { targets: AT_HERO });

      expect(handDefs(s)).toEqual([ANCHOR, A]);
    });

    it("R386 an Upgrade deals 3; a Degrade of the threshold to 5 stops 4 mana drawing; an Upgrade of draw draws 2", () => {
      const dmg = scenario({ p1: { hand: [BURN, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR] } });
      stepParam(dmg.card(BURN), "damage", 1);
      dmg.play(BURN, { targets: AT_HERO });
      dmg.expectHealth("p2", 27);

      const harder = scenario({ p1: { hand: [BURN, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR] } });
      stepParam(harder.card(BURN), "threshold", 1);
      harder.play(BURN, { targets: AT_HERO });
      expect(handDefs(harder)).toEqual([ANCHOR]);

      const more = scenario({ p1: { hand: [BURN, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR] } });
      stepParam(more.card(BURN), "draw", 1);
      more.play(BURN, { targets: AT_HERO });
      expect(handDefs(more)).toEqual([ANCHOR, A, B]);
    });
  });

  describe("radiant", () => {
    it("deals 4, and with max mana 4 draws 1 even with no mana left", () => {
      const s = scenario({ p1: { hand: [{ def: BURN, radiant: true }, ANCHOR], library: [A], mana: 0 }, p2: { hand: [ANCHOR] } });

      s.play(BURN, { targets: AT_HERO });

      s.expectHealth("p2", 26);
      expect(handDefs(s)).toEqual([ANCHOR, A]);
    });

    it("§2.3 with max mana 3 it draws nothing, however much mana is left", () => {
      const s = scenario({ turn: 5, p1: { hand: [{ def: BURN, radiant: true }, ANCHOR], library: [A], mana: 9 }, p2: { hand: [ANCHOR] } });
      expect(s.view("p1").you.mana.max).toBe(3);

      s.play(BURN, { targets: AT_HERO });

      s.expectHealth("p2", 26);
      expect(handDefs(s)).toEqual([ANCHOR]);
    });

    it("R386 a Degrade of the threshold to 5 stops max mana 4 drawing", () => {
      const s = scenario({ p1: { hand: [{ def: BURN, radiant: true }, ANCHOR], library: [A] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(BURN), "threshold", 1);

      s.play(BURN, { targets: AT_HERO });

      expect(handDefs(s)).toEqual([ANCHOR]);
    });
  });
});
