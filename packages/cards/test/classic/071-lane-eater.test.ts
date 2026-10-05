// C #71 Lane Eater (SPEC §8.6 row 71; BUILD M9 row C 71). (3) Unit, Common, 4/4 → 8/8: Cry: destroy
// every other card in this lane, then Lock this lane. Radiant: the enemy cards and the enemy side only.

import { describe, expect, it } from "vitest";
import { isLocked, legalActions } from "@jackioh/engine";
import type { PlayerId, Row } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const LANE_EATER = "classic-071";
const VANILLA = "core-008"; // 4/4
const TIMMY = "core-011"; // 3/3
const FIENDER = "core-092"; // Felinor Fiender: Stack
const DEFENDER = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn
const THE_ROCK = "core-066"; // Tribute 1, Indestructible
const HIT_JOB = "core-016";
const MANA_WELL = "core-006"; // Field Spell
const GOING_LONG = "core-084"; // Field Spell
const SHEEPISH = "core-041"; // Trap
const GIFTED = "core-064"; // Field Spell
const TOKEN_MAKER = "core-015"; // Me and Mr Token: Cry: summon a Rush Token
const STOCKPILE = "core-005";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

function locked(s: Scenario, player: PlayerId, row: Row, lane: number): boolean {
  return isLocked(s.state, { player, row, lane });
}

/** The four zones of lane 2, and whether each is Locked: p1 units, p1 backrow, p2 units, p2 backrow. */
function laneTwoLocks(s: Scenario): boolean[] {
  return [locked(s, "p1", "units", 2), locked(s, "p1", "backrow", 2), locked(s, "p2", "units", 2), locked(s, "p2", "backrow", 2)];
}

/** Lane 2 full on both sides, and cards in lanes 1 and 3 that the Cry must not touch. */
function board(radiant = false): Scenario {
  return scenario({
    p1: {
      hand: [{ def: LANE_EATER, radiant }, STOCKPILE],
      field: [{ def: TIMMY, lane: 1 }],
      backrow: [{ def: MANA_WELL, lane: 2 }, { def: GIFTED, lane: 1 }],
      library: SPARE.library,
      mana: 10,
    },
    p2: {
      field: [{ def: VANILLA, lane: 2 }, { def: TIMMY, lane: 3 }],
      backrow: [{ def: SHEEPISH, lane: 2, faceUp: false }, { def: GOING_LONG, lane: 3 }],
      ...SPARE,
    },
  });
}

