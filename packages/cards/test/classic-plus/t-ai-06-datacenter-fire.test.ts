// T-AI-6 Datacenter Fire — SPEC §8.7 row T-AI-6, BUILD M9 Classic+ row T-AI-6: "Destroys every Field
// Spell on both sides, yours included (a Claude's Datacenter too); Field Traps and Traps are no Field
// Spells; an Indestructible one (Heroic Power) stays and doesn't count; then each hero takes one hit of
// 1 per Field Spell destroyed, one instance of N as 'for each' in one sentence always is (so Spell Damage
// adds once and Anime Armor caps it at 1); none destroyed, no damage; a destroyed Field Spell that
// prints Death fires it (§4.5); its preview is the damage each hero would take now (R280); radiant only
// enemy Field Spells, and 2 damage to the enemy hero for each".
//
// The preview's own proofs (labels, values against the resolution, what the hook reads) are in
// `../preview.test.ts`. Each hero's hit is one `damage` event of N, so a per-hit cap (C+ #11 Anime Armor)
// caps it once, and an Ivory Tower holding a Unit (R418) is swept: both are proved through fixtures in
// `packages/engine/test/effects-datacenter.test.ts`, since neither card's script (C+ #11, #33) is part of
// this slice.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-06-datacenter-fire";

const FIRE = "classicplus-t-ai-06";
const DATACENTER = "classicplus-078"; // (2) Field Spell
const TWINSPELL = "core-079"; // (2) Field Spell
const FARM = "core-058"; // Rush Token Farm, Field Spell
const HEROIC_POWER = "core-098"; // Field Spell, Indestructible
const BEAR = "core-060"; // (1) Trap
const BREAD = "core-018"; // Field Trap
const SOLARIUS = "classicplus-038"; // Unit printing Spell Damage +2
const BAUBLE = "classicplus-061"; // (1) Field Spell: "Death: Add 2 Stockpiles to your hand. Each costs (0)."
const STOCKPILE = "core-005";
const VANILLA = "core-008";

function fire(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: FIRE, radiant: radiantFace }, VANILLA], library: [VANILLA], ...p1 },
    p2: { hand: [VANILLA], library: [VANILLA], ...p2 },
  }).play(FIRE);
}

