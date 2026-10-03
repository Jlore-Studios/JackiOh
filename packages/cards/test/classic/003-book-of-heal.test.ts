// C #3 Book of Heal — SPEC §8.6 row 3, BUILD M9 Classic row C 3: "Heal a target 9, either side: a
// Unit up to its max health, a hero with no cap (R19); a full-health Unit gains nothing; radiant 18;
// its tuned number (heal, step 2) reads through `param()` (R386)".

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/003-book-of-heal";

const BOOK = "classic-003";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt; Radiant 18/18.
const FILLER = "core-005"; // a card that keeps a hand from auto-ending the turn (§2.5).

function at(s: Scenario, player: "p1" | "p2", lane: number): Selection[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

function healedAmounts(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "healed" ? [event.amount] : []));
}

describe("C #3 Book of Heal", () => {
  it("declares one target, any unit or hero on either side (R19), and runs one script on both faces", () => {
    expect(def.id).toBe(BOOK);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R19 heals your own damaged Unit, never past its max health", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], field: [{ def: MENACE, damage: 5 }] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: at(s, "p1", 1) });

      s.expectStats(MENACE, { health: 9, maxHealth: 9 });
      expect(healedAmounts(s)).toEqual([5]);
    });

    it("R19 heals an enemy Unit 9: either side is legal", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true, damage: 12 }] } });

      s.play(BOOK, { targets: at(s, "p2", 1) });

      s.expectStats(MENACE, { health: 15, maxHealth: 18 });
      expect(healedAmounts(s)).toEqual([9]);
    });

    it("R19 heals your hero 9 with no cap: a hero at 30 reaches 39", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p1", 39);
    });

    it("R19 heals the enemy hero too", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], health: 12 } });

      s.play(BOOK, { targets: [{ pick: "hero", player: "p2" }] });

      s.expectHealth("p2", 21);
    });

    it("a full-health Unit gains nothing: no heal happens", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], field: [MENACE] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: at(s, "p1", 1) });

      s.expectStats(MENACE, { health: 9, maxHealth: 9 });
      expect(healedAmounts(s)).toEqual([]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R386 an Upgrade moves heal by its declared step of 2: it heals 11", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 10 }, p2: { hand: [FILLER] } });
      stepParam(s.card(BOOK), "heal", 1);

      s.play(BOOK, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p1", 21);
    });

    it("R386 a Degrade moves it the other way by 2: it heals 7", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER], health: 10 }, p2: { hand: [FILLER] } });
      stepParam(s.card(BOOK), "heal", -1);

      s.play(BOOK, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p1", 17);
    });
  });

  describe("radiant", () => {
    it("heals a target 18: a hero at 30 reaches 48", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });

      s.play(BOOK, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p1", 48);
    });

    it("R19 heals a Unit 18, still never past its max health", () => {
      const s = scenario({
        p1: { hand: [{ def: BOOK, radiant: true }, FILLER], field: [{ def: MENACE, radiant: true, damage: 17 }, { def: MENACE, damage: 8 }] },
        p2: { hand: [FILLER] },
      });

      s.play(BOOK, { targets: at(s, "p1", 1) });
      s.expectStats(s.unit("p1", 1) ?? MENACE, { health: 18, maxHealth: 18 });
      expect(healedAmounts(s)).toEqual([17]);
    });

    it("R386 an Upgrade on the Radiant face steps from 18: it heals 20", () => {
      const s = scenario({ p1: { hand: [{ def: BOOK, radiant: true }, FILLER], health: 10 }, p2: { hand: [FILLER] } });
      stepParam(s.card(BOOK), "heal", 1);

      s.play(BOOK, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p1", 30);
    });
  });
});
