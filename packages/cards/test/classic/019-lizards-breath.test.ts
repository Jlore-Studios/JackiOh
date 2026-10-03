// C #19 Lizard's Breath — SPEC §8.6 row 19, BUILD M9 Classic row C 19: "One hit on the play's target
// (any Unit or hero): 2, or 6 when Exile counts; your own deck, graveyard and exile are counted as it
// resolves (this Spell, resolving, is in none); the largest pile adds its effect: Deck draws 1,
// Graveyard gives 2 mana this turn, Exile adds 4 damage; ties go to the pile listed first (Deck, then
// Graveyard, then Exile), so three empty piles pick Deck and draw (fatigue from an empty deck); the
// draw and the mana follow the hit; its preview names the pile that would count now (R280); radiant:
// 4, or 8 with Exile, and the two largest piles add their effects with the same ties; the preview
// names both; its tuned numbers (damage, draw, mana, extra damage) read through `param()` (R386)".
//
// The preview's proofs (R280) are in `../preview.test.ts`, with the other cards'.

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/019-lizards-breath";

const BREATH = "classic-019";
const MENACE = "core-019";
const FILLER = "core-005";
const ANCHOR = "core-010";
const X = "core-008";

const AT_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

function piles(deck: number, graveyard: number, exile: number): Pick<SideSetup, "library" | "graveyard" | "exile"> {
  return {
    library: Array.from({ length: deck }, () => X),
    graveyard: Array.from({ length: graveyard }, () => FILLER),
    exile: Array.from({ length: exile }, () => FILLER),
  };
}

function hits(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.sourceId !== null ? [event.amount] : []));
}

function drawn(s: Scenario): number {
  return s.events.filter((event) => event.type === "drawn").length;
}

function breath(face: "base" | "radiant", deck: number, graveyard: number, exile: number, mana?: number): Scenario {
  return scenario({
    p1: { hand: [{ def: BREATH, radiant: face === "radiant" }, ANCHOR], ...piles(deck, graveyard, exile), ...(mana === undefined ? {} : { mana }) },
    p2: { hand: [ANCHOR], field: [{ def: MENACE, radiant: true }] },
  });
}

describe("C #19 Lizard's Breath", () => {
  it("declares one target, any unit or hero on either side, and a preview on both faces", () => {
    expect(def.id).toBe(BREATH);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(base.preview).toBeTypeOf("function");
    expect(radiant.preview).toBeTypeOf("function");
  });

  describe("base", () => {
    it("the Deck largest: one hit of 2, then draw 1", () => {
      const s = breath("base", 3, 1, 1);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([2]);
      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 3);
      s.expectEvents("damage", "drawn");
    });

    it("a Unit target is legal too: the hit lands on it", () => {
      const s = breath("base", 3, 1, 1);
      const menace = s.card(MENACE);

      s.play(BREATH, { targets: [{ pick: "instance", instanceId: menace.id }] });

      s.expectStats(menace, { health: 16 });
    });

    it("the Graveyard largest: 2 damage, then 2 mana this turn", () => {
      const s = breath("base", 1, 3, 1);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([2]);
      expect(drawn(s)).toBe(0);
      s.expectMana("p1", 5);
      s.expectEvents("damage", "manaChanged");
    });

    it("§2.3 the Graveyard's mana is temporary: current mana rises, max mana stays 4", () => {
      const s = breath("base", 1, 3, 1);

      s.play(BREATH, { targets: AT_HERO });

      expect(s.view("p1").you.mana).toEqual({ current: 5, max: 4 });
    });

    it("ties go to the pile listed first: Deck over Exile", () => {
      const s = breath("base", 2, 0, 2);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([2]);
      expect(drawn(s)).toBe(1);
    });

    it("the Exile largest: one hit of 6", () => {
      const s = breath("base", 1, 1, 3);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([6]);
      s.expectHealth("p2", 24);
      expect(drawn(s)).toBe(0);
      s.expectMana("p1", 3);
    });

    it("counted as it resolves: this Spell is in no pile, so a 2–2 Deck and Graveyard tie goes to the Deck", () => {
      const s = breath("base", 2, 2, 0);

      s.play(BREATH, { targets: AT_HERO });

      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 3);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(BREATH);
    });

    it("ties go to the pile listed first: Graveyard over Exile", () => {
      const s = breath("base", 0, 2, 2);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([2]);
      s.expectMana("p1", 5);
    });

    it("§2.4 three empty piles pick the Deck, and its draw is a fatigue", () => {
      const s = breath("base", 0, 0, 0);

      s.play(BREATH, { targets: AT_HERO });

      expect(s.events.filter((event) => event.type === "fatigue")).toHaveLength(1);
      s.expectHealth("p1", 29).expectHealth("p2", 28);
    });

    it("R386 each number tunes: damage 3, draw 2, mana 3, extra damage 5", () => {
      const dmg = breath("base", 3, 1, 1);
      stepParam(dmg.card(BREATH), "damage", 1);
      dmg.play(BREATH, { targets: AT_HERO });
      expect(hits(dmg)).toEqual([3]);

      const more = breath("base", 3, 1, 1);
      stepParam(more.card(BREATH), "draw", 1);
      more.play(BREATH, { targets: AT_HERO });
      expect(drawn(more)).toBe(2);

      const mana = breath("base", 1, 3, 1);
      stepParam(mana.card(BREATH), "mana", 1);
      mana.play(BREATH, { targets: AT_HERO });
      mana.expectMana("p1", 6);

      const extra = breath("base", 1, 1, 3);
      stepParam(extra.card(BREATH), "extraDamage", 1);
      extra.play(BREATH, { targets: AT_HERO });
      expect(hits(extra)).toEqual([7]);
    });
  });

  describe("radiant", () => {
    it("the Exile and the Graveyard largest: one hit of 8, then 2 mana", () => {
      const s = breath("radiant", 1, 3, 5);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([8]);
      expect(drawn(s)).toBe(0);
      s.expectMana("p1", 5);
    });

    it("the Deck and the Exile largest: one hit of 8, then draw 1", () => {
      const s = breath("radiant", 4, 1, 3);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([8]);
      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 3);
    });

    it("the Deck and the Graveyard largest: one hit of 4, then the draw, then the mana", () => {
      const s = breath("radiant", 4, 3, 1);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([4]);
      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 5);
      s.expectEvents("damage", "drawn", "manaChanged");
    });

    it("ties for second place go to the pile listed first: Exile largest, a Deck–Graveyard tie behind it picks the Deck", () => {
      const s = breath("radiant", 2, 2, 5);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([8]);
      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 3);
    });

    it("ties go to the piles listed first: three equal piles pick the Deck and the Graveyard", () => {
      const s = breath("radiant", 2, 2, 2);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([4]);
      expect(drawn(s)).toBe(1);
      s.expectMana("p1", 5);
    });

    it("R386 an Upgrade of damage on the Radiant face steps 4 to 5, and with Exile 9", () => {
      const s = breath("radiant", 4, 1, 3);
      stepParam(s.card(BREATH), "damage", 1);

      s.play(BREATH, { targets: AT_HERO });

      expect(hits(s)).toEqual([9]);
    });
  });
});
