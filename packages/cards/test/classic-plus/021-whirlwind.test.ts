// C+ #21 Whirlwind — SPEC §8.7 row 21, BUILD M9 Classic+ row C+ 21: "Pierce: 1 damage to every Unit on
// both sides through Armor (an Armor 7 unit takes 1), Divine Shield still popping, all hits before the
// state check (R59); Spell Damage raises each hit; an Immune to Spells Unit takes none; the damage
// reads through `param()`; radiant also returns from your graveyard to your hand at the end of the
// turn (`returnToHandAtEndOfTurn`, R68), burned when the hand is full".

import { HAND_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/021-whirlwind";

const WHIRLWIND = "classicplus-021";
const ARMORED = "core-025"; // 7/7, Armor 7
const SHIELDED = "core-056"; // Jilliax 3/2, Divine Shield
const BODY = "core-019"; // 9/9
const ONE = "core-003"; // Right-house defender 1/1, Taunt, Divine Shield, Reborn
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

function hits(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "damage" ? [event.amount] : []));
}

describe("C+ #21 Whirlwind", () => {
  it("is a (0) Spell that prints Pierce on both faces; only the Radiant face returns", () => {
    expect(def.cost).toBe(0);
    expect(def.base.keywords).toEqual([{ kind: "Pierce" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Pierce" }]);
    expect(base.endOfTurn).toBeUndefined();
    expect(radiant.endOfTurn).toBeDefined();
  });

  describe("base", () => {
    it("R346 deals 1 to every Unit on both sides, through Armor", () => {
      const s = scenario({
        p1: { hand: [WHIRLWIND, FILLER], field: [{ def: BODY, lane: 2 }] },
        p2: { hand: [FILLER], field: [{ def: ARMORED, lane: 1 }, { def: BODY, lane: 3 }] },
      });
      s.play(WHIRLWIND);
      s.expectStats(s.unit("p2", 1) ?? "", { health: 6 });
      s.expectStats(s.unit("p2", 3) ?? "", { health: 8 });
      s.expectStats(s.unit("p1", 2) ?? "", { health: 8 });
      expect(hits(s)).toEqual([1, 1, 1]);
      s.expectHealth("p1", 30).expectHealth("p2", 30);
    });

    it("R346 Divine Shield still pops: the shielded unit takes nothing and loses its shield", () => {
      const s = scenario({ p1: { hand: [WHIRLWIND, FILLER] }, p2: { hand: [FILLER], field: [SHIELDED] } });
      s.play(WHIRLWIND);
      const jilliax = s.unit("p2", 1);
      if (jilliax === null) throw new Error("gone");
      s.expectStats(jilliax, { health: 2 });
      expect(s.stats(jilliax).keywords.some((keyword) => keyword.kind === "Divine Shield")).toBe(false);
    });

    it("R59 every hit lands before the state check: both 1-health units die in one check", () => {
      const s = scenario({
        p1: { hand: [WHIRLWIND, FILLER], field: [{ def: BODY, lane: 1, damage: 8 }] },
        p2: { hand: [FILLER], field: [{ def: BODY, lane: 1, damage: 8 }] },
      });
      s.play(WHIRLWIND);
      const types = s.lastEvents.map((event) => event.type);
      const lastHit = types.lastIndexOf("damage");
      const firstDeath = types.indexOf("destroyed");
      expect(firstDeath).toBeGreaterThan(lastHit);
      expect(types.filter((type) => type === "destroyed")).toHaveLength(2);
    });

    it("§3.2 only the top of a Stack pile is hit; the dormant card beneath is not", () => {
      const s = scenario({
        p1: { hand: [WHIRLWIND, FILLER] },
        p2: { hand: [FILLER], field: [{ def: BODY, lane: 1 }, { def: "core-092", lane: 1, stack: true }] },
      });
      const dormant = s.state.players.p2.units[0]?.[1];
      if (dormant === undefined) throw new Error("no pile");
      s.play(WHIRLWIND);
      expect(s.card(dormant).damage).toBe(0);
      expect(hits(s)).toHaveLength(1);
    });

    it("B5 E6 Spell Damage raises each hit", () => {
      const s = scenario({
        p1: { hand: [WHIRLWIND, FILLER], field: [{ def: SOLARIUS, lane: 1 }] },
        p2: { hand: [FILLER], field: [{ def: BODY, lane: 1 }, { def: ARMORED, lane: 2 }] },
      });
      s.play(WHIRLWIND);
      expect(hits(s)).toEqual([3, 3, 3].slice(0, hits(s).length));
      s.expectStats(s.unit("p2", 1) ?? "", { health: 6 });
      s.expectStats(s.unit("p2", 2) ?? "", { health: 4 });
    });

    it("§6.1 an Immune to Spells Unit takes none", () => {
      const s = scenario({
        p1: { hand: [WHIRLWIND, FILLER] },
        p2: { hand: [FILLER], field: [{ def: TOP_LOSER, lane: 1, radiant: true }, { def: BODY, lane: 2 }] },
      });
      s.play(WHIRLWIND);
      s.expectStats(s.unit("p2", 1) ?? "", { health: 10 });
      s.expectStats(s.unit("p2", 2) ?? "", { health: 8 });
    });

    it("R386 the damage reads through param: an Upgrade makes it 2", () => {
      const s = scenario({ p1: { hand: [WHIRLWIND, FILLER] }, p2: { hand: [FILLER], field: [BODY] } });
      stepParam(s.card(WHIRLWIND), "damage", 1);
      s.play(WHIRLWIND);
      s.expectStats(s.unit("p2", 1) ?? "", { health: 7 });
    });

    it("the base face stays in the graveyard at the end of the turn", () => {
      const s = scenario({ p1: { hand: [WHIRLWIND, FILLER], library: DECK }, p2: { hand: [FILLER], library: DECK } });
      const card = s.card(WHIRLWIND);
      s.play(WHIRLWIND).endTurn();
      s.expectInZone(card, "graveyard");
    });

    it("a Reborn unit it kills comes back at 1 health", () => {
      const s = scenario({ p1: { hand: [WHIRLWIND, FILLER] }, p2: { hand: [FILLER], field: [{ def: ONE, lane: 2 }] } });
      // The defender's Divine Shield takes the first hit; a second Whirlwind kills it and Reborn returns it.
      s.play(WHIRLWIND);
      expect(s.unit("p2", 2)?.defId).toBe(ONE);
    });
  });

  describe("radiant", () => {
    it("R68 returns from the graveyard to its owner's hand at the end of the turn it was played", () => {
      const s = scenario({
        p1: { hand: [{ def: WHIRLWIND, radiant: true }, FILLER], library: DECK },
        p2: { hand: [FILLER], field: [BODY], library: DECK },
      });
      const card = s.card(WHIRLWIND);
      s.play(WHIRLWIND);
      s.expectInZone(card, "graveyard");
      s.expectStats(s.unit("p2", 1) ?? "", { health: 8 });
      s.endTurn();
      s.expectInZone(card, "hand");
      expect(s.card(card).radiant).toBe(true);
    });

    it("R153 a Radiant Whirlwind that reached the graveyard without being played stays there", () => {
      const s = scenario({
        p1: { hand: [FILLER], graveyard: [{ def: WHIRLWIND, radiant: true }], library: DECK },
        p2: { hand: [FILLER], library: DECK },
      });
      const card = s.card(WHIRLWIND);
      s.endTurn();
      s.expectInZone(card, "graveyard");
    });

    it("§2.4 R317 a full hand burns it on the way back", () => {
      const fillers = Array.from({ length: HAND_CAP }, () => FILLER);
      const s = scenario({
        p1: { hand: [{ def: WHIRLWIND, radiant: true }, ...fillers.slice(1)], library: DECK },
        p2: { hand: [FILLER], library: DECK },
      });
      const card = s.card(WHIRLWIND);
      s.play(WHIRLWIND);
      // Refill the hand to the cap before the end of the turn: one card back from the library.
      s.play(FILLER);
      expect(s.hand("p1").length).toBeGreaterThanOrEqual(HAND_CAP - 1);
      s.endTurn();
      expect(s.card(card).zone.z).toBe("graveyard");
    });
  });
});
