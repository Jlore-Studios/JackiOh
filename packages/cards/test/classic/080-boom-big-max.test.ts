// C #80 BOOM! Big Max (SPEC §8.6 row 80; BUILD M9 row C 80). (4) Unit, Legendary, 26/8 → 26/16:
// Tribute 2, Rush, Trample, Indestructible; Radiant: Tribute 2, Charge, Trample, Indestructible. The
// Radiant attack, 26 by the designer's number, is a named exception (`STAT_EXCEPTIONS`) in
// `test/radiant-standard.test.ts` (R275).

import { describe, expect, it } from "vitest";
import { legalActions } from "@jackioh/engine";
import { hasKeyword } from "@jackioh/shared";
import { scenario, type SideSetup } from "../_harness";

const BOOM = "classic-080";
const VANILLA = "core-008"; // 4/4
const TIMMY = "core-011"; // 3/3
const SHEEP = "core-t-sheep"; // worth 2 Tributes
const HIT_JOB = "core-016";
const STOCKPILE = "core-005";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

describe("C #80 BOOM! Big Max", () => {
  describe("base", () => {
    it("R101 Tribute 2: it can't be played without Units worth 2; one Unit is not enough", () => {
      const none = scenario({ p1: { hand: [BOOM, STOCKPILE], library: SPARE.library }, p2: SPARE });
      expect(() => none.play(BOOM)).toThrow(/Tribute 2/);
      const one = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY], library: SPARE.library }, p2: SPARE });
      expect(() => one.play(BOOM, { tributes: [TIMMY] })).toThrow();
    });

    it("R101 `legalActions` agrees: no play with one Unit, and with two only the pair that pays", () => {
      const one = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY], library: SPARE.library }, p2: SPARE });
      const boomOf = (s: typeof one) => s.card(BOOM).id;
      expect(legalActions(one.state, "p1").some((a) => a.type === "play" && a.instanceId === boomOf(one))).toBe(false);
      const two = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA], library: SPARE.library }, p2: SPARE });
      const plays = legalActions(two.state, "p1").flatMap((a) => (a.type === "play" && a.instanceId === boomOf(two) ? [a] : []));
      expect(plays.length).toBeGreaterThan(0);
      const pair = [two.card(TIMMY).id, two.card(VANILLA).id].sort();
      for (const play of plays) expect([...(play.tributes ?? [])].sort()).toEqual(pair);
    });

    it("R101 two Units pay, and so does a Sheep Token alone, worth 2", () => {
      const two = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA], library: SPARE.library }, p2: SPARE });
      two.play(BOOM, { tributes: [TIMMY, VANILLA] });
      two.expectInZone(BOOM, "field").expectInZone(TIMMY, "graveyard");
      const sheep = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [SHEEP], library: SPARE.library }, p2: SPARE });
      sheep.play(BOOM, { tributes: [SHEEP] });
      sheep.expectInZone(BOOM, "field");
    });

    it("R391 on a full row it may take a tributed one-card pile's zone", () => {
      const s = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA, VANILLA, VANILLA, VANILLA], library: SPARE.library }, p2: SPARE });
      const second = s.unit("p1", 2)!;
      s.play(BOOM, { zone: 1, tributes: [TIMMY, second.id] });
      expect(s.unit("p1", 1)?.defId).toBe(BOOM);
    });

    it("§4.1, R63 Rush and Trample: it attacks a Unit the turn it enters, the excess over the defender's health hitting their hero", () => {
      const s = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, SHEEP], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      s.play(BOOM, { tributes: [SHEEP] });
      expect(() => s.attack(BOOM, "hero")).toThrow();
      s.attack(BOOM, VANILLA);
      // 26 into a 4/4: 4 to it, 22 to its controller's hero.
      s.expectHealth("p2", 8);
      s.expectStats(BOOM, { attack: 26, health: 8 });
    });

    it("R46 Indestructible: it takes no damage, and a destroy knocks it into Attack Position", () => {
      const s = scenario({ p1: { field: [{ def: BOOM, position: "DEF" }], ...SPARE }, p2: { field: [VANILLA], hand: [HIT_JOB, STOCKPILE], library: SPARE.library }, active: "p2" });
      s.attack(VANILLA, BOOM);
      expect(s.card(BOOM).damage).toBe(0);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BOOM).id }] });
      s.expectInZone(BOOM, "field");
      expect(s.stats(BOOM).position).toBe("ATK");
    });

    it("R347 it never has Taunt, not even in Defense Position", () => {
      const s = scenario({ p1: { field: [{ def: BOOM, position: "DEF" }], ...SPARE }, p2: SPARE });
      expect(hasKeyword(s.stats(BOOM).keywords, "Indestructible")).toBe(true);
      expect(hasKeyword(s.stats(BOOM).keywords, "Taunt")).toBe(false);
    });
  });

  describe("radiant", () => {
    it("§8.6 26/16 Charge: it may attack the hero the turn it enters", () => {
      const s = scenario({ p1: { hand: [{ def: BOOM, radiant: true }, STOCKPILE], field: [SHEEP], library: SPARE.library }, p2: SPARE });
      s.play(BOOM, { tributes: [SHEEP] });
      s.expectStats(BOOM, { attack: 26, health: 16 });
      s.attack(BOOM, "hero");
      s.expectHealth("p2", 4);
    });

    it("R101 the Radiant face keeps Tribute 2, Trample and Indestructible", () => {
      const s = scenario({ p1: { hand: [{ def: BOOM, radiant: true }, STOCKPILE], field: [TIMMY], library: SPARE.library }, p2: SPARE });
      expect(() => s.play(BOOM, { tributes: [TIMMY] })).toThrow();
      const field = scenario({ p1: { field: [{ def: BOOM, radiant: true }] } });
      const keywords = field.stats(BOOM).keywords;
      expect(["Charge", "Trample", "Indestructible"].every((kind) => hasKeyword(keywords, kind as "Charge"))).toBe(true);
      expect(hasKeyword(keywords, "Rush")).toBe(false);
    });
  });
});
