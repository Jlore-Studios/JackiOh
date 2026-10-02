// C #16 Book of Flame — SPEC §8.6 row 16, BUILD M9 Classic row C 16: "One targeted hit of 4 on any
// Unit or hero, either side; it is the Book of Flame C #23 and C #29 make (`classic-016`, R381);
// radiant 8; its tuned number (damage) reads through `param()` (R386)".

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/016-book-of-flame";

const BOOK = "classic-016";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const ARMORED = "core-025"; // (4) Unit 7/7, Armor 7.
const FILLER = "core-005";

function at(s: Scenario, player: "p1" | "p2", lane: number): Selection[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

function hits(s: Scenario): { targetId: string; amount: number }[] {
  return s.events.flatMap((event) => (event.type === "damage" ? [{ targetId: event.targetId, amount: event.amount }] : []));
}

describe("C #16 Book of Flame", () => {
  it("declares one target, any unit or hero on either side, and runs one script on both faces", () => {
    expect(def.id).toBe(BOOK);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
  });

  it("R381 it is the one card named Book of Flame, which C #23 and C #29 name; C #55 is Book of Wildfire", () => {
    const cards = Object.values(CATALOG);
    expect(cards.filter((card) => card.name === "Book of Flame").map((card) => card.id)).toEqual([BOOK]);
    expect(CATALOG["classic-055"]?.name).toBe("Book of Wildfire");
  });

  it("R279 R381 C #23 Devil's Pact and C #29 Book of Vital Kill name this card, and no entry names Book of Wildfire for it", () => {
    expect(CATALOG["classic-023"]?.refs ?? []).toContain(BOOK);
    expect(CATALOG["classic-029"]?.refs ?? []).toContain(BOOK);
    expect(CATALOG["classic-023"]?.refs ?? []).not.toContain("classic-055");
    expect(CATALOG["classic-029"]?.refs ?? []).not.toContain("classic-055");
  });

  describe("base", () => {
    it("deals 4 to an enemy Unit in one hit", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 5, maxHealth: 9 });
      expect(hits(s)).toHaveLength(1);
    });

    it("deals 4 to your own Unit: either side is legal", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], field: [MENACE] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: at(s, "p1", 1) });

      s.expectStats(MENACE, { health: 5 });
    });

    it("deals 4 to the enemy hero, and to your own if you pick it", () => {
      const s = scenario({ p1: { hand: [BOOK, BOOK, FILLER], mana: 3 }, p2: { hand: [FILLER] } });
      const [first, second] = s.hand("p1");
      if (first === undefined || second === undefined) throw new Error("two Books in hand");

      s.play(first, { targets: [{ pick: "hero", player: "p2" }] });
      s.play(second, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p2", 26).expectHealth("p1", 26);
    });

    it("§4.4 the hit goes through the pipeline: Armor 7 takes all 4", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [ARMORED] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(ARMORED, { health: 7 });
    });

    it("R386 an Upgrade makes it deal 5, a Degrade 3", () => {
      const up = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(up.card(BOOK), "damage", 1);
      up.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] });
      up.expectHealth("p2", 25);

      const down = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(down.card(BOOK), "damage", -1);
      down.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] });
      down.expectHealth("p2", 27);
    });
  });

  describe("radiant", () => {
    it("deals 8 in one hit", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 1, maxHealth: 9 });
      expect(hits(s).map((hit) => hit.amount)).toEqual([8]);
    });

    it("deals 8 to a hero", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] });

      s.expectHealth("p2", 22);
    });

    it("R386 an Upgrade steps the Radiant 8 to 9", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(BOOK), "damage", 1);

      s.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] });

      s.expectHealth("p2", 21);
    });
  });
});
