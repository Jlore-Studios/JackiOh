// C #79 Risky Die — SPEC §8.6 row 79, BUILD M9 Classic row C 79: "Draw 3; the cards those draws put in
// your hand (not a cast-on-draw card, a burned one or a draw a limit stopped) cost (1) less (`costMod`,
// R78), then each of them that costs (1) or more is exiled; an X-cost card costs 0 in hand (R65) and
// stays; cards already in your hand are untouched; kept cards are never named in the opponent's view
// and exiled ones are public; radiant: only those that cost (2) or more are exiled; its tuned numbers
// (draw, kept threshold) read through `param()` (R386)".

import { reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/079-risky-die";
import { askingCastOnDraw } from "../_askingCast";
import { scenario, type Scenario } from "../_harness";

const RISKY = "classic-079";
const MACHINE = "classic-049"; // Anti-Greed Machine: players can't draw more than 1 card each turn.
const FILLER = "core-010"; // (0) Spell
const STOCKPILE = "core-005"; // (1) Spell
const ARMOR = "core-073"; // (2) Field Spell
const MENACE = "core-019"; // (3) Unit
const DIVIDEND = "core-024"; // (X) Spell
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw
const VANILLA = "core-008"; // (1) Unit

function drawnIds(events: readonly GameEvent[], player: PlayerId = "p1"): string[] {
  return events.flatMap((event) => (event.type === "drawn" && event.player === player ? [event.instanceId] : []));
}

function handCosts(s: Scenario): Record<string, number> {
  const hand = s.view("p1").you.hand;
  return Object.fromEntries((Array.isArray(hand) ? hand : []).map((card) => [card.defId, card.cost]));
}

describe("C #79 Risky Die", () => {
  it("is a (1) Spell; its draw count and kept threshold are declared numbers", () => {
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.params).toEqual([
      { key: "draw", base: 3, radiant: 3, better: "up", step: 1, min: 1 },
      { key: "threshold", base: 0, radiant: 1, better: "up", step: 1, min: 0 },
    ]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("draws 3; they cost (1) less; those that still cost (1) or more are exiled", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [FILLER, STOCKPILE, MENACE, VANILLA] } });
      s.play(RISKY);
      expect(drawnIds(s.lastEvents)).toHaveLength(3);
      s.expectInZone(FILLER, "hand").expectInZone(STOCKPILE, "hand").expectInZone(MENACE, "exile");
      expect(s.card(STOCKPILE).costMod).toBe(-1);
      expect(s.card(FILLER).costMod).toBe(-1);
      expect(handCosts(s)[STOCKPILE]).toBe(0);
    });

    it("R78 the discount persists: a kept card costs (1) less in later turns", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [STOCKPILE, FILLER, FILLER, VANILLA] } });
      s.play(RISKY).endTurn().endTurn();
      expect(s.card(STOCKPILE).costMod).toBe(-1);
      expect(handCosts(s)[STOCKPILE]).toBe(0);
    });

    it("cards already in your hand are untouched", () => {
      const s = scenario({ p1: { hand: [RISKY, STOCKPILE], library: [FILLER, FILLER, FILLER] } });
      const held = s.card(STOCKPILE);
      s.play(RISKY);
      s.expectInZone(held, "hand");
      expect(s.card(held).costMod).toBe(0);
    });

    it("R65 an X-cost card costs 0 in hand and stays", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [DIVIDEND, FILLER, FILLER] } });
      s.play(RISKY);
      s.expectInZone(DIVIDEND, "hand");
    });

    it("R58 a card cast on draw never reaches your hand: it is not one of them, and the draw repeats", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [CN_VIRUS, MENACE, STOCKPILE, FILLER] } });
      s.play(RISKY);
      s.expectInZone(CN_VIRUS, "graveyard");
      // The three draws took CN-Virus (cast, the draw repeating into Menace), Stockpile and Filler.
      s.expectInZone(MENACE, "exile").expectInZone(STOCKPILE, "hand").expectInZone(FILLER, "hand");
      expect(s.card(CN_VIRUS).costMod).toBe(0);
    });

    it("§2.4 a burned card is not one of them: it stays in your graveyard at its own cost", () => {
      const nine = Array.from({ length: 9 }, () => VANILLA);
      const s = scenario({ p1: { hand: [RISKY, ...nine], library: [FILLER, MENACE, STOCKPILE] } });
      s.play(RISKY);
      // Risky Die left the hand (9), the first draw fills it (10), the next two burn.
      s.expectInZone(FILLER, "hand").expectInZone(MENACE, "graveyard").expectInZone(STOCKPILE, "graveyard");
      expect(s.card(MENACE).costMod).toBe(0);
    });

    it("§2.4 a fatigue draw puts nothing in your hand: from a deck of one, one card is judged and two hits land", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [MENACE] } });
      s.play(RISKY);
      s.expectInZone(MENACE, "exile");
      expect(s.state.players.p1.fatigueCount).toBe(2);
      s.expectHealth("p1", 27);
      expect(s.hand("p1").map((card) => [card.defId, card.costMod])).toEqual([[VANILLA, 0]]);
    });

    it("B5 E3 a draw a limit stopped draws nothing: under Anti-Greed Machine only the first card is one of them", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], field: [MACHINE], library: [MENACE, STOCKPILE, FILLER] } });
      s.play(RISKY);
      expect(drawnIds(s.lastEvents)).toHaveLength(1);
      s.expectInZone(MENACE, "exile");
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([STOCKPILE, FILLER]);
    });

    it("§9.3 a cast on draw that asks pauses the draws; after a JSON round trip they finish, and only the drawn cards are judged", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [MENACE, STOCKPILE, FILLER] } });
      askingCastOnDraw(s);
      s.play(RISKY);
      // The asking cast is cast on the first draw and asks for its discard.
      expect(s.state.pending?.kind).toBe("hand");
      const vanilla = s.card(VANILLA);
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c79-round-trip",
        choiceId: s.state.pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: vanilla.id }],
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      const after = live.state;
      const where = (defId: string): string | undefined =>
        [...after.players.p1.hand, ...after.players.p1.exile, ...after.players.p1.graveyard, ...after.players.p1.library].find(
          (card) => card.defId === defId,
        )?.zone.z;
      expect(where(MENACE)).toBe("exile");
      expect(where(STOCKPILE)).toBe("hand");
      expect(where(FILLER)).toBe("hand");
      expect(where(VANILLA)).toBe("graveyard");
      expect(after.pending).toBeNull();
    });

    it("R97 kept cards are never named in the opponent's view; exiled ones are public", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [FILLER, STOCKPILE, MENACE] } });
      s.play(RISKY);
      const theirs = s.view("p2");
      const kept = [s.card(FILLER).id, s.card(STOCKPILE).id];
      const text = JSON.stringify(theirs.events);
      for (const id of kept) expect(text).not.toContain(id);
      expect(theirs.opponent.exile.map((card) => card.defId)).toEqual([MENACE]);
    });

    it("R386 its draw count is declared: an Upgrade's step draws 4", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [FILLER, FILLER, FILLER, FILLER, FILLER] } });
      stepParam(s.card(RISKY), "draw", 1);
      s.play(RISKY);
      expect(drawnIds(s.lastEvents)).toHaveLength(4);
    });

    it("R386 its kept threshold is declared: an Upgrade's step keeps a card that costs (1)", () => {
      const s = scenario({ p1: { hand: [RISKY, VANILLA], library: [ARMOR, FILLER, FILLER] } });
      stepParam(s.card(RISKY), "threshold", 1);
      s.play(RISKY);
      s.expectInZone(ARMOR, "hand");
      expect(handCosts(s)[ARMOR]).toBe(1);
    });
  });

  describe("radiant", () => {
    it("only those that still cost (2) or more are exiled: a (2) card kept at (1), a (3) card exiled", () => {
      const s = scenario({ p1: { hand: [{ def: RISKY, radiant: true }, VANILLA], library: [ARMOR, MENACE, STOCKPILE] } });
      s.play(RISKY);
      s.expectInZone(ARMOR, "hand").expectInZone(STOCKPILE, "hand").expectInZone(MENACE, "exile");
      expect(handCosts(s)[ARMOR]).toBe(1);
    });

    it("R386 its threshold steps from (1): a Degrade's step exiles the (2) card too", () => {
      const s = scenario({ p1: { hand: [{ def: RISKY, radiant: true }, VANILLA], library: [ARMOR, FILLER, FILLER] } });
      stepParam(s.card(RISKY), "threshold", -1);
      s.play(RISKY);
      s.expectInZone(ARMOR, "exile");
    });

    it("draws 3, and a card cast on draw is still none of them", () => {
      const s = scenario({ p1: { hand: [{ def: RISKY, radiant: true }, VANILLA], library: [CN_VIRUS, ARMOR, FILLER, FILLER] } });
      s.play(RISKY);
      s.expectInZone(CN_VIRUS, "graveyard").expectInZone(ARMOR, "hand");
    });
  });
});
