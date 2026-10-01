// C+ #60 Doctors Orders — SPEC §8.7 row 60, BUILD M9 Classic+ row C+ 60: "A Field Spell with no stats
// (R421): its Cry and each start of your turn add an All Purpose Apple (C+ #59) to your hand, burned
// when the hand is full; nothing at the opponent's start of turn; it needs an open backrow zone to
// play; the count reads through `param()`; radiant the Apple is Radiant".

import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/060-doctors-orders";

const ORDERS = "classicplus-060";
const APPLE = "classicplus-059";
const WELL = "core-006"; // Mana Well, a Field Spell to fill a backrow
const FILLER = "core-005";

/** The Apples that reached a hand in the last step (a draw reports `addedToHand` too). */
function added(s: Scenario, player: "p1" | "p2" = "p1"): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.filter(
    (event): event is Extract<GameEvent, { type: "addedToHand" }> =>
      event.type === "addedToHand" && event.player === player && event.defId === APPLE,
  );
}

function inHand(opts: { radiant?: boolean; fillers?: number; backrow?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: {
      hand: [{ def: ORDERS, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)],
      backrow: opts.backrow ?? [],
      library: [FILLER, FILLER],
    },
    p2: { hand: [FILLER], library: [FILLER, FILLER] },
  });
}

/** Doctors Orders already in p1's backrow, p2 about to end their turn. */
function standing(opts: { radiant?: boolean; fillers?: number } = {}): Scenario {
  return scenario({
    active: "p2",
    turn: 10,
    p1: {
      backrow: [{ def: ORDERS, ...(opts.radiant === true ? { radiant: true } : {}) }],
      hand: Array.from({ length: opts.fillers ?? 1 }, () => FILLER),
      library: [FILLER, FILLER],
    },
    p2: { hand: [FILLER], library: [FILLER, FILLER] },
  });
}

describe("C+ #60 Doctors Orders", () => {
  it("R421 is a (1) Field Spell with no stats and no Fruit tag, naming the Apple", () => {
    expect(def.type).toBe("Field Spell");
    expect(def.base.attack).toBeUndefined();
    expect(def.tags).toEqual([]);
    expect(def.refs).toEqual([APPLE]);
  });

  describe("base", () => {
    it("§6.2 its Cry adds an All Purpose Apple to your hand as it enters", () => {
      const s = inHand();
      s.play(ORDERS);
      s.expectInZone(ORDERS, "field");
      const apples = added(s);
      expect(apples.map((event) => event.defId)).toEqual([APPLE]);
      expect(s.card(apples[0]?.instanceId ?? "").radiant).toBe(false);
    });

    it("R62 each start of your turn adds another, before the draw", () => {
      const s = standing();
      s.endTurn();
      expect(added(s).map((event) => event.defId)).toEqual([APPLE]);
      const order = s.lastEvents.map((event) => (event.type === "addedToHand" && event.defId === APPLE ? "apple" : event.type));
      expect(order.indexOf("apple")).toBeGreaterThan(order.indexOf("turnStarted"));
      expect(order.indexOf("apple")).toBeLessThan(order.indexOf("drawn"));
    });

    it("R62 nothing at the opponent's start of turn", () => {
      const s = inHand();
      s.play(ORDERS);
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(added(s, "p1")).toEqual([]);
    });

    it("§3.2 it needs an open backrow zone to play", () => {
      const s = inHand({ backrow: [WELL, WELL, WELL, WELL, WELL] });
      expect(() => s.play(ORDERS)).toThrow();
      s.expectInZone(ORDERS, "hand");
    });

    it("§2.4 R4 a full hand burns the Apple", () => {
      const s = standing({ fillers: 10 });
      s.endTurn();
      const burned = s.lastEvents.find((event) => event.type === "burned");
      expect(burned?.type === "burned" ? burned.defId : "").toBe(APPLE);
    });

    it("R97 the opponent sees the add under the sentinel", () => {
      const s = standing();
      s.endTurn();
      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(theirs.length).toBeGreaterThan(0);
      for (const event of theirs) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
    });

    it("R386 an Upgrade adds 2 Apples, on the Cry and at the start of turn", () => {
      const s = inHand();
      stepParam(s.card(ORDERS), "apples", 1);
      s.play(ORDERS);
      expect(added(s)).toHaveLength(2);

      const later = standing();
      stepParam(later.card(ORDERS), "apples", 1);
      later.endTurn();
      expect(added(later)).toHaveLength(2);
    });
  });

  describe("radiant", () => {
    it("R74 the Apple of its Cry and of each start of turn is Radiant", () => {
      const s = inHand({ radiant: true });
      s.play(ORDERS);
      expect(added(s).map((event) => s.card(event.instanceId).radiant)).toEqual([true]);

      const later = standing({ radiant: true });
      later.endTurn();
      expect(added(later).map((event) => later.card(event.instanceId).radiant)).toEqual([true]);
    });

    it("the Radiant Apple it adds plays as a Radiant Apple: a Radiant Rush Token, heal 4, 2 damage", () => {
      const s = inHand({ radiant: true });
      s.play(ORDERS);
      const apple = added(s)[0]?.instanceId ?? "";
      s.play(apple, { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.unit("p1", 1)?.radiant).toBe(true);
      s.expectHealth("p1", 34).expectHealth("p2", 28);
    });
  });
});
