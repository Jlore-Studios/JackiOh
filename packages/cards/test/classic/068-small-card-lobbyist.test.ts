// C #68 Small Card Lobbyist — SPEC §8.6 row 68, BUILD M9 Classic row C 68: "Aura: every (3)+ Cost card
// either player could play (in a hand, or in a graveyard a permission lets its owner play from, R65)
// costs (1) more, the threshold read before this aura adds its 1, as R363 reads Curvature's, so a (2)
// Cost card is not lifted into range; X-cost cards are untouched (R65); casts pay nothing (R70); gone
// when it leaves; a hidden hand card's `costChanged` shows −1 to the other player (R177); radiant
// 22/26: the opponent can't play a (3)+ Cost card at all, checked last (R65) and absent from their
// `legalActions`, an X card for X of 3 or more included; casts still happen; your plays are free of
// it; its tuned numbers (surcharge, threshold) read through `param()` (R386)".
//
// A graveyard play needs a permission card (C #28, C #74, C #90), each another workstream's; the
// engine's cost-rules tests prove the graveyard half of `effectiveCost` through fixtures. A cast of a
// (3)+ Cost card is shown with a Cast-on-draw card a `costMod` has priced at (3).

import { legalActions, stepParam } from "@jackioh/engine";
import type { PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/068-small-card-lobbyist";
import { scenario, type Scenario } from "../_harness";

const LOBBYIST = "classic-068";
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const FIG = "core-047"; // (3) Spell: Heal a target 20.
const NETHER = "core-088"; // (4) Spell: Destroy all permanents.
const ARMOR = "core-073"; // (2) Field Spell
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const ECLIPSE = "core-035"; // (1) Spell: Deal 3 damage; your next Spell this turn costs (1) less.
const DIVIDEND = "core-024"; // (X) Spell
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw
const REMINISCE = "core-072"; // (1) Spell: Discover a card from your GY. It costs (1) less. Exile this.
const VANILLA = "core-008"; // (1) Unit 4/4
const FILLER = "core-010"; // (0) Spell

function handCost(s: Scenario, player: PlayerId, defId: string): number | undefined {
  const hand = s.view(player).you.hand;
  return Array.isArray(hand) ? hand.find((card) => card.defId === defId)?.cost : undefined;
}

function offered(s: Scenario, player: PlayerId, defId: string): boolean {
  const id = s.card(defId).id;
  return legalActions(s.state, player).some((action) => action.type === "play" && action.instanceId === id);
}

describe("C #68 Small Card Lobbyist", () => {
  it("is a (4) 11/13 Unit (22/26 Radiant) with no keywords, its two numbers declared", () => {
    expect(def.cost).toBe(4);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([11, 13, 22, 26]);
    expect(def.params).toEqual([
      { key: "surcharge", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
      { key: "threshold", base: 3, radiant: 3, better: "down", step: 1, min: 1 },
    ]);
    expect(base.costAura).toBeTypeOf("function");
    expect(radiant.costAura).toBeTypeOf("function");
  });

  describe("base", () => {
    it("is an 11/13 on the field", () => {
      const s = scenario({ p1: { hand: [LOBBYIST, FILLER] } });
      s.play(LOBBYIST).expectStats(LOBBYIST, { attack: 11, health: 13 });
    });

    it("every (3)+ Cost card in either player's hand costs (1) more, Spells and Units alike", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, NETHER], field: [LOBBYIST] }, p2: { hand: [FIG, MENACE] } });
      expect(handCost(s, "p1", HIT_JOB)).toBe(4);
      expect(handCost(s, "p1", NETHER)).toBe(5);
      expect(handCost(s, "p2", FIG)).toBe(4);
      expect(handCost(s, "p2", MENACE)).toBe(4);
    });

    it("a play pays the surcharge", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [LOBBYIST] }, p2: { field: [VANILLA] } });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] }).expectMana("p1", 0);
    });

    it("R363 a (2) Cost card is not lifted into range: the threshold reads the cost before its own (1)", () => {
      const s = scenario({ p1: { hand: [ARMOR, VANILLA], field: [LOBBYIST] } });
      expect(handCost(s, "p1", ARMOR)).toBe(2);
      expect(handCost(s, "p1", VANILLA)).toBe(1);
    });

    it("R363 R65 a (3) Cost card a discount has brought to (2) is out of range too", () => {
      const s = scenario({
        p1: { hand: [ECLIPSE, HIT_JOB, FILLER], field: [LOBBYIST], mana: 9 },
      });
      s.play(ECLIPSE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(handCost(s, "p1", HIT_JOB)).toBe(2);
    });

    it("R65 an X-cost card is untouched: X of 3 costs 3", () => {
      const s = scenario({ p1: { hand: [DIVIDEND, FILLER], field: [LOBBYIST] } });
      s.play(DIVIDEND, { x: 3, modes: ["damage"], targets: [{ pick: "hero", player: "p2" }] }).expectMana("p1", 1);
    });

    it("R70 a cast pays nothing: a Cast-on-draw card priced at (3) is cast for (0)", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [LOBBYIST] },
        p2: { hand: [FILLER], library: [{ def: CN_VIRUS, costMod: 2 }, VANILLA] },
      });
      s.endTurn();
      const cast = s.lastEvents.find((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS);
      expect(cast).toMatchObject({ player: "p2", costPaid: 0 });
    });

    it("gone when it leaves: destroyed, a (3) Cost card costs (3) again", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FIG], field: [LOBBYIST], mana: 9 } });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(LOBBYIST).id }] });
      s.expectInZone(LOBBYIST, "graveyard");
      expect(handCost(s, "p1", FIG)).toBe(3);
    });

    it("R177 a hidden hand card's `costChanged` shows −1 and the sentinel to the other player", () => {
      // Reminisce returns a (4) Spell that costs (1) less: (3), lifted to (4) by the aura.
      const s = scenario({ p1: { hand: [REMINISCE, FILLER], field: [LOBBYIST], graveyard: [NETHER] } });
      s.play(REMINISCE).answer(NETHER);
      const nether = s.card(NETHER);
      s.expectInZone(nether, "hand");
      expect(handCost(s, "p1", NETHER)).toBe(4);
      const own = s.view("p1").events.filter((event) => event.type === "costChanged" && event.instanceId === nether.id);
      expect(own).toEqual([{ type: "costChanged", instanceId: nether.id, cost: 4 }]);
      const theirs = s.view("p2").events.filter((event) => event.type === "costChanged");
      expect(theirs).toEqual([{ type: "costChanged", instanceId: "hidden", cost: -1 }]);
    });

    it("R386 its surcharge is declared: an Upgrade's step makes (3)+ Cost cards cost (2) more", () => {
      const s = scenario({ p1: { hand: [HIT_JOB], field: [LOBBYIST] } });
      stepParam(s.card(LOBBYIST), "surcharge", 1);
      expect(handCost(s, "p1", HIT_JOB)).toBe(5);
    });

    it("R386 its threshold is declared, and a Degrade moves it toward harder: (4)+, so a (3) card is spared", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, NETHER], field: [LOBBYIST] } });
      stepParam(s.card(LOBBYIST), "threshold", 1);
      expect(handCost(s, "p1", HIT_JOB)).toBe(3);
      expect(handCost(s, "p1", NETHER)).toBe(5);
    });
  });

  describe("radiant", () => {
    it("is a 22/26 on the field", () => {
      const s = scenario({ p1: { hand: [{ def: LOBBYIST, radiant: true }, FILLER] } });
      s.play(LOBBYIST).expectStats(LOBBYIST, { attack: 22, health: 26 });
    });

    it("R65 the opponent can't play a (3)+ Cost card: absent from their legalActions, and refused", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: LOBBYIST, radiant: true }] },
        p2: { hand: [HIT_JOB, FIG, ARMOR, FILLER], field: [VANILLA] },
        active: "p2",
      });
      expect(offered(s, "p2", HIT_JOB)).toBe(false);
      expect(offered(s, "p2", FIG)).toBe(false);
      expect(offered(s, "p2", ARMOR)).toBe(true);
      expect(() => s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] })).toThrow(
        /can't play \(3\)\+ Cost cards/,
      );
      // The price itself is not raised: a ban prices nothing.
      expect(handCost(s, "p2", HIT_JOB)).toBe(3);
    });

    it("R65 an X card is banned for an X of 3 or more: only X of 1 and 2 are offered", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: LOBBYIST, radiant: true }] },
        p2: { hand: [DIVIDEND, FILLER] },
        active: "p2",
      });
      const id = s.card(DIVIDEND).id;
      const xs = new Set(
        legalActions(s.state, "p2").flatMap((action) => (action.type === "play" && action.instanceId === id ? [action.x] : [])),
      );
      expect([...xs].sort()).toEqual([1, 2]);
      expect(() => s.play(DIVIDEND, { x: 3, modes: ["mana"] })).toThrow(/can't play/);
    });

    it("R65 checked last, on the finished price: a (4) Cost card discounted to (2) may be played", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: LOBBYIST, radiant: true }] },
        p2: { hand: [{ def: NETHER, costMod: -2 }, FILLER] },
        active: "p2",
      });
      expect(offered(s, "p2", NETHER)).toBe(true);
      s.play(NETHER);
      s.expectInZone(LOBBYIST, "graveyard");
    });

    it("your plays are free of it: its controller plays a (3) Cost card at (3)", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [{ def: LOBBYIST, radiant: true }] }, p2: { field: [VANILLA] } });
      expect(offered(s, "p1", HIT_JOB)).toBe(true);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] }).expectMana("p1", 1);
    });

    it("R70 casts still happen: the opponent's Cast-on-draw card priced at (3) is cast", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: LOBBYIST, radiant: true }] },
        p2: { hand: [FILLER], library: [{ def: CN_VIRUS, costMod: 2 }, VANILLA] },
      });
      s.endTurn();
      expect(s.lastEvents.find((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS)).toMatchObject({ player: "p2" });
    });

    it("R386 its declared threshold moves toward harder on a Degrade: (4)+, so the opponent may play a (3)", () => {
      const sp = scenario({
        p1: { hand: [FILLER], field: [{ def: LOBBYIST, radiant: true }, VANILLA] },
        p2: { hand: [HIT_JOB, NETHER, FILLER] },
        active: "p2",
      });
      expect(offered(sp, "p2", HIT_JOB)).toBe(false);
      stepParam(sp.card(LOBBYIST), "threshold", 1);
      expect(offered(sp, "p2", HIT_JOB)).toBe(true);
      expect(offered(sp, "p2", NETHER)).toBe(false);
    });
  });
});
