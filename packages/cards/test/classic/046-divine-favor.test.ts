// C #46 Divine Favor — SPEC §8.6 row 46, BUILD M9 Classic row C 46: "Read as it resolves (this Spell has
// left your hand): draws one at a time until your hand holds as many cards as the opponent's; level or
// ahead → no draw; a draw that adds no card (fatigue, a burn, a cast-on-draw card, a draw a limit
// stops) ends it, so it never loops; its preview is the number of draws it asks for now (R280); radiant:
// until you hold twice as many; its tuned number (multiplier) reads through `param()` (R386)".
//
// The preview's proofs are in `test/preview.test.ts` (its C #46 section), with the set of hooked cards.

import { stepParam, type GameState, reduce } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/046-divine-favor";
import { askingCastOnDraw } from "../_askingCast";
import { scenario } from "../_harness";

const FAVOR = "classic-046";
const MACHINE = "classic-049"; // Anti-Greed Machine: players can't draw more than 1 card each turn.
const FILLER = "core-010"; // (0) Spell
const STOCKPILE = "core-005";
const MENACE = "core-019";
const CN_VIRUS = "core-090-1"; // Cast on draw

function drawn(events: readonly GameEvent[], player: PlayerId = "p1"): GameEvent[] {
  return events.filter((event) => event.type === "drawn" && event.player === player);
}

const many = (count: number, defId: string = STOCKPILE): string[] => Array.from({ length: count }, () => defId);

describe("C #46 Divine Favor", () => {
  it("is a (1) Spell; its multiplier is a declared number (1, Radiant 2); it declares a preview", () => {
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.params).toEqual([{ key: "multiplier", base: 1, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(base.preview).toBeTypeOf("function");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("draws until your hand holds as many cards as the opponent's, this Spell already out of it", () => {
      const s = scenario({ p1: { hand: [FAVOR, FILLER], library: many(6, MENACE) }, p2: { hand: many(4) } });
      s.play(FAVOR);
      expect(drawn(s.lastEvents)).toHaveLength(3);
      expect(s.hand("p1")).toHaveLength(4);
    });

    it("level or ahead: no draw", () => {
      const level = scenario({ p1: { hand: [FAVOR, FILLER, FILLER], library: many(3, MENACE) }, p2: { hand: many(2) } });
      level.play(FAVOR);
      expect(drawn(level.lastEvents)).toHaveLength(0);
      const ahead = scenario({ p1: { hand: [FAVOR, FILLER, FILLER, FILLER], library: many(3, MENACE) }, p2: { hand: many(1) } });
      ahead.play(FAVOR);
      expect(drawn(ahead.lastEvents)).toHaveLength(0);
    });

    it("§2.4 a fatigue hit adds no card and ends it: one hit however far behind", () => {
      const s = scenario({ p1: { hand: [FAVOR], library: [MENACE] }, p2: { hand: many(6) } });
      s.play(FAVOR);
      expect(drawn(s.lastEvents)).toHaveLength(1);
      expect(s.state.players.p1.fatigueCount).toBe(1);
    });

    it("§2.4 a burn at the hand cap adds no card and ends it", () => {
      const s = scenario({ p1: { hand: [{ def: FAVOR, radiant: true }, ...many(9, FILLER)], library: many(5, MENACE) }, p2: { hand: many(6) } });
      s.play(FAVOR);
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(3);
    });

    it("R58 a card cast on draw adds no card and ends it", () => {
      const s = scenario({ p1: { hand: [FAVOR], library: [CN_VIRUS, MENACE, MENACE, MENACE] }, p2: { hand: many(4) } });
      s.play(FAVOR);
      expect(s.lastEvents.some((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS)).toBe(true);
      // The draw's chain repeats into one card (§2.4), and the draws stop there.
      expect(s.hand("p1")).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("§9.3 a cast on draw that asks ends it too; the answer finishes that draw's chain, after a JSON round trip", () => {
      const s = scenario({ p1: { hand: [FAVOR, FILLER], library: [MENACE, MENACE] }, p2: { hand: many(4) } });
      askingCastOnDraw(s);
      s.play(FAVOR);
      expect(s.state.pending?.kind).toBe("hand");
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c46-round-trip",
        choiceId: s.state.pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: s.card(FILLER).id }],
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      // The asking cast's chain repeats into one Menace; Divine Favor asks for no more.
      expect(live.state.players.p1.hand.map((card) => card.defId)).toEqual([MENACE]);
      expect(live.state.players.p1.library).toHaveLength(1);
    });

    it("B5 E3 a draw a limit stops adds no card and ends it", () => {
      const s = scenario({ p1: { hand: [FAVOR], field: [MACHINE], library: many(4, MENACE) }, p2: { hand: many(4) } });
      s.play(FAVOR);
      expect(drawn(s.lastEvents)).toHaveLength(1);
      expect(s.lastEvents.filter((event) => event.type === "drawLimited")).toHaveLength(1);
    });

    it("reads the opponent's hand as it resolves: their hand is the mark, whatever their deck holds", () => {
      const s = scenario({ p1: { hand: [FAVOR], library: many(5, MENACE) }, p2: { hand: many(2), library: many(9, MENACE) } });
      s.play(FAVOR);
      expect(drawn(s.lastEvents)).toHaveLength(2);
    });

    it("R386 its multiplier is declared: an Upgrade's step makes it 2×", () => {
      const s = scenario({ p1: { hand: [FAVOR, FILLER], library: many(6, MENACE) }, p2: { hand: many(3) } });
      stepParam(s.card(FAVOR), "multiplier", 1);
      s.play(FAVOR);
      expect(s.hand("p1")).toHaveLength(6);
    });

    it("R97 the drawn cards are never named in the opponent's view", () => {
      const s = scenario({ p1: { hand: [FAVOR], library: many(2, MENACE) }, p2: { hand: many(2) } });
      s.play(FAVOR);
      const ids = s.hand("p1").map((card) => card.id);
      const text = JSON.stringify(s.view("p2"));
      for (const id of ids) expect(text).not.toContain(id);
    });
  });

  describe("radiant", () => {
    it("draws until you hold twice as many cards as the opponent", () => {
      const s = scenario({ p1: { hand: [{ def: FAVOR, radiant: true }, FILLER], library: many(8, MENACE) }, p2: { hand: many(3) } });
      s.play(FAVOR);
      expect(s.hand("p1")).toHaveLength(6);
      expect(drawn(s.lastEvents)).toHaveLength(5);
    });

    it("at twice as many already, no draw", () => {
      const s = scenario({ p1: { hand: [{ def: FAVOR, radiant: true }, FILLER, FILLER], library: many(3, MENACE) }, p2: { hand: many(1) } });
      s.play(FAVOR);
      expect(drawn(s.lastEvents)).toHaveLength(0);
    });

    it("R386 its multiplier steps from 2: a Degrade's step makes it 1×", () => {
      const s = scenario({ p1: { hand: [{ def: FAVOR, radiant: true }], library: many(8, MENACE) }, p2: { hand: many(3) } });
      stepParam(s.card(FAVOR), "multiplier", -1);
      s.play(FAVOR);
      expect(s.hand("p1")).toHaveLength(3);
    });

    it("§2.4 a fatigue hit ends it on the Radiant face too", () => {
      const s = scenario({ p1: { hand: [{ def: FAVOR, radiant: true }], library: [] }, p2: { hand: many(3) } });
      s.play(FAVOR);
      expect(s.state.players.p1.fatigueCount).toBe(1);
      expect(s.hand("p1")).toHaveLength(0);
    });
  });
});
