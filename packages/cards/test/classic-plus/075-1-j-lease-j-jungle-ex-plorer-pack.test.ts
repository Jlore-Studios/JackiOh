// C+ #75.1 J-lease J-Jungle EX-plorer Pack — SPEC §8.7 row 75.1, BUILD M9 Classic+ row C+ 75.1: "Cast on
// draw (R70, R58): adds 5 random Radiant non-token Classic or Classic+ cards (never Core) to your hand,
// repeats allowed, a full hand burning the rest, then you draw again; played from a hand it does the
// same; the five are hidden from the opponent (R97); the count reads through `param()`; radiant they also
// cost (0)".

import { defOf, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/075-1-j-lease-j-jungle-ex-plorer-pack";

const PACK = "classicplus-075-1";
const STOCKPILE = "core-005"; // (1) "Draw 2. Heal your hero 2."
const FILLER = "core-011"; // Tempo Timmy, deck filler
const MENACE = "core-019";

function generated(s: Scenario): Extract<GameEvent, { type: "addedToHand" | "burned" }>[] {
  return s.lastEvents.flatMap((event) =>
    (event.type === "addedToHand" || event.type === "burned") && event.defId !== FILLER && event.defId !== MENACE ? [event] : [],
  );
}

describe("C+ #75.1 J-lease J-Jungle EX-plorer Pack", () => {
  it("is a (2) Spell token (printed Legendary) that casts itself on draw", () => {
    expect(def.id).toBe(PACK);
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(2);
    expect(def.printedRarity).toBe("Legendary");
    expect(base.staticFlags?.castOnDraw).toBe(true);
    expect(radiant.staticFlags?.castOnDraw).toBe(true);
  });

  describe("base", () => {
    it("R58 drawn, it casts itself: 5 random Radiant Classic or Classic+ cards reach your hand, then you draw again", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, MENACE], library: [PACK, FILLER, FILLER] }, p2: { hand: [MENACE] } });
      s.play(STOCKPILE);

      s.expectInZone(PACK, "graveyard");
      const added = generated(s).filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(5);
      for (const event of added) {
        const card = s.card(event.instanceId);
        const printed = defOf(s.state, card.defId);
        expect(["Classic", "Classic+"]).toContain(printed.set);
        expect(printed.token).toBe(false);
        expect(card.radiant).toBe(true);
        expect(card.costOverride).toBeUndefined();
      }
      // R58: the draw repeats after the cast, and Stockpile's second draw follows: both Timmies drawn.
      expect(s.pile("p1", "library")).toEqual([]);
      expect(s.hand("p1").filter((card) => card.defId === FILLER)).toHaveLength(2);
      s.expectEvents("drawn", "cardPlayed", "addedToHand", "drawn");
    });

    it("R70 played from a hand it does the same", () => {
      const s = scenario({ p1: { hand: [PACK, MENACE] }, p2: { hand: [MENACE] } });
      s.play(PACK);
      expect(generated(s).filter((event) => event.type === "addedToHand")).toHaveLength(5);
      s.expectInZone(PACK, "graveyard");
    });

    it("R380 over many seeds the cards are of Classic and Classic+ only, never Core, never a token", () => {
      const sets = new Set<string>();
      for (let i = 0; i < 20; i += 1) {
        const s = scenario({ seed: `pack-${i}`, p1: { hand: [PACK, MENACE] }, p2: { hand: [MENACE] } });
        s.play(PACK);
        for (const event of generated(s)) {
          const printed = defOf(s.state, event.defId);
          sets.add(printed.set);
          expect(printed.token).toBe(false);
        }
      }
      expect([...sets].sort()).toEqual(["Classic", "Classic+"]);
    });

    it("§2.4 R4 a full hand burns the rest", () => {
      const s = scenario({ p1: { hand: [PACK, ...Array.from({ length: 7 }, () => MENACE)] }, p2: { hand: [MENACE] } });
      s.play(PACK);
      const events = generated(s);
      expect(events.filter((event) => event.type === "addedToHand")).toHaveLength(3);
      expect(events.filter((event) => event.type === "burned")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(10);
    });

    it("R97 the five reach the opponent under the sentinel", () => {
      const s = scenario({ p1: { hand: [PACK, MENACE] }, p2: { hand: [MENACE] } });
      s.play(PACK);
      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(theirs).toHaveLength(5);
      expect(theirs.every((event) => event.type === "addedToHand" && event.defId === "hidden" && event.instanceId === "hidden")).toBe(true);
    });

    it("R386 an Upgrade adds 6; a Degrade 4", () => {
      const up = scenario({ p1: { hand: [PACK, MENACE] }, p2: { hand: [MENACE] } });
      stepParam(up.card(PACK), "cards", 1);
      up.play(PACK);
      expect(generated(up)).toHaveLength(6);

      const down = scenario({ p1: { hand: [PACK, MENACE] }, p2: { hand: [MENACE] } });
      stepParam(down.card(PACK), "cards", -1);
      down.play(PACK);
      expect(generated(down)).toHaveLength(4);
    });
  });

  describe("radiant", () => {
    it("the five are Radiant and cost (0)", () => {
      const s = scenario({ p1: { hand: [{ def: PACK, radiant: true }, MENACE] }, p2: { hand: [MENACE] } });
      s.play(PACK);
      const added = generated(s).filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(5);
      expect(added.every((event) => s.card(event.instanceId).radiant && s.card(event.instanceId).costOverride === 0)).toBe(true);
      const hand = s.view("p1").you.hand;
      expect(Array.isArray(hand) && hand.filter((card) => card.cost === 0)).toHaveLength(5);
    });

    it("R58 a drawn Radiant Pack casts itself the same way", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, MENACE], library: [{ def: PACK, radiant: true }, FILLER] }, p2: { hand: [MENACE] } });
      s.play(STOCKPILE);
      const added = generated(s).filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(5);
      expect(added.every((event) => s.card(event.instanceId).costOverride === 0)).toBe(true);
    });

    it("§2.4 a burned card keeps no price", () => {
      const s = scenario({ p1: { hand: [{ def: PACK, radiant: true }, ...Array.from({ length: 9 }, () => MENACE)] }, p2: { hand: [MENACE] } });
      s.play(PACK);
      const burned = generated(s).filter((event) => event.type === "burned");
      expect(burned).toHaveLength(4);
      expect(burned.every((event) => s.card(event.instanceId).costOverride === undefined)).toBe(true);
    });
  });
});
