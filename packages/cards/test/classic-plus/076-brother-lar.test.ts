// C+ #76 Brother Lar — SPEC §8.7 row 76, BUILD M9 Classic+ row C+ 76: "Death summons a Brother Ping
// (C+ #76.1) into your leftmost open unit zone, Lar's own zone included unless a granted Reborn reserves
// it (R64); exile or a bounce summons nothing; radiant the Ping is Radiant".

import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/076-brother-lar";

const LAR = "classicplus-076";
const PING = "classicplus-076-1";
const FILLER = "core-005";
const MENACE = "core-019"; // 9/9 Taunt
const TIMMY = "core-011";
const HIT_JOB = "core-016"; // (3) "Destroy target Unit."
const COLLATERAL = "core-034"; // (4) "Exile target permanent and a random card from your opponent's deck."
const FLOOD = "core-017"; // (4) "Bounce all Units."

function target(s: Scenario, ref: string): Selection[] {
  return [{ pick: "instance", instanceId: s.card(ref).id }];
}

function pings(s: Scenario): ReturnType<Scenario["card"]>[] {
  return [1, 2, 3, 4, 5].flatMap((lane) => {
    const unit = s.unit("p1", lane);
    return unit !== null && unit.defId === PING ? [unit] : [];
  });
}

describe("C+ #76 Brother Lar", () => {
  it("is a (1) 1/1 CN Human Unit (2/2 Radiant) naming Brother Ping", () => {
    expect(def.id).toBe(LAR);
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([1, 1, 1, 2, 2]);
    expect(def.tags).toEqual(["CN", "Human"]);
    expect(def.refs).toEqual([PING]);
    expect(typeof base.death).toBe("function");
    expect(typeof radiant.death).toBe("function");
  });

  describe("base", () => {
    it("§6.2 dying in combat, it leaves a Brother Ping in its own zone, the leftmost open one", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [LAR] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.attack(LAR, MENACE);
      s.expectInZone(LAR, "graveyard");
      expect(s.unit("p1", 1)?.defId).toBe(PING);
      expect(s.unit("p1", 1)?.radiant).toBe(false);
      s.expectEvents("destroyed", "summoned");
    });

    it("R64 destroyed by an effect in lane 3, Ping takes the leftmost open zone", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [{ def: LAR, lane: 3 }] }, p2: { hand: [FILLER] } });
      s.play(HIT_JOB, { targets: target(s, LAR) });
      expect(s.unit("p1", 1)?.defId).toBe(PING);
      expect(s.unit("p1", 3)).toBeNull();
    });

    it("R1 Ping is summoned, so it fires no Cry and is not a play", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [LAR] }, p2: { hand: [FILLER] } });
      s.play(HIT_JOB, { targets: target(s, LAR) });
      expect(s.lastEvents.filter((event) => event.type === "cardPlayed").map((event) => event.type === "cardPlayed" && event.defId)).toEqual([HIT_JOB]);
    });

    it("R64 a granted Reborn reserves Lar's zone for its return, so Ping goes to the next open zone", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [LAR] }, p2: { hand: [FILLER] } });
      s.card(LAR).grantedKeywords = [{ kind: "Reborn" }];
      s.play(HIT_JOB, { targets: target(s, LAR) });
      expect(s.unit("p1", 2)?.defId).toBe(PING);
      expect(s.unit("p1", 1)?.defId).toBe(LAR);
    });

    it("R64 a full row summons nothing — the Ping has nowhere to go", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [LAR, TIMMY, TIMMY, TIMMY, TIMMY] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.card(LAR).grantedKeywords = [{ kind: "Reborn" }];
      s.attack(LAR, MENACE);
      expect(pings(s)).toEqual([]);
    });

    it("§6.2 exiled, it has no Death: no Ping", () => {
      const s = scenario({ p1: { hand: [COLLATERAL, FILLER], field: [LAR] }, p2: { hand: [FILLER], library: [FILLER] } });
      s.play(COLLATERAL, { targets: target(s, LAR) });
      s.expectInZone(LAR, "exile");
      expect(pings(s)).toEqual([]);
    });

    it("§6.2 bounced, it has no Death: no Ping", () => {
      const s = scenario({ p1: { hand: [FLOOD, FILLER], field: [LAR] }, p2: { hand: [FILLER] } });
      s.play(FLOOD);
      s.expectInZone(LAR, "hand");
      expect(pings(s)).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("R74 its Death summons a Radiant Brother Ping, an 8/8", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: LAR, radiant: true }] }, p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] } });
      s.attack(LAR, MENACE);
      const ping = s.unit("p1", 1);
      expect(ping?.defId).toBe(PING);
      expect(ping?.radiant).toBe(true);
      s.expectStats(ping ?? PING, { attack: 8, maxHealth: 8 });
    });

    it("§6.2 exiled, the Radiant face summons nothing either", () => {
      const s = scenario({ p1: { hand: [COLLATERAL, FILLER], field: [{ def: LAR, radiant: true }] }, p2: { hand: [FILLER], library: [FILLER] } });
      s.play(COLLATERAL, { targets: target(s, LAR) });
      expect(pings(s)).toEqual([]);
    });
  });
});
