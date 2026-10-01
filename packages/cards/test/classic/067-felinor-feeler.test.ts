// C #67 Felinor Feeler (SPEC §8.6 row 67; BUILD M9 row C 67). (1) Unit, Human, Common, 2/4 → 4/8:
// Pierce; Cry: switch every enemy Unit to Defense Position. Radiant: Pierce, Rush; the same Cry.

import { describe, expect, it } from "vitest";
import { beneathAt } from "@jackioh/engine";
import { catalog } from "../../src/query";
import { def } from "../../src/scripts/classic/067-felinor-feeler";
import { scenario, type SideSetup } from "../_harness";

const FEELER = "classic-067";
const VANILLA = "core-008"; // 4/4
const TIMMY = "core-011"; // 3/3
const PILLOW = "core-065-1"; // Spikey Pillow: can't be in Defense Position
const ARMORED = "core-025"; // 4-mana 7/7, Armor 7
const STOCKPILE = "core-005";
const FIENDER = "core-092"; // Stack; has the stats of all your Felinors

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

describe("C #67 Felinor Feeler", () => {
  describe("base", () => {
    it("R20 Cry: every enemy Unit switches to Defense Position as an effect, spending no exertion; yours stay", () => {
      const s = scenario({ p1: { hand: [FEELER, STOCKPILE], field: [TIMMY], library: SPARE.library }, p2: { field: [VANILLA, TIMMY], ...SPARE } });
      s.play(FEELER);
      for (const lane of [1, 2]) {
        const unit = s.unit("p2", lane);
        expect(s.stats(unit!).position).toBe("DEF");
        expect(s.card(unit!).exertion.switched).toBe(false);
      }
      expect(s.stats(s.unit("p1", 1)!).position).toBe("ATK");
      expect(s.lastEvents.filter((e) => e.type === "positionSwitched")).toHaveLength(2);
    });

    it("R91 a Unit already in Defense Position stays, with no switch reported", () => {
      const s = scenario({ p1: { hand: [FEELER, STOCKPILE], library: SPARE.library }, p2: { field: [{ def: VANILLA, position: "DEF" }, TIMMY], ...SPARE } });
      s.play(FEELER);
      expect(s.stats(s.unit("p2", 1)!).position).toBe("DEF");
      expect(s.stats(s.unit("p2", 2)!).position).toBe("DEF");
      const switched = s.lastEvents.flatMap((e) => (e.type === "positionSwitched" ? [e.instanceId] : []));
      expect(switched).toEqual([s.unit("p2", 2)!.id]);
    });

    it("§4.1 Spikey Pillow, which can't be in Defense Position, stays in Attack", () => {
      const s = scenario({ p1: { hand: [FEELER, STOCKPILE], library: SPARE.library }, p2: { field: [PILLOW, VANILLA], ...SPARE } });
      s.play(FEELER);
      expect(s.stats(s.unit("p2", 1)!).position).toBe("ATK");
      expect(s.stats(s.unit("p2", 2)!).position).toBe("DEF");
    });

    it("R346 Pierce: its hits skip Armor", () => {
      const s = scenario({ p1: { field: [FEELER], ...SPARE }, p2: { field: [ARMORED], ...SPARE } });
      s.attack(FEELER, ARMORED);
      // 2 attack into Armor 7: Pierce lands all 2.
      expect(s.card(ARMORED).damage).toBe(2);
    });

    it("§8.6 tagged Human, not Felinor: a Felinor pool never finds it", () => {
      expect(def.tags).toContain("Human");
      expect(def.tags).not.toContain("Felinor");
      expect(catalog.query({ tags: ["Felinor"] }).map((card) => card.id)).not.toContain(FEELER);
      expect(catalog.query({ tags: ["Human"] }).map((card) => card.id)).toContain(FEELER);
    });

    it("§8.6 nor a Felinor count: Felinor Fiender's stats are the same beside it", () => {
      const alone = scenario({ p1: { field: [FIENDER] } });
      const beside = scenario({ p1: { field: [FIENDER, FEELER] } });
      expect(beside.stats(FIENDER)).toMatchObject({ attack: alone.stats(FIENDER).attack, health: alone.stats(FIENDER).health });
    });

    it("R13 a card dormant under an enemy Stack pile is not on the field and keeps its position", () => {
      const s = scenario({ p1: { hand: [FEELER, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA, { def: FIENDER, stack: true }], ...SPARE } });
      const dormant = beneathAt(s.state, { player: "p2", row: "units", lane: 1 })[0]!;
      expect(dormant.defId).toBe(VANILLA);
      s.play(FEELER);
      expect(s.stats(s.unit("p2", 1)!).position).toBe("DEF");
      expect(s.card(dormant).position).toBe("ATK");
      expect(s.lastEvents.filter((e) => e.type === "positionSwitched")).toHaveLength(1);
    });

    it("§8.6 a 2/4 with Pierce and no Rush: it cannot attack the turn it enters", () => {
      const s = scenario({ p1: { hand: [FEELER, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA], ...SPARE } });
      s.play(FEELER);
      s.expectStats(FEELER, { attack: 2, health: 4 });
      expect(() => s.attack(FEELER, VANILLA)).toThrow();
    });
  });

  describe("radiant", () => {
    it("§8.6 4/8 Pierce, Rush: the same Cry, and it may attack a Unit the turn it enters", () => {
      const s = scenario({ p1: { hand: [{ def: FEELER, radiant: true }, STOCKPILE], library: SPARE.library }, p2: { field: [VANILLA, PILLOW], ...SPARE } });
      s.play(FEELER);
      s.expectStats(FEELER, { attack: 4, health: 8 });
      expect(s.stats(s.unit("p2", 1)!).position).toBe("DEF");
      expect(s.stats(s.unit("p2", 2)!).position).toBe("ATK");
      const foe = s.unit("p2", 1)!;
      s.attack(FEELER, foe);
      // 4 into the Vanilla's Defense Armor 1: Pierce lands all 4.
      s.expectInZone(foe, "graveyard");
      expect(() => s.attack(FEELER, "hero")).toThrow();
    });
  });
});
