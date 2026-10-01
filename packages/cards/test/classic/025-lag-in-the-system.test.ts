// C #25 Lag in the System — SPEC §8.6 row 25, BUILD M9 Classic row C 25: "Exiles every (1) Cost or
// less card on both fields, in both hands and in both decks, costs read per R65 at resolution: an X
// card in a hand or deck costs 0 and goes, one on the field costs its X (0 with none chosen, R396);
// face-down and Indestructible cards (C #90 In Too Deep) included; graveyards and exile untouched; the
// Spell itself is resolving and spared; no event carries a deck position; radiant: the opponent's
// field, hand and deck only; its tuned number (threshold) reads through `param()` (R386)".
//
// An X card on the field "played for X": the test stands a C+ #69 Buff Billy on the field with its
// stats given (`statsOverride`) and records the X it was played for on the instance, as a play would
// (`CardInstance.x`, §2.3).

import { describe, expect, it } from "vitest";
import { stepParam, type CardInstance } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/025-lag-in-the-system";

const LAG = "classic-025";
const REPLENISH = "core-010"; // (0) Spell
const INFINITE = "core-075"; // (0) Field Spell
const VANILLA = "core-008"; // (1) Unit 4/4
const TIMMY = "core-011"; // (1) Unit 3/3
const STOCKPILE = "core-005"; // (1) Spell
const SHEEPISH = "core-041"; // (1) Trap
const BREAD = "core-018"; // (1) Field Trap
const STATE_OF_GAME = "classic-041"; // (1) Unit, Indestructible
const IN_TOO_DEEP = "classic-090"; // (1) Field Spell, Indestructible
const DIVIDEND = "core-024"; // (X) Spell
const BILLY = "classicplus-069"; // (X) Unit
const FELINORS = "core-012"; // (2) Unit 3/4
const EXPERIMENT = "core-085"; // (2) Trap
const MENACE = "core-019"; // (3) Unit 9/9
const MANA_WELL = "core-006"; // (3) Field Spell
const SEVEN = "core-025"; // (4) Unit 7/7
const TOE_CRACKER = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."
const RUSH_TOKEN = "core-t-rush"; // (1) Unit token

function ids(cards: readonly CardInstance[]): string[] {
  return cards.map((card) => card.id);
}

function lagBoard(radiantFace = false): Scenario {
  return scenario({
    p1: {
      hand: [{ def: LAG, radiant: radiantFace }, FELINORS, VANILLA],
      field: [TIMMY, MENACE],
      backrow: [{ def: SHEEPISH, faceUp: false }, MANA_WELL],
      library: [REPLENISH, SEVEN],
      graveyard: [STOCKPILE],
      exile: [TIMMY],
    },
    p2: {
      hand: [FELINORS, STOCKPILE],
      field: [VANILLA, SEVEN],
      backrow: [{ def: BREAD, faceUp: false }, EXPERIMENT],
      library: [INFINITE, MENACE, SHEEPISH],
      graveyard: [VANILLA],
      exile: [REPLENISH],
    },
  });
}

