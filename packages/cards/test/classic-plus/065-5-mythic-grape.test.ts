// C+ #65.5 Mythic Grape — SPEC §8.7 row 65.5, BUILD M9 Classic+ row C+ 65.5: "Each other card in your
// hand goes to your graveyard (not a discard) and is replaced by a random non-token Mythic card of any
// set that costs (0), never a Grape; an empty hand does nothing and draws nothing (R129); the old cards
// are public in the graveyard, the new ones hidden (R97); radiant the Mythics are Radiant".

import { defOf } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-5-mythic-grape";

const MYTHIC_GRAPE = "classicplus-065-5";
const FILLER = "core-005";
const TIMMY = "core-011";
const RUSH = "core-t-rush"; // a unit-token card

describe("C+ #65.5 Mythic Grape", () => {
  it("is a (0) Fruit Spell token with the printed rarity Mythic, whose own rarity is Token", () => {
    expect(def.id).toBe(MYTHIC_GRAPE);
    expect(def.cost).toBe(0);
    expect(def.rarity).toBe("Token");
    expect(def.printedRarity).toBe("Mythic");
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  describe("base", () => {
    it("§6.3 each other hand card goes to your graveyard and is replaced one for one by a Mythic costing (0)", () => {
      const s = scenario({ p1: { hand: [MYTHIC_GRAPE, FILLER, TIMMY, FILLER] }, p2: { hand: [FILLER] } });
      const old = s.hand("p1").filter((card) => card.defId !== MYTHIC_GRAPE);
      s.play(MYTHIC_GRAPE);

      expect(old.every((card) => s.card(card).zone.z === "graveyard")).toBe(true);
      const hand = s.hand("p1");
      expect(hand).toHaveLength(3);
      for (const card of hand) {
        const mythic = defOf(s.state, card.defId);
        expect(mythic.rarity).toBe("Mythic");
        expect(mythic.token).toBe(false);
        expect(card.costOverride).toBe(0);
        expect(card.radiant).toBe(false);
      }
      expect(s.view("p1").you.hand).toEqual(expect.arrayContaining([expect.objectContaining({ cost: 0 })]));
    });

    it("BUILD it is not a discard: no `discarded` event, each card reported by `enteredGraveyard`", () => {
      const s = scenario({ p1: { hand: [MYTHIC_GRAPE, FILLER, TIMMY] }, p2: { hand: [FILLER] } });
      const old = s.hand("p1").filter((card) => card.defId !== MYTHIC_GRAPE).map((card) => card.id);
      s.play(MYTHIC_GRAPE);
      expect(s.lastEvents.some((event) => event.type === "discarded")).toBe(false);
      const landed = s.lastEvents.flatMap((event) => (event.type === "enteredGraveyard" ? [event.instanceId] : []));
      expect(old.every((id) => landed.includes(id))).toBe(true);
    });

    it("R380 R387 over many seeds the new cards are non-token Mythics of every set, never a Grape", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 40; i += 1) {
        const s = scenario({ seed: `mythic-${i}`, p1: { hand: [MYTHIC_GRAPE, FILLER, FILLER, FILLER] }, p2: { hand: [FILLER] } });
        s.play(MYTHIC_GRAPE);
        for (const card of s.hand("p1")) seen.add(card.defId);
      }
      expect([...seen].some((id) => id.startsWith("classicplus-065"))).toBe(false);
      const sets = new Set([...seen].map((id) => id.split("-")[0]));
      expect(sets.size).toBeGreaterThan(1);
    });

    it("R11 a unit-token card in the hand ceases to exist and is still replaced", () => {
      const s = scenario({ p1: { hand: [MYTHIC_GRAPE, RUSH, FILLER] }, p2: { hand: [FILLER] } });
      const token = s.card(RUSH);
      s.play(MYTHIC_GRAPE);
      expect(s.pile("p1", "graveyard").some((card) => card.id === token.id)).toBe(false);
      expect(s.hand("p1")).toHaveLength(2);
    });

    it("R129 an empty hand does nothing and draws nothing from the rng", () => {
      const s = scenario({ p1: { hand: [MYTHIC_GRAPE] }, p2: { hand: [FILLER] } });
      const cursor = s.state.rngCursor;
      s.play(MYTHIC_GRAPE);
      expect(s.hand("p1")).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
      expect(s.lastEvents.some((event) => event.type === "addedToHand")).toBe(false);
    });

    it("R97 the old cards are public in the graveyard; the new ones reach the opponent under the sentinel", () => {
      const s = scenario({ p1: { hand: [MYTHIC_GRAPE, TIMMY] }, p2: { hand: [FILLER] } });
      const old = s.card(TIMMY);
      s.play(MYTHIC_GRAPE);
      const theirs = s.view("p2");
      expect(theirs.opponent.graveyard.some((card) => card.instanceId === old.id && card.defId === TIMMY)).toBe(true);
      const added = theirs.events.filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(1);
      expect(added.every((event) => event.type === "addedToHand" && event.defId === "hidden")).toBe(true);
      const mythic = s.hand("p1")[0];
      expect(JSON.stringify(theirs)).not.toContain(mythic?.id ?? "?");
    });
  });

  describe("radiant", () => {
    it("R74 the Mythics are Radiant and cost (0)", () => {
      const s = scenario({ p1: { hand: [{ def: MYTHIC_GRAPE, radiant: true }, FILLER, TIMMY] }, p2: { hand: [FILLER] } });
      s.play(MYTHIC_GRAPE);
      const hand = s.hand("p1");
      expect(hand).toHaveLength(2);
      expect(hand.every((card) => card.radiant && card.costOverride === 0)).toBe(true);
      expect(hand.every((card) => defOf(s.state, card.defId).rarity === "Mythic")).toBe(true);
    });

    it("R129 an empty hand does nothing on the Radiant face either", () => {
      const s = scenario({ p1: { hand: [{ def: MYTHIC_GRAPE, radiant: true }] }, p2: { hand: [FILLER] } });
      const cursor = s.state.rngCursor;
      s.play(MYTHIC_GRAPE);
      expect(s.hand("p1")).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });
  });
});
