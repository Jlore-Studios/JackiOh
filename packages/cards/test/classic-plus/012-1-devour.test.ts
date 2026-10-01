// C+ #12.1 Devour — SPEC §8.7 row 12.1, BUILD M9 Classic+ row C+ 12.1: "Destroys a target Unit and your
// hero takes damage equal to that Unit's current health read before the destroy, one instance from
// Devour through your Armor and caps (Anime Armor clamps it to 1), raised by your own Spell Damage
// (§4.4 step 0); an Indestructible target survives and the damage still happens; radiant heals your
// hero by that health instead, a heal that an opposing Blood Moon in force turns into Pierce damage
// (R413)".

import { HERO_HEALTH } from "@jackioh/engine";
import type { GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const DEVOUR = "classicplus-012-1";
const MENACE = "core-019"; // 9/9
const ROCK = "core-066"; // 10/10 Indestructible
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const ANIME_ARMOR = "classicplus-011"; // Aura: your hero takes at most 1 damage at a time
const BLOOD_MOON = "classicplus-022"; // Trap: enemy heals become Pierce damage
const FILLER = "core-005";

function at(s: Scenario, player: "p1" | "p2", lane = 1): Selection[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

function devour(radiant: boolean, p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: DEVOUR, radiant }, FILLER], ...p1 },
    p2: { hand: [FILLER], field: [MENACE], ...p2 },
  });
}

function hitsOn(s: Scenario, targetId: string): Extract<GameEvent, { type: "damage" }>[] {
  return s.events.filter((e): e is Extract<GameEvent, { type: "damage" }> => e.type === "damage" && e.targetId === targetId);
}

describe("C+ #12.1 Devour", () => {
  describe("base", () => {
    it("destroys a target Unit and your hero takes damage equal to its health, one instance from Devour", () => {
      const s = devour(false);
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectInZone(MENACE, "graveyard").expectHealth("p1", HERO_HEALTH - 9);
      expect(hitsOn(s, "hero-p1")).toHaveLength(1);
      expect(hitsOn(s, "hero-p1")[0]?.sourceId).toBe(s.card(DEVOUR).id);
    });

    it("reads its current health, damage included, before the destroy; either side is legal", () => {
      const s = devour(false, { field: [{ def: MENACE, damage: 5 }] }, { field: [] });
      s.play(DEVOUR, { targets: at(s, "p1") });
      s.expectHealth("p1", HERO_HEALTH - 4);
    });

    it("§4.4 your hero's Armor takes its share", () => {
      const s = devour(false, { armor: 5 });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectHealth("p1", HERO_HEALTH - 4);
    });

    it("§4.4 step 0 your own Spell Damage raises it", () => {
      const s = devour(false, { field: [SOLARIUS] });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectHealth("p1", HERO_HEALTH - 11);
    });

    it("Anime Armor clamps the hit to 1", () => {
      const s = devour(false, { field: [ANIME_ARMOR] });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectHealth("p1", HERO_HEALTH - 1);
    });

    it("R46 an Indestructible target survives and the damage still happens", () => {
      const s = devour(false, {}, { field: [ROCK] });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectInZone(ROCK, "field").expectHealth("p1", HERO_HEALTH - 10);
    });

    it("targets a Unit only, never a hero", () => {
      const s = devour(false);
      expect(() => s.play(DEVOUR, { targets: [{ pick: "hero", player: "p2" }] })).toThrow();
    });
  });

  describe("radiant", () => {
    it("destroys the Unit and heals your hero by its health instead", () => {
      const s = devour(true, { health: 10 });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectInZone(MENACE, "graveyard").expectHealth("p1", 19);
      expect(hitsOn(s, "hero-p1")).toHaveLength(0);
    });

    it("R46 an Indestructible target survives and the heal still happens", () => {
      const s = devour(true, { health: 10 }, { field: [ROCK] });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectInZone(ROCK, "field").expectHealth("p1", 20);
    });

    it("R413 an opposing Blood Moon in force turns the heal into Pierce damage", () => {
      const s = devour(true, { health: 20 }, { backrow: [{ def: BLOOD_MOON, faceUp: false }] });
      s.play(DEVOUR, { targets: at(s, "p2") });
      s.expectInZone(MENACE, "graveyard").expectHealth("p1", 11);
    });
  });
});
