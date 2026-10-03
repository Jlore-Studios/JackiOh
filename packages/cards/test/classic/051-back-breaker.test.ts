// C #51 Back Breaker (SPEC §8.6 row 51; BUILD M9 row C 51). (1) Unit, Common, 3/2 → 6/4: Stack;
// Death: destroy every backrow card (Radiant: every enemy backrow card).

import { describe, expect, it } from "vitest";
import { animateCard, legalActions } from "@jackioh/engine";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const BACK_BREAKER = "classic-051";
const VANILLA = "core-008"; // 4/4
const MENACE = "core-019"; // 9/9 Taunt
const HIT_JOB = "core-016"; // Spell 3: destroy target Unit
const FLOOD = "core-017"; // Spell 4: bounce all Units
const COLLATERAL = "core-034"; // Spell 4: exile target permanent and a random card of the enemy deck
const THE_ROCK = "core-066"; // Tribute 1, Indestructible
const MANA_WELL = "core-006"; // Field Spell
const GOING_LONG = "core-084"; // Field Spell
const SHEEPISH = "core-041"; // Trap
const HONEYPOT = "core-060"; // Trap
const HEROIC_POWER = "core-098"; // Field Spell, Indestructible
const LOCKDOWN = "classic-084"; // Field Spell, Indestructible
const IN_TOO_DEEP = "classic-090"; // Field Spell, Indestructible
const TESLA = "classic-005"; // Field Trap, Animated
const STOCKPILE = "core-005";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

function backrowIds(s: Scenario, player: "p1" | "p2"): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.backrow(player, lane)?.defId ?? null);
}

function bothBackrows(): Pick<SideSetup, "backrow"> & { p2Backrow: SideSetup["backrow"] } {
  return {
    backrow: [MANA_WELL, { def: SHEEPISH, faceUp: false }],
    p2Backrow: [GOING_LONG, { def: HONEYPOT, faceUp: false }],
  };
}

