// C #18 Glitch in the System — SPEC §8.6 row 18, BUILD M9 Classic row C 18: "The number is chosen
// with the play (R81) from `GLITCH_NUMBERS`, the same eleven options every time, so they reveal
// nothing; it exiles every card of that cost on the Field alone, both fields and nothing else
// (balance patch 1), costs read per R65 at resolution (a field card at its own cost; an X card its X
// on the field, 0 when it arrived without a chosen X, R396), face-down and Indestructible cards
// included; graveyards and exile untouched; the Spell itself is resolving and spared; a number
// nothing costs exiles nothing; the exiled cards are public; radiant: the opponent's field, hand and
// deck, where a hand card is read at its hand cost and a deck card at its own (R65, R396);
// no tuned numbers".
//
// An X card on the field "played for X": the test stands a C+ #69 Buff Billy on the field with its
// stats given and records the X it was played for on the instance, as a play would (`CardInstance.x`,
// §2.3).

import { describe, expect, it } from "vitest";
import { legalActions } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/018-glitch-in-the-system";

const GLITCH = "classic-018";
const NUMBERS = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
const REPLENISH = "core-010"; // (0) Spell
const VANILLA = "core-008"; // (1) Unit
const STOCKPILE = "core-005"; // (1) Spell
const FELINORS = "core-012"; // (2) Unit
const EXPERIMENT = "core-085"; // (2) Trap
const TWINSPELL = "core-079"; // (2) Field Spell
const MENACE = "core-019"; // (3) Unit
const MANA_WELL = "core-006"; // (3) Field Spell
const SEVEN = "core-025"; // (4) Unit
const ROCK = "core-066"; // (4) Unit, Indestructible, Tribute 1
const DIVIDEND = "core-024"; // (X) Spell
const BILLY = "classicplus-069"; // (X) Unit
const TOE_CRACKER = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."

function glitchBoard(radiantFace = false): Scenario {
  return scenario({
    p1: {
      hand: [{ def: GLITCH, radiant: radiantFace }, FELINORS, VANILLA],
      field: [FELINORS, MENACE],
      backrow: [{ def: EXPERIMENT, faceUp: false }, MANA_WELL],
      library: [FELINORS, SEVEN],
      graveyard: [FELINORS],
      exile: [FELINORS],
    },
    p2: {
      hand: [FELINORS, STOCKPILE],
      field: [VANILLA, FELINORS],
      backrow: [TWINSPELL],
      library: [REPLENISH, FELINORS],
      graveyard: [FELINORS],
      exile: [FELINORS],
    },
  });
}

function felinorsIn(s: Scenario, player: "p1" | "p2"): { field: number; hand: number; library: number; graveyard: number; exile: number } {
  const count = (zone: "hand" | "library" | "graveyard" | "exile"): number =>
    s.pile(player, zone).filter((card) => card.defId === FELINORS).length;
  const field = [1, 2, 3, 4, 5].filter((lane) => s.unit(player, lane)?.defId === FELINORS).length;
  return { field, hand: count("hand"), library: count("library"), graveyard: count("graveyard"), exile: count("exile") };
}

