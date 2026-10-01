// C+ #12.3 Fluffy Grip — SPEC §8.7 row 12.3, BUILD M9 Classic+ row C+ 12.3: "A random Unit card of the
// opponent's deck (R60) moves to your hand and becomes yours (R12), `costOverride` 0; a deck with no
// Unit, or an empty deck, gives nothing and draws nothing (R129); a full hand burns it into your
// graveyard, now its owner's (R12); `stolen` names the card only to you, the opponent reading the
// sentinel and a deck one card smaller (R97); a unit-token card follows R11; radiant it also becomes
// Radiant".

import { HAND_CAP, effectiveCost } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const GRIP = "classicplus-012-3";
const MENACE = "core-019"; // (3) Unit
const ROCK = "core-066"; // (4) Unit
const LUNAR = "core-035"; // a Spell
const FILLER = "core-005"; // a Spell
const RUSH_TOKEN = "core-t-rush"; // a unit-token card (R11)

function grip(radiant: boolean, p2Library: SideSetup["library"], p1: SideSetup = {}, seed?: string): Scenario {
  const s = scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { hand: [{ def: GRIP, radiant }, FILLER], ...p1 },
    p2: { hand: [FILLER], library: p2Library },
  });
  s.play(GRIP);
  return s;
}

describe("C+ #12.3 Fluffy Grip", () => {
  describe("base", () => {
    it("R12 a Unit card of the opponent's deck moves to your hand, becomes yours and costs (0)", () => {
      const s = grip(false, [LUNAR, MENACE, FILLER]);
      const taken = s.hand("p1").find((card) => card.defId === MENACE);
      expect(taken).toBeDefined();
      expect(taken?.owner).toBe("p1");
      expect(taken?.costOverride).toBe(0);
      expect(effectiveCost(s.state, taken ?? s.card(MENACE))).toBe(0);
      expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([LUNAR, FILLER]);
    });

    it("R60 the Unit is a random one of the deck's Units, never a Spell", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 16; i += 1) {
        const s = grip(false, [LUNAR, MENACE, FILLER, ROCK, LUNAR], {}, `grip-${i}`);
        const taken = s.hand("p1").filter((card) => card.owner === "p1" && card.defId !== FILLER);
        expect(taken).toHaveLength(1);
        seen.add(taken[0]?.defId ?? "");
      }
      expect([...seen].sort()).toEqual([MENACE, ROCK].sort());
    });

    it("R129 a deck with no Unit, or an empty one, gives nothing and draws nothing from the rng", () => {
      const spells = scenario({ p1: { hand: [GRIP, FILLER] }, p2: { hand: [FILLER], library: [LUNAR, FILLER] } });
      const cursor = spells.state.rngCursor;
      spells.play(GRIP);
      expect(spells.state.rngCursor).toBe(cursor);
      expect(spells.pile("p2", "library")).toHaveLength(2);
      expect(spells.hand("p1").map((card) => card.defId)).toEqual([FILLER]);

      const empty = grip(false, []);
      expect(empty.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
    });

    it("R12 a full hand burns it into your graveyard, now its owner's", () => {
      // Grip and ten more: after Grip is played the hand is at its cap.
      const full = Array.from({ length: HAND_CAP }, () => FILLER);
      const s = grip(false, [MENACE], { hand: [GRIP, ...full] });
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      const burned = s.pile("p1", "graveyard").find((card) => card.defId === MENACE);
      expect(burned?.owner).toBe("p1");
      expect(s.pile("p2", "graveyard").some((card) => card.defId === MENACE)).toBe(false);
    });

    it("R11 R218 a unit-token card leaves a library only by a draw, so the steal passes over it", () => {
      const only = scenario({ p1: { hand: [GRIP, FILLER] }, p2: { hand: [FILLER], library: [RUSH_TOKEN] } });
      const cursor = only.state.rngCursor;
      only.play(GRIP);
      expect(only.pile("p2", "library").map((card) => card.defId)).toEqual([RUSH_TOKEN]);
      expect(only.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
      expect(only.state.rngCursor).toBe(cursor);

      for (let i = 0; i < 6; i += 1) {
        const s = grip(false, [RUSH_TOKEN, MENACE, RUSH_TOKEN], {}, `grip-token-${i}`);
        expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER, MENACE]);
        expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([RUSH_TOKEN, RUSH_TOKEN]);
      }
    });

    it("R97 `stolen` names the card to you only; the opponent sees the sentinel and a smaller deck", () => {
      const s = grip(false, [MENACE, FILLER]);
      const mine = s.view("p1").events.find((event) => event.type === "stolen");
      expect(mine).toMatchObject({ type: "stolen", defId: MENACE, from: "p2", to: "p1" });
      const theirs = s.view("p2").events.find((event) => event.type === "stolen");
      expect(theirs).toBeDefined();
      expect(theirs?.type === "stolen" && theirs.defId).not.toBe(MENACE);
      expect(JSON.stringify(s.view("p2"))).not.toContain(MENACE);
      expect(s.view("p2").you.libraryCount).toBe(1);
    });
  });

  describe("radiant", () => {
    it("the stolen Unit also becomes Radiant", () => {
      const s = grip(true, [MENACE]);
      const taken = s.hand("p1").find((card) => card.defId === MENACE);
      expect(taken?.radiant).toBe(true);
      expect(taken?.costOverride).toBe(0);
      expect(taken?.owner).toBe("p1");
    });
  });
});
