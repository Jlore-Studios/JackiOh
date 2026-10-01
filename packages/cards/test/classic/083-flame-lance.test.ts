// C #83 Flame Lance (SPEC §8.6 row 83; BUILD M9 row C 83). (3) Spell, Common: Trample; deal {damage}
// damage to a Unit — damage 11, Radiant 22, step 2.

import { describe, expect, it } from "vitest";
import { legalActions, stepParam } from "@jackioh/engine";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const LANCE = "classic-083";
const VANILLA = "core-008"; // 4/4
const JILLIAX = "core-056"; // 3/2 Rush, Taunt, Lifesteal, Divine Shield
const THE_ROCK = "core-066"; // 10/10 Indestructible
const STOCKPILE = "core-005";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

function lanceAt(s: Scenario, instanceId: string): Scenario {
  return s.play(LANCE, { targets: [{ pick: "instance", instanceId }] });
}

function hitsOf(s: Scenario): { targetId: string; amount: number }[] {
  return s.lastEvents.flatMap((e) => (e.type === "damage" ? [{ targetId: e.targetId, amount: e.amount }] : []));
}

describe("C #83 Flame Lance", () => {
  describe("base", () => {
    it("R90 a Unit target only: no hero is offered, and naming one is refused", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      const lance = s.card(LANCE);
      const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.instanceId === lance.id);
      expect(plays.length).toBeGreaterThan(0);
      for (const play of plays) {
        if (play.type !== "play") continue;
        expect(play.targets?.every((t) => t.pick === "instance")).toBe(true);
      }
      expect(() => s.play(LANCE, { targets: [{ pick: "hero", player: "p2" }] })).toThrow();
    });

    it("§4.4 step 9 Trample: 11 into a 4/4, the 7 beyond its health a new instance on its controller's hero", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      const foe = s.unit("p2", 1)!;
      lanceAt(s, foe.id);
      expect(hitsOf(s)).toEqual([
        { targetId: foe.id, amount: 4 },
        { targetId: "hero-p2", amount: 7 },
      ]);
      s.expectInZone(foe, "graveyard").expectHealth("p2", 23);
    });

    it("R63 the target's health before the hit is its current health: 11 into a 4/4 with 3 damage tramples 10", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [{ def: VANILLA, damage: 3 }], ...SPARE } });
      const foe = s.unit("p2", 1)!;
      lanceAt(s, foe.id);
      expect(hitsOf(s)).toEqual([
        { targetId: foe.id, amount: 1 },
        { targetId: "hero-p2", amount: 10 },
      ]);
    });

    it("R90 with no Unit on the field it is still played, `legalActions` agreeing, and fizzles: no hit, no hero damage", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: SPARE });
      const lance = s.card(LANCE);
      expect(legalActions(s.state, "p1").some((a) => a.type === "play" && a.instanceId === lance.id)).toBe(true);
      s.play(LANCE);
      expect(hitsOf(s)).toEqual([]);
      s.expectInZone(lance, "graveyard").expectHealth("p1", 30).expectHealth("p2", 30);
    });

    it("§8.6 either side: on your own Unit the excess hits your own hero", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], field: [VANILLA], library: SPARE.library }, p2: SPARE });
      lanceAt(s, s.unit("p1", 1)!.id);
      s.expectHealth("p1", 23);
    });

    it("§4.4 step 2 Armor lowers the hit before the excess is taken", () => {
      // A Unit in Defense Position has Armor 1: 11 − 1 = 10, 4 to it and 6 beyond.
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [{ def: VANILLA, position: "DEF" }], ...SPARE } });
      const foe = s.unit("p2", 1)!;
      lanceAt(s, foe.id);
      expect(hitsOf(s)).toEqual([
        { targetId: foe.id, amount: 4 },
        { targetId: "hero-p2", amount: 6 },
      ]);
    });

    it("§4.4 steps 1 and 4: a Divine Shield or an Indestructible target stops the hit, and nothing tramples", () => {
      const shield = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [JILLIAX], ...SPARE } });
      lanceAt(shield, shield.unit("p2", 1)!.id);
      expect(hitsOf(shield)).toEqual([]);
      expect(shield.lastEvents.some((e) => e.type === "divineShieldLost")).toBe(true);
      shield.expectHealth("p2", 30);

      const rock = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [THE_ROCK], ...SPARE } });
      lanceAt(rock, rock.unit("p2", 1)!.id);
      expect(hitsOf(rock)).toEqual([]);
      rock.expectHealth("p2", 30);
    });

    it("R386 its tuned number: an Upgrade's step of 2 makes it 13, 9 beyond a 4/4", () => {
      const s = scenario({ p1: { hand: [LANCE, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      stepParam(s.card(LANCE), "damage", 1);
      lanceAt(s, s.unit("p2", 1)!.id);
      expect(hitsOf(s).map((hit) => hit.amount)).toEqual([4, 9]);
    });
  });

  describe("radiant", () => {
    it("§8.6 22 damage: 18 beyond a 4/4", () => {
      const s = scenario({ p1: { hand: [{ def: LANCE, radiant: true }, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      lanceAt(s, s.unit("p2", 1)!.id);
      expect(hitsOf(s).map((hit) => hit.amount)).toEqual([4, 18]);
      s.expectHealth("p2", 12);
    });

    it("R386 a Degrade's step of 2 makes the Radiant 20", () => {
      const s = scenario({ p1: { hand: [{ def: LANCE, radiant: true }, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      stepParam(s.card(LANCE), "damage", -1);
      lanceAt(s, s.unit("p2", 1)!.id);
      expect(hitsOf(s).map((hit) => hit.amount)).toEqual([4, 16]);
    });
  });
});
