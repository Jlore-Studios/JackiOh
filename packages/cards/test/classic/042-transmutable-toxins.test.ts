// C #42 Transmutable Toxins — SPEC §8.6 row 42, BUILD M9 Classic row C 42: "Aura (§10.4 layer 5):
// your Units have +1/+1 for each Plague Token on them and enemy Units −1/−1, recomputed on every
// change, so a token placed later applies at once; the −1/−1 lowers max health and an enemy can die of
// it at the state check, an Indestructible one too once its max health reaches 0 (R69); gone when it
// leaves; Activate, once per turn (R384): one token on each of two different random Units on the field,
// either side (R60: one Unit → one token; none → nothing); C #27 doubles its share; not a play;
// radiant: +2/+2 and −2/−2; its tuned numbers (tokens, stats per token) read through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/042-transmutable-toxins";

const TOXINS = "classic-042";
const SLIME = "classic-027"; // (0) Unit 1/1: "Plague Tokens placed on this are multiplied by {multiplier}." (2)
const FAUCI = "core-091"; // (2) Unit 1/6: "Whenever this takes damage, it gets a Plague Token."
const ECLIPSE = "core-035"; // (1) Spell: "Deal 3 damage to a target."
const COLLATERAL = "core-034"; // (4) Spell: "Exile target permanent and a random card from your opponent's deck."
const STATE_OF_GAME = "classic-041"; // (1) Unit 3/3 Indestructible
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const TIMMY = "core-011"; // (1) Unit 3/3
const STOCKPILE = "core-005"; // (1) Spell, a spare card (§2.5)

function tokensOn(s: Scenario, player: "p1" | "p2", lane: number): number {
  return s.unit(player, lane)?.counters.plague ?? 0;
}

function withToxins(
  radiantFace: boolean,
  mine: readonly (string | { def: string; counters?: { plague?: number } })[],
  theirs: readonly (string | { def: string; counters?: { plague?: number } })[],
): Scenario {
  return scenario({
    p1: { hand: [ECLIPSE, STOCKPILE], backrow: [{ def: TOXINS, radiant: radiantFace }], field: [...mine], library: [STOCKPILE] },
    p2: { hand: [COLLATERAL, STOCKPILE], field: [...theirs], library: [STOCKPILE, STOCKPILE] },
  });
}

