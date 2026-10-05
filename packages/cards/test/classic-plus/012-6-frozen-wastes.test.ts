// C+ #12.6 Frozen Wastes — SPEC §8.7 row 12.6, R408, BUILD M9 Classic+ row C+ 12.6: "Spell whose
// unlabelled text is its Cry (R408): destroys every Unit on both sides, then exiles the top card of
// your deck once for each Unit that died (Indestructible survivors don't count, a Reborn unit that died
// does), a short deck exiling what it has with no fatigue; the exiles are public (R97); afterwards it
// goes to the graveyard; its preview is the cards it would exile now (R280); radiant exiles from the top of the
// opponent's deck instead". The R280 proof is here (its preview reads deck sizes, never contents).

import type { CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { def } from "../../src/scripts/classic-plus/012-6-frozen-wastes";

const WASTES = "classicplus-012-6";
const MENACE = "core-019"; // 9/9
const ROCK = "core-066"; // Indestructible
const DEFENDER = "core-003"; // Taunt, Divine Shield, Reborn
const MANA_WELL = "core-006"; // Field Spell
const MAGIC_JAMMED = "core-036"; // destroy target backrow card
const STOCKPILE = "core-005";
const LUNAR = "core-035";

const deck = (n: number): string[] => Array.from({ length: n }, (_, i) => (i % 2 === 0 ? LUNAR : STOCKPILE));

function wastes(radiant: boolean, p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: WASTES, radiant }, MAGIC_JAMMED, STOCKPILE], field: [MENACE], library: deck(6), mana: 8, ...p1 },
    p2: { hand: [STOCKPILE], field: [MENACE, ROCK, DEFENDER], library: deck(6), ...p2 },
  });
}

function previewOf(s: Scenario): number | undefined {
  const hand = s.view("p1").you.hand;
  if (!Array.isArray(hand)) throw new Error("own hand in full");
  return hand.find((card: CardView) => card.defId === WASTES)?.preview?.[0]?.value;
}

describe("C+ #12.6 Frozen Wastes", () => {
  describe("base", () => {
    it("R408 its Cry destroys every Unit on both sides; an Indestructible one survives", () => {
      const s = wastes(false);
      s.play(WASTES);
      expect(s.pile("p1", "graveyard").some((card) => card.defId === MENACE)).toBe(true);
      expect(s.pile("p2", "graveyard").some((card) => card.defId === MENACE)).toBe(true);
      s.expectInZone(ROCK, "field");
    });

    it("R408 exiles the top card of your deck once per Unit that died: a Reborn unit counts, the Indestructible one doesn't", () => {
      const s = wastes(false);
      const top = s.pile("p1", "library").slice(0, 3).map((card) => card.id);
      s.play(WASTES);
      // Your Menace, their Menace and their Defender (back again by Reborn): three.
      expect(s.pile("p1", "exile").map((card) => card.id)).toEqual(top);
      expect(s.pile("p1", "library")).toHaveLength(3);
      expect(s.unit("p2", 3)?.defId).toBe(DEFENDER);
      expect(s.pile("p2", "exile")).toHaveLength(0);
    });

    it("a short deck exiles what it has, with no fatigue", () => {
      const s = wastes(false, { library: deck(1) });
      s.play(WASTES);
      expect(s.pile("p1", "exile")).toHaveLength(1);
      expect(s.events.some((event) => event.type === "fatigue")).toBe(false);
    });

    it("R97 the exiles are public", () => {
      const s = wastes(false);
      s.play(WASTES);
      const exiled = s.view("p2").events.filter((event) => event.type === "exiled");
      expect(exiled).toHaveLength(3);
      for (const event of exiled) if (event.type === "exiled") expect(event.defId).not.toBe("hidden");
      expect(s.view("p2").opponent.exile).toHaveLength(3);
    });

    it("R408 afterwards it goes to the graveyard with no further text", () => {
      const s = wastes(false, { field: [] }, { field: [] });
      s.play(WASTES);
      s.expectInZone(WASTES, "graveyard");
    });

    it("a Spell, it plays with a full backrow", () => {
      const s = wastes(false, { backrow: [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL] });
      s.play(WASTES);
      s.expectInZone(WASTES, "graveyard");
    });

    it("R280 its preview is the cards it would exile now, and the Cry then exiles that many", () => {
      const s = wastes(false);
      expect(previewOf(s)).toBe(3);
      s.play(WASTES);
      expect(s.pile("p1", "exile")).toHaveLength(3);

      const short = wastes(false, { library: deck(2) });
      expect(previewOf(short)).toBe(2);

      const empty = wastes(false, { field: [] }, { field: [ROCK] });
      expect(previewOf(empty)).toBe(0);
    });

    it("R280 the preview label sits in each face's text", () => {
      const s = wastes(false);
      const hand = s.view("p1").you.hand;
      if (!Array.isArray(hand)) throw new Error("own hand in full");
      const label = hand.find((card: CardView) => card.defId === WASTES)?.preview?.[0]?.label ?? "";
      expect(label).not.toBe("");
      expect(def.base.text).toContain(label);
      expect(def.radiant.text).toContain(label);
    });
  });

  describe("radiant", () => {
    it("exiles from the top of the opponent's deck instead", () => {
      const s = wastes(true);
      const top = s.pile("p2", "library").slice(0, 3).map((card) => card.id);
      s.play(WASTES);
      expect(s.pile("p2", "exile").map((card) => card.id)).toEqual(top);
      expect(s.pile("p1", "exile")).toHaveLength(0);
    });

    it("R280 its preview reads the opponent's deck size", () => {
      const s = wastes(true, {}, { library: deck(2) });
      expect(previewOf(s)).toBe(2);
    });
  });
});
