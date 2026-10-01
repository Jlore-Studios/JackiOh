// C+ #40 Appropriations — SPEC §8.7 row 40, E38, E39, R60, R80, R81, R97, R177, R311, R348, R380,
// R440, R581, BUILD M9 row C+ 40. Education's verb is proved in packages/engine/test/shuffle-random.test.ts
// (a Book it made is cast as it is drawn, R58), the grants and buffs in the engine's E38 tests.

import { LIBRARY_CAP, hasEnchantment, type CardInstance } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/index";
import { base, def, radiant } from "../../src/scripts/classic-plus/040-appropriations";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const APPROPRIATIONS = "classicplus-040";
const TIMMY = "core-011"; // (1) 3/3 Rush, First Strike
const VANILLA = "core-008"; // (1) 4/4
const MENACE = "core-019"; // (3) 9/9 Taunt
const ARMORED = "core-025"; // (4) 7/7 Armor 7
const ECLIPSE = "core-035"; // a Spell
const FILLER = "core-005";

function appropriations(opts: { radiant?: boolean; p1?: SideSetup; p2?: SideSetup } = {}): Scenario {
  return scenario({
    p1: {
      hand: [{ def: APPROPRIATIONS, radiant: opts.radiant === true }, VANILLA, ECLIPSE],
      field: [TIMMY],
      library: [MENACE, FILLER],
      ...opts.p1,
    },
    p2: { hand: [VANILLA], field: [TIMMY], library: [FILLER], ...opts.p2 },
  });
}

function mine(s: Scenario, defId: string, zone: "hand" | "library"): CardInstance {
  const card = s.pile("p1", zone).find((held) => held.defId === defId);
  if (card === undefined) throw new Error(`${defId} in p1's ${zone}`);
  return card;
}

function keywordsOf(s: Scenario, card: CardInstance): string[] {
  return s.stats(card).keywords.map((keyword) => keyword.kind);
}

