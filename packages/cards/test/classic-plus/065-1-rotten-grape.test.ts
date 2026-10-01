// C+ #65.1 Rotten Grape — SPEC §8.7 row 65.1, BUILD M9 Classic+ row C+ 65.1: "Your hero loses 5 health
// (R18): no Armor, no cap, no Blood Moon, no on-damage effect, and at 0 you lose at the state check;
// radiant loses 1".

import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-1-rotten-grape";

const ROTTEN = "classicplus-065-1";
const FILLER = "core-005";
const GOING_LONG = "core-084"; // Field Spell: "Your hero has Armor 2."
const ANTI_ONESHOT = "core-073"; // Field Spell: "Your hero can't take more than 5 damage at once."

describe("C+ #65.1 Rotten Grape", () => {
  it("is a (1) Fruit Spell token with the printed rarity Common", () => {
    expect(def.id).toBe(ROTTEN);
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["Fruit", "Token"]);
    expect(def.rarity).toBe("Token");
    expect(def.printedRarity).toBe("Common");
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  describe("base", () => {
    it("R18 your hero loses 5 health: a healthLost, never a damage instance", () => {
      const s = scenario({ p1: { hand: [ROTTEN, FILLER] }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);

      s.expectHealth("p1", 25).expectHealth("p2", 30);
      expect(s.lastEvents.filter((event) => event.type === "healthLost")).toEqual([{ type: "healthLost", player: "p1", amount: 5 }]);
      expect(s.lastEvents.some((event) => event.type === "damage")).toBe(false);
      s.expectInZone(ROTTEN, "graveyard");
    });

    it("R18 Armor takes none of it: a hero with Armor 2 still loses 5", () => {
      const s = scenario({ p1: { hand: [ROTTEN, FILLER], backrow: [{ def: GOING_LONG, faceUp: true }] }, p2: { hand: [FILLER] } });
      expect(s.view("p1").you.hero.armor).toBe(2);
      s.play(ROTTEN);
      s.expectHealth("p1", 25);
    });

    it("R18 it is no hit, so no per-hit cap or damage rule sees it (Anti-oneshot Armor answers nothing)", () => {
      const s = scenario({ p1: { hand: [ROTTEN, FILLER], backrow: [{ def: ANTI_ONESHOT, faceUp: true }] }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);
      s.expectHealth("p1", 25);
      expect(s.lastEvents.some((event) => event.type === "damage")).toBe(false);
    });

    it("§4.5 at 0 you lose at the state check after it", () => {
      const s = scenario({ p1: { hand: [ROTTEN, FILLER], health: 5 }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);
      expect(s.state.result).toMatchObject({ winner: "p2" });
    });

    it("R18 below 5 health it takes you below 0 all the same", () => {
      const s = scenario({ p1: { hand: [ROTTEN, FILLER], health: 3 }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);
      expect(s.state.result).toMatchObject({ winner: "p2" });
    });
  });

  describe("radiant", () => {
    it("R275 its upgrade is the smaller loss: your hero loses 1", () => {
      const s = scenario({ p1: { hand: [{ def: ROTTEN, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);
      s.expectHealth("p1", 29);
      expect(s.lastEvents.filter((event) => event.type === "healthLost")).toEqual([{ type: "healthLost", player: "p1", amount: 1 }]);
    });

    it("R18 Armor takes none of the Radiant loss either", () => {
      const s = scenario({
        p1: { hand: [{ def: ROTTEN, radiant: true }, FILLER], backrow: [{ def: GOING_LONG, faceUp: true }] },
        p2: { hand: [FILLER] },
      });
      s.play(ROTTEN);
      s.expectHealth("p1", 29);
    });

    it("§4.5 at 1 health the Radiant face still loses you the game", () => {
      const s = scenario({ p1: { hand: [{ def: ROTTEN, radiant: true }, FILLER], health: 1 }, p2: { hand: [FILLER] } });
      s.play(ROTTEN);
      expect(s.state.result).toMatchObject({ winner: "p2" });
    });
  });
});