describe("C #51 Back Breaker", () => {
  describe("base", () => {
    it("§3.2 Stack: it may be played onto an occupied unit zone, the card beneath dormant (R13) and resuming when it leaves", () => {
      const s = scenario({ p1: { hand: [BACK_BREAKER, HIT_JOB, STOCKPILE], field: [VANILLA], library: SPARE.library, mana: 10 }, p2: SPARE });
      const breaker = s.card(BACK_BREAKER);
      expect(legalActions(s.state, "p1").some((a) => a.type === "play" && a.instanceId === breaker.id && a.zone?.lane === 1)).toBe(true);
      s.play(BACK_BREAKER, { zone: 1 });
      expect(s.unit("p1", 1)?.defId).toBe(BACK_BREAKER);
      expect(s.view("p1").you.units[0]).toMatchObject({ defId: BACK_BREAKER, buried: 1 });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      s.expectInZone(BACK_BREAKER, "graveyard");
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
    });

    it("§8.6 Death by a destroy: every backrow card on both sides, face-down ones included", () => {
      const { backrow, p2Backrow } = bothBackrows();
      const s = scenario({
        p1: { field: [BACK_BREAKER], backrow, hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: { backrow: p2Backrow, ...SPARE },
      });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      expect(backrowIds(s, "p1")).toEqual([null, null, null, null, null]);
      expect(backrowIds(s, "p2")).toEqual([null, null, null, null, null]);
      for (const id of [MANA_WELL, SHEEPISH]) expect(s.pile("p1", "graveyard").map((c) => c.defId)).toContain(id);
      for (const id of [GOING_LONG, HONEYPOT]) expect(s.pile("p2", "graveyard").map((c) => c.defId)).toContain(id);
    });

    it("§4.5 a combat death fires it too", () => {
      const s = scenario({ p1: { field: [BACK_BREAKER], backrow: [MANA_WELL], ...SPARE }, p2: { field: [MENACE], backrow: [GOING_LONG], ...SPARE } });
      s.attack(BACK_BREAKER, MENACE);
      s.expectInZone(BACK_BREAKER, "graveyard");
      expect(backrowIds(s, "p1")[0]).toBeNull();
      expect(backrowIds(s, "p2")[0]).toBeNull();
    });

    it("§6.3 a Tribute is a death: tributed for The Rock, its Death fires", () => {
      const s = scenario({ p1: { field: [BACK_BREAKER], backrow: [MANA_WELL], hand: [THE_ROCK, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { backrow: [GOING_LONG], ...SPARE } });
      s.play(THE_ROCK, { tributes: [BACK_BREAKER] });
      s.expectInZone(BACK_BREAKER, "graveyard");
      expect(backrowIds(s, "p1")[0]).toBeNull();
      expect(backrowIds(s, "p2")[0]).toBeNull();
    });

    it("R46 Indestructible backrow cards stay: Heroic Power, Lockdown, In Too Deep", () => {
      const s = scenario({
        p1: { field: [BACK_BREAKER], backrow: [HEROIC_POWER, IN_TOO_DEEP], hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: { backrow: [LOCKDOWN, MANA_WELL], ...SPARE },
      });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      expect(backrowIds(s, "p1").slice(0, 2)).toEqual([HEROIC_POWER, IN_TOO_DEEP]);
      expect(backrowIds(s, "p2").slice(0, 2)).toEqual([LOCKDOWN, null]);
    });

    it("R13 only the top of a backrow pile is destroyed, and the card beneath resumes", () => {
      // A backrow pile (B5 E21): a Mana Well stacked onto Going Long, as a Stack play would put it.
      const s = scenario({
        p1: { field: [BACK_BREAKER], hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: { backrow: [GOING_LONG, { def: MANA_WELL, stack: true }], ...SPARE },
      });
      const top = s.backrow("p2", 1)!;
      expect(top.defId).toBe(MANA_WELL);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      s.expectInZone(top, "graveyard");
      expect(s.backrow("p2", 1)?.defId).toBe(GOING_LONG);
    });

    it("R383 an animated card stands in the unit row and is spared", () => {
      const s = scenario({
        p1: { field: [BACK_BREAKER], hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: { backrow: [TESLA, MANA_WELL], ...SPARE },
      });
      // Tesla steps into the unit row as its firing's last step would (B3.1, R383).
      expect(animateCard({ state: s.state, events: [] }, s.card(TESLA))).toBe(true);
      expect(s.unit("p2", 1)?.defId).toBe(TESLA);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      expect(s.unit("p2", 1)?.defId).toBe(TESLA);
      expect(backrowIds(s, "p2")[1]).toBeNull();
    });

    it("§8.6 a bounce or an exile is no death: nothing fires", () => {
      const bounced = scenario({ p1: { field: [BACK_BREAKER], backrow: [MANA_WELL], hand: [FLOOD, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { backrow: [GOING_LONG], ...SPARE } });
      bounced.play(FLOOD);
      bounced.expectInZone(BACK_BREAKER, "hand");
      expect(backrowIds(bounced, "p1")[0]).toBe(MANA_WELL);
      expect(backrowIds(bounced, "p2")[0]).toBe(GOING_LONG);

      const exiled = scenario({ p1: { backrow: [MANA_WELL], hand: [COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { field: [BACK_BREAKER], backrow: [GOING_LONG], ...SPARE } });
      exiled.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: exiled.card(BACK_BREAKER).id }] });
      exiled.expectInZone(BACK_BREAKER, "exile");
      expect(backrowIds(exiled, "p1")[0]).toBe(MANA_WELL);
      expect(backrowIds(exiled, "p2")[0]).toBe(GOING_LONG);
    });

    it("§8.6 3/2 with Stack", () => {
      const s = scenario({ p1: { field: [BACK_BREAKER] } });
      s.expectStats(BACK_BREAKER, { attack: 3, health: 2 });
      expect(s.stats(BACK_BREAKER).keywords).toContainEqual({ kind: "Stack" });
    });
  });

  describe("radiant", () => {
    it("§8.6 6/4: its Death destroys the enemy backrow only, face-down cards included", () => {
      const { backrow, p2Backrow } = bothBackrows();
      const s = scenario({
        p1: { field: [{ def: BACK_BREAKER, radiant: true }], backrow, hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: { backrow: p2Backrow, ...SPARE },
      });
      s.expectStats(BACK_BREAKER, { attack: 6, health: 4 });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      expect(backrowIds(s, "p1").slice(0, 2)).toEqual([MANA_WELL, SHEEPISH]);
      expect(backrowIds(s, "p2")).toEqual([null, null, null, null, null]);
    });

    it("R89 'enemy' is the opponent of the player it died under: on p2's side it takes p1's backrow", () => {
      const s = scenario({
        active: "p2",
        p1: { backrow: [MANA_WELL], ...SPARE },
        p2: { field: [{ def: BACK_BREAKER, radiant: true }], backrow: [GOING_LONG], hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
      });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(BACK_BREAKER).id }] });
      expect(backrowIds(s, "p1")[0]).toBeNull();
      expect(backrowIds(s, "p2")[0]).toBe(GOING_LONG);
    });
  });
});
