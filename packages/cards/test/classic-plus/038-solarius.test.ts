// C+ #38 Solarius — SPEC §8.7 row 38, BUILD M9 Classic+ row C+ 38: "Spell Damage +2 while on the
// field: each hit of a Spell you play or cast gains 2 (§4.4 step 0), every hit of a multi-hit Spell,
// never a Field Spell's, a Trap's, a Unit's or an activation's hit, never the opponent's Spells; two
// sources add; no Cry on either face (balance patch 1); Death shuffles a Solarius Prime (C+ #38.1)
// into your deck at a random position (R80's cap), shown in your library list; radiant Spell Damage +5,
// the Solarius Prime Radiant".
//
// Spell Damage is a numbered keyword (§6.1), so B3.4's X change tunes it rather than a param (R482).
// A Trap's hit is proved in C+ #22 Blood Moon's test, the one Trap of these sets whose text deals
// damage from itself.

import { HERO_HEALTH, LIBRARY_CAP, stepParam, subsystems, type CardInstance } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/038-solarius";

const SOLARIUS = "classicplus-038";
const PRIME = "classicplus-038-1";
const BONE_STORM = "classicplus-036-1";
const LUNAR_ECLIPSE = "core-035"; // (1) Spell: deal 3 damage to a target.
const ECHOES = "core-040"; // Field Spell: start of turn, damage to the enemy hero = your exile count.
const SORCERER = "core-068"; // Unit, Cry: deal 4 damage to a target.
const HEROIC = "core-098"; // Field Spell, its "burn" power: 2 damage to each opposing hero.
const MENACE = "core-019"; // 9/9 Taunt.
const FILLER = "core-005";

const AT_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

