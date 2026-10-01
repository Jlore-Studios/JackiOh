// C+ #20 Mushroom Power — SPEC §8.7 row 20, BUILD M9 Classic+ row C+ 20: "Cry gives the Units in the
// unit zones beside it on its side (lanes N−1 and N+1) +2/+2 permanently: one neighbour in lane 1 or 5,
// nothing across the lane or in the backrow, nothing when the neighbours are empty; the buffs stay
// after it leaves; the buff reads through `param()`; radiant +4/+4".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/020-mushroom-power";

const MUSHROOM = "classicplus-020";
const BODY = "core-019"; // Midrange Menace 9/9 Taunt
const SMALL = "core-012"; // Duplicating Felinors 3/4
const FIELD_SPELL = "core-075"; // Infinite Reserves, which touches no Unit
const FILLER = "core-005";

describe("C+ #20 Mushroom Power", () => {
  it("is a (1) 2/2 Unit whose Cry declares no choice; both faces run one script", () => {
    expect(def.id).toBe(MUSHROOM);
    expect(def.cost).toBe(1);
    expect(base.targets).toBeUndefined();
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§3.1 gives both neighbours +2/+2 and leaves lanes further off, the other side and the backrow alone", () => {
      const s = scenario({
        p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 1 }, { def: SMALL, lane: 2 }, { def: BODY, lane: 4 }], backrow: [{ def: FIELD_SPELL, lane: 4 }] },
        p2: { hand: [FILLER], field: [{ def: BODY, lane: 3 }] },
      });
      const far = s.unit("p1", 1);
      const left = s.unit("p1", 2);
      const right = s.unit("p1", 4);
      const across = s.unit("p2", 3);
      if (far === null || left === null || right === null || across === null) throw new Error("setup");

      s.play(MUSHROOM, { zone: 3 });

      s.expectStats(left, { attack: 5, health: 6, maxHealth: 6 });
      s.expectStats(right, { attack: 11, health: 11 });
      s.expectStats(far, { attack: 9, health: 9 });
      s.expectStats(across, { attack: 9, health: 9 });
      s.expectStats(MUSHROOM, { attack: 2, health: 2 });
    });

    it("§3.1 in lane 1 it has one neighbour, lane 2", () => {
      const s = scenario({ p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 2 }] }, p2: { hand: [FILLER] } });
      const neighbour = s.unit("p1", 2);
      if (neighbour === null) throw new Error("setup");
      s.play(MUSHROOM, { zone: 1 });
      s.expectStats(neighbour, { attack: 11, health: 11 });
    });

    it("§3.1 in lane 5 it has one neighbour, lane 4", () => {
      const s = scenario({ p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 4 }] }, p2: { hand: [FILLER] } });
      const neighbour = s.unit("p1", 4);
      if (neighbour === null) throw new Error("setup");
      s.play(MUSHROOM, { zone: 5 });
      s.expectStats(neighbour, { attack: 11, health: 11 });
    });

    it("with both neighbouring zones empty nothing is buffed", () => {
      const s = scenario({ p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 1 }] }, p2: { hand: [FILLER] } });
      s.play(MUSHROOM, { zone: 3 });
      expect(s.events.some((event) => event.type === "buffed")).toBe(false);
    });

    it("§3.2 only the top of a Stack pile is a neighbour; the dormant card beneath is not buffed", () => {
      const s = scenario({
        p1: { hand: [MUSHROOM, FILLER], field: [{ def: SMALL, lane: 2 }, { def: "core-092", lane: 2, stack: true }] },
        p2: { hand: [FILLER] },
      });
      const dormant = s.state.players.p1.units[1]?.[1];
      if (dormant === undefined) throw new Error("no pile");
      s.play(MUSHROOM, { zone: 3 });
      expect(s.card(dormant).buffs).toEqual({ attack: 0, health: 0 });
      expect(s.events.filter((event) => event.type === "buffed")).toHaveLength(1);
    });

    it("§10.4 the buffs are permanent: they stay after Mushroom Power leaves the field", () => {
      const s = scenario({
        p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 2 }] },
        p2: { hand: ["core-016", FILLER], field: [] },
        active: "p1",
      });
      const neighbour = s.unit("p1", 2);
      if (neighbour === null) throw new Error("setup");
      s.play(MUSHROOM, { zone: 3 });
      const mushroom = s.unit("p1", 3);
      if (mushroom === null) throw new Error("no mushroom");
      s.endTurn();
      s.play("core-016", { targets: [{ pick: "instance", instanceId: mushroom.id }] });
      s.expectInZone(mushroom, "graveyard");
      s.expectStats(neighbour, { attack: 11, health: 11 });
    });

    it("R386 the buff reads through param: an Upgrade makes it +3/+3", () => {
      const s = scenario({ p1: { hand: [MUSHROOM, FILLER], field: [{ def: BODY, lane: 2 }] }, p2: { hand: [FILLER] } });
      stepParam(s.card(MUSHROOM), "buff", 1);
      const neighbour = s.unit("p1", 2);
      if (neighbour === null) throw new Error("setup");
      s.play(MUSHROOM, { zone: 3 });
      s.expectStats(neighbour, { attack: 12, health: 12 });
    });
  });

  describe("radiant", () => {
    it("is 4/4 and gives each neighbour +4/+4", () => {
      const s = scenario({
        p1: { hand: [{ def: MUSHROOM, radiant: true }, FILLER], field: [{ def: SMALL, lane: 2 }, { def: BODY, lane: 4 }] },
        p2: { hand: [FILLER] },
      });
      const left = s.unit("p1", 2);
      const right = s.unit("p1", 4);
      if (left === null || right === null) throw new Error("setup");
      s.play(MUSHROOM, { zone: 3 });
      s.expectStats(MUSHROOM, { attack: 4, health: 4 });
      s.expectStats(left, { attack: 7, health: 8 });
      s.expectStats(right, { attack: 13, health: 13 });
    });
  });
});
