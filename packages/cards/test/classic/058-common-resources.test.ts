// C #58 Common Resources — SPEC §8.6 row 58, BUILD M9 Classic row C 58: "Start of your turn: a draw of
// yours taken from the bottom of the opponent's deck, the card becoming yours (its owner changes, R12),
// under your hand cap (a burn goes to your graveyard, R317), your cast on draw and your draw limit; an
// empty enemy deck gives nothing and nobody takes fatigue; the opponent's view never names the card
// (R97) and no event carries a deck position; radiant: at the start and at the end of your turn; its
// tuned number (cards) reads through `param()` (R386)".
//
// The draw limit is C #49 Anti-Greed Machine's (B5 E3); the start-of-turn trigger runs before the
// turn's own draw (§2.2, R62), so it is the turn's first draw and the turn's own draw is the one a
// limit of 1 stops.

import { drawsThisTurn, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/058-common-resources";
import { scenario, type Scenario } from "../_harness";

const RESOURCES = "classic-058";
const MACHINE = "classic-049"; // Radiant: your opponent can't draw more than 1 card each turn.
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const STOCKPILE = "core-005"; // (1) Spell
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw: take 1 damage
const FILLER = "core-010"; // (0) Spell
const HINDER = "core-021"; // (0) Spell, Cast on draw: your opponent has 1 less mana next turn. Discard 1.
const INCOME_TAX = "classic-009"; // Trap: when the cards your opponent has drawn in a turn reach 2 …

function drawnBy(events: readonly GameEvent[], player: PlayerId): Extract<GameEvent, { type: "drawn" }>[] {
  return events.filter((event): event is Extract<GameEvent, { type: "drawn" }> => event.type === "drawn" && event.player === player);
}

/** p1's Common Resources face-up in the backrow, on p2's turn; `endTurn()` starts p1's turn. */
function waiting(opts: {
  radiant?: boolean;
  p1Library?: readonly string[];
  p2Library?: readonly string[];
  p1Hand?: readonly string[];
  p2Field?: readonly (string | { def: string; radiant?: boolean })[];
}): Scenario {
  return scenario({
    p1: {
      hand: opts.p1Hand ?? [FILLER],
      backrow: [{ def: RESOURCES, radiant: opts.radiant === true, faceUp: true }],
      library: [...(opts.p1Library ?? [STOCKPILE, STOCKPILE])],
    },
    p2: { hand: [FILLER], library: [...(opts.p2Library ?? [VANILLA, MENACE])], field: [...(opts.p2Field ?? [])] },
    active: "p2",
  });
}

describe("C #58 Common Resources", () => {
  it("is a (2) Field Spell whose count of cards is a declared number", () => {
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toBe(2);
    expect(def.params).toEqual([{ key: "cards", base: 1, radiant: 1, better: "up", step: 1, min: 1 }]);
    expect(base.startOfTurn).toBeTypeOf("function");
    expect(base.endOfTurn).toBeUndefined();
    expect(radiant.startOfTurn).toBeTypeOf("function");
    expect(radiant.endOfTurn).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R12 at the start of your turn you draw the bottom card of the opponent's deck, and it becomes yours", () => {
      const s = waiting({});
      const bottom = s.card(MENACE);
      s.endTurn();
      s.expectInZone(bottom, "hand");
      expect(s.card(bottom).owner).toBe("p1");
      expect(s.hand("p1").map((card) => card.id)).toContain(bottom.id);
      expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([VANILLA]);
      // Then the turn's own draw, from your own deck.
      expect(drawnBy(s.lastEvents, "p1").map((event) => event.defId)).toEqual([MENACE, STOCKPILE]);
    });

    it("§2.2 R62 it is a start-of-turn trigger of yours only: the opponent's turn draws nothing from it", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: RESOURCES, faceUp: true }], library: [STOCKPILE] },
        p2: { hand: [FILLER], library: [VANILLA, MENACE] },
      });
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(0);
      expect(s.pile("p2", "library")).toHaveLength(1);
    });

    it("it is your draw: it counts toward your draws this turn", () => {
      const s = waiting({});
      s.endTurn();
      expect(drawsThisTurn(s.state, "p1")).toBe(2);
    });

    it("R317 under your hand cap: at a full hand it burns into your graveyard, yours", () => {
      const fullHand = Array.from({ length: 10 }, () => FILLER);
      const s = waiting({ p1Hand: fullHand });
      const bottom = s.card(MENACE);
      s.endTurn();
      s.expectInZone(bottom, "graveyard");
      expect(s.card(bottom).owner).toBe("p1");
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toContain(bottom.id);
      expect(s.lastEvents.some((event) => event.type === "burned")).toBe(true);
    });

    it("R58 your cast on draw: a Cast-on-draw card at the bottom of their deck is cast for you", () => {
      const s = waiting({ p2Library: [VANILLA, CN_VIRUS] });
      s.endTurn();
      const cast = s.lastEvents.find((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS);
      expect(cast).toMatchObject({ player: "p1", costPaid: 0 });
      // "Take 1 damage" is its caster's: p1's hero.
      expect(s.lastEvents.some((event) => event.type === "damage" && event.targetId === "hero-p1" && event.amount === 1)).toBe(true);
    });

    it("R58 §9.3 a cast on draw that asks is yours to answer, and the answer finishes the turn after a JSON round trip", () => {
      const s = waiting({ p1Hand: [FILLER, VANILLA], p2Library: [VANILLA, MENACE, HINDER] });
      s.endTurn();
      const pending = s.state.pending;
      expect(pending).toMatchObject({ playerId: "p1", kind: "hand" });
      const discard = s.hand("p1").find((card) => card.defId === VANILLA);
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c58-round-trip",
        choiceId: pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: discard?.id ?? "" }],
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      // R70, R81: the cast's declared discard was asked as the cast began; answered, it is p1's play.
      expect(live.events.find((event) => event.type === "cardPlayed" && event.defId === HINDER)).toMatchObject({ player: "p1" });
      const p1 = live.state.players.p1;
      expect(p1.graveyard.map((card) => [card.defId, card.owner])).toEqual([
        [VANILLA, "p1"],
        [HINDER, "p1"],
      ]);
      // §2.4's repeat of the draw and the turn's own draw are p1's own draws, from p1's deck: only the
      // one bottom card left p2's deck.
      expect(live.state.players.p2.library.map((card) => card.defId)).toEqual([VANILLA, MENACE]);
      expect(p1.hand.map((card) => card.defId)).toEqual([FILLER, STOCKPILE, STOCKPILE]);
      expect(live.state.pending).toBeNull();
    });

    it("§10.1 it is your draw in your per-turn count: with the turn's own draw it sets off C #9 Income Tax", () => {
      const s = scenario({
        p1: { hand: [FILLER, VANILLA], backrow: [{ def: RESOURCES, faceUp: true }], library: [STOCKPILE] },
        p2: { hand: [FILLER], backrow: [{ def: INCOME_TAX, faceUp: false }], library: [VANILLA, MENACE] },
        active: "p2",
      });
      const taken = s.card(MENACE);
      s.endTurn();
      expect(s.events.some((event) => event.type === "trapFired")).toBe(true);
      expect(s.state.pending).toMatchObject({ playerId: "p1", kind: "hand" });
      s.answer([{ pick: "instance", instanceId: taken.id }]);
      expect(s.hand("p1").map((card) => card.id)).toEqual([taken.id]);
      expect(s.hand("p2").map((card) => card.defId).sort()).toEqual([FILLER, FILLER, STOCKPILE, VANILLA].sort());
    });

    it("§2.4 your draw limit: under the opponent's Radiant Anti-Greed Machine it is your one draw, and your turn's own draw is stopped", () => {
      const s = waiting({ p2Field: [{ def: MACHINE, radiant: true }] });
      s.endTurn();
      expect(drawnBy(s.lastEvents, "p1").map((event) => event.defId)).toEqual([MENACE]);
      expect(s.lastEvents.filter((event) => event.type === "drawLimited" && event.player === "p1")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("an empty enemy deck gives nothing, and nobody takes fatigue", () => {
      const s = waiting({ p2Library: [] });
      s.endTurn();
      expect(s.state.players.p1.fatigueCount).toBe(0);
      expect(s.state.players.p2.fatigueCount).toBe(0);
      expect(drawnBy(s.lastEvents, "p1").map((event) => event.defId)).toEqual([STOCKPILE]);
    });

    it("R97 the opponent's view never names the card, and no event carries a deck position", () => {
      const s = waiting({});
      const bottom = s.card(MENACE);
      s.endTurn();
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: s.hand("p1").length });
      for (const event of theirs.events.filter((e) => e.type === "drawn" || e.type === "stolen")) {
        expect(JSON.stringify(event)).not.toContain(bottom.id);
        expect(JSON.stringify(event)).not.toContain(MENACE);
      }
      for (const viewer of ["p1", "p2"] as const) {
        for (const event of s.view(viewer).events) expect(event).not.toHaveProperty("position", expect.any(Number));
      }
      // Its new owner reads it.
      expect(JSON.stringify(s.view("p1").you.hand)).toContain(bottom.id);
    });

    it("R386 its count is the declared number: an Upgrade's step draws 2 from the bottom, bottom first", () => {
      const s = waiting({ p2Library: [STOCKPILE, VANILLA, MENACE] });
      stepParam(s.card(RESOURCES), "cards", 1);
      s.endTurn();
      expect(drawnBy(s.lastEvents, "p1").map((event) => event.defId)).toEqual([MENACE, VANILLA, STOCKPILE]);
      expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([STOCKPILE]);
    });
  });

  describe("radiant", () => {
    it("draws from the bottom of the opponent's deck at the start of your turn and at its end", () => {
      const s = waiting({ radiant: true, p2Library: [STOCKPILE, VANILLA, MENACE] });
      s.endTurn();
      expect(drawnBy(s.lastEvents, "p1").map((event) => event.defId)).toEqual([MENACE, STOCKPILE]);
      s.endTurn();
      const endDraws = drawnBy(s.lastEvents, "p1").map((event) => event.defId);
      expect(endDraws).toEqual([VANILLA]);
      expect(s.card(VANILLA).owner).toBe("p1");
    });

    it("R386 its declared count steps for both: an Upgrade draws 2 at each end", () => {
      const s = waiting({ radiant: true, p2Library: [VANILLA, VANILLA, MENACE, MENACE] });
      stepParam(s.card(RESOURCES), "cards", 1);
      s.endTurn();
      expect(drawnBy(s.lastEvents, "p1").filter((event) => event.defId === MENACE)).toHaveLength(2);
      s.endTurn();
      expect(drawnBy(s.lastEvents, "p1").filter((event) => event.defId === VANILLA)).toHaveLength(2);
    });

    it("an empty enemy deck at the end of the turn gives nothing, and no fatigue", () => {
      const s = waiting({ radiant: true, p2Library: [MENACE] });
      s.endTurn();
      expect(s.pile("p2", "library")).toHaveLength(0);
      s.endTurn();
      // Up to p2's own start of turn (whose own draw from its empty deck is fatigue of its own, §2.4),
      // p1's end of turn drew nothing and hit nobody.
      const events = s.lastEvents;
      const p2Starts = events.findIndex((event) => event.type === "turnStarted");
      const before = events.slice(0, p2Starts < 0 ? events.length : p2Starts);
      expect(before.some((event) => event.type === "drawn" || event.type === "fatigue")).toBe(false);
      expect(s.state.players.p1.fatigueCount).toBe(0);
    });
  });
});
