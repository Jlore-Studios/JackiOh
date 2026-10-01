// C #73 Nurse Cleaver (SPEC §8.6 row 73; BUILD M9 row C 73). (2) Unit, Common, 3/6 → 6/12: Rush,
// Cleave, Lifesteal; Radiant: Charge, Cleave, Lifesteal. Keywords only.

import { describe, expect, it } from "vitest";
import { scenario, type SideSetup } from "../_harness";

const CLEAVER = "classic-073";
const VANILLA = "core-008"; // 4/4
const STOCKPILE = "core-005";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

/** Three 4/4s side by side, the middle one the defender. */
const ROW: SideSetup = { field: [VANILLA, VANILLA, VANILLA], ...SPARE };

describe("C #73 Nurse Cleaver", () => {
  describe("base", () => {
    it("§4.1 Rush: it attacks a Unit the turn it enters, never the hero", () => {
      const s = scenario({ p1: { hand: [CLEAVER, STOCKPILE], library: SPARE.library }, p2: ROW });
      s.play(CLEAVER);
      s.expectStats(CLEAVER, { attack: 3, health: 6 });
      expect(() => s.attack(CLEAVER, "hero")).toThrow();
      s.attack(CLEAVER, s.unit("p2", 2)!);
      expect(s.card(s.unit("p2", 2)!).damage).toBe(3);
    });

    it("§4.4 step 10 Cleave: the defender's neighbours are hit too, each as a separate instance", () => {
      const s = scenario({ p1: { field: [CLEAVER], ...SPARE }, p2: ROW });
      const [left, middle, right] = [s.unit("p2", 1)!, s.unit("p2", 2)!, s.unit("p2", 3)!];
      s.attack(CLEAVER, middle);
      const hits = s.lastEvents.flatMap((e) => (e.type === "damage" && e.sourceId === s.card(CLEAVER).id ? [e.targetId] : []));
      expect(hits.sort()).toEqual([left.id, middle.id, right.id].sort());
      expect([left, middle, right].map((unit) => s.card(unit).damage)).toEqual([3, 3, 3]);
    });

    it("§4.4 step 8 Lifesteal: your hero heals for each hit it deals, the Cleave hits included", () => {
      const s = scenario({ p1: { field: [CLEAVER], health: 10, ...SPARE }, p2: ROW });
      s.attack(CLEAVER, s.unit("p2", 2)!);
      // Three hits of 3; the defender's 4 back does not heal anyone.
      s.expectHealth("p1", 19);
    });
  });

  describe("radiant", () => {
    it("§4.1 6/12 Charge, Cleave, Lifesteal: it may attack the hero the turn it enters", () => {
      const s = scenario({ p1: { hand: [{ def: CLEAVER, radiant: true }, STOCKPILE], health: 10, library: SPARE.library }, p2: SPARE });
      s.play(CLEAVER);
      s.expectStats(CLEAVER, { attack: 6, health: 12 });
      s.attack(CLEAVER, "hero");
      s.expectHealth("p2", 24).expectHealth("p1", 16);
    });

    it("§4.4 its Cleave and Lifesteal: three hits of 6 heal 18", () => {
      const s = scenario({ p1: { field: [{ def: CLEAVER, radiant: true }], health: 10, ...SPARE }, p2: { field: [VANILLA, VANILLA, VANILLA], ...SPARE } });
      const middle = s.unit("p2", 2)!;
      s.attack(CLEAVER, middle);
      // Three hits of 6 (no Trample, so each deals its whole 6, R63), and each heals.
      const dealt = s.lastEvents.flatMap((e) => (e.type === "damage" && e.sourceId === s.card(CLEAVER).id ? [e.amount] : []));
      expect(dealt).toEqual([6, 6, 6]);
      s.expectHealth("p1", 28);
    });
  });
});
