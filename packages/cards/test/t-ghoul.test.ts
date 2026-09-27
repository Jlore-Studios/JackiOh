// T-ghoul Ghoul Token (SPEC §7; R11, R346, R349, R353; §4.4, §5.1, §10.4). The token patch v0.1.1
// (issue #27) added: "X/X Unit with Pierce", summoned by #74 Adaptive UI with its X.
//
// Base: an X/X (printed 0/0, the X a `statsOverride` like the Bread Token's) whose every hit ignores
// Armor (R346). Radiant: the designer printed no Radiant form, so R349's fallback is its Radiant
// face — the base face with its attack and health doubled, the X/X included. The two `describe`s
// below prove each face; the data block proves the catalog entry says so.

import { describe, expect, it } from "vitest";
import {
  applyEffects,
  createRng,
  faceOf,
  isUnitToken,
  makeContext,
  query,
  unitView,
  type CardInstance,
  type EngineSink,
} from "@jackioh/engine";
import type { Effect } from "@jackioh/engine";
import { fuseCards, setRadiant, summon } from "@jackioh/engine/effects";
import { base, def, radiant } from "../src/scripts/t-ghoul";
import { scenario, type Scenario } from "./_harness";

const SEED = "t-ghoul";
const PIERCE = { kind: "Pierce" } as const;
/** #25 4-mana 7/7: Armor 7, the wall a plain hit cannot get through. */
const WALL = "core-025";
/** #84 Going Long: hero Armor 2 on its base price. */
const GOING_LONG = "core-084";

/** Apply engine effects as p1, the way the card that summons the token would (#74 owns the call). */
function run(s: Scenario, effects: Effect[]): void {
  const state = s.state;
  const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
  applyEffects(effects, makeContext(sink, null, { controller: "p1" }));
  state.rngCursor = sink.rng.cursor;
}

function summonGhoul(s: Scenario, x: number, opts: { radiant?: boolean } = {}): CardInstance {
  run(s, [summon({ defId: def.id, player: "self", lane: 1, statsOverride: { attack: x, health: x }, ...opts })]);
  const token = s.unit("p1", 1);
  expect(token?.defId, "the summon should have put a Ghoul Token in p1's lane 1").toBe(def.id);
  return token as CardInstance;
}

