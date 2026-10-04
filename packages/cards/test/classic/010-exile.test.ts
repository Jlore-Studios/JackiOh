// C #10 Exile — SPEC §8.6 row 10, BUILD M9 Classic row C 10: "Face-down (R33); fires on the opponent's
// play of a card whose cost paid is (2) or less (R56), so a free cast always qualifies (R70), in the
// announce window before the card moves (§10.5); counters it: it never resolves or enters the field,
// no Cry, not counted as played by the turn's or the game's counts, Combo, Quickstriker or Ceaseless
// Void, its mana and Tributes stay spent, and it goes to exile, not the graveyard; a (3)+ Cost play
// and your own plays leave it set; a face-down set is announced to the opponent by its zone and cost
// only (§10.5 step 3a), so their `cardAnnounced` names no card and the exile then shows the card;
// radiant: (3) or less, then exile random enemy permanents one at a time, each with a cost (R65 on the
// field; an X card its X, 0 with none chosen, R396) no more than the budget left, budget = 3 minus the
// countered card's cost paid, until the budget is 0 or nothing fits, a (0) Cost permanent always
// fitting while the budget is above 0; its name is a rules word, so "exile" in other texts is no
// reference to it (R381); its tuned number (threshold) reads through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/010-exile";
import { CATALOG } from "../../src/catalog-data";

const EXILE = "classic-010";
const TIMMY = "core-011"; // (1) Unit 3/3.
const TOKEN_MAN = "core-015"; // (1) Unit, "Cry: Summon a Rush Token."
const POINTMASTER = "core-020"; // (2) Unit 7/1.
const MENACE = "core-019"; // (3) Unit 9/9.
const VANILLA = "core-008"; // (1) Unit 4/4.
const STOCKPILE = "core-005"; // (1) Spell.
const BEAR = "core-060"; // (1) Trap.
const HINDER = "core-021"; // (0) Spell, cast on draw.
const SLIME = "classic-027"; // (0) Unit 1/1.
const HEROIC = "core-098"; // (X) Field Spell.
const ROCK = "core-066"; // (4) Unit 10/10, Tribute 1, Indestructible.

function armed(radiantFace = false, lane = 2): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: EXILE, radiant: radiantFace, faceUp: false, lane };
}

