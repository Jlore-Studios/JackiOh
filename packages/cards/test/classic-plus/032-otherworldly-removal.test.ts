// C+ #32 Otherworldly Removal — SPEC §8.7 row 32, BUILD M9 Classic+ row C+ 32: "Adds Execute, Brawl
// and Blade Storm (C+ #32.1–#32.3) to your hand in that order, a full hand burning what doesn't fit
// (with 8 other cards in hand, Blade Storm burns); they are Spell tokens and go to the graveyard when
// played (R11); hidden from the opponent (R97); radiant all three Radiant".

import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/032-otherworldly-removal";

const REMOVAL = "classicplus-032";
const EXECUTE = "classicplus-032-1";
const BRAWL = "classicplus-032-2";
const STORM = "classicplus-032-3";
const FILLER = "core-005";
const HIDDEN = "hidden";

function added(events: readonly { type: string }[]): { defId: string; instanceId: string }[] {
  return events.flatMap((event) =>
    event.type === "addedToHand" ? [event as unknown as { defId: string; instanceId: string }] : [],
  );
}

describe("C+ #32 Otherworldly Removal", () => {
  it("is the card it says", () => {
    expect(def.id).toBe(REMOVAL);
  });

  describe("base", () => {
    it("§8.7 adds an Execute, a Brawl and a Blade Storm to your hand, in that order", () => {
      const s = scenario({ p1: { hand: [REMOVAL, FILLER] } });
      s.play(REMOVAL);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER, EXECUTE, BRAWL, STORM]);
      expect(s.hand("p1").every((card) => !card.radiant)).toBe(true);
      expect(added(s.events).map((event) => event.defId)).toEqual([EXECUTE, BRAWL, STORM]);
      s.expectInZone(REMOVAL, "graveyard");
    });

    it("R4 a full hand burns what doesn't fit: with 8 other cards in hand, Blade Storm burns", () => {
      const s = scenario({ p1: { hand: [REMOVAL, ...Array.from({ length: 8 }, () => FILLER)] } });
      s.play(REMOVAL);
      const held = s.hand("p1").map((card) => card.defId);
      expect(held).toHaveLength(10);
      expect(held.slice(8)).toEqual([EXECUTE, BRAWL]);
      expect(s.events.some((event) => event.type === "burned" && event.defId === STORM)).toBe(true);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(STORM);
    });

    it("R11 they are Spell tokens: one played goes to the graveyard like any Spell", () => {
      const s = scenario({ p1: { hand: [REMOVAL, FILLER], mana: 8 } });
      s.play(REMOVAL);
      s.play(BRAWL);
      s.expectInZone(BRAWL, "graveyard");
    });

    it("R97 the opponent sees three cards added and never which", () => {
      const s = scenario({ p1: { hand: [REMOVAL, FILLER] } });
      s.play(REMOVAL);
      const theirs = added(s.view("p2").events);
      expect(theirs).toHaveLength(3);
      for (const event of theirs) {
        expect(event.defId).toBe(HIDDEN);
        expect(event.instanceId).toBe(HIDDEN);
      }
      expect(added(s.view("p1").events).map((event) => event.defId)).toEqual([EXECUTE, BRAWL, STORM]);
    });
  });

  describe("radiant", () => {
    it("§8.7 adds a Radiant Execute, a Radiant Brawl and a Radiant Blade Storm", () => {
      const s = scenario({ p1: { hand: [{ def: REMOVAL, radiant: true }, FILLER] } });
      s.play(REMOVAL);
      const fresh = s.hand("p1").filter((card) => card.defId !== FILLER);
      expect(fresh.map((card) => card.defId)).toEqual([EXECUTE, BRAWL, STORM]);
      expect(fresh.every((card) => card.radiant)).toBe(true);
    });
  });
});
