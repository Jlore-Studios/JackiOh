// C+ #63 Fruit Tree — SPEC §8.7 row 63, BUILD M9 Classic+ row C+ 63: "Field Spell: at each start of
// your turn adds a random card of the Fruit pool (R382) that costs (0), never Fruit Tree (R387); nothing
// at the opponent's start of turn; a full hand burns; hidden from the opponent (R97); the count is
// fixed at 1 with no tunable (balance patch 1); radiant the Fruit is Radiant".

import { defOf, stepParam, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/063-fruit-tree";

const TREE = "classicplus-063";
const FILLER = "core-005"; // (1) Spell Stockpile: a hand card so turns do not auto-end, and deck filler.
const GRAPES = ["classicplus-065-1", "classicplus-065-2", "classicplus-065-3", "classicplus-065-4", "classicplus-065-5"];

/** The cards p1's start of turn added before its draw: `addedToHand` between `turnStarted` and `drawn`. */
function fruitsAdded(s: Scenario, player: "p1" | "p2" = "p1"): Extract<GameEvent, { type: "addedToHand" }>[] {
  const events = s.lastEvents;
  const start = events.findIndex((event) => event.type === "turnStarted" && event.player === player);
  if (start < 0) return [];
  const out: Extract<GameEvent, { type: "addedToHand" }>[] = [];
  for (const event of events.slice(start + 1)) {
    if (event.type === "drawn" && event.player === player) break;
    if (event.type === "addedToHand" && event.player === player) out.push(event);
  }
  return out;
}

/** A board with the Tree in p1's backrow and p1 about to start a turn (p2 ends theirs). */
function grown(opts: { radiant?: boolean; seed?: string; hand?: readonly string[] } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "fruit-tree",
    active: "p2",
    turn: 10,
    p1: {
      backrow: [{ def: TREE, faceUp: true, ...(opts.radiant === true ? { radiant: true } : {}) }],
      hand: opts.hand ?? [FILLER],
      library: [FILLER, FILLER, FILLER],
    },
    p2: { hand: [FILLER], library: [FILLER, FILLER, FILLER] },
  });
}