describe("C+ #40 Appropriations", () => {
  it("R81 declares its four modes with the play, on both faces", () => {
    expect(def.id).toBe(APPROPRIATIONS);
    expect(def.cost).toBe("X");
    for (const face of [base, radiant]) expect(face.modes).toEqual([{ kind: "mode", options: ["Military", "Education", "Culture", "Healthcare"] }]);
  });

  it("R348 X is at least 1", () => {
    const s = appropriations();
    expect(() => s.play(APPROPRIATIONS, { x: 0, modes: ["Military"] })).toThrow(/X/);
  });

  describe("Military", () => {
    it("E38 your Units on the field, in your hand and in your deck get +2X Attack and Rush; Spells and the opponent's Units do not", () => {
      const s = appropriations().play(APPROPRIATIONS, { x: 2, modes: ["Military"] });
      const timmy = s.unit("p1", 1) as CardInstance;
      s.expectStats(timmy, { attack: 7, health: 3 });
      expect(keywordsOf(s, timmy)).toContain("Rush");
      for (const card of [mine(s, VANILLA, "hand"), mine(s, MENACE, "library")]) {
        expect(card.buffs.attack).toBe(4);
        expect(card.grantedKeywords.map((keyword) => keyword.kind)).toContain("Rush");
      }
      expect(mine(s, ECLIPSE, "hand").buffs.attack).toBe(0);
      expect(mine(s, FILLER, "library").buffs.attack).toBe(0);
      s.expectStats(s.unit("p2", 1) as CardInstance, { attack: 3 });
    });

    it("E38 a hand Unit carries the buff and Rush onto the field as it enters", () => {
      const s = appropriations().play(APPROPRIATIONS, { x: 1, modes: ["Military"] });
      s.play(VANILLA, { zone: 2 });
      const vanilla = s.unit("p1", 2) as CardInstance;
      s.expectStats(vanilla, { attack: 6, health: 4 });
      expect(keywordsOf(s, vanilla)).toContain("Rush");
    });

    it("R97 R440 hand and deck changes are hidden from the opponent; the deck's are unread by its owner (R311)", () => {
      const s = appropriations().play(APPROPRIATIONS, { x: 1, modes: ["Military"] });
      const hidden = [mine(s, VANILLA, "hand").id, mine(s, MENACE, "library").id];
      const named = (events: GameEvent[]): string[] =>
        events.flatMap((event) => (event.type === "buffed" || event.type === "keywordGranted" ? [event.instanceId] : []));
      for (const id of hidden) expect(named(s.view("p2").events)).not.toContain(id);
      expect(s.view("p1").you.ownLibrary?.cards).toEqual(
        expect.arrayContaining([{ defId: MENACE, radiant: false, count: 1 }]),
      );
    });

    it("radiant: +5X Attack and Rush", () => {
      const s = appropriations({ radiant: true }).play(APPROPRIATIONS, { x: 2, modes: ["Military"] });
      s.expectStats(s.unit("p1", 1) as CardInstance, { attack: 13 });
      expect(mine(s, VANILLA, "hand").buffs.attack).toBe(10);
    });
  });

  describe("Education", () => {
    it("E39 R380 shuffles 2X random non-token Books into your deck, Radiant, with Cast on draw and target-enemies riding them", () => {
      const s = appropriations({ p1: { library: [] } }).play(APPROPRIATIONS, { x: 2, modes: ["Education"] });
      const library = s.pile("p1", "library");
      expect(library).toHaveLength(4);
      for (const card of library) {
        expect(cardDef(card.defId).tags).toContain("Book");
        expect(cardDef(card.defId).token).toBe(false);
        expect(card.radiant).toBe(true);
        expect(hasEnchantment(card, "castOnDraw")).toBe(true);
        expect(hasEnchantment(card, "targetEnemies")).toBe(true);
      }
    });

    it("R311 the shuffle-in is open to its owner: the deck list shows the Radiant Books; the opponent sees a count", () => {
      const s = appropriations({ p1: { library: [] } }).play(APPROPRIATIONS, { x: 1, modes: ["Education"] });
      const listed = s.view("p1").you.ownLibrary?.cards ?? [];
      expect(listed.reduce((sum, entry) => sum + entry.count, 0)).toBe(2);
      for (const entry of listed) expect(entry.radiant).toBe(true);
      expect(s.view("p2").opponent.libraryCount).toBe(2);
      expect(s.view("p2").opponent.ownLibrary).toBeUndefined();
    });

    it("R80 a full deck turns the rest away", () => {
      const s = appropriations({ p1: { library: Array.from({ length: LIBRARY_CAP - 1 }, () => FILLER) } });
      s.play(APPROPRIATIONS, { x: 2, modes: ["Education"] });
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP);
      expect(s.events.filter((event) => event.type === "libraryOverflow")).toHaveLength(3);
    });

    it("radiant: 5X Books", () => {
      const s = appropriations({ radiant: true, p1: { library: [] } }).play(APPROPRIATIONS, { x: 2, modes: ["Education"] });
      expect(s.pile("p1", "library")).toHaveLength(10);
    });
  });

  describe("Culture", () => {
    it("R60 each card on your field, in your hand and in your deck rolls its own 10X%: over seeds some turn and some do not", () => {
      let turned = 0;
      let stayed = 0;
      for (let n = 0; n < 10; n += 1) {
        const s = scenario({
          seed: `culture-${n}`,
          p1: { hand: [APPROPRIATIONS, VANILLA, ECLIPSE], field: [TIMMY], library: [MENACE, FILLER] },
          p2: { hand: [VANILLA], field: [TIMMY] },
        }).play(APPROPRIATIONS, { x: 4, modes: ["Culture"] });
        for (const card of [...s.hand("p1"), ...s.pile("p1", "library"), s.unit("p1", 1) as CardInstance]) {
          if (card.radiant) turned += 1;
          else stayed += 1;
        }
        expect(s.unit("p2", 1)?.radiant).toBe(false);
        expect(s.hand("p2")[0]?.radiant).toBe(false);
      }
      expect(turned).toBeGreaterThan(0);
      expect(stayed).toBeGreaterThan(0);
    });

    it("R581 every card is rolled, a Radiant one too, so the draws never count the hidden Radiant cards", () => {
      const rolls = (handRadiant: boolean): number => {
        const s = appropriations({ p1: { hand: [APPROPRIATIONS, { def: VANILLA, radiant: handRadiant }, ECLIPSE], field: [TIMMY], library: [MENACE, FILLER] } });
        const cursor = s.state.rngCursor;
        s.play(APPROPRIATIONS, { x: 1, modes: ["Culture"] });
        return s.state.rngCursor - cursor;
      };
      // Field 1 + hand 2 + deck 2: five rolls whether or not the hand's Vanilla is Radiant already.
      expect(rolls(false)).toBe(5);
      expect(rolls(true)).toBe(5);
    });

    it("R311 R177 radiant at X = 4 is 100%: every card turns, the deck's listed as they went in, the hand's hidden from the opponent", () => {
      const s = appropriations({ radiant: true }).play(APPROPRIATIONS, { x: 4, modes: ["Culture"] });
      for (const card of [...s.hand("p1"), ...s.pile("p1", "library")]) expect(card.radiant).toBe(true);
      s.expectStats(s.unit("p1", 1) as CardInstance, { attack: 6, health: 6 });
      for (const entry of s.view("p1").you.ownLibrary?.cards ?? []) expect(entry.radiant).toBe(false);
      const handIds = s.hand("p1").map((card) => card.id);
      for (const event of s.view("p2").events) {
        if (event.type === "radiantSet") expect(handIds).not.toContain(event.instanceId);
      }
    });
  });

  describe("Healthcare", () => {
    it("E38 your Units everywhere get +2X Health and Armor X, which stacks with printed Armor", () => {
      const s = appropriations({ p1: { field: [TIMMY, ARMORED] } }).play(APPROPRIATIONS, { x: 2, modes: ["Healthcare"] });
      s.expectStats(s.unit("p1", 1) as CardInstance, { attack: 3, health: 7 });
      expect(s.stats(s.unit("p1", 1) as CardInstance).armor).toBe(2);
      expect(s.stats(s.unit("p1", 2) as CardInstance).armor).toBe(9);
      const vanilla = mine(s, VANILLA, "hand");
      expect(vanilla.buffs.health).toBe(4);
      expect(vanilla.grantedKeywords).toEqual([{ kind: "Armor", n: 2 }]);
      expect(mine(s, MENACE, "library").buffs.health).toBe(4);
      s.expectStats(s.unit("p2", 1) as CardInstance, { health: 3 });
    });

    it("radiant: +7X Health and Armor 2X", () => {
      const s = appropriations({ radiant: true }).play(APPROPRIATIONS, { x: 1, modes: ["Healthcare"] });
      s.expectStats(s.unit("p1", 1) as CardInstance, { health: 10 });
      expect(s.stats(s.unit("p1", 1) as CardInstance).armor).toBe(2);
    });
  });
});