describe("C #42 Transmutable Toxins", () => {
  it("declares its two numbers (R386): tokens 2, stats per token 1 (Radiant 2), and one Activate", () => {
    expect(def.params).toEqual([
      { key: "tokens", base: 2, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "stats", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    expect(base.activations?.map((ability) => ability.uses)).toEqual([1]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("Aura: your Units have +1/+1 for each Plague Token on them", () => {
      const s = withToxins(false, [{ def: VANILLA, counters: { plague: 2 } }, TIMMY], []);
      s.expectStats(VANILLA, { attack: 6, health: 6, maxHealth: 6 });
      s.expectStats(TIMMY, { attack: 3, health: 3, maxHealth: 3 });
    });

    it("Aura: enemy Units have −1/−1 for each Plague Token on them", () => {
      const s = withToxins(false, [], [{ def: MENACE, counters: { plague: 3 } }]);
      s.expectStats(MENACE, { attack: 6, health: 6, maxHealth: 6 });
    });

    it("recomputed on every change: a token placed later applies at once", () => {
      // A Fed Fauci (1/6) hit for 3 gets a token: the enemy's is −1/−1 at once (max health 5, 2 left),
      // yours +1/+1 (max health 7, 4 left).
      const s = scenario({
        p1: { hand: [ECLIPSE, ECLIPSE, STOCKPILE], backrow: [TOXINS], field: [FAUCI] },
        p2: { hand: [STOCKPILE], field: [FAUCI] },
      });
      const theirs = s.unit("p2", 1);
      const mine = s.unit("p1", 1);
      if (theirs === null || mine === null) throw new Error("setup");
      s.play(s.hand("p1")[0] ?? ECLIPSE, { targets: [{ pick: "instance", instanceId: theirs.id }] });
      expect(tokensOn(s, "p2", 1)).toBe(1);
      s.expectStats(theirs, { attack: 0, maxHealth: 5, health: 2 });
      s.play(s.hand("p1")[0] ?? ECLIPSE, { targets: [{ pick: "instance", instanceId: mine.id }] });
      expect(tokensOn(s, "p1", 1)).toBe(1);
      s.expectStats(mine, { attack: 2, maxHealth: 7, health: 4 });
    });

    it("§4.5 the −1/−1 lowers max health: an enemy it brings to 0 dies at the state check", () => {
      const s = scenario({
        p1: { hand: [TOXINS, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: TIMMY, counters: { plague: 3 } }, { def: VANILLA, counters: { plague: 1 } }] },
      });
      const timmy = s.card(TIMMY);
      s.play(TOXINS, { zone: 1 });
      s.expectInZone(timmy, "graveyard");
      s.expectStats(VANILLA, { attack: 3, maxHealth: 3 });
    });

    it("R69 an Indestructible enemy dies too once its max health reaches 0", () => {
      const s = scenario({
        p1: { hand: [TOXINS, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: STATE_OF_GAME, counters: { plague: 3 } }] },
      });
      const state = s.card(STATE_OF_GAME);
      s.play(TOXINS, { zone: 1 });
      s.expectInZone(state, "graveyard");
    });

    it("gone when it leaves: the stats return", () => {
      const s = withToxins(false, [{ def: VANILLA, counters: { plague: 2 } }], []);
      s.endTurn();
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(TOXINS).id }] });
      s.expectInZone(TOXINS, "exile");
      s.expectStats(VANILLA, { attack: 4, health: 4, maxHealth: 4 });
    });

    it("a unit with no token is untouched", () => {
      const s = withToxins(false, [VANILLA], [MENACE]);
      s.expectStats(VANILLA, { attack: 4, maxHealth: 4 });
      s.expectStats(MENACE, { attack: 9, maxHealth: 9 });
    });

    it("R386 an Upgrade of stats per token makes it +2/+2 and −2/−2", () => {
      const s = withToxins(false, [{ def: VANILLA, counters: { plague: 1 } }], [{ def: MENACE, counters: { plague: 1 } }]);
      stepParam(s.card(TOXINS), "stats", 1);
      s.expectStats(VANILLA, { attack: 6, maxHealth: 6 });
      s.expectStats(MENACE, { attack: 7, maxHealth: 7 });
    });

    it("R384 Activate: one token on each of two different random Units, either side", () => {
      const s = withToxins(false, [VANILLA, TIMMY], [MENACE]);
      s.activate(TOXINS);
      const tokens = [tokensOn(s, "p1", 1), tokensOn(s, "p1", 2), tokensOn(s, "p2", 1)];
      expect(tokens.filter((count) => count === 1)).toHaveLength(2);
      expect(tokens.filter((count) => count === 0)).toHaveLength(1);
    });

    it("R384 once per turn: a second Activate is refused", () => {
      const s = withToxins(false, [VANILLA, TIMMY], [MENACE]);
      s.activate(TOXINS);
      expect(() => s.activate(TOXINS)).toThrow();
    });

    it("R60 with one Unit on the field, it gets one token; with none, nothing", () => {
      const one = withToxins(false, [], [MENACE]);
      one.activate(TOXINS);
      expect(tokensOn(one, "p2", 1)).toBe(1);
      const none = withToxins(false, [], []);
      none.activate(TOXINS);
      expect(none.events.some((event) => event.type === "counterChanged")).toBe(false);
    });

    it("E19 a C #27 Pestilent Slime doubles its share", () => {
      const s = withToxins(false, [SLIME], [MENACE]);
      s.activate(TOXINS);
      expect(tokensOn(s, "p1", 1)).toBe(2);
      expect(tokensOn(s, "p2", 1)).toBe(1);
    });

    it("an Activate is not a play: nothing that answers plays sees it", () => {
      const s = withToxins(false, [VANILLA], [MENACE]);
      s.activate(TOXINS);
      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
    });

    it("R386 an Upgrade of tokens places on 3 different Units", () => {
      const s = withToxins(false, [VANILLA, TIMMY], [MENACE]);
      stepParam(s.card(TOXINS), "tokens", 1);
      s.activate(TOXINS);
      expect([tokensOn(s, "p1", 1), tokensOn(s, "p1", 2), tokensOn(s, "p2", 1)]).toEqual([1, 1, 1]);
    });
  });

  describe("radiant", () => {
    it("+2/+2 for each token on your Units and −2/−2 on the enemy's", () => {
      const s = withToxins(true, [{ def: VANILLA, counters: { plague: 2 } }], [{ def: MENACE, counters: { plague: 2 } }]);
      s.expectStats(VANILLA, { attack: 8, health: 8, maxHealth: 8 });
      s.expectStats(MENACE, { attack: 5, health: 5, maxHealth: 5 });
    });

    it("an enemy it brings to 0 max health dies", () => {
      const s = scenario({
        p1: { hand: [{ def: TOXINS, radiant: true }, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: VANILLA, counters: { plague: 2 } }] },
      });
      const vanilla = s.card(VANILLA);
      s.play(TOXINS, { zone: 1 });
      s.expectInZone(vanilla, "graveyard");
    });

    it("R384 Activate: the same two random Units, one token each", () => {
      const s = withToxins(true, [VANILLA], [MENACE]);
      s.activate(TOXINS);
      expect([tokensOn(s, "p1", 1), tokensOn(s, "p2", 1)]).toEqual([1, 1]);
      s.expectStats(VANILLA, { attack: 6, maxHealth: 6 });
      s.expectStats(MENACE, { attack: 7, maxHealth: 7 });
    });
  });
});
