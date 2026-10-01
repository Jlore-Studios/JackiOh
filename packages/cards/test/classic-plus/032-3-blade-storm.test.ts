// C+ #32.3 Blade Storm — SPEC §8.7 row 32.3, BUILD M9 Classic+ row C+ 32.3: "Rounds of 1 damage to every
// Unit, each round one effect list followed by its own state check (one of the two in-list checks R59
// names, beside R283's), Death triggers resolving before the next round; it stops after a round in which
// any Unit died (a Reborn death counts), when no Unit is left, or after its round cap, 30
// (`BLADE_STORM_ROUNDS`), which reads through `param()` (step 8), so a Degrade or Upgrade moves it on this
// card only; Divine Shields pop in the first round; Spell Damage raises every round's hits; a board of
// only Armor 1 or Indestructible Units runs exactly 30 rounds and stops; radiant hits enemy Units only, a
// death on either side still stopping it".
//
// The engine proves the rounds against fixture cards too (`packages/engine/test/effects-plus-c.test.ts`),
// a Death hook that asks inside a round's check among them.

import { BLADE_STORM_ROUNDS, hashState, paramDeclOf, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import type { CardInstance } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/032-3-blade-storm";

const STORM = "classicplus-032-3";
const MENACE = "core-019"; // 9/9 Taunt
const TIMMY = "core-011"; // 3/3
const JILLIAX = "core-056"; // 3/2, Divine Shield
const DEFENDER = "core-003"; // 1/1, Divine Shield, Reborn
const SEVEN = "core-025"; // 7/7, Armor 7
const UNBREAKABLE = "classic-041"; // 3/3, Indestructible
const SOLARIUS = "classicplus-038"; // 3/2, Spell Damage +2
const HOGAR = "classicplus-028"; // 3/4 Taunt, Reborn; Death: heal your hero 3
const FILLER = "core-005";
/** A 3/50 Tempo Timmy: it outlives any storm, so its hits count the rounds. */
const COUNTER = { def: TIMMY, statsOverride: { attack: 3, health: 50 } };

function storm(p1: SideSetup, p2: SideSetup, options: { radiant?: boolean; tune?: number } = {}): Scenario {
  const s = scenario({
    p1: { ...p1, hand: [{ def: STORM, radiant: options.radiant === true }, FILLER] },
    p2: { hand: [FILLER], ...p2 },
  });
  if (options.tune !== undefined) stepParam(s.card(STORM), "rounds", options.tune);
  s.play(STORM);
  return s;
}

function hits(s: Scenario, card: CardInstance | null): number[] {
  if (card === null) throw new Error("no such unit");
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === card.id ? [event.amount] : []));
}

const deaths = (s: Scenario): string[] => s.events.flatMap((event) => (event.type === "destroyed" ? [event.defId] : []));