describe("T-ghoul Ghoul Token (SPEC §7, R353)", () => {
  describe("card data (§7)", () => {
    it("R353 prints a 0-cost Unit Token, printed 0/0 for its X/X, with Pierce and no other text", () => {
      expect(def.id).toBe("core-t-ghoul");
      expect(def.index).toBe("T-ghoul");
      expect(def.name).toBe("Ghoul Token");
      expect(def.cost).toBe(0);
      expect(def.type).toBe("Unit");
      expect(def.token).toBe(true);
      expect(def.tags).toEqual(["Token"]);
      expect(def.base).toEqual({ attack: 0, health: 0, keywords: [PIERCE], text: "Pierce." });
    });

    it("R349 prints no Radiant form, so its Radiant face is the fallback: the base face doubled", () => {
      expect(def.radiantFallback).toBe(true);
      expect(def.radiant).toEqual({ ...def.base, attack: 2 * (def.base.attack ?? 0), health: 2 * (def.base.health ?? 0) });
    });

    it("§7 needs no script for either face, and the radiant Script is the base Script", () => {
      expect(base).toEqual({});
      expect(radiant).toBe(base);
    });

    it("§5.1 is in no random pool: tokens are out of every query that does not name them", () => {
      expect(query({}).map((card) => card.id)).not.toContain(def.id);
      expect(query({ type: "Unit", costRange: { max: 0 } }).map((card) => card.id)).not.toContain(def.id);
    });
  });

  describe("base", () => {
    it("R353 is the X/X its summon names, with Pierce through §10.4 layer 1", () => {
      const s = scenario({ seed: SEED });
      const ghoul = summonGhoul(s, 3);
      s.expectStats(ghoul, { attack: 3, health: 3, maxHealth: 3 });
      expect(faceOf(s.state, ghoul)).toEqual({ attack: 3, health: 3, keywords: [PIERCE] });
      expect(unitView(s.state, ghoul).keywords).toEqual([PIERCE]);
    });

    it("R346 its hit on a unit ignores that unit's Armor: 3 through Armor 7", () => {
      const s = scenario({ seed: SEED, p1: { field: [{ def: def.id, statsOverride: { attack: 3, health: 3 } }] }, p2: { field: [WALL] } });
      const wall = s.unit("p2", 1) as CardInstance;
      s.attack(s.unit("p1", 1) as CardInstance, wall);
      s.expectStats(wall, { health: 4, maxHealth: 7 });
      const hit = s.lastEvents.find((event) => event.type === "damage" && event.targetId === wall.id);
      expect(hit).toMatchObject({ amount: 3, combat: true });
    });

    it("R346 its hit on a hero ignores that hero's Armor (#84 Going Long)", () => {
      const s = scenario({
        seed: SEED,
        p1: { field: [{ def: def.id, statsOverride: { attack: 3, health: 3 } }] },
        p2: { backrow: [GOING_LONG] },
      });
      expect(s.view("p2").you.hero.armor).toBe(2);
      s.attack(s.unit("p1", 1) as CardInstance, "hero");
      s.expectHealth("p2", 27);
    });

    it("R346 the unit it attacks still pays its own Armor on the strike back", () => {
      // The Ghoul pierces; the wall does not, so its 7 comes back in full against a Ghoul with no
      // Armor, and a Ghoul in Defense Position would take 7 − 1.
      const s = scenario({ seed: SEED, p1: { field: [{ def: def.id, statsOverride: { attack: 9, health: 9 } }] }, p2: { field: [WALL] } });
      const ghoul = s.unit("p1", 1) as CardInstance;
      s.attack(ghoul, s.unit("p2", 1) as CardInstance);
      s.expectStats(ghoul, { health: 2 });
      s.expectInZone(s.card(WALL), "graveyard");
    });

    it("R11 a Ghoul Token that dies ceases to exist and never reaches a graveyard", () => {
      const s = scenario({ seed: SEED, p1: { field: [{ def: def.id, statsOverride: { attack: 1, health: 1 } }] }, p2: { field: [WALL] } });
      const ghoul = s.unit("p1", 1) as CardInstance;
      expect(isUnitToken(s.state, ghoul)).toBe(true);
      s.attack(ghoul, s.unit("p2", 1) as CardInstance);
      s.expectInZone(ghoul, "gone");
      expect(s.pile("p1", "graveyard")).toEqual([]);
    });
  });

  describe("radiant (R349's fallback)", () => {
    it("R349 a Ghoul summoned Radiant as 3/3 is a 6/6 with Pierce: the X doubles", () => {
      const s = scenario({ seed: SEED });
      const ghoul = summonGhoul(s, 3, { radiant: true });
      expect(ghoul.radiant).toBe(true);
      s.expectStats(ghoul, { attack: 6, health: 6, maxHealth: 6 });
      expect(unitView(s.state, ghoul).keywords).toEqual([PIERCE]);
    });

    it("R349 a Ghoul made Radiant on the field doubles its base-stat layer and keeps its damage (§5.2)", () => {
      const s = scenario({
        seed: SEED,
        p1: { field: [{ def: def.id, statsOverride: { attack: 4, health: 4 }, damage: 1 }] },
      });
      const ghoul = s.unit("p1", 1) as CardInstance;
      s.expectStats(ghoul, { attack: 4, health: 3, maxHealth: 4 });
      run(s, [setRadiant({ instanceId: ghoul.id })]);
      s.expectStats(ghoul, { attack: 8, health: 7, maxHealth: 8 });
      expect(s.card(ghoul).statsOverride, "the X on the instance is the base face's; the Radiant face reads it doubled").toEqual({
        attack: 4,
        health: 4,
      });
    });

    it("R349 a Radiant Ghoul still pierces", () => {
      const s = scenario({
        seed: SEED,
        p1: { field: [{ def: def.id, radiant: true, statsOverride: { attack: 2, health: 2 } }] },
        p2: { field: [WALL] },
      });
      const wall = s.unit("p2", 1) as CardInstance;
      s.attack(s.unit("p1", 1) as CardInstance, wall);
      s.expectStats(wall, { health: 3 });
    });

    it("R349 R77 a fusion onto a Ghoul sums its X on the base face and its doubled X on the Radiant face", () => {
      const s = scenario({
        seed: SEED,
        p1: { field: [{ def: def.id, statsOverride: { attack: 3, health: 3 } }], hand: ["core-011"] },
      });
      const ghoul = s.unit("p1", 1) as CardInstance;
      const timmy = s.hand("p1")[0] as CardInstance;
      run(s, [fuseCards({ instanceIds: [timmy.id], targetInstanceId: ghoul.id })]);
      const fused = s.unit("p1", 1) as CardInstance;
      const fusedDef = s.state.transientDefs[fused.defId];
      // #11 Tempo Timmy is 3/3 → 6/6; the Ghoul adds 3/3 on the base face and 6/6 on the Radiant one.
      expect(fusedDef?.base).toMatchObject({ attack: 6, health: 6 });
      expect(fusedDef?.radiant).toMatchObject({ attack: 12, health: 12 });
      expect(fusedDef?.base.keywords).toContainEqual(PIERCE);
    });
  });
});
