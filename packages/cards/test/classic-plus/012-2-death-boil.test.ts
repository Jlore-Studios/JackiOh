// C+ #12.2 Death Boil — SPEC §8.7 row 12.2, BUILD M9 Classic+ row C+ 12.2: "A target Unit or hero: an
// enemy takes 6 damage (Spell Damage raises it), one of yours is healed 6 (a hero past 30, a unit up to
// its max health); the amount reads through `param()` (step 2); radiant 12 and 12".

import { HERO_HEALTH, stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const BOIL = "classicplus-012-2";
const MENACE = "core-019"; // 9/9
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const FILLER = "core-005";

function boil(radiant: boolean, p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: BOIL, radiant }, FILLER], ...p1 },
    p2: { hand: [FILLER], field: [MENACE], ...p2 },
  });
}

const unitAt = (s: Scenario, player: "p1" | "p2"): Selection[] => [{ pick: "instance", instanceId: s.unit(player, 1)?.id ?? "" }];

describe("C+ #12.2 Death Boil", () => {
  describe("base", () => {
    it("an enemy Unit takes 6 damage", () => {
      const s = boil(false);
      s.play(BOIL, { targets: unitAt(s, "p2") });
      s.expectStats(MENACE, { health: 3 });
    });

    it("the enemy hero takes 6; Spell Damage raises it", () => {
      const s = boil(false, { field: [SOLARIUS] });
      s.play(BOIL, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", HERO_HEALTH - 8);
    });

    it("your own Unit is healed 6, never past its max health", () => {
      const s = boil(false, { field: [{ def: MENACE, damage: 8 }] }, { field: [] });
      s.play(BOIL, { targets: unitAt(s, "p1") });
      s.expectStats(MENACE, { health: 7, maxHealth: 9 });
      const full = boil(false, { field: [{ def: MENACE, damage: 2 }] }, { field: [] });
      full.play(BOIL, { targets: unitAt(full, "p1") });
      full.expectStats(MENACE, { health: 9 });
    });

    it("R19 your hero is healed 6, past 30", () => {
      const s = boil(false);
      s.play(BOIL, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", HERO_HEALTH + 6);
      expect(s.events.some((event) => event.type === "damage")).toBe(false);
    });

    it("R386 the amount reads through param(): an Upgrade steps it by 2", () => {
      const s = boil(false);
      stepParam(s.card(BOIL), "amount", 1);
      s.play(BOIL, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", HERO_HEALTH - 8);
    });
  });

  describe("radiant", () => {
    it("12 damage to an enemy", () => {
      const s = boil(true);
      s.play(BOIL, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", HERO_HEALTH - 12);
    });

    it("12 healing to one of yours", () => {
      const s = boil(true, { health: 10 });
      s.play(BOIL, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 22);
    });
  });
});
