// C+ #59 All Purpose Apple — SPEC §8.7 row 59, BUILD M9 Classic+ row C+ 59: "Summons a Rush Token (3/3
// Rush) into your leftmost open zone (none on a full board, the rest still resolving), heals your hero
// 2 (past 30 allowed) and deals 1 damage to a target chosen with the play, any Unit or hero, which
// Spell Damage raises; heal and damage read through `param()`; radiant a Radiant Rush Token, heal 4,
// 2 damage".

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def } from "../../src/scripts/classic-plus/059-all-purpose-apple";

const APPLE = "classicplus-059";
const RUSH = "core-t-rush";
const VANILLA = "core-008"; // Mr. Vanilla 4/4
const SOLARIUS = "classicplus-038"; // a Unit printing Spell Damage +2
const FILLER = "core-005";

const ENEMY_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

function apple(opts: { radiant?: boolean; field?: readonly string[]; health?: number } = {}): Scenario {
  return scenario({
    p1: {
      hand: [{ def: APPLE, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER],
      field: opts.field ?? [],
      ...(opts.health === undefined ? {} : { health: opts.health }),
    },
    p2: { hand: [FILLER], field: [VANILLA] },
  });
}

describe("C+ #59 All Purpose Apple", () => {
  it("is a (1) Spell, Fruit that declares one target, any Unit or hero", () => {
    expect(def.id).toBe(APPLE);
    expect(def.tags).toEqual(["Fruit"]);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
  });

  describe("base", () => {
    it("summons a Rush Token, heals your hero 2, then deals 1 damage, in that order", () => {
      const s = apple({ health: 20 });
      s.play(APPLE, { targets: ENEMY_HERO });
      const token = s.unit("p1", 1);
      expect(token?.defId).toBe(RUSH);
      expect(token?.radiant).toBe(false);
      s.expectStats(token ?? "", { attack: 3, health: 3 });
      s.expectHealth("p1", 22).expectHealth("p2", 29);
      s.expectEvents("summoned", "healed", "damage");
    });

    it("R19 the heal goes past 30", () => {
      const s = apple();
      s.play(APPLE, { targets: ENEMY_HERO });
      s.expectHealth("p1", 32);
    });

    it("R81 the damage may hit any Unit, yours included", () => {
      const s = apple({ field: [VANILLA] });
      s.play(APPLE, { targets: [{ pick: "instance", instanceId: s.unit("p1", 1)?.id ?? "" }] });
      s.expectStats(s.unit("p1", 1) ?? "", { health: 3 });
      expect(s.unit("p1", 2)?.defId).toBe(RUSH);
    });

    it("R64 the token takes your leftmost open zone", () => {
      const s = apple({ field: [VANILLA] });
      s.play(APPLE, { targets: ENEMY_HERO });
      expect(s.unit("p1", 2)?.defId).toBe(RUSH);
    });

    it("§3.2 a full board summons nothing and the rest still resolves", () => {
      const s = apple({ field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], health: 20 });
      s.play(APPLE, { targets: ENEMY_HERO });
      expect(s.lastEvents.some((event) => event.type === "summoned")).toBe(false);
      s.expectHealth("p1", 22).expectHealth("p2", 29);
    });

    it("E6 Spell Damage raises the hit (Spell Damage +2: 3)", () => {
      const s = apple({ field: [SOLARIUS] });
      s.play(APPLE, { targets: ENEMY_HERO });
      s.expectHealth("p2", 27);
    });

    it("R386 an Upgrade heals 3 and deals 2; a Degrade never takes either below 1", () => {
      const up = apple({ health: 20 });
      stepParam(up.card(APPLE), "heal", 1);
      stepParam(up.card(APPLE), "damage", 1);
      up.play(APPLE, { targets: ENEMY_HERO });
      up.expectHealth("p1", 23).expectHealth("p2", 28);

      const down = apple({ health: 20 });
      stepParam(down.card(APPLE), "heal", -5);
      stepParam(down.card(APPLE), "damage", -5);
      down.play(APPLE, { targets: ENEMY_HERO });
      down.expectHealth("p1", 21).expectHealth("p2", 29);
    });
  });

  describe("radiant", () => {
    it("§7 a Radiant Rush Token (6/6 Rush, Cleave), heal 4 and 2 damage", () => {
      const s = apple({ radiant: true, health: 20 });
      s.play(APPLE, { targets: [{ pick: "instance", instanceId: s.unit("p2", 1)?.id ?? "" }] });
      const token = s.unit("p1", 1);
      expect(token?.defId).toBe(RUSH);
      expect(token?.radiant).toBe(true);
      s.expectStats(token ?? "", { attack: 6, health: 6 });
      s.expectHealth("p1", 24);
      s.expectStats(s.unit("p2", 1) ?? "", { health: 2 });
    });

    it("R386 the Radiant numbers step from 4 and 2", () => {
      const s = apple({ radiant: true, health: 20 });
      stepParam(s.card(APPLE), "heal", 1);
      stepParam(s.card(APPLE), "damage", 1);
      s.play(APPLE, { targets: ENEMY_HERO });
      s.expectHealth("p1", 25).expectHealth("p2", 27);
    });
  });
});