describe("C+ #32.3 Blade Storm", () => {
  it("declares its round cap, BLADE_STORM_ROUNDS on both faces, step 8 (R386)", () => {
    expect(def.id).toBe(STORM);
    expect(base).not.toBe(radiant);
    expect(BLADE_STORM_ROUNDS).toBe(30);
    expect(def.params).toEqual([{ key: "rounds", base: BLADE_STORM_ROUNDS, radiant: BLADE_STORM_ROUNDS, better: "up", step: 8, min: 1 }]);
  });

  describe("base", () => {
    it("R59 1 damage to every Unit, round after round, until one dies: the 3/3 dies in round 3 and the storm stops", () => {
      const s = storm({ field: [TIMMY] }, { field: [MENACE] });
      const menace = s.unit("p2", 1);
      expect(hits(s, menace)).toEqual([1, 1, 1]);
      expect(deaths(s)).toEqual([TIMMY]);
      s.expectStats(MENACE, { health: 6 });
      s.expectInZone(STORM, "graveyard");
    });

    it("R59 Divine Shields pop in the first round", () => {
      const s = storm({ field: [JILLIAX] }, { field: [MENACE] });
      const types = s.events.map((event) => event.type);
      expect(types.indexOf("divineShieldLost")).toBeLessThan(types.indexOf("destroyed"));
      expect(hits(s, s.unit("p2", 1))).toEqual([1, 1, 1]);
      expect(deaths(s)).toEqual([JILLIAX]);
    });

    it("R59 a Reborn death counts: the storm stops and the unit comes back", () => {
      const s = storm({ field: [DEFENDER] }, { field: [MENACE] });
      expect(deaths(s)).toEqual([DEFENDER]);
      expect(hits(s, s.unit("p2", 1))).toEqual([1, 1]);
      s.expectInZone(DEFENDER, "field");
      s.expectStats(DEFENDER, { health: 1 });
    });

    it("R59 its Death hooks resolve in the round that killed it, and no round follows", () => {
      const s = storm({ field: [MENACE] }, { field: [HOGAR], health: 20 });
      // Round 4 kills the 3/4: its Death heals 3 and Reborn brings it back at 1, which nothing hits again.
      s.expectHealth("p2", 23);
      s.expectStats(HOGAR, { health: 1 });
      expect(hits(s, s.unit("p1", 1))).toEqual([1, 1, 1, 1]);
    });

    it("R59 every hit of a round lands before its state check: Spell Damage +2 kills Solarius and the 3/3 together", () => {
      const s = storm({ field: [SOLARIUS] }, { field: [TIMMY, MENACE] });
      expect(deaths(s).sort()).toEqual([SOLARIUS, TIMMY].sort());
      expect(hits(s, s.unit("p2", 2))).toEqual([3]);
    });

    it("R59 a board of only Armor or Indestructible Units: no hit lands, nothing dies, and the storm ends", () => {
      const s = storm({ field: [SEVEN] }, { field: [UNBREAKABLE] });
      expect(s.events.filter((event) => event.type === "damage")).toEqual([]);
      expect(deaths(s)).toEqual([]);
      expect(s.state.pending).toBeNull();
      s.expectInZone(STORM, "graveyard");
    });

    it("R59 with no death it runs exactly 30 rounds, BLADE_STORM_ROUNDS, and stops", () => {
      const s = storm({ field: [SEVEN, COUNTER] }, { field: [UNBREAKABLE] });
      expect(hits(s, s.unit("p1", 2))).toHaveLength(BLADE_STORM_ROUNDS);
      expect(deaths(s)).toEqual([]);
      s.expectStats(s.unit("p1", 2) ?? TIMMY, { health: 50 - BLADE_STORM_ROUNDS });
    });

    it("§9.3 the storm replays from a JSON copy to the same hash and events", () => {
      const s = scenario({ p1: { hand: [STORM, FILLER], field: [TIMMY, COUNTER] }, p2: { hand: [FILLER], field: [HOGAR] } });
      const action = { type: "play", instanceId: s.card(STORM).id, playerId: "p1", nonce: "storm-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      const again = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });

    it("§8.7 with no Unit to hit it does nothing", () => {
      const s = storm({}, {});
      expect(s.events.filter((event) => event.type === "damage")).toEqual([]);
      s.expectInZone(STORM, "graveyard");
    });

    it("R386 a Degrade moves the cap by its step of 8: 22 rounds", () => {
      const s = storm({}, { field: [UNBREAKABLE, COUNTER] }, { tune: -1 });
      expect(hits(s, s.unit("p2", 2))).toHaveLength(BLADE_STORM_ROUNDS - 8);
    });

    it("R386 an Upgrade moves it the other way: 38 rounds", () => {
      const s = storm({}, { field: [UNBREAKABLE, COUNTER] }, { tune: 1 });
      expect(hits(s, s.unit("p2", 2))).toHaveLength(BLADE_STORM_ROUNDS + 8);
      expect(paramDeclOf(s.state, STORM, "rounds")?.step).toBe(8);
    });
  });

  describe("radiant", () => {
    it("§8.7 hits enemy Units only: your own are never touched", () => {
      const s = storm({ field: [TIMMY] }, { field: [TIMMY, MENACE] }, { radiant: true });
      expect(hits(s, s.unit("p1", 1))).toEqual([]);
      expect(hits(s, s.unit("p2", 2))).toEqual([1, 1, 1]);
      expect(deaths(s)).toEqual([TIMMY]);
      expect(s.unit("p1", 1)?.defId).toBe(TIMMY);
    });

    it("R59 Spell Damage raises every round's hit on the enemy", () => {
      const s = storm({ field: [SOLARIUS] }, { field: [MENACE] }, { radiant: true });
      expect(deaths(s)).toEqual([MENACE]);
      expect(s.events.filter((event) => event.type === "damage").map((event) => event.amount)).toEqual([3, 3, 3]);
      s.expectInZone(SOLARIUS, "field");
    });

    it("R59 an enemy board nothing kills runs the 30 rounds too, your own Units untouched", () => {
      const s = storm({ field: [TIMMY] }, { field: [UNBREAKABLE, COUNTER] }, { radiant: true });
      expect(hits(s, s.unit("p2", 2))).toHaveLength(BLADE_STORM_ROUNDS);
      expect(hits(s, s.unit("p1", 1))).toEqual([]);
    });
  });
});
