// C #80 BOOM! Big Max (SPEC §8.6 row 80; BUILD M9 row C 80). (4) Unit, Legendary, 13/8 → 26/16:
// Tribute 3, Rush, Trample, Indestructible; Radiant: Tribute 3, Charge, Trample, Indestructible
// (balance patch 1: the base attack 26 → 13 and Tribute 2 → 3, so the Radiant face doubles exactly, R275).

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
    it("R101 Tribute 3: it can't be played without Units worth 3; two Units, or a Sheep alone, are not enough", () => {
      const none = scenario({ p1: { hand: [BOOM, STOCKPILE], library: SPARE.library }, p2: SPARE });
      expect(() => none.play(BOOM)).toThrow(/Tribute 3/);
      const two = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA], library: SPARE.library }, p2: SPARE });
      expect(() => two.play(BOOM, { tributes: [TIMMY, VANILLA] })).toThrow();
      const sheep = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [SHEEP], library: SPARE.library }, p2: SPARE });
      expect(() => sheep.play(BOOM, { tributes: [SHEEP] })).toThrow();
    });

    it("R101 `legalActions` agrees: no play with two Units, and with three only the three that pay", () => {
      const two = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA], library: SPARE.library }, p2: SPARE });
      const boomOf = (s: typeof two) => s.card(BOOM).id;
      expect(legalActions(two.state, "p1").some((a) => a.type === "play" && a.instanceId === boomOf(two))).toBe(false);
      const three = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA, VANILLA], library: SPARE.library }, p2: SPARE });
      const plays = legalActions(three.state, "p1").flatMap((a) => (a.type === "play" && a.instanceId === boomOf(three) ? [a] : []));
      expect(plays.length).toBeGreaterThan(0);
      const all = [1, 2, 3].map((lane) => three.unit("p1", lane)?.id ?? "").sort();
      for (const play of plays) expect([...(play.tributes ?? [])].sort()).toEqual(all);
    });

    it("R101 three Units pay, and so does a Sheep Token, worth 2, with one more Unit", () => {
      const three = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA, VANILLA], library: SPARE.library }, p2: SPARE });
      three.play(BOOM, { tributes: [1, 2, 3].map((lane) => three.unit("p1", lane)?.id ?? "") });
      three.expectInZone(BOOM, "field").expectInZone(TIMMY, "graveyard");
      const sheep = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [SHEEP, TIMMY], library: SPARE.library }, p2: SPARE });
      sheep.play(BOOM, { tributes: [SHEEP, TIMMY] });
      sheep.expectInZone(BOOM, "field");
    });

    it("R391 on a full row it may take a tributed one-card pile's zone", () => {
      const s = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, VANILLA, VANILLA, VANILLA, VANILLA], library: SPARE.library }, p2: SPARE });
      const second = s.unit("p1", 2)!;
      const third = s.unit("p1", 3)!;
      s.play(BOOM, { zone: 1, tributes: [TIMMY, second.id, third.id] });
      expect(s.unit("p1", 1)?.defId).toBe(BOOM);
    });

    it("§4.1, R63 Rush and Trample: it attacks a Unit the turn it enters, the excess over the defender's health hitting their hero", () => {
      const s = scenario({ p1: { hand: [BOOM, STOCKPILE], field: [TIMMY, SHEEP], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      s.play(BOOM, { tributes: [SHEEP, TIMMY] });
      expect(() => s.attack(BOOM, "hero")).toThrow();
      s.attack(BOOM, VANILLA);
      // 13 into a 4/4: 4 to it, 9 to its controller's hero.
      s.expectHealth("p2", 21);
      s.expectStats(BOOM, { attack: 13, health: 8 });
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
      const s = scenario({ p1: { hand: [{ def: BOOM, radiant: true }, STOCKPILE], field: [SHEEP, TIMMY], library: SPARE.library }, p2: SPARE });
      s.play(BOOM, { tributes: [SHEEP, TIMMY] });
      s.expectStats(BOOM, { attack: 26, health: 16 });
      s.attack(BOOM, "hero");
      s.expectHealth("p2", 4);
    });

    it("R101 the Radiant face keeps Tribute 3, Trample and Indestructible", () => {
      const s = scenario({ p1: { hand: [{ def: BOOM, radiant: true }, STOCKPILE], field: [TIMMY, VANILLA], library: SPARE.library }, p2: SPARE });
      expect(() => s.play(BOOM, { tributes: [TIMMY, VANILLA] })).toThrow();
      const field = scenario({ p1: { field: [{ def: BOOM, radiant: true }] } });
      const keywords = field.stats(BOOM).keywords;
      expect(["Charge", "Trample", "Indestructible"].every((kind) => hasKeyword(keywords, kind as "Charge"))).toBe(true);
      expect(hasKeyword(keywords, "Rush")).toBe(false);
    });
  });
});
