// C+ #50 Adaptive Growth — SPEC §8.7 row 50, BUILD M9 Classic+ row C+ 50: "Cast on draw (R70, R58): if
// you control fewer Units than your opponent, every Unit on both sides gets −2/−2 (max health falls,
// units at 0 die at the state check, an Indestructible one too, R69), otherwise every Unit gets +2/+2,
// all permanent; equal counts take the second branch; played from a hand it does the same;
// `conditionMet` in hand answers whether you control fewer Units now (R195); both numbers read through
// `param()`; radiant fewer: enemy Units −3/−3; otherwise your Units +3/+3".
// The `conditionMet` proofs live in packages/cards/test/condition-active.test.ts (README §5).

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type FieldSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/050-adaptive-growth";

const GROWTH = "classicplus-050";
const VANILLA = "core-008"; // Mr. Vanilla 4/4
const ROCK = "core-066"; // The Rock, Indestructible
const FIENDER = "core-092"; // Felinor Fiender, Stack
const FILLER = "core-005";

/** Adaptive Growth on top of p1's library, p2 about to end their turn so p1 draws it. */
function drawn(opts: { radiant?: boolean; mine: readonly FieldSetup[]; theirs: readonly FieldSetup[] }): Scenario {
  return scenario({
    active: "p2",
    turn: 10,
    p1: { field: opts.mine, hand: [FILLER], library: [{ def: GROWTH, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER, FILLER] },
    p2: { field: opts.theirs, hand: [FILLER], library: [FILLER] },
  });
}

/** Adaptive Growth in p1's hand, to play. */
function held(opts: { radiant?: boolean; mine: readonly FieldSetup[]; theirs: readonly FieldSetup[] }): Scenario {
  return scenario({
    p1: { field: opts.mine, hand: [{ def: GROWTH, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER] },
    p2: { field: opts.theirs, hand: [FILLER] },
  });
}

describe("C+ #50 Adaptive Growth", () => {
  it("is a (1) Spell that casts itself on draw, on both faces", () => {
    expect(def.type).toBe("Spell");
    expect(base.staticFlags?.castOnDraw).toBe(true);
    expect(radiant.staticFlags?.castOnDraw).toBe(true);
  });

  describe("base", () => {
    it("R58 R70 drawn, it is cast at once, and the draw goes on to the next card", () => {
      const s = drawn({ mine: [VANILLA], theirs: [VANILLA] });
      s.endTurn();
      s.expectInZone(GROWTH, "graveyard");
      expect(s.events.some((event) => event.type === "cardPlayed" && event.instanceId === s.card(GROWTH).id)).toBe(true);
      expect(s.hand("p1").filter((card) => card.defId === FILLER)).toHaveLength(2);
    });

    it("§10.4 fewer Units than the opponent: every Unit on both sides gets −2/−2, max health too", () => {
      const s = drawn({ mine: [VANILLA], theirs: [VANILLA, VANILLA] });
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 2, health: 2, maxHealth: 2 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 2, health: 2, maxHealth: 2 });
      s.expectStats(s.unit("p2", 2) ?? "", { attack: 2, health: 2, maxHealth: 2 });
    });

    it("§4.5 a Unit brought to 0 max health dies at the state check", () => {
      const s = drawn({ mine: [], theirs: [{ def: VANILLA, statsOverride: { attack: 2, health: 2 } }] });
      const token = s.unit("p2", 1);
      s.endTurn();
      s.expectInZone(token ?? "", "graveyard");
      expect(s.events.some((event) => event.type === "destroyed" && event.instanceId === token?.id)).toBe(true);
    });

    it("R69 an Indestructible Unit at 0 max health dies too", () => {
      const s = drawn({ mine: [], theirs: [{ def: ROCK, statsOverride: { attack: 10, health: 2 } }] });
      const rock = s.unit("p2", 1);
      s.endTurn();
      s.expectInZone(rock ?? "", "graveyard");
    });

    it("otherwise every Unit on both sides gets +2/+2, permanently", () => {
      const s = drawn({ mine: [VANILLA, VANILLA], theirs: [VANILLA] });
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 6, health: 6 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 6, health: 6 });
      s.endTurn().endTurn();
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 6, health: 6 });
    });

    it("equal counts take the second branch (+2/+2), an empty board included", () => {
      const s = drawn({ mine: [VANILLA], theirs: [VANILLA] });
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 6, health: 6 });

      const empty = drawn({ mine: [], theirs: [] });
      empty.endTurn();
      expect(empty.lastEvents.some((event) => event.type === "buffed")).toBe(false);
      empty.expectInZone(GROWTH, "graveyard");
    });

    it("§3.2 the counts read the tops of the piles: a Stack pile is one Unit", () => {
      const s = drawn({ mine: [VANILLA, { def: FIENDER, stack: true }], theirs: [VANILLA, VANILLA] });
      const buried = s.state.players.p1.units[0]?.find((card) => card.defId === VANILLA);
      if (buried === undefined) throw new Error("no pile");
      s.endTurn();
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 2, health: 2 });
      // The dormant card is not on the field for effects (R13): it keeps its 4/4.
      expect(s.card(buried.id).buffs).toEqual({ attack: 0, health: 0 });
    });

    it("§6.2 played from a hand it does the same", () => {
      const fewer = held({ mine: [VANILLA], theirs: [VANILLA, VANILLA] });
      fewer.play(GROWTH);
      fewer.expectStats(fewer.unit("p1", 1) ?? "", { attack: 2, health: 2 });

      const more = held({ mine: [VANILLA], theirs: [] });
      more.play(GROWTH);
      more.expectStats(more.unit("p1", 1) ?? "", { attack: 6, health: 6 });
    });

    it("R386 an Upgrade moves both numbers: +3/+3 and −3/−3", () => {
      const up = held({ mine: [VANILLA], theirs: [] });
      stepParam(up.card(GROWTH), "buff", 1);
      up.play(GROWTH);
      up.expectStats(up.unit("p1", 1) ?? "", { attack: 7, health: 7 });

      const down = held({ mine: [VANILLA], theirs: [VANILLA, VANILLA] });
      stepParam(down.card(GROWTH), "debuff", 1);
      down.play(GROWTH);
      down.expectStats(down.unit("p1", 1) ?? "", { attack: 1, health: 1 });
    });
  });

  describe("radiant", () => {
    it("fewer Units: only enemy Units get −3/−3", () => {
      const s = drawn({ radiant: true, mine: [VANILLA], theirs: [VANILLA, { def: VANILLA, damage: 1 }] });
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 4, health: 4 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 1, health: 1 });
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("otherwise only your Units get +3/+3", () => {
      const s = drawn({ radiant: true, mine: [VANILLA], theirs: [VANILLA] });
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 7, health: 7 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 4, health: 4 });
    });

    it("R386 the Radiant debuff and buff read through `param()`", () => {
      const fewer = held({ radiant: true, mine: [VANILLA], theirs: [{ def: VANILLA, statsOverride: { attack: 9, health: 9 } }, VANILLA] });
      stepParam(fewer.card(GROWTH), "debuff", 1);
      fewer.play(GROWTH);
      fewer.expectStats(fewer.unit("p2", 1) ?? "", { attack: 5, health: 5 });

      const more = held({ radiant: true, mine: [VANILLA], theirs: [] });
      stepParam(more.card(GROWTH), "buff", -1);
      more.play(GROWTH);
      more.expectStats(more.unit("p1", 1) ?? "", { attack: 6, health: 6 });
    });
  });
});
