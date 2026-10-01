// C #85 King Wagtoggle (SPEC §8.6 row 85; BUILD M9 row C 85). (4) Unit, Legendary, 5/5 → 10/10: Cry:
// swap decks with your opponent. Radiant: then Recruit {recruits} (1, ↑).

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import { scenario, type SideSetup } from "../_harness";

const KING = "classic-085";
const VANILLA = "core-008"; // 4/4
const TIMMY = "core-011";
const MENACE = "core-019";
const STOCKPILE = "core-005";
const NOTEBOOK = "core-051-1"; // Spell 1: draw 1
const MANA_WELL = "core-006"; // Field Spell
const SHEEPISH = "core-041"; // Trap

const P2_HAND: SideSetup = { hand: [STOCKPILE] };

describe("C #85 King Wagtoggle", () => {
  describe("base", () => {
    it("R73 Cry: the decks swap whole, and each card's owner becomes the player whose deck now holds it (R12)", () => {
      const s = scenario({ p1: { hand: [KING, STOCKPILE], library: [VANILLA, TIMMY] }, p2: { ...P2_HAND, library: [MENACE, STOCKPILE, MANA_WELL] } });
      const theirs = s.pile("p2", "library").map((card) => card.id);
      const mine = s.pile("p1", "library").map((card) => card.id);
      s.play(KING);
      expect(s.pile("p1", "library").map((card) => card.id)).toEqual(theirs);
      expect(s.pile("p2", "library").map((card) => card.id)).toEqual(mine);
      expect(s.pile("p1", "library").every((card) => card.owner === "p1")).toBe(true);
      expect(s.pile("p2", "library").every((card) => card.owner === "p2")).toBe(true);
      // No event carries a position: the swap is one `swapped`.
      expect(s.lastEvents.filter((e) => e.type === "swapped")).toEqual([{ type: "swapped", what: "library" }]);
      expect(s.lastEvents.some((e) => e.type === "shuffledIn")).toBe(false);
    });

    it("R73 the fatigue counts stay with the players, and an empty deck swaps too", () => {
      const s = scenario({ p1: { hand: [NOTEBOOK, KING, STOCKPILE], library: [], mana: 5 }, p2: { ...P2_HAND, library: [MENACE, TIMMY, VANILLA] } });
      s.play(NOTEBOOK);
      expect(s.state.players.p1.fatigueCount).toBe(1);
      s.play(KING);
      expect(s.pile("p1", "library")).toHaveLength(3);
      expect(s.pile("p2", "library")).toEqual([]);
      expect([s.state.players.p1.fatigueCount, s.state.players.p2.fatigueCount]).toEqual([1, 0]);
    });

    it("R311 each player's deck list names only what they were shown: the new deck is unknown cards", () => {
      const s = scenario({ p1: { hand: [KING, STOCKPILE], library: [VANILLA, TIMMY] }, p2: { ...P2_HAND, library: [MENACE, STOCKPILE, MANA_WELL] } });
      expect(s.view("p1").you.ownLibrary?.cards.length).toBeGreaterThan(0);
      s.play(KING);
      for (const viewer of ["p1", "p2"] as const) {
        const list = s.view(viewer).you.ownLibrary;
        expect(list?.cards).toEqual([]);
        expect(list?.unknown).toBe(s.pile(viewer, "library").length);
      }
    });

    it("§8.6 a 5/5", () => {
      const s = scenario({ p1: { hand: [KING, STOCKPILE], library: [VANILLA] }, p2: P2_HAND });
      s.play(KING);
      s.expectStats(KING, { attack: 5, health: 5 });
      expect(s.unit("p1", 2)).toBeNull();
    });
  });

  describe("radiant", () => {
    it("§6.3 then it Recruits the first permanent from your new deck", () => {
      const s = scenario({ p1: { hand: [{ def: KING, radiant: true }, STOCKPILE], library: [TIMMY] }, p2: { ...P2_HAND, library: [STOCKPILE, MENACE, VANILLA] } });
      s.play(KING);
      s.expectStats(KING, { attack: 10, health: 10 });
      const recruit = s.unit("p1", 2)!;
      expect(recruit.defId).toBe(MENACE);
      expect(recruit.owner).toBe("p1");
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
    });

    it("§6.3 no permanent in the new deck, or a full row, recruits nothing", () => {
      const none = scenario({ p1: { hand: [{ def: KING, radiant: true }, STOCKPILE], library: [TIMMY] }, p2: { ...P2_HAND, library: [STOCKPILE] } });
      none.play(KING);
      expect(none.unit("p1", 2)).toBeNull();

      const full = scenario({
        p1: { hand: [{ def: KING, radiant: true }, STOCKPILE], field: [{ def: TIMMY, lane: 1 }, { def: TIMMY, lane: 2 }, { def: TIMMY, lane: 3 }, { def: TIMMY, lane: 4 }], library: [] },
        p2: { ...P2_HAND, library: [MENACE] },
      });
      full.play(KING, { zone: 5 });
      expect(full.pile("p1", "library").map((card) => card.defId)).toEqual([MENACE]);
    });

    it("R33 a Trap it recruits lands face-down, and the other player is not told which", () => {
      const s = scenario({ p1: { hand: [{ def: KING, radiant: true }, STOCKPILE], library: [TIMMY] }, p2: { ...P2_HAND, library: [SHEEPISH] } });
      s.play(KING);
      const trap = s.backrow("p1", 1)!;
      expect(trap.defId).toBe(SHEEPISH);
      expect(trap.faceUp === true).toBe(false);
      expect(s.view("p2").opponent.backrow[0]).toMatchObject({ faceDown: true });
      expect(JSON.stringify(s.view("p2").events)).not.toContain(SHEEPISH);
    });

    it("R386 its tuned number: one Upgrade makes it Recruit 2", () => {
      const s = scenario({ p1: { hand: [{ def: KING, radiant: true }, STOCKPILE], library: [TIMMY] }, p2: { ...P2_HAND, library: [MENACE, VANILLA, TIMMY] } });
      stepParam(s.card(KING), "recruits", 1);
      s.play(KING);
      expect([s.unit("p1", 2)?.defId, s.unit("p1", 3)?.defId]).toEqual([MENACE, VANILLA]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TIMMY]);
    });
  });
});
