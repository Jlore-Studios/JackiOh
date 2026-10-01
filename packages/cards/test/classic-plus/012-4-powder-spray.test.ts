// C+ #12.4 Powder Spray — SPEC §8.7 row 12.4, BUILD M9 Classic+ row C+ 12.4: "3 damage to the enemy
// hero and to each enemy Unit, separate instances all landing before the state check (R59); Spell
// Damage raises each hit; an Immune to Spells Unit takes none; your side is untouched; the damage reads
// through `param()`; radiant 6".

import { HERO_HEALTH, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const SPRAY = "classicplus-012-4";
const MENACE = "core-019"; // 9/9
const POINTMASTER = "core-020"; // 7/1
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const FILLER = "core-005";

function spray(radiant: boolean, p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  const s = scenario({
    p1: { hand: [{ def: SPRAY, radiant }, FILLER], field: [MENACE], ...p1 },
    p2: { hand: [FILLER], field: [MENACE, POINTMASTER], ...p2 },
  });
  s.play(SPRAY);
  return s;
}

describe("C+ #12.4 Powder Spray", () => {
  describe("base", () => {
    it("3 damage to the enemy hero and each enemy Unit; your side untouched", () => {
      const s = spray(false);
      s.expectHealth("p2", HERO_HEALTH - 3).expectHealth("p1", HERO_HEALTH);
      expect(s.stats(s.unit("p2", 1) ?? "").health).toBe(6);
      expect(s.stats(s.unit("p1", 1) ?? "").health).toBe(9);
    });

    it("R59 separate instances, all landing before the state check", () => {
      const s = spray(false);
      const hits = s.events.flatMap((e, i) => (e.type === "damage" ? [i] : []));
      const deaths = s.events.flatMap((e, i) => (e.type === "destroyed" ? [i] : []));
      expect(hits).toHaveLength(3);
      expect(deaths).toHaveLength(1); // the Pointmaster
      expect(Math.max(...hits)).toBeLessThan(Math.min(...deaths));
    });

    it("Spell Damage raises each hit", () => {
      const s = spray(false, { field: [SOLARIUS] });
      s.expectHealth("p2", HERO_HEALTH - 5);
      expect(s.stats(s.unit("p2", 1) ?? "").health).toBe(4);
    });

    it("an Immune to Spells Unit takes none", () => {
      const s = spray(false, {}, { field: [{ def: TOP_LOSER, radiant: true }] });
      expect(s.stats(TOP_LOSER).health).toBe(10);
      s.expectHealth("p2", HERO_HEALTH - 3);
    });

    it("R386 the damage reads through param(): an Upgrade deals 4", () => {
      const s = scenario({ p1: { hand: [SPRAY, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(SPRAY), "damage", 1);
      s.play(SPRAY);
      s.expectHealth("p2", HERO_HEALTH - 4);
    });
  });

  describe("radiant", () => {
    it("6 damage to each enemy", () => {
      const s = spray(true);
      s.expectHealth("p2", HERO_HEALTH - 6);
      expect(s.stats(s.unit("p2", 1) ?? "").health).toBe(3);
    });
  });
});