describe("C+ #38 Solarius", () => {
  it("is tagged Catalyst, and both faces name its Prime spaced, Solarius Prime (patch v0.2.Y)", () => {
    expect(def.tags).toEqual(["Catalyst"]);
    expect(def.base.text).toContain("Death: Shuffle a Solarius Prime into your deck.");
    expect(def.radiant.text).toContain("Death: Shuffle a Radiant Solarius Prime into your deck.");
    expect(def.refs).toContain(PRIME);
  });

  describe("base", () => {
    it("prints Spell Damage +2 (a keyword, not a param) and declares nothing", () => {
      const s = scenario({ p1: { field: [SOLARIUS] } });
      expect(s.stats(SOLARIUS).keywords).toContainEqual({ kind: "Spell Damage", n: 2 });
    });

    it("§4.4 step 0: a Spell you play deals 2 more per hit", () => {
      const s = scenario({ p1: { hand: [LUNAR_ECLIPSE, FILLER], field: [SOLARIUS] }, p2: { hand: [FILLER] } });
      s.play(LUNAR_ECLIPSE, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 5);
    });

    it("every hit of a multi-hit Spell gains 2", () => {
      const s = scenario({
        p1: { hand: [BONE_STORM, FILLER], field: [SOLARIUS] },
        p2: { hand: [FILLER], field: [MENACE, MENACE] },
      });
      s.play(BONE_STORM);
      s.expectHealth("p2", HERO_HEALTH - 3);
      for (const lane of [1, 2]) expect(s.stats(s.unit("p2", lane) as CardInstance).health).toBe(6);
    });

    it("two sources add: two Solarius make +4", () => {
      const s = scenario({ p1: { hand: [LUNAR_ECLIPSE, FILLER], field: [SOLARIUS, SOLARIUS] }, p2: { hand: [FILLER] } });
      s.play(LUNAR_ECLIPSE, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 7);
    });

    it("never the opponent's Spells", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [SOLARIUS] },
        p2: { hand: [LUNAR_ECLIPSE, FILLER] },
      });
      s.play(LUNAR_ECLIPSE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", HERO_HEALTH - 3);
    });

    it("never a Unit's hit: a Cry's damage is not raised", () => {
      const s = scenario({ p1: { hand: [SORCERER, FILLER], field: [SOLARIUS] }, p2: { hand: [FILLER] } });
      s.play(SORCERER, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 4);
    });

    it("never a unit's combat hit", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [SOLARIUS] }, p2: { hand: [FILLER] } });
      s.attack(SOLARIUS, "hero");
      s.expectHealth("p2", HERO_HEALTH - 3);
    });

    it("never a Field Spell's hit: Echoes of the Forgotten deals its exile count unraised", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [SOLARIUS], backrow: [ECHOES], exile: [FILLER, FILLER], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], library: [FILLER, FILLER] },
      });
      s.endTurn();
      s.endTurn(); // p1's start of turn: Echoes deals 2.
      s.expectHealth("p2", HERO_HEALTH - 2);
    });

    it("never an activation's hit: Heroic Power's burn deals 2", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [SOLARIUS], backrow: [HEROIC], mana: 4 }, p2: { hand: [FILLER] } });
      const power = s.backrow("p1", 1) as CardInstance;
      power.memory[subsystems.POWER_KEY] = "burn";
      s.activate(power);
      s.expectHealth("p2", HERO_HEALTH - 2);
    });

    it("stops raising once it has left the field", () => {
      const s = scenario({
        p1: { hand: [LUNAR_ECLIPSE, LUNAR_ECLIPSE, FILLER], field: [SOLARIUS] },
        p2: { hand: [FILLER] },
      });
      const [first, second] = s.hand("p1");
      s.play(first as CardInstance, { targets: [{ pick: "instance", instanceId: s.card(SOLARIUS).id }] });
      s.expectInZone(SOLARIUS, "graveyard");
      s.play(second as CardInstance, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 3);
    });

    it("no Cry on either face: playing draws nothing", () => {
      const s = scenario({ p1: { hand: [SOLARIUS, FILLER], library: [LUNAR_ECLIPSE, FILLER] }, p2: { hand: [FILLER] } });
      s.play(SOLARIUS);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
    });

    it("R1 summoned, not played, it draws nothing (a Recruit)", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [HEROIC], library: [SOLARIUS, FILLER], mana: 4 },
        p2: { hand: [FILLER] },
      });
      const power = s.backrow("p1", 1) as CardInstance;
      power.memory[subsystems.POWER_KEY] = "recruit";
      s.activate(power);
      expect(s.unit("p1", 1)?.defId).toBe(SOLARIUS);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
    });

    it("Death shuffles a base Solarius Prime into your deck at a random position, shown in your list", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [SOLARIUS], library: Array.from({ length: 12 }, () => FILLER) },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.attack(SOLARIUS, MENACE);
      s.expectInZone(SOLARIUS, "graveyard");
      const library = s.pile("p1", "library");
      const primes = library.filter((card) => card.defId === PRIME);
      expect(primes).toHaveLength(1);
      expect(primes[0]?.radiant).toBe(false);
      expect(library).toHaveLength(13);
      expect(s.view("p1").you.ownLibrary?.cards).toContainEqual({ defId: PRIME, radiant: false, count: 1 });
      expect(s.events.some((event) => event.type === "shuffledIn" && event.defId === PRIME)).toBe(true);
    });

    it("the Solarius Prime goes in at a position the rng picks, not always the top", () => {
      const positions = new Set<number>();
      for (const seed of ["sol-a", "sol-b", "sol-c", "sol-d", "sol-e", "sol-f"]) {
        const s = scenario({
          seed,
          p1: { hand: [FILLER], field: [SOLARIUS], library: Array.from({ length: 12 }, () => FILLER) },
          p2: { hand: [FILLER], field: [MENACE] },
        });
        s.attack(SOLARIUS, MENACE);
        positions.add(s.pile("p1", "library").findIndex((card) => card.defId === PRIME));
      }
      expect(positions.size).toBeGreaterThan(1);
    });

    it("R80 a full deck turns the Solarius Prime away", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [SOLARIUS], library: Array.from({ length: LIBRARY_CAP }, () => FILLER) },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.attack(SOLARIUS, MENACE);
      expect(s.pile("p1", "library").some((card) => card.defId === PRIME)).toBe(false);
      expect(s.events.some((event) => event.type === "libraryOverflow" && event.defId === PRIME)).toBe(true);
    });

    it("R386 its Spell Damage is a numbered keyword B3.4's X change tunes: one Upgrade step makes it +3", () => {
      const s = scenario({ p1: { hand: [LUNAR_ECLIPSE, FILLER], field: [SOLARIUS] }, p2: { hand: [FILLER] } });
      s.card(SOLARIUS).tuning = { x: { "Spell Damage": 1 } };
      expect(s.stats(SOLARIUS).keywords).toContainEqual({ kind: "Spell Damage", n: 3 });
      s.play(LUNAR_ECLIPSE, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 6);
    });

    it("R386 no draw to tune: a draw tuning still draws nothing", () => {
      const s = scenario({ p1: { hand: [SOLARIUS, FILLER], library: [LUNAR_ECLIPSE, FILLER, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(SOLARIUS), "draw", 1);
      s.play(SOLARIUS);
      expect(s.hand("p1")).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("Spell Damage +5: a 3-damage Spell deals 8", () => {
      const s = scenario({ p1: { hand: [LUNAR_ECLIPSE, FILLER], field: [{ def: SOLARIUS, radiant: true }] }, p2: { hand: [FILLER] } });
      s.play(LUNAR_ECLIPSE, { targets: AT_HERO });
      s.expectHealth("p2", HERO_HEALTH - 8);
    });

    it("no Cry on the Radiant face either: playing draws nothing", () => {
      const s = scenario({
        p1: { hand: [{ def: SOLARIUS, radiant: true }, FILLER], library: [LUNAR_ECLIPSE, FILLER, FILLER] },
        p2: { hand: [FILLER] },
      });
      s.play(SOLARIUS);
      expect(s.hand("p1")).toHaveLength(1);
    });

    it("Death shuffles a Radiant Solarius Prime", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: SOLARIUS, radiant: true }], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] },
      });
      s.attack(SOLARIUS, MENACE);
      s.expectInZone(SOLARIUS, "graveyard");
      const primes = s.pile("p1", "library").filter((card) => card.defId === PRIME);
      expect(primes.map((card) => card.radiant)).toEqual([true]);
      expect(s.view("p1").you.ownLibrary?.cards).toContainEqual({ defId: PRIME, radiant: true, count: 1 });
    });
  });
});
