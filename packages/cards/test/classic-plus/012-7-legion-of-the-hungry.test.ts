// C+ #12.7 Legion of the Hungry — SPEC §8.7 row 12.7, R408, BUILD M9 Classic+ row C+ 12.7: "Field Spell
// whose unlabelled text is its Cry (R408): exiles 5 different random cards of your deck (all if fewer,
// R60), then summons the Units among them out of exile into your leftmost open zones, no Cry (R1),
// until your board is full, the rest staying exiled; an empty deck does nothing and draws nothing
// (R129); the exiles are public and the deck's order stays hidden (R97); the card count reads through
// `param()`; radiant the summoned Units are made Radiant".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type PileSetup, type Scenario, type SideSetup } from "../_harness";
import { expectAnimated } from "../_animated";

const LEGION = "classicplus-012-7";
const TOKENS = "core-015"; // (1) Unit 1/1, Cry: summon a Rush Token
const MENACE = "core-019"; // (3) Unit
const POINTMASTER = "core-020"; // (2) Unit
const RUSH = "core-t-rush";
const LUNAR = "core-035"; // a Spell
const STOCKPILE = "core-005"; // a Spell

function legion(radiant: boolean, library: readonly PileSetup[], p1: SideSetup = {}, seed?: string): Scenario {
  const s = scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { hand: [{ def: LEGION, radiant }, STOCKPILE], library, mana: 8, ...p1 },
    p2: { hand: [STOCKPILE] },
  });
  s.play(LEGION);
  return s;
}

/** The Units it summoned, lane by lane: not the Legion itself, which animates into a unit zone (patch v0.2.10, R383). */
const unitsOf = (s: Scenario): string[] =>
  [1, 2, 3, 4, 5].flatMap((lane) => {
    const unit = s.unit("p1", lane);
    return unit === null || unit.defId === LEGION ? [] : [unit.defId];
  });

describe("C+ #12.7 Legion of the Hungry", () => {
  describe("base", () => {
    it("R408 R60 its Cry exiles 5 different random cards of your deck", () => {
      const s = legion(false, [LUNAR, STOCKPILE, LUNAR, STOCKPILE, LUNAR, STOCKPILE, LUNAR, STOCKPILE]);
      expect(s.pile("p1", "exile")).toHaveLength(5);
      expect(new Set(s.pile("p1", "exile").map((card) => card.id)).size).toBe(5);
      expect(s.pile("p1", "library")).toHaveLength(3);
    });

    it("all of them when fewer than 5", () => {
      const s = legion(false, [LUNAR, STOCKPILE, MENACE]);
      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(unitsOf(s)).toEqual([MENACE]);
    });

    it("R1 the Units among them are summoned out of exile, with no Cry; the rest stay exiled", () => {
      const s = legion(false, [TOKENS, LUNAR, MENACE]);
      expect(unitsOf(s).sort()).toEqual([MENACE, TOKENS].sort());
      expect(s.events.some((event) => event.type === "summoned" && event.defId === RUSH)).toBe(false);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([LUNAR]);
    });

    it("R64 they fill the leftmost open zones in the order they were exiled, until the board is full", () => {
      for (let i = 0; i < 6; i += 1) {
        const s = legion(false, [MENACE, POINTMASTER, TOKENS], { field: [MENACE, { def: MENACE, lane: 5 }] }, `legion-${i}`);
        // The Legion itself animates first, into the leftmost open unit zone (2), before its Cry (R383).
        expect(s.unit("p1", 2)?.defId).toBe(LEGION);
        // Two open zones (3 and 4) for three Units: the first two exiled are summoned, the third stays.
        const exiledOrder = s.events.flatMap((event) => (event.type === "exiled" ? [event.instanceId] : []));
        expect(s.unit("p1", 3)?.id).toBe(exiledOrder[0]);
        expect(s.unit("p1", 4)?.id).toBe(exiledOrder[1]);
        expect(s.pile("p1", "exile").map((card) => card.id)).toEqual([exiledOrder[2]]);
      }
    });

    it("R11 a unit-token card exiled from the deck ceases to exist and is not summoned", () => {
      const s = legion(false, [RUSH, LUNAR]);
      expect(unitsOf(s)).toEqual([]);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([LUNAR]);
    });

    it("R129 an empty deck does nothing and draws nothing from the rng", () => {
      const s = scenario({ p1: { hand: [LEGION, STOCKPILE], library: [], mana: 8 }, p2: { hand: [STOCKPILE] } });
      const cursor = s.state.rngCursor;
      s.play(LEGION);
      expect(s.state.rngCursor).toBe(cursor);
      // Animated (patch v0.2.10, R383), it stands in unit lane 1, and its Cry did nothing.
      expect(s.unit("p1", 1)?.defId).toBe(LEGION);
    });

    it("R97 the exiles are public; no event carries a deck position", () => {
      const s = legion(false, [LUNAR, STOCKPILE, MENACE, LUNAR, STOCKPILE, LUNAR]);
      const exiled = s.view("p2").events.filter((event) => event.type === "exiled");
      expect(exiled).toHaveLength(5);
      for (const event of exiled) if (event.type === "exiled") expect(event.defId).not.toBe("hidden");
      expect(JSON.stringify(s.view("p2").events)).not.toContain('"position"');
    });

    it("R386 the card count reads through param(): a Degrade exiles 4", () => {
      const s = scenario({ p1: { hand: [LEGION, STOCKPILE], library: [LUNAR, LUNAR, LUNAR, LUNAR, LUNAR, LUNAR], mana: 8 }, p2: { hand: [STOCKPILE] } });
      stepParam(s.card(LEGION), "cards", -1);
      s.play(LEGION);
      expect(s.pile("p1", "exile")).toHaveLength(4);
    });
  });

  describe("radiant", () => {
    it("the summoned Units are made Radiant; a Unit left in exile is not", () => {
      // The Legion animates into unit lane 3, leaving lane 4 to one of the two Units.
      const s = legion(true, [MENACE, POINTMASTER, LUNAR], { field: [MENACE, MENACE, { def: MENACE, lane: 5 }] });
      const summoned = s.unit("p1", 4);
      expect(summoned?.radiant).toBe(true);
      const left = s.pile("p1", "exile").filter((card) => card.defId !== LUNAR);
      expect(left).toHaveLength(1);
      expect(left[0]?.radiant).toBe(false);
    });
  });
});

describe("C+ #12.7 Legion of the Hungry: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 2/2 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classicplus-012-7", stats: { attack: 2, health: 2 } });
  });

  it("R383 radiant: a 4/4 Unit", () => {
    expectAnimated({ def: "classicplus-012-7", radiant: true, stats: { attack: 4, health: 4 } });
  });
});