function setup(p2: SideSetup = {}, radiantFace = false, p1: SideSetup = {}): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [VANILLA], backrow: [armed(radiantFace)], library: [VANILLA, VANILLA], ...p1 },
    p2: { hand: [TIMMY, POINTMASTER, STOCKPILE], library: [VANILLA, VANILLA, VANILLA], ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #10 Exile", () => {
  it("is a Trap on both faces whose condition lives in `when` (R99)", () => {
    expect(def.type).toBe("Trap");
    expect(base.triggers?.[0]?.when).toBeTypeOf("function");
    expect(radiant.triggers?.[0]?.when).toBeTypeOf("function");
  });

  it("R381 its name is a rules word: no other card's text refers to it unless its refs list it", () => {
    const referring = Object.values(CATALOG).filter((card) => (card.refs ?? []).includes(EXILE));
    expect(referring).toEqual([]);
  });

  describe("base", () => {
    it("R33 it is face-down, and the opponent's view never names it", () => {
      const s = setup();
      expect(s.card(EXILE).faceUp).toBe(false);
      expect(JSON.stringify(s.view("p2"))).not.toContain(EXILE);
    });

    it("R448 counters the opponent's (1) Cost Unit before it enters the field, and exiles it", () => {
      const s = setup();
      const timmy = s.card(TIMMY);

      s.play(timmy, { zone: 1 });

      s.expectInZone(timmy, "exile");
      expect(s.unit("p2", 1)).toBeNull();
      expect(count(s.lastEvents, "cardPlayed")).toBe(0);
      expect(count(s.lastEvents, "summoned")).toBe(0);
      s.expectEvents("cardAnnounced", "trapFired", "countered", "exiled");
      expect(s.pile("p2", "graveyard").map((card) => card.id)).not.toContain(timmy.id);
    });

    it("R448 no Cry, no count, and the mana stays spent", () => {
      const s = setup({ hand: [TOKEN_MAN, VANILLA] });
      const played = s.state.players.p2.turnLog.cardsPlayed;
      const game = s.state.counters.played;

      s.play(TOKEN_MAN, { zone: 1 });

      expect(s.unit("p2", 2)).toBeNull();
      expect(count(s.lastEvents, "summoned")).toBe(0);
      expect(s.state.players.p2.turnLog.cardsPlayed).toBe(played);
      expect(s.state.counters.played).toBe(game);
      s.expectMana("p2", 3);
    });

    it("R56 a Spell costing (1) is countered and exiled too", () => {
      const s = setup();
      const spell = s.card(STOCKPILE);

      s.play(spell);

      s.expectInZone(spell, "exile");
      expect(count(s.lastEvents, "drawn")).toBe(0);
    });

    it("R70 a free cast always qualifies: a cast-on-draw card is countered and exiled", () => {
      const s = scenario({
        p1: { hand: [VANILLA], backrow: [armed()], library: [VANILLA, VANILLA] },
        p2: { hand: [VANILLA], library: [{ def: HINDER, radiant: true }, VANILLA, VANILLA] },
      });
      const hinder = s.card(HINDER);

      s.endTurn();

      s.expectInZone(hinder, "exile");
      expect(s.state.players.p1.mana.nextTurnMod).toBe(0);
    });

    it("R56 a (2) Cost play is countered and exiled too", () => {
      const s = setup();

      s.play(POINTMASTER, { zone: 1 });

      s.expectInZone(POINTMASTER, "exile");
      s.expectEvents("cardAnnounced", "trapFired", "countered", "exiled");
    });

    it("R56 a (3) Cost play leaves it set", () => {
      const s = setup({ hand: [MENACE, VANILLA] });

      s.play(MENACE, { zone: 1 });

      s.expectInZone(MENACE, "field");
      expect(count(s.events, "countered")).toBe(0);
      expect(s.backrow("p1", 2)?.faceUp).toBe(false);
    });

    it("its controller's own plays leave it set", () => {
      const s = scenario({
        p1: { hand: [TIMMY, VANILLA], backrow: [armed()] },
        p2: { hand: [VANILLA] },
      });

      s.play(TIMMY, { zone: 1 });

      s.expectInZone(TIMMY, "field");
      expect(s.backrow("p1", 2)?.faceUp).toBe(false);
    });

    it("§5.1 the Trap is consumed when it fires", () => {
      const s = setup();
      const trap = s.card(EXILE);

      s.play(TIMMY, { zone: 1 });

      s.expectInZone(trap, "graveyard");
    });

    it("R227 a face-down set is announced to the other player by zone and cost only; the exile then shows it", () => {
      const s = setup({ hand: [BEAR, VANILLA] });
      const bear = s.card(BEAR);

      s.play(bear, { zone: 3 });

      const seen = s.view("p1").events ?? [];
      const announced = seen.find((event) => event.type === "cardAnnounced");
      expect(announced).toBeDefined();
      expect(JSON.stringify(announced)).not.toContain(BEAR);
      const exiled = seen.find((event) => event.type === "exiled");
      expect(exiled).toMatchObject({ defId: BEAR });
      expect(s.pile("p2", "exile").map((card) => card.defId)).toEqual([BEAR]);
    });

    it("R386 an Upgrade of its threshold reaches a (3) Cost play", () => {
      const s = setup({ hand: [MENACE, VANILLA] });
      stepParam(s.card(EXILE), "threshold", 1);

      s.play(MENACE, { zone: 1 });

      s.expectInZone(MENACE, "exile");
    });
  });

  describe("radiant", () => {
    it("counters and exiles a (3) Cost play", () => {
      const s = setup({ hand: [MENACE, VANILLA] }, true);

      s.play(MENACE, { zone: 1 });

      s.expectInZone(MENACE, "exile");
    });

    it("a (4) Cost play leaves it set", () => {
      const s = setup({ hand: ["core-043", VANILLA] }, true);

      s.play("core-043", { zone: 1 });

      expect(count(s.events, "countered")).toBe(0);
    });

    it("then exiles enemy permanents within the budget, 3 minus the cost paid", () => {
      // A (1) Cost play leaves a budget of 2: the (2) Cost Pointmaster fits, the (3) Cost Menace never.
      const s = setup({ field: [POINTMASTER, MENACE] }, true);
      const point = s.unit("p2", 1);
      const menace = s.unit("p2", 2);
      if (point === null || menace === null) throw new Error("fixture");

      s.play(TIMMY, { zone: 3 });

      s.expectInZone(point, "exile");
      s.expectInZone(menace, "field");
    });

    it("never exceeds the budget: the exiled permanents' costs total at most the budget", () => {
      const s = setup({ field: [VANILLA, VANILLA, POINTMASTER, MENACE] }, true);

      s.play(TIMMY, { zone: 5 });

      const exiled = s.pile("p2", "exile").filter((card) => card.defId !== TIMMY);
      const total = exiled.reduce((sum, card) => sum + (card.defId === POINTMASTER ? 2 : 1), 0);
      expect(total).toBeGreaterThan(0);
      expect(total).toBeLessThanOrEqual(2);
      s.expectInZone(MENACE, "field");
    });

    it("a (0) Cost permanent always fits while the budget is above 0", () => {
      // A (2) Cost play leaves 1; the Slime costs 0 and fits, and then nothing else does.
      const s = setup({ field: [SLIME, MENACE] }, true);
      const slime = s.card(SLIME);

      s.play(POINTMASTER, { zone: 3 });

      s.expectInZone(slime, "exile");
      s.expectInZone(MENACE, "field");
    });

    it("a (3) Cost play leaves no budget: only the countered card goes", () => {
      const s = setup({ hand: [MENACE, VANILLA], field: [SLIME] }, true);

      s.play(MENACE, { zone: 2 });

      s.expectInZone(SLIME, "field");
    });

    it("reaches the enemy backrow, a face-down trap included, and exile shows it", () => {
      const s = setup({ backrow: [{ def: BEAR, faceUp: false, lane: 1 }], field: [MENACE] }, true);
      const bear = s.card(BEAR);

      s.play(TIMMY, { zone: 2 });

      s.expectInZone(bear, "exile");
      expect(JSON.stringify(s.view("p1").opponent.exile)).toContain(BEAR);
    });

    it("R13 a card dormant under a Stack pile is not on the field: only the top of the pile is weighed", () => {
      // Budget 2: the dormant (1) Cost Vanilla would fit, the (3) Cost Menace on top of it never does.
      const s = setup({ field: [{ def: VANILLA, lane: 1 }, { def: MENACE, stack: true }] }, true);
      const dormant = s.card(VANILLA);

      s.play(TIMMY, { zone: 2 });

      s.expectInZone(TIMMY, "exile");
      s.expectInZone(dormant, "field");
      expect(s.unit("p2", 1)?.defId).toBe(MENACE);
      expect(s.pile("p2", "exile").map((card) => card.defId)).toEqual([TIMMY]);
    });

    it("never exiles its controller's own permanents", () => {
      const s = setup({ field: [MENACE] }, true, { field: [SLIME, VANILLA] });
      const mine = [s.unit("p1", 1), s.unit("p1", 2)];

      s.play(TIMMY, { zone: 2 });

      for (const card of mine) {
        if (card === null) throw new Error("fixture");
        s.expectInZone(card, "field");
      }
    });

    it("R396 an X card on the field costs the X it was played for, and 0 with none chosen", () => {
      const s = setup({ backrow: [{ def: HEROIC, faceUp: true, lane: 1 }], field: [MENACE] }, true);
      const heroic = s.card(HEROIC);
      heroic.x = 3;

      // Budget 2: a Heroic Power played for 3 does not fit.
      s.play(TIMMY, { zone: 2 });
      s.expectInZone(heroic, "field");

      const t = setup({ backrow: [{ def: HEROIC, faceUp: true, lane: 1 }], field: [MENACE] }, true);
      const none = t.card(HEROIC);
      delete none.x;
      t.play(TIMMY, { zone: 2 });
      t.expectInZone(none, "exile");
    });

    it("R448 a countered Tribute card's Tribute stays spent (an Upgrade to (4) reaches The Rock)", () => {
      const s = setup({ hand: [ROCK, VANILLA], field: [{ def: TIMMY, lane: 1 }] }, true);
      stepParam(s.card(EXILE), "threshold", 1);
      const timmy = s.unit("p2", 1);
      if (timmy === null) throw new Error("fixture");

      s.play(ROCK, { zone: 2, tributes: [timmy.id] });

      s.expectInZone(ROCK, "exile");
      s.expectInZone(timmy, "graveyard");
      s.expectMana("p2", 0);
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("R386 a Degrade of its threshold makes it answer (2) or less, with a budget of 2 minus the cost", () => {
      const s = setup({ hand: [MENACE, TIMMY], field: [POINTMASTER] }, true);
      stepParam(s.card(EXILE), "threshold", -1);

      s.play(MENACE, { zone: 2 });
      expect(count(s.events, "countered")).toBe(0);
      s.play(TIMMY, { zone: 3 });

      s.expectInZone(TIMMY, "exile");
      // Budget 2 − 1 = 1: the (2) Cost Pointmaster no longer fits.
      s.expectInZone(POINTMASTER, "field");
    });
  });
});