describe("C #18 Glitch in the System", () => {
  it("declares no numbers, and the number choice as a mode of kind number, 0 to 10", () => {
    expect(def.params).toBeUndefined();
    expect(base.modes).toEqual([{ kind: "number", options: NUMBERS }]);
    expect(radiant.modes).toEqual(base.modes);
    expect(base.targets).toBeUndefined();
  });

  describe("base", () => {
    it("R81 the number is chosen with the play: legalActions offers the same eleven every time", () => {
      const full = glitchBoard();
      const empty = scenario({ p1: { hand: [GLITCH, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      for (const s of [full, empty]) {
        const glitch = s.card(GLITCH);
        const offered = legalActions(s.state, "p1").flatMap((action) =>
          action.type === "play" && action.instanceId === glitch.id ? [action.modes ?? []] : [],
        );
        expect(offered).toEqual(NUMBERS.map((number) => [number]));
      }
    });

    it("a number outside the list is refused", () => {
      const s = glitchBoard();
      expect(() => s.play(GLITCH, { modes: ["11"] })).toThrow();
      expect(() => s.play(GLITCH, { modes: [] })).toThrow();
    });

    it("exiles every card of that cost on both fields, and nothing anywhere else", () => {
      const s = glitchBoard();
      s.play(GLITCH, { modes: ["2"] });
      expect(felinorsIn(s, "p1")).toEqual({ field: 0, hand: 1, library: 1, graveyard: 1, exile: 2 });
      expect(felinorsIn(s, "p2")).toEqual({ field: 0, hand: 1, library: 1, graveyard: 1, exile: 2 });
      // The face-down (2) Trap and the (2) Field Spell go too.
      expect(s.backrow("p1", 1)).toBeNull();
      expect(s.backrow("p2", 1)).toBeNull();
      // Other costs stay.
      expect(s.unit("p1", 2)?.defId).toBe(MENACE);
      expect(s.unit("p2", 1)?.defId).toBe(VANILLA);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FELINORS, VANILLA]);
      expect(s.hand("p2").map((card) => card.defId)).toEqual([FELINORS, STOCKPILE]);
    });

    it("graveyards and exile are untouched: their (2) Cost cards stay where they are", () => {
      const s = glitchBoard();
      const graves = [...s.pile("p1", "graveyard"), ...s.pile("p2", "graveyard")];
      s.play(GLITCH, { modes: ["2"] });
      for (const card of graves) s.expectInZone(card, "graveyard");
    });

    it("the Spell itself is resolving and spared, though it costs (3)", () => {
      const s = glitchBoard();
      const glitch = s.card(GLITCH);
      s.play(GLITCH, { modes: ["3"] });
      s.expectInZone(glitch, "graveyard");
      expect(s.unit("p1", 2)).toBeNull();
      expect(s.backrow("p1", 2)).toBeNull();
    });

    it("face-down and Indestructible cards are included", () => {
      const s = scenario({
        p1: { hand: [GLITCH, STOCKPILE], field: [ROCK] },
        p2: { hand: [STOCKPILE], field: [SEVEN], backrow: [{ def: EXPERIMENT, faceUp: false }] },
      });
      const rock = s.card(ROCK);
      s.play(GLITCH, { modes: ["4"] });
      s.expectInZone(rock, "exile");
      s.expectInZone(SEVEN, "exile");
    });

    it("R65 costs at resolution: a set card at its own cost (a (2) Trap under Toe Cracker costs (2))", () => {
      const s = scenario({
        p1: { hand: [GLITCH, STOCKPILE], field: [TOE_CRACKER], backrow: [{ def: EXPERIMENT, faceUp: false }] },
        p2: { hand: [STOCKPILE] },
      });
      const trap = s.card(EXPERIMENT);
      s.play(GLITCH, { modes: ["0"] });
      s.expectInZone(trap, "field");
      const two = scenario({
        p1: { hand: [GLITCH, STOCKPILE], field: [TOE_CRACKER], backrow: [{ def: EXPERIMENT, faceUp: false }] },
        p2: { hand: [STOCKPILE] },
      });
      two.play(GLITCH, { modes: ["2"] });
      two.expectInZone(EXPERIMENT, "exile");
    });

    it("R65 a costMod counts on the field: the Menace at (2) goes with a 2", () => {
      const s = scenario({
        p1: { hand: [GLITCH, STOCKPILE], field: [{ def: MENACE, costMod: -1 }] },
        p2: { hand: [STOCKPILE], field: [{ def: SEVEN, costMod: -2 }] },
      });
      s.play(GLITCH, { modes: ["2"] });
      s.expectInZone(MENACE, "exile");
      s.expectInZone(SEVEN, "exile");
    });

    it("R396 an X card on the field costs its X: played for 3, a 3 exiles it and a 0 does not", () => {
      const three = scenario({
        p1: { hand: [GLITCH, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: BILLY, statsOverride: { attack: 9, health: 9 } }] },
      });
      three.card(BILLY).x = 3;
      three.play(GLITCH, { modes: ["3"] });
      three.expectInZone(BILLY, "exile");

      const zero = scenario({
        p1: { hand: [GLITCH, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: BILLY, statsOverride: { attack: 9, health: 9 } }] },
      });
      zero.card(BILLY).x = 3;
      zero.play(GLITCH, { modes: ["0"] });
      zero.expectInZone(BILLY, "field");
    });

    it("R396 an X card on the field with no X chosen costs 0", () => {
      const s = scenario({
        p1: { hand: [GLITCH, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [{ def: BILLY, statsOverride: { attack: 3, health: 3 } }] },
      });
      const billy = s.card(BILLY);
      s.play(GLITCH, { modes: ["0"] });
      s.expectInZone(billy, "exile");
    });

    it("a number nothing costs exiles nothing", () => {
      const s = glitchBoard();
      s.play(GLITCH, { modes: ["10"] });
      expect(s.events.some((event) => event.type === "exiled")).toBe(false);
      s.expectInZone(GLITCH, "graveyard");
    });

    it("the exiled cards are public to both players", () => {
      const s = glitchBoard();
      s.play(GLITCH, { modes: ["2"] });
      const exiled = s.events.filter((event) => event.type === "exiled");
      expect(exiled).toHaveLength(4);
      for (const event of exiled) expect(Object.keys(event).sort()).toEqual(["defId", "instanceId", "owner", "type"]);
      for (const viewer of ["p1", "p2"] as const) {
        expect(s.view(viewer).events.filter((event) => event.type === "exiled")).toEqual(exiled);
      }
    });

    it("R13 a card dormant under a Stack pile is not on the field", () => {
      const s = scenario({
        p1: { hand: [GLITCH, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [FELINORS, { def: MENACE, stack: true }] },
      });
      const felinors = s.card(FELINORS);
      s.play(GLITCH, { modes: ["2"] });
      s.expectInZone(felinors, "field");
    });
  });

  describe("radiant", () => {
    it("exiles every card of that cost on the opponent's field, in their hand and in their deck only", () => {
      const s = glitchBoard(true);
      s.play(GLITCH, { modes: ["2"] });
      expect(felinorsIn(s, "p2")).toEqual({ field: 0, hand: 0, library: 0, graveyard: 1, exile: 4 });
      expect(s.backrow("p2", 1)).toBeNull();
      // Yours are untouched.
      expect(felinorsIn(s, "p1")).toEqual({ field: 1, hand: 1, library: 1, graveyard: 1, exile: 1 });
      expect(s.backrow("p1", 1)?.defId).toBe(EXPERIMENT);
    });

    it("offers the same eleven numbers", () => {
      const s = glitchBoard(true);
      const glitch = s.card(GLITCH);
      const offered = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === glitch.id ? [action.modes ?? []] : [],
      );
      expect(offered).toEqual(NUMBERS.map((number) => [number]));
    });

    it("their graveyard and exile are untouched, and a number nothing of theirs costs exiles nothing", () => {
      const s = glitchBoard(true);
      s.play(GLITCH, { modes: ["4"] });
      expect(s.events.some((event) => event.type === "exiled")).toBe(false);
      expect(felinorsIn(s, "p2")).toEqual({ field: 1, hand: 1, library: 1, graveyard: 1, exile: 1 });
    });

    it("R65 a hand card at its hand cost (a (2) Trap under Toe Cracker costs 0)", () => {
      const s = scenario({
        p1: { hand: [{ def: GLITCH, radiant: true }, STOCKPILE] },
        p2: { hand: [EXPERIMENT, STOCKPILE], field: [TOE_CRACKER], library: [REPLENISH] },
      });
      s.play(GLITCH, { modes: ["0"] });
      // The opponent's hand Trap costs (0) under their Toe Cracker, so the 0 takes it.
      s.expectInZone(EXPERIMENT, "exile");
    });

    it("R65 a deck card at its own cost (a (2) Trap in the deck costs (2))", () => {
      const s = scenario({
        p1: { hand: [{ def: GLITCH, radiant: true }, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [TOE_CRACKER], library: [EXPERIMENT] },
      });
      s.play(GLITCH, { modes: ["0"] });
      s.expectInZone(EXPERIMENT, "library");
      const two = scenario({
        p1: { hand: [{ def: GLITCH, radiant: true }, STOCKPILE] },
        p2: { hand: [STOCKPILE], field: [TOE_CRACKER], library: [EXPERIMENT] },
      });
      two.play(GLITCH, { modes: ["2"] });
      two.expectInZone(EXPERIMENT, "exile");
    });

    it("R396 an X card in a hand or a deck costs 0", () => {
      const s = scenario({
        p1: { hand: [{ def: GLITCH, radiant: true }, STOCKPILE] },
        p2: { hand: [DIVIDEND, STOCKPILE], library: [BILLY] },
      });
      s.play(GLITCH, { modes: ["0"] });
      s.expectInZone(DIVIDEND, "exile");
      s.expectInZone(BILLY, "exile");
    });
  });
});