describe("C #25 Lag in the System", () => {
  it("declares its one number, threshold (R386)", () => {
    expect(def.params).toEqual([{ key: "threshold", base: 1, radiant: 1, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("exiles every (1) Cost or less card on both fields, in both hands and in both decks", () => {
      const s = lagBoard();
      const gone = [
        s.unit("p1", 1),
        s.backrow("p1", 1),
        s.pile("p1", "hand")[2],
        s.pile("p1", "library")[0],
        s.unit("p2", 1),
        s.backrow("p2", 1),
        s.pile("p2", "hand")[1],
        s.pile("p2", "library")[0],
        s.pile("p2", "library")[2],
      ].map((card) => {
        if (card === null || card === undefined) throw new Error("setup: a card is missing");
        return card;
      });
      s.play(LAG);
      for (const card of gone) s.expectInZone(card, "exile");
      // What costs more stays where it was.
      expect(s.unit("p1", 2)?.defId).toBe(MENACE);
      expect(s.backrow("p1", 2)?.defId).toBe(MANA_WELL);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FELINORS]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([SEVEN]);
      expect(s.unit("p2", 2)?.defId).toBe(SEVEN);
      expect(s.backrow("p2", 2)?.defId).toBe(EXPERIMENT);
      expect(s.hand("p2").map((card) => card.defId)).toEqual([FELINORS]);
      expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([MENACE]);
    });

    it("graveyards and exile are untouched", () => {
      const s = lagBoard();
      const graves = [...ids(s.pile("p1", "graveyard")), ...ids(s.pile("p2", "graveyard"))];
      s.play(LAG);
      for (const id of graves) s.expectInZone(id, "graveyard");
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([STOCKPILE, LAG]);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([VANILLA]);
    });

    it("the Spell itself is resolving, so it is spared and lands in the graveyard", () => {
      const s = lagBoard();
      const lag = s.card(LAG);
      s.play(LAG);
      s.expectInZone(lag, "graveyard");
      expect(s.events.some((event) => event.type === "exiled" && event.instanceId === lag.id)).toBe(false);
    });

    it("face-down and Indestructible cards are included", () => {
      const s = scenario({
        p1: { hand: [LAG, FELINORS], field: [STATE_OF_GAME], backrow: [IN_TOO_DEEP] },
        p2: { hand: [STOCKPILE, FELINORS], backrow: [{ def: SHEEPISH, faceUp: false }] },
      });
      const state = s.card(STATE_OF_GAME);
      const deep = s.card(IN_TOO_DEEP);
      const trap = s.card(SHEEPISH);
      s.play(LAG);
      s.expectInZone(state, "exile");
      s.expectInZone(deep, "exile");
      s.expectInZone(trap, "exile");
    });

    it("R65 a hand card is read at its hand cost: a (2) Trap under Cloaked Toe Cracker costs (0) and goes", () => {
      const s = scenario({
        p1: { hand: [LAG, EXPERIMENT, { def: FELINORS, costMod: -1 }, MENACE], field: [TOE_CRACKER] },
        p2: { hand: [STOCKPILE] },
      });
      const trap = s.card(EXPERIMENT);
      const felinors = s.card(FELINORS);
      s.play(LAG);
      s.expectInZone(trap, "exile");
      s.expectInZone(felinors, "exile");
      // The Toe Cracker itself costs (2) and stays.
      s.expectInZone(TOE_CRACKER, "field");
      s.expectInZone(MENACE, "hand");
    });

    it("R396 an X card in a hand or a deck costs 0 and goes", () => {
      const s = scenario({
        p1: { hand: [LAG, DIVIDEND, FELINORS], library: [DIVIDEND] },
        p2: { hand: [STOCKPILE, BILLY], library: [BILLY] },
      });
      const xCards = [...s.hand("p1").filter((c) => c.defId === DIVIDEND), ...s.pile("p1", "library")];
      const billys = [...s.hand("p2").filter((c) => c.defId === BILLY), ...s.pile("p2", "library")];
      s.play(LAG);
      for (const card of [...xCards, ...billys]) s.expectInZone(card, "exile");
    });

    it("R396 an X card on the field costs the X it was played for: played for 3 it stays", () => {
      const s = scenario({
        p1: { hand: [LAG, FELINORS] },
        p2: { hand: [STOCKPILE], field: [{ def: BILLY, statsOverride: { attack: 9, health: 9 } }] },
      });
      s.card(BILLY).x = 3;
      s.play(LAG);
      s.expectInZone(BILLY, "field");
    });

    it("R396 an X card on the field with no X chosen (a summon) costs 0 and goes", () => {
      const s = scenario({
        p1: { hand: [LAG, FELINORS] },
        p2: { hand: [STOCKPILE], field: [{ def: BILLY, statsOverride: { attack: 3, health: 3 } }] },
      });
      const billy = s.card(BILLY);
      s.play(LAG);
      s.expectInZone(billy, "exile");
    });

    it("R11 a unit token exiled from the field ceases to exist", () => {
      const s = scenario({ p1: { hand: [LAG, FELINORS] }, p2: { hand: [STOCKPILE], field: [RUSH_TOKEN] } });
      const token = s.card(RUSH_TOKEN);
      s.play(LAG);
      s.expectInZone(token, "gone");
      expect(s.unit("p2", 1)).toBeNull();
    });

    it("R13 a card dormant under a Stack pile is not on the field, and is not exiled when the top goes", () => {
      const s = scenario({
        p1: { hand: [LAG, FELINORS] },
        p2: { hand: [MENACE], field: [VANILLA, { def: TIMMY, stack: true }] },
      });
      const vanilla = s.card(VANILLA);
      const timmy = s.card(TIMMY);
      s.play(LAG);
      s.expectInZone(timmy, "exile");
      // The Vanilla beneath was dormant when the set was read; it now tops the lane.
      s.expectInZone(vanilla, "field");
      expect(s.unit("p2", 1)?.id).toBe(vanilla.id);
    });

    it("no event carries a deck position, and each exile is public to both players", () => {
      const s = lagBoard();
      s.play(LAG);
      const exiled = s.events.filter((event) => event.type === "exiled");
      expect(exiled.length).toBeGreaterThan(0);
      for (const event of exiled) expect(Object.keys(event).sort()).toEqual(["defId", "instanceId", "owner", "type"]);
      for (const viewer of ["p1", "p2"] as const) {
        const seen = s.view(viewer).events.filter((event) => event.type === "exiled");
        expect(seen).toEqual(exiled);
      }
    });

    it("with nothing that cheap anywhere, nothing is exiled", () => {
      const s = scenario({ p1: { hand: [LAG, MENACE], field: [SEVEN] }, p2: { hand: [MENACE], library: [SEVEN] } });
      s.play(LAG);
      expect(s.events.some((event) => event.type === "exiled")).toBe(false);
    });

    it("R386 a Degrade of the threshold never goes below (1)", () => {
      const s = lagBoard();
      stepParam(s.card(LAG), "threshold", -1);
      s.play(LAG);
      // A (1) Cost card still goes: the Vanilla in your hand and the Timmy on your field.
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FELINORS]);
      expect(s.unit("p1", 1)).toBeNull();
    });

    it("R386 an Upgrade of the threshold reaches (2) Cost cards", () => {
      const s = lagBoard();
      stepParam(s.card(LAG), "threshold", 1);
      s.play(LAG);
      expect(s.hand("p1")).toEqual([]);
      expect(s.hand("p2")).toEqual([]);
      expect(s.backrow("p2", 2)).toBeNull();
      expect(s.unit("p1", 2)?.defId).toBe(MENACE);
    });
  });

  describe("radiant", () => {
    it("exiles the opponent's (1) Cost or less cards on their field, in their hand and in their deck", () => {
      const s = lagBoard(true);
      const theirs = [s.unit("p2", 1), s.backrow("p2", 1), s.pile("p2", "hand")[1], s.pile("p2", "library")[0], s.pile("p2", "library")[2]];
      s.play(LAG);
      for (const card of theirs) {
        if (card === null || card === undefined) throw new Error("setup: a card is missing");
        s.expectInZone(card, "exile");
      }
      expect(s.hand("p2").map((card) => card.defId)).toEqual([FELINORS]);
    });

    it("your own cheap cards are untouched", () => {
      const s = lagBoard(true);
      s.play(LAG);
      expect(s.unit("p1", 1)?.defId).toBe(TIMMY);
      expect(s.backrow("p1", 1)?.defId).toBe(SHEEPISH);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FELINORS, VANILLA]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([REPLENISH, SEVEN]);
    });

    it("their graveyard and exile are untouched, and the Spell is spared", () => {
      const s = lagBoard(true);
      s.play(LAG);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([VANILLA]);
      expect(s.pile("p2", "exile").map((card) => card.defId)).toEqual([REPLENISH, VANILLA, BREAD, STOCKPILE, INFINITE, SHEEPISH]);
      s.expectInZone(LAG, "graveyard");
    });

    it("R386 an Upgrade of the threshold reaches their (2) Cost cards", () => {
      const s = lagBoard(true);
      stepParam(s.card(LAG), "threshold", 1);
      s.play(LAG);
      expect(s.hand("p2")).toEqual([]);
      expect(s.backrow("p2", 2)).toBeNull();
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FELINORS, VANILLA]);
    });
  });
});
