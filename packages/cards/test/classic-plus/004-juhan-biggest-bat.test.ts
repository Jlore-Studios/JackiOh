// C+ #4 Juhan Biggest Bat — SPEC §8.7 row 4, BUILD M9 Classic+ row C+ 4: "Stack; played onto your
// occupied unit zone, every dormant card beneath becomes a copy of Juhan on its face, keeping each old
// card's owner and controller and staying dormant, an Immutable card staying itself (R23); no copy
// fires a Cry (R1); when the top Juhan leaves, the next Juhan resumes; played onto an empty zone nothing
// is transformed; radiant Stack, First Strike, and the copies are Radiant".

import { beneathAt } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/004-juhan-biggest-bat";

const JUHAN = "classicplus-004";
const VANILLA = "core-008"; // (1) 4/4.
const FIENDER = "core-092"; // (2) 5/7 Stack.
const MENACE = "core-019"; // Radiant: Taunt, Immutable.
const MIND_CONTROL = "core-049"; // (4) Steal target enemy permanent.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const FILLER = "core-010";
const STOCKPILE = "core-005";

const LANE_1 = { player: "p1", row: "units", lane: 1 } as const;

function setup(p1: SideSetup, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: JUHAN, radiant: radiantFace }, FILLER], library: [STOCKPILE, STOCKPILE], ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], ...p2 },
  });
}

function beneath(s: Scenario) {
  return beneathAt(s.state, LANE_1);
}

function count(s: Scenario, type: string): number {
  return s.events.filter((event) => event.type === type).length;
}

describe("C+ #4 Juhan Biggest Bat", () => {
  it("is a (3) 9/6 CN Unit with Stack (Radiant 18/12 with Stack and First Strike); one script runs both faces", () => {
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([3, 9, 6, 18, 12]);
    expect(def.base.keywords.map((k) => k.kind)).toEqual(["Stack"]);
    expect(def.radiant.keywords.map((k) => k.kind)).toEqual(["Stack", "First Strike"]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("E24 played onto a pile, every dormant card beneath becomes a dormant Juhan; the old cards cease to exist", () => {
      const s = setup({ field: [VANILLA, { def: FIENDER, stack: true }] });
      const oldIds = [s.unit("p1", 1)!.id, ...beneath(s).map((card) => card.id)];
      expect(oldIds).toHaveLength(2);
      const juhan = s.card(JUHAN);

      s.play(juhan, { zone: 1 });

      expect(s.unit("p1", 1)?.id).toBe(juhan.id);
      const under = beneath(s);
      expect(under.map((card) => card.defId)).toEqual([JUHAN, JUHAN]);
      expect(under.every((card) => card.radiant === false && card.owner === "p1" && card.controller === "p1")).toBe(true);
      expect(count(s, "transformed")).toBe(2);
      for (const id of oldIds) s.expectInZone(id, "gone");
    });

    it("R1 no copy fires a Cry: one play, one Cry, one transform per card beneath", () => {
      const s = setup({ field: [VANILLA, { def: FIENDER, stack: true }] });
      const juhan = s.card(JUHAN);

      s.play(juhan, { zone: 1 });

      expect(count(s, "cardPlayed")).toBe(1);
      expect(count(s, "transformed")).toBe(2);
      // The one `summoned` is the played Juhan's own arrival; a copy is made in place, not summoned.
      expect(s.events.filter((event) => event.type === "summoned").map((event) => event.instanceId)).toEqual([juhan.id]);
    });

    it("§6.3 Replace each copy keeps the old card's owner and controller", () => {
      const s = setup({ hand: [MIND_CONTROL, { def: JUHAN }], mana: 7 }, false, { field: [VANILLA] });
      const stolen = s.unit("p2", 1)!;

      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: stolen.id }] });
      const lane = s.card(stolen).zone;
      if (lane.z !== "field") throw new Error("Mind Control did not take the Vanilla");
      s.play(JUHAN, { zone: lane.lane });

      const copy = beneathAt(s.state, { player: "p1", row: "units", lane: lane.lane })[0];
      expect(copy?.defId).toBe(JUHAN);
      // The stolen Vanilla has been p1's since the steal (R669), and its copy is too.
      expect(copy?.owner).toBe("p1");
      expect(copy?.controller).toBe("p1");
    });

    it("R23 an Immutable card beneath stays itself", () => {
      const s = setup({ field: [{ def: MENACE, radiant: true }, { def: FIENDER, stack: true }] });

      s.play(JUHAN, { zone: 1 });

      expect(beneath(s).map((card) => card.defId)).toEqual([JUHAN, MENACE]);
      expect(count(s, "transformed")).toBe(1);
    });

    it("§3.2 when the top Juhan leaves, the next Juhan resumes", () => {
      const s = setup({ hand: [{ def: JUHAN }, HIT_JOB, FILLER], field: [VANILLA, { def: FIENDER, stack: true }], mana: 6 });
      const top = s.card(JUHAN);
      s.play(top, { zone: 1 });
      const next = beneath(s)[0]!;

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: top.id }] });

      s.expectInZone(top, "graveyard");
      expect(s.unit("p1", 1)?.id).toBe(next.id);
      expect(s.unit("p1", 1)?.defId).toBe(JUHAN);
      expect(beneath(s).map((card) => card.defId)).toEqual([JUHAN]);
      s.expectStats(next, { attack: 9, health: 6 });
    });

    it("played onto an empty zone it transforms nothing", () => {
      const s = setup({ field: [VANILLA] });

      s.play(JUHAN, { zone: 2 });

      expect(count(s, "transformed")).toBe(0);
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
      expect(beneathAt(s.state, { player: "p1", row: "units", lane: 2 })).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("has Stack and First Strike, and the copies beneath are Radiant Juhans", () => {
      const s = setup({ field: [VANILLA, { def: FIENDER, stack: true }] }, true);
      const juhan = s.card(JUHAN);

      s.play(juhan, { zone: 1 });

      expect(s.stats(juhan).keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Stack", "First Strike"]));
      s.expectStats(juhan, { attack: 18, health: 12 });
      expect(beneath(s).map((card) => [card.defId, card.radiant])).toEqual([
        [JUHAN, true],
        [JUHAN, true],
      ]);
    });
  });
});