function hitsOn(s: Scenario, player: PlayerId): number[] {
  return s.lastEvents.flatMap((event) => (event.type === "damage" && event.targetId === `hero-${player}` ? [event.amount] : []));
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

const up = (defId: string, lane: number): { def: string; faceUp: boolean; lane: number } => ({ def: defId, faceUp: true, lane });
const down = (defId: string, lane: number): { def: string; faceUp: boolean; lane: number } => ({ def: defId, faceUp: false, lane });

describe("T-AI-6 Datacenter Fire", () => {
  it("is a (2) AI Spell token with a preview on both faces", () => {
    expect(def.cost).toBe(2);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["AI", "Token"]);
    expect(base.preview).toBeTypeOf("function");
    expect(radiant.preview).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R59 destroys every Field Spell on both sides, your own Claude's Datacenter too", () => {
      const s = fire({ backrow: [up(DATACENTER, 1)] }, { backrow: [up(TWINSPELL, 1), up(FARM, 3)] });
      s.expectInZone(DATACENTER, "graveyard").expectInZone(TWINSPELL, "graveyard").expectInZone(FARM, "graveyard");
      expect(count(s.lastEvents, "destroyed")).toBe(3);
    });

    it("§4.4 each hero takes one hit of 1 per Field Spell destroyed", () => {
      const s = fire({ backrow: [up(DATACENTER, 1)] }, { backrow: [up(TWINSPELL, 1), up(FARM, 3)] });
      expect(hitsOn(s, "p1")).toEqual([3]);
      expect(hitsOn(s, "p2")).toEqual([3]);
      s.expectHealth("p1", 27).expectHealth("p2", 27);
    });

    it("Traps and Field Traps are no Field Spells: they stay and don't count", () => {
      const s = fire({ backrow: [down(BEAR, 2)] }, { backrow: [down(BREAD, 1), up(TWINSPELL, 2)] });
      s.expectInZone(BEAR, "field").expectInZone(BREAD, "field").expectInZone(TWINSPELL, "graveyard");
      expect(hitsOn(s, "p1")).toEqual([1]);
    });

    it("R46 an Indestructible Heroic Power stays and doesn't count", () => {
      const s = fire({ backrow: [up(HEROIC_POWER, 1)] }, { backrow: [up(TWINSPELL, 1)] });
      s.expectInZone(HEROIC_POWER, "field");
      expect(hitsOn(s, "p2")).toEqual([1]);
    });

    it("§4.5 a destroyed Field Spell that prints Death fires it: Bauble Bubble's Stockpiles reach its owner", () => {
      const s = fire({}, { backrow: [up(BAUBLE, 1)] });
      s.expectInZone(BAUBLE, "graveyard");
      const stockpiles = s.hand("p2").filter((card) => card.defId === STOCKPILE);
      expect(stockpiles).toHaveLength(2);
      expect(stockpiles.every((card) => card.costOverride === 0)).toBe(true);
      expect(hitsOn(s, "p2")).toEqual([1]);
    });

    it("R129 none destroyed, no damage", () => {
      const s = fire({ backrow: [up(HEROIC_POWER, 1), down(BEAR, 2)] });
      expect(count(s.lastEvents, "damage")).toBe(0);
      expect(count(s.lastEvents, "destroyed")).toBe(0);
      s.expectHealth("p1", 30).expectHealth("p2", 30);
    });

    it("R588 R383 an Animated Field Spell standing in a unit zone is a Unit there: neither destroyed nor counted", () => {
      // C+ #12.8 Frostspatula, "Animated on your turn": p1's turn starts and it steps into a unit zone.
      const s = scenario({
        active: "p2",
        p1: { hand: [FIRE, VANILLA], backrow: [up("classicplus-012-8", 1)], library: [VANILLA] },
        p2: { hand: [VANILLA], backrow: [up(TWINSPELL, 1)], library: [VANILLA] },
      });
      s.endTurn();
      const spatula = s.unit("p1", 1);
      expect(spatula?.defId).toBe("classicplus-012-8");
      s.play(FIRE);
      expect(s.unit("p1", 1)?.id).toBe(spatula?.id);
      s.expectInZone(TWINSPELL, "graveyard");
      expect(hitsOn(s, "p1")).toEqual([1]);
    });

    it("§4.4 step 0 Spell Damage raises each hero's one hit once", () => {
      const s = fire({ field: [SOLARIUS], backrow: [up(DATACENTER, 1)] }, { backrow: [up(TWINSPELL, 1)] });
      expect(hitsOn(s, "p1")).toEqual([4]);
      expect(hitsOn(s, "p2")).toEqual([4]);
    });
  });

  describe("radiant", () => {
    it("destroys only the enemy's Field Spells; yours stay", () => {
      const s = fire({ backrow: [up(DATACENTER, 1)] }, { backrow: [up(TWINSPELL, 1), up(FARM, 2)] }, true);
      s.expectInZone(DATACENTER, "field").expectInZone(TWINSPELL, "graveyard").expectInZone(FARM, "graveyard");
    });

    it("§4.4 the enemy hero takes one hit of 2 per Field Spell destroyed, and your hero none", () => {
      const s = fire({ backrow: [up(DATACENTER, 1)] }, { backrow: [up(TWINSPELL, 1), up(FARM, 2)] }, true);
      expect(hitsOn(s, "p2")).toEqual([4]);
      expect(hitsOn(s, "p1")).toEqual([]);
      s.expectHealth("p1", 30).expectHealth("p2", 26);
    });

    it("R46 R129 an enemy Heroic Power stays; none destroyed, no damage", () => {
      const s = fire({}, { backrow: [up(HEROIC_POWER, 1), down(BEAR, 2)] }, true);
      s.expectInZone(HEROIC_POWER, "field");
      expect(count(s.lastEvents, "damage")).toBe(0);
    });
  });
});