describe("C #71 Lane Eater", () => {
  describe("base", () => {
    it("§8.6 Cry: the top card of every other zone in its lane is destroyed — your backrow zone and both of the opponent's", () => {
      const s = board();
      const vanilla = s.unit("p2", 2)!;
      const trap = s.backrow("p2", 2)!;
      const well = s.backrow("p1", 2)!;
      s.play(LANE_EATER, { zone: 2 });
      s.expectInZone(vanilla, "graveyard").expectInZone(trap, "graveyard").expectInZone(well, "graveyard");
      // Lane Eater stays; lanes 1 and 3 are untouched.
      expect(s.unit("p1", 2)?.defId).toBe(LANE_EATER);
      expect([s.unit("p1", 1)?.defId, s.backrow("p1", 1)?.defId, s.unit("p2", 3)?.defId, s.backrow("p2", 3)?.defId]).toEqual([
        TIMMY,
        GIFTED,
        TIMMY,
        GOING_LONG,
      ]);
    });

    it("§3.2 then all four zones of the lane are Locked, and a Lock evicts nothing: Lane Eater stays in its Locked zone", () => {
      const s = board();
      s.play(LANE_EATER, { zone: 2 });
      expect(laneTwoLocks(s)).toEqual([true, true, true, true]);
      expect(s.lastEvents.filter((e) => e.type === "locked")).toHaveLength(4);
      expect(s.unit("p1", 2)?.defId).toBe(LANE_EATER);
      expect([locked(s, "p1", "units", 1), locked(s, "p2", "units", 3)]).toEqual([false, false]);
    });

    it("R13 a dormant card beneath a destroyed top resumes, and stays in its Locked zone", () => {
      const s = scenario({
        p1: { hand: [LANE_EATER, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { field: [{ def: VANILLA, lane: 2 }, { def: FIENDER, lane: 2, stack: true }], ...SPARE },
      });
      const top = s.unit("p2", 2)!;
      s.play(LANE_EATER, { zone: 2 });
      s.expectInZone(top, "graveyard");
      expect(s.unit("p2", 2)?.defId).toBe(VANILLA);
      expect(locked(s, "p2", "units", 2)).toBe(true);
    });

    it("R46 an Indestructible card stays, in a Locked zone", () => {
      const s = scenario({ p1: { hand: [LANE_EATER, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { field: [{ def: THE_ROCK, lane: 2 }], ...SPARE } });
      s.play(LANE_EATER, { zone: 2 });
      expect(s.unit("p2", 2)?.defId).toBe(THE_ROCK);
      expect(locked(s, "p2", "units", 2)).toBe(true);
    });

    it("R47 R688 a destroyed Reborn Unit returns to its now-Locked zone: a Lock refuses plays, not the return", () => {
      const s = scenario({ p1: { hand: [LANE_EATER, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { field: [{ def: DEFENDER, lane: 2 }], ...SPARE } });
      const defenderId = s.unit("p2", 2)!.id;
      s.play(LANE_EATER, { zone: 2 });
      expect(s.unit("p2", 2)?.id).toBe(defenderId);
      expect(s.unit("p2", 2)?.defId).toBe(DEFENDER);
      expect(locked(s, "p2", "units", 2)).toBe(true);
    });

    it("R13 a backrow pile: only its top is destroyed, and the card beneath resumes in the Locked zone", () => {
      const s = scenario({
        p1: { hand: [LANE_EATER, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { backrow: [{ def: GOING_LONG, lane: 2 }, { def: MANA_WELL, lane: 2, stack: true }], ...SPARE },
      });
      const top = s.backrow("p2", 2)!;
      s.play(LANE_EATER, { zone: 2 });
      s.expectInZone(top, "graveyard");
      expect(s.backrow("p2", 2)?.defId).toBe(GOING_LONG);
      expect(locked(s, "p2", "backrow", 2)).toBe(true);
    });

    it("§3.2 once it has left, `legalActions` never offers its empty Locked zone, and a summon passes it over", () => {
      const s = scenario({
        p1: { hand: [LANE_EATER, HIT_JOB, TOKEN_MAKER, VANILLA, STOCKPILE], field: [{ def: TIMMY, lane: 1 }], library: SPARE.library, mana: 20 },
        p2: SPARE,
      });
      s.play(LANE_EATER, { zone: 2 });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(LANE_EATER).id }] });
      expect(s.unit("p1", 2)).toBeNull();
      const vanilla = s.card(VANILLA);
      const zones = legalActions(s.state, "p1").flatMap((a) =>
        a.type === "play" && a.instanceId === vanilla.id && a.zone !== undefined ? [a.zone.lane] : [],
      );
      expect(zones).toEqual([3, 4, 5]);
      // Me and Mr Token takes lane 3; the Rush Token it summons passes empty, Locked lane 2 for lane 4.
      s.play(TOKEN_MAKER, { zone: 3 });
      expect(s.unit("p1", 2)).toBeNull();
      expect(s.unit("p1", 4)?.defId).toBe("core-t-rush");
    });

    it("§3.2 later plays into those zones are refused, and its zone stays Locked after it leaves", () => {
      const s = scenario({
        p1: { hand: [LANE_EATER, HIT_JOB, MANA_WELL, VANILLA, STOCKPILE], library: SPARE.library, mana: 20 },
        p2: SPARE,
      });
      s.play(LANE_EATER, { zone: 2 });
      expect(() => s.play(MANA_WELL, { zone: 2 })).toThrow();
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(LANE_EATER).id }] });
      s.expectInZone(LANE_EATER, "graveyard");
      expect(locked(s, "p1", "units", 2)).toBe(true);
      expect(() => s.play(VANILLA, { zone: 2 })).toThrow();
      // The lanes beside it still take plays.
      s.play(VANILLA, { zone: 3 });
      expect(s.unit("p1", 3)?.defId).toBe(VANILLA);
    });

    it("§8.6 a 4/4", () => {
      const s = board();
      s.play(LANE_EATER, { zone: 2 });
      s.expectStats(LANE_EATER, { attack: 4, health: 4 });
    });
  });

  describe("radiant", () => {
    it("§8.6 8/8: it destroys only the enemy cards in its lane, and Locks only the enemy's two zones", () => {
      const s = board(true);
      const vanilla = s.unit("p2", 2)!;
      const trap = s.backrow("p2", 2)!;
      s.play(LANE_EATER, { zone: 2 });
      s.expectStats(LANE_EATER, { attack: 8, health: 8 });
      s.expectInZone(vanilla, "graveyard").expectInZone(trap, "graveyard");
      expect(s.backrow("p1", 2)?.defId).toBe(MANA_WELL);
      expect(laneTwoLocks(s)).toEqual([false, false, true, true]);
      expect(s.lastEvents.filter((e) => e.type === "locked")).toHaveLength(2);
    });
  });
});