describe("C+ #63 Fruit Tree", () => {
  it("is a Field Spell whose two faces run one shape of hook", () => {
    expect(def.id).toBe(TREE);
    expect(def.type).toBe("Field Spell");
    expect(typeof base.startOfTurn).toBe("function");
    expect(typeof radiant.startOfTurn).toBe("function");
  });

  describe("base", () => {
    it("R62 at the start of your turn adds one random Fruit to your hand, before the draw, costing (0)", () => {
      const s = grown();
      s.endTurn();

      const added = fruitsAdded(s);
      expect(added).toHaveLength(1);
      const fruit = s.card(added[0]?.instanceId ?? "");
      expect(fruit.zone.z).toBe("hand");
      expect(defOf(s.state, fruit.defId).tags).toContain("Fruit");
      expect(fruit.costOverride).toBe(0);
      expect(fruit.radiant).toBe(false);
      s.expectEvents("turnStarted", "addedToHand", "drawn");
    });

    it("R62 nothing at the opponent's start of turn", () => {
      const s = scenario({
        seed: "fruit-tree",
        active: "p1",
        p1: { backrow: [{ def: TREE, faceUp: true }], hand: [FILLER], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], library: [FILLER, FILLER] },
      });
      const before = s.hand("p1").length;
      s.endTurn();

      expect(s.state.active).toBe("p2");
      expect(fruitsAdded(s, "p1")).toEqual([]);
      expect(s.lastEvents.filter((event) => event.type === "addedToHand" && event.player === "p1")).toEqual([]);
      expect(s.hand("p1")).toHaveLength(before);
    });

    it("R382 R387 over many seeds it hands out only the Fruit pool — Grapes included — and never Fruit Tree", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 120; i += 1) {
        const s = grown({ seed: `tree-${i}` });
        s.endTurn();
        for (const event of fruitsAdded(s)) seen.add(event.defId);
      }
      expect(seen.has(TREE)).toBe(false);
      for (const id of seen) {
        const card = defOf(grown().state, id);
        expect(card.tags).toContain("Fruit");
      }
      expect([...seen].some((id) => GRAPES.includes(id))).toBe(true);
      expect([...seen].some((id) => !GRAPES.includes(id))).toBe(true);
    });

    it("§2.4 R4 a full hand burns the Fruit, which keeps no price in the graveyard", () => {
      const s = grown({ hand: Array.from({ length: 10 }, () => FILLER) });
      s.endTurn();

      const burned = s.lastEvents.find((event) => event.type === "burned");
      expect(burned).toBeDefined();
      const fruit = s.card(burned?.type === "burned" ? burned.instanceId : "");
      expect(fruit.zone.z).toBe("graveyard");
      expect(fruit.costOverride).toBeUndefined();
      expect(fruitsAdded(s)).toEqual([]);
    });

    it("R97 the opponent's view names no Fruit: the add reaches them under the sentinel", () => {
      const s = grown();
      s.endTurn();
      const fruit = fruitsAdded(s)[0];
      expect(fruit).toBeDefined();

      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(theirs.length).toBeGreaterThan(0);
      for (const event of theirs) {
        if (event.type !== "addedToHand") continue;
        expect(event.instanceId).toBe("hidden");
        expect(event.defId).toBe("hidden");
      }
      expect(JSON.stringify(s.view("p2"))).not.toContain(fruit?.instanceId ?? "?");
      const mine = s.view("p1").you.hand;
      expect(Array.isArray(mine) && mine.some((card) => card.instanceId === fruit?.instanceId)).toBe(true);
    });

    it("R386 the count is fixed at 1: an Upgrade or a Degrade still adds exactly one Fruit", () => {
      const up = grown();
      stepParam(up.card(TREE), "fruits", 1);
      up.endTurn();
      expect(fruitsAdded(up)).toHaveLength(1);

      const down = grown();
      stepParam(down.card(TREE), "fruits", -1);
      down.endTurn();
      expect(fruitsAdded(down)).toHaveLength(1);
    });

    it("§3.2 off the field it adds nothing: a Tree in the hand has no start of turn", () => {
      const s = scenario({
        seed: "fruit-tree",
        active: "p2",
        turn: 10,
        p1: { hand: [TREE, FILLER], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], library: [FILLER, FILLER] },
      });
      s.endTurn();
      expect(fruitsAdded(s)).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("R74 the Fruit it adds is Radiant and costs (0)", () => {
      const s = grown({ radiant: true });
      s.endTurn();

      const added = fruitsAdded(s);
      expect(added).toHaveLength(1);
      const fruit = s.card(added[0]?.instanceId ?? "");
      expect(fruit.radiant).toBe(true);
      expect(fruit.costOverride).toBe(0);
    });

    it("R387 never Fruit Tree either, and a Grape comes Radiant too", () => {
      const seen: { defId: string; radiant: boolean }[] = [];
      for (let i = 0; i < 80; i += 1) {
        const s = grown({ radiant: true, seed: `rtree-${i}` });
        s.endTurn();
        for (const event of fruitsAdded(s)) seen.push({ defId: event.defId, radiant: s.card(event.instanceId).radiant });
      }
      expect(seen.some((entry) => entry.defId === TREE)).toBe(false);
      expect(seen.every((entry) => entry.radiant)).toBe(true);
      expect(seen.some((entry) => GRAPES.includes(entry.defId))).toBe(true);
    });

    it("R386 the Radiant count is fixed at 1 too", () => {
      const s = grown({ radiant: true });
      stepParam(s.card(TREE), "fruits", 1);
      s.endTurn();
      expect(fruitsAdded(s)).toHaveLength(1);
      expect(fruitsAdded(s).every((event) => s.card(event.instanceId).radiant)).toBe(true);
    });

    it("§9.3 the same seed adds the same Fruit, and the state survives a JSON round trip", () => {
      const a = grown({ radiant: true, seed: "same" });
      const b = grown({ radiant: true, seed: "same" });
      a.endTurn();
      b.endTurn();
      expect(fruitsAdded(a).map((event) => event.defId)).toEqual(fruitsAdded(b).map((event) => event.defId));
      const revived = JSON.parse(JSON.stringify(a.state)) as GameState;
      expect(revived).toEqual(a.state);
    });
  });
});
