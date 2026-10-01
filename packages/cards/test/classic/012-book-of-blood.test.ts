// C #12 Book of Blood — SPEC §8.6 row 12, BUILD M9 Classic row C 12: "Targets a Unit only, either
// side (no hero offered); one hit of 5; its own Lifesteal heals your hero the amount dealt (R85), so
// Armor lowers the heal and a Divine Shield leaves it at 0 (R63); radiant 10; its tuned number
// (damage) reads through `param()` (R386)".

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/012-book-of-blood";

const BOOK = "classic-012";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt; Radiant 18/18.
const SHIELDED = "core-003"; // Right-house defender 1/1 Taunt, Divine Shield, Reborn.
const POINTMASTER = "core-020"; // (2) Unit 7/1 First Strike.
const ROCK = "core-066"; // The Rock 10/10 Indestructible.
const FILLER = "core-005";

function at(s: Scenario, player: "p1" | "p2", lane: number): Selection[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

function hits(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "damage" ? [event.amount] : []));
}

function heroHeals(s: Scenario, player: "p1" | "p2"): number[] {
  return s.events.flatMap((event) => (event.type === "healed" && event.targetId === `hero-${player}` ? [event.amount] : []));
}

describe("C #12 Book of Blood", () => {
  it("declares one Unit target on either side, and runs one script on both faces", () => {
    expect(def.id).toBe(BOOK);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("deals 5 to an enemy Unit in one hit, and R85 its Lifesteal heals your hero 5", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [MENACE] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 4 });
      expect(hits(s)).toEqual([5]);
      s.expectHealth("p1", 25);
      expect(heroHeals(s, "p1")).toEqual([5]);
    });

    it("targets your own Unit too, and still heals you", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], field: [MENACE], health: 20 }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: at(s, "p1", 1) });

      s.expectStats(MENACE, { health: 4 });
      s.expectHealth("p1", 25);
    });

    it("offers no hero: a hero pick is refused", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });

      expect(() => s.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] })).toThrow();
      s.expectInZone(BOOK, "hand");
    });

    it("with no Unit on the board it fizzles: nothing is dealt and nobody is healed", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER] } });

      s.play(BOOK);

      expect(hits(s)).toEqual([]);
      s.expectHealth("p1", 20).expectInZone(BOOK, "graveyard");
    });

    it("R85 Armor lowers the heal: a Unit in Defense Position (Armor 1) takes 4 and you heal 4", () => {
      const s = scenario({
        p1: { hand: [BOOK, FILLER], health: 20 },
        p2: { hand: [FILLER], field: [{ def: MENACE, position: "DEF" }] },
      });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 5 });
      s.expectHealth("p1", 24);
    });

    it("R63 a Divine Shield takes the hit whole, and the heal is 0", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [SHIELDED] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectInZone(SHIELDED, "field").expectEvents("divineShieldLost");
      s.expectHealth("p1", 20);
      expect(heroHeals(s, "p1")).toEqual([]);
    });

    it("§4.4 step 5 R85 the heal is the amount dealt, which is not capped at the Unit's health: 5 on a 7/1 heals 5", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [POINTMASTER] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectInZone(POINTMASTER, "graveyard");
      expect(hits(s)).toEqual([5]);
      expect(heroHeals(s, "p1")).toEqual([5]);
      s.expectHealth("p1", 25);
    });

    it("§4.4 step 4 R85 an Indestructible Unit takes no damage, so the heal is 0", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [ROCK] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(ROCK, { health: 10 }).expectInZone(ROCK, "field");
      expect(hits(s)).toEqual([]);
      expect(heroHeals(s, "p1")).toEqual([]);
      s.expectHealth("p1", 20);
    });

    it("R386 an Upgrade makes it deal and heal 6; a Degrade 4", () => {
      const up = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [MENACE] } });
      stepParam(up.card(BOOK), "damage", 1);
      up.play(BOOK, { targets: at(up, "p2", 1) });
      up.expectStats(MENACE, { health: 3 }).expectHealth("p1", 26);

      const down = scenario({ p1: { hand: [BOOK, FILLER], health: 20 }, p2: { hand: [FILLER], field: [MENACE] } });
      stepParam(down.card(BOOK), "damage", -1);
      down.play(BOOK, { targets: at(down, "p2", 1) });
      down.expectStats(MENACE, { health: 5 }).expectHealth("p1", 24);
    });
  });

  describe("radiant", () => {
    it("deals 10 in one hit and heals your hero 10", () => {
      const s = scenario({
        p1: { hand: [{ def: BOOK, radiant: true }, FILLER], health: 20 },
        p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] },
      });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 8, maxHealth: 18 });
      expect(hits(s)).toEqual([10]);
      s.expectHealth("p1", 30);
    });

    it("R63 a Divine Shield still leaves the Radiant heal at 0", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER], health: 20 }, p2: { hand: [FILLER], field: [SHIELDED] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectHealth("p1", 20);
    });

    it("R386 an Upgrade steps the Radiant 10 to 11", () => {
      const s = scenario({
        p1: { hand: [{ def: BOOK, radiant: true }, FILLER], health: 10 },
        p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] },
      });
      stepParam(s.card(BOOK), "damage", 1);

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 7 }).expectHealth("p1", 21);
    });
  });
});
