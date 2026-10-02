// C+ #36 Conjure Bones — SPEC §8.7 row 36, BUILD M9 Classic+ row C+ 36: "Shuffles 7 Bone Storms
// (C+ #36.1) into your deck, each at a uniformly random position; the embiggen price (4) chosen with
// the play (R81) shuffles 17; a deck at `LIBRARY_CAP` (60) turns the rest away with `libraryOverflow`
// (R80); your library list shows them (R311) and `shuffledIn` positions are blank for both players
// (R97); count and paid count read through `param()` (steps 2 and 4); radiant the Bone Storms are
// Radiant".

import { LIBRARY_CAP, stepParam } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const BONES = "classicplus-036";
const BONE_STORM = "classicplus-036-1";
const FILLER = "core-005"; // Stockpile: inert in a library.

function bonesIn(s: Scenario, player: PlayerId = "p1"): number {
  return s.pile(player, "library").filter((card) => card.defId === BONE_STORM).length;
}

function eventsOf<T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

function cast(radiant: boolean, opts: { embiggen?: boolean; library?: number } = {}): Scenario {
  const s = scenario({
    p1: {
      hand: [{ def: BONES, radiant }, FILLER],
      library: Array.from({ length: opts.library ?? 10 }, () => FILLER),
      mana: 4,
    },
    p2: { hand: [FILLER] },
  });
  s.play(BONES, opts.embiggen === true ? { embiggen: true } : {});
  return s;
}

describe("C+ #36 Conjure Bones", () => {
  describe("base", () => {
    it("shuffles 7 base Bone Storms into your deck", () => {
      const s = cast(false);
      expect(bonesIn(s)).toBe(7);
      expect(s.pile("p1", "library")).toHaveLength(17);
      expect(s.pile("p1", "library").filter((card) => card.defId === BONE_STORM).every((card) => !card.radiant)).toBe(true);
      expect(eventsOf(s, "shuffledIn")).toHaveLength(7);
      s.expectMana("p1", 2);
    });

    it("each goes in at a random position: the 7 are not stacked on top or at the bottom", () => {
      const s = cast(false, { library: 20 });
      const library = s.pile("p1", "library");
      const at = library.flatMap((card, index) => (card.defId === BONE_STORM ? [index] : []));
      expect(at).toHaveLength(7);
      expect(at).not.toEqual([0, 1, 2, 3, 4, 5, 6]);
      expect(at).not.toEqual([20, 21, 22, 23, 24, 25, 26]);
    });

    it("R81 the embiggen price (4) is chosen with the play and shuffles 17", () => {
      const s = cast(false, { embiggen: true });
      expect(bonesIn(s)).toBe(17);
      s.expectMana("p1", 0);
    });

    it("R80 a deck at LIBRARY_CAP turns the rest away, each with libraryOverflow", () => {
      const s = cast(false, { library: LIBRARY_CAP - 3 });
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP);
      expect(bonesIn(s)).toBe(3);
      const overflow = eventsOf(s, "libraryOverflow");
      expect(overflow).toHaveLength(4);
      expect(overflow.every((event) => event.defId === BONE_STORM && event.player === "p1")).toBe(true);
    });

    it("R311 your library list shows them, as they went in openly", () => {
      const s = cast(false);
      const own = s.view("p1").you.ownLibrary;
      expect(own?.cards).toContainEqual({ defId: BONE_STORM, radiant: false, count: 7 });
      expect(own?.unknown).toBe(0);
    });

    it("R97 shuffledIn carries no position for either player", () => {
      const s = cast(false);
      for (const viewer of ["p1", "p2"] as const) {
        const shuffled = s.view(viewer).events.filter((event) => event.type === "shuffledIn");
        expect(shuffled.length).toBeGreaterThan(0);
        const real = eventsOf(s, "shuffledIn").map((event) => event.position);
        for (const event of shuffled) {
          if (event.type !== "shuffledIn") continue;
          expect(real).not.toContain(event.position);
        }
      }
      // The opponent sees a count, not the list.
      expect(s.view("p2").opponent.ownLibrary).toBeUndefined();
      expect(s.view("p2").opponent.libraryCount).toBe(17);
    });

    it("R386 count and paid count read through param(): steps 2 and 4", () => {
      const up = scenario({ p1: { hand: [BONES, FILLER], library: [FILLER], mana: 4 }, p2: { hand: [FILLER] } });
      stepParam(up.card(BONES), "count", 1);
      up.play(BONES);
      expect(bonesIn(up)).toBe(9);

      const paid = scenario({ p1: { hand: [BONES, FILLER], library: [FILLER], mana: 4 }, p2: { hand: [FILLER] } });
      stepParam(paid.card(BONES), "paidCount", -1);
      paid.play(BONES, { embiggen: true });
      expect(bonesIn(paid)).toBe(13);
    });
  });

  describe("radiant", () => {
    it("shuffles 7 Radiant Bone Storms", () => {
      const s = cast(true);
      const storms = s.pile("p1", "library").filter((card) => card.defId === BONE_STORM);
      expect(storms).toHaveLength(7);
      expect(storms.every((card) => card.radiant)).toBe(true);
      expect(s.view("p1").you.ownLibrary?.cards).toContainEqual({ defId: BONE_STORM, radiant: true, count: 7 });
    });

    it("paid (4): 17 Radiant Bone Storms", () => {
      const s = cast(true, { embiggen: true });
      const storms = s.pile("p1", "library").filter((card) => card.defId === BONE_STORM);
      expect(storms).toHaveLength(17);
      expect(storms.every((card) => card.radiant)).toBe(true);
    });
  });
});
