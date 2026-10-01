// C #18 Glitch in the System — SPEC §8.6 row 18, BUILD M9 Classic row C 18: "The number is chosen
// with the play (R81) from `GLITCH_NUMBERS`, the same eleven options every time, so they reveal
// nothing; it exiles every card of that cost on both fields, in both hands and in both decks, costs
// read per R65 at resolution (a hand card at its hand cost, a deck or field card at its own; an X card
// its X on the field and 0 anywhere else, 0 too when it arrived without a chosen X, R396), face-down
// and Indestructible cards included; graveyards and exile untouched; the Spell itself is resolving
// and spared; a number nothing costs exiles nothing; the exiled cards are public and no event carries
// a deck position; radiant: the opponent's field, hand and deck only; no tuned numbers".
//
// An X card on the field "played for X": C+ #69 Buff Billy's X stats are E40's (`xStats`), which this
// worktree's engine does not have yet, so the test stands one on the field with its stats given and
// records the X it was played for on the instance, as a play would (`CardInstance.x`, §2.3).

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

    it("exiles every card of that cost on both fields, in both hands and in both decks", () => {
      const s = glitchBoard();
      s.play(GLITCH, { modes: ["2"] });
      expect(felinorsIn(s, "p1")).toEqual({ field: 0, hand: 0, library: 0, graveyard: 1, exile: 4 });
      expect(felinorsIn(s, "p2")).toEqual({ field: 0, hand: 0, library: 0, graveyard: 1, exile: 4 });
      // The face-down (2) Trap and the (2) Field Spell go too.
      expect(s.backrow("p1", 1)).toBeNull();
      expect(s.backrow("p2", 1)).toBeNull();
      // Other costs stay.
      expect(s.unit("p1", 2)?.defId).toBe(MENACE);
      expect(s.unit("p2", 1)?.defId).toBe(VANILLA);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([VANILLA]);
      expect(s.hand("p2").map((card) => card.defId)).toEqual([STOCKPILE]);
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

    it("R65 costs at resolution: a hand card at its hand cost (a (2) Trap under Toe Cracker costs 0)", () => {
      const s = scenario({
        p1: { hand: [GLITCH, EXPERIMENT, { def: MENACE, costMod: -1 }, STOCKPILE], field: [TOE_CRACKER] },
        p2: { hand: [STOCKPILE], library: [REPLENISH] },
      });
      const trap = s.card(EXPERIMENT);
      s.play(GLITCH, { modes: ["0"] });
      s.expectInZone(trap, "exile");
      s.expectInZone(REPLENISH, "exile");
      // The Menace with costMod −1 costs (2) now: not 0, and it stays.
      s.expectInZone(MENACE, "hand");
    });

    it("R65 a deck or field card at its own cost: under Toe Cracker a (2) Trap in the deck or set costs (2)", () => {
      // The aura prices a play, which takes a card from the hand: the deck's Trap and the set one
      // are read at their own (2), so a 0 leaves them and a 2 takes them.
      const board = () =>
        scenario({
          p1: {
            hand: [GLITCH, EXPERIMENT, STOCKPILE],
            field: [TOE_CRACKER],
            backrow: [{ def: EXPERIMENT, faceUp: false }],
            library: [EXPERIMENT],
          },
          p2: { hand: [STOCKPILE] },
        });
      const zero = board();
      const [inHand] = zero.hand("p1").filter((card) => card.defId === EXPERIMENT);
      const set = zero.backrow("p1", 1);
      const [inDeck] = zero.pile("p1", "library");
      zero.play(GLITCH, { modes: ["0"] });
      zero.expectInZone(inHand ?? "missing", "exile");
      zero.expectInZone(set ?? "missing", "field");
      zero.expectInZone(inDeck ?? "missing", "library");

      const two = board();
      const setTwo = two.backrow("p1", 1);
      const [deckTwo] = two.pile("p1", "library");
      const [handTwo] = two.hand("p1").filter((card) => card.defId === EXPERIMENT);
      two.play(GLITCH, { modes: ["2"] });
      two.expectInZone(setTwo ?? "missing", "exile");
      two.expectInZone(deckTwo ?? "missing", "exile");
      two.expectInZone(handTwo ?? "missing", "hand");
      two.expectInZone(TOE_CRACKER, "exile");
    });

    it("R65 a costMod counts wherever the card is: the Menace at (2) goes with a 2", () => {
      const s = scenario({
        p1: { hand: [GLITCH, { def: MENACE, costMod: -1 }, STOCKPILE] },
        p2: { hand: [STOCKPILE], library: [{ def: SEVEN, costMod: -2 }] },
      });
      s.play(GLITCH, { modes: ["2"] });
      s.expectInZone(MENACE, "exile");
      s.expectInZone(SEVEN, "exile");
    });

    it("R396 an X card in a hand or a deck costs 0", () => {
      const s = scenario({
        p1: { hand: [GLITCH, DIVIDEND, STOCKPILE], library: [BILLY] },
        p2: { hand: [DIVIDEND], library: [BILLY] },
      });
      const xCards = [...s.hand("p1").filter((c) => c.defId === DIVIDEND), ...s.pile("p1", "library"), ...s.hand("p2"), ...s.pile("p2", "library")];
      s.play(GLITCH, { modes: ["0"] });
      for (const card of xCards) s.expectInZone(card, "exile");
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

    it("the exiled cards are public to both players and no event carries a deck position", () => {
      const s = glitchBoard();
      s.play(GLITCH, { modes: ["2"] });
      const exiled = s.events.filter((event) => event.type === "exiled");
      expect(exiled).toHaveLength(8);
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
  });
});
