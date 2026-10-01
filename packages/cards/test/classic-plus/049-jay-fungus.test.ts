// C+ #49 Jay Fungus — SPEC §8.7 row 49, BUILD M9 Classic+ row C+ 49: "Taunt; at the end of your turn
// one random hand card whose cost is above 0 and not X gets `costMod` −2 (R65), the cost flooring at
// 0; no such card, nothing and no random draw (R129); the opponent's `costChanged` reads the sentinel
// and −1 (R177); the discount reads through `param()`; radiant −20, taking any such card to (0)".

import { effectiveCost, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/049-jay-fungus";

const FUNGUS = "classicplus-049";
const X_SPELL = "core-024"; // Efficiency Dividend, X Cost
const FREE = "core-010"; // Rapid Replenish, (0)
const THREE = "core-016"; // Hit Job, (3)
const ONE = "core-005"; // Stockpile, (1)

function grown(opts: { radiant?: boolean; hand?: readonly string[]; fungus?: boolean; seed?: string } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "jay-fungus",
    p1: {
      field: opts.fungus === false ? [] : [{ def: FUNGUS, ...(opts.radiant === true ? { radiant: true } : {}) }],
      hand: opts.hand ?? [X_SPELL, FREE, THREE],
      library: [ONE, ONE],
    },
    p2: { hand: [ONE], library: [ONE, ONE] },
  });
}

function costChanges(s: Scenario): { instanceId: string; cost: number }[] {
  return s.lastEvents.flatMap((event) => (event.type === "costChanged" ? [{ instanceId: event.instanceId, cost: event.cost }] : []));
}

describe("C+ #49 Jay Fungus", () => {
  it("is a 3/6 Taunt (6/12 Radiant) whose two faces run one script", () => {
    expect(def.id).toBe(FUNGUS);
    const s = grown();
    expect(s.stats(FUNGUS).keywords).toEqual(expect.arrayContaining([{ kind: "Taunt" }]));
    s.expectStats(FUNGUS, { attack: 3, health: 6 });
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R65 at the end of your turn the one hand card above (0) that is not X costs (2) less", () => {
      const s = grown();
      s.endTurn();
      const three = s.card(THREE);
      expect(three.costMod).toBe(-2);
      expect(effectiveCost(s.state, three)).toBe(1);
      expect(s.card(X_SPELL).costMod).toBe(0);
      expect(s.card(FREE).costMod).toBe(0);
      expect(costChanges(s)).toEqual([{ instanceId: three.id, cost: 1 }]);
    });

    it("§2.3 the cost floors at (0): a (1) Cost card goes to (0)", () => {
      const s = grown({ hand: [ONE] });
      s.endTurn();
      const one = s.hand("p1").find((card) => card.defId === ONE);
      expect(one?.costMod).toBe(-2);
      expect(one === undefined ? -1 : effectiveCost(s.state, one)).toBe(0);
    });

    it("R60 the card is random among those it can make cheaper", () => {
      const picked = new Set<number>();
      for (let i = 0; i < 40; i += 1) {
        const s = grown({ hand: [THREE, THREE, THREE], seed: `fungus-${i}` });
        s.endTurn();
        picked.add(s.hand("p1").findIndex((card) => card.costMod === -2));
      }
      expect([...picked].sort()).toEqual([0, 1, 2]);
    });

    it("R129 no card it can make cheaper: nothing changes and no random number is drawn", () => {
      const withFungus = grown({ hand: [X_SPELL, FREE] });
      const without = grown({ hand: [X_SPELL, FREE], fungus: false });
      withFungus.endTurn();
      without.endTurn();
      expect(costChanges(withFungus)).toEqual([]);
      expect(withFungus.state.rngCursor).toBe(without.state.rngCursor);
    });

    it("§6.2 the opponent's end of turn does nothing", () => {
      const s = grown();
      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(costChanges(s)).toEqual([]);
      expect(s.card(THREE).costMod).toBe(-2);
    });

    it("R177 the opponent's `costChanged` names no card and reads −1", () => {
      const s = grown();
      s.endTurn();
      const theirs = s.view("p2").events.filter((event) => event.type === "costChanged");
      expect(theirs).toEqual([{ type: "costChanged", instanceId: "hidden", cost: -1 }]);
      const mine = s.view("p1").events.filter((event) => event.type === "costChanged");
      expect(mine).toEqual([{ type: "costChanged", instanceId: s.card(THREE).id, cost: 1 }]);
    });

    it("R386 an Upgrade makes it (3) less; a Degrade (1) less, never below 1", () => {
      const up = grown();
      stepParam(up.card(FUNGUS), "discount", 1);
      up.endTurn();
      expect(up.card(THREE).costMod).toBe(-3);

      const down = grown();
      stepParam(down.card(FUNGUS), "discount", -5);
      down.endTurn();
      expect(down.card(THREE).costMod).toBe(-1);
    });
  });

  describe("radiant", () => {
    it("6/12 Taunt; the card costs (20) less, which takes any such card to (0)", () => {
      const s = grown({ radiant: true });
      s.expectStats(FUNGUS, { attack: 6, health: 12 });
      s.endTurn();
      const three = s.card(THREE);
      expect(three.costMod).toBe(-20);
      expect(effectiveCost(s.state, three)).toBe(0);
      expect(s.card(X_SPELL).costMod).toBe(0);
    });

    it("R129 the Radiant face skips (0) and X cards too, drawing nothing", () => {
      const withFungus = grown({ radiant: true, hand: [FREE, X_SPELL] });
      const without = grown({ hand: [FREE, X_SPELL], fungus: false });
      withFungus.endTurn();
      without.endTurn();
      expect(withFungus.hand("p1").every((card) => card.costMod === 0)).toBe(true);
      expect(withFungus.state.rngCursor).toBe(without.state.rngCursor);
    });
  });
});
