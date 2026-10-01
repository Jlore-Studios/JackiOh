// C #56 Spell Tyrant — SPEC §8.6 row 56, BUILD M9 Classic row C 56: "Cry: choose up to 3 Spells (the
// Spell type) in your graveyard and cast each in turn (free, counted as played, your choices, R70),
// each exiled after it resolves instead of returning to the graveyard; fewer than 3 → those there;
// none → nothing; radiant 10/10: every Spell there as the Cry begins, oldest first, with no choice; a
// Spell a cast puts in the graveyard is not cast; its tuned number (spells) reads through `param()`
// (R386)".
//
// The Spells are Core cards with their own tests: Stockpile (draw 2, heal 2), Friend of Felinors
// (fill your board with Felinor Tokens), 5pek Controller (switch every Unit's position), Lunar Eclipse
// (3 damage to a target, chosen as the cast begins) and Zao Gao (discard 2 at random). Mana Well is a
// Field Spell and Mr. Vanilla a Unit, neither of them a Spell.

import { cardsPlayedThisTurn, legalActions, LIBRARY_CAP, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/056-spell-tyrant";

const TYRANT = "classic-056";
const STOCKPILE = "core-005"; // (1) Spell: draw 2, heal your hero 2.
const FRIENDS = "core-062"; // (1) Spell: fill your board with Felinor Tokens.
const SPEK = "core-048"; // (0) Spell: switch the position of every Unit.
const LUNAR = "core-035"; // (1) Spell: deal 3 damage to a target.
const ZAO_GAO = "core-080"; // (2) Spell: discard 2 random cards; summon 2 Rush Tokens.
const PILE_ON = "classic-060"; // C #60 (5) Spell: Recruit every permanent in your deck; if this would go to your graveyard, bottom of your deck instead.
const MANA_WELL = "core-006"; // Field Spell
const VANILLA = "core-008"; // Unit
const MENACE = "core-019"; // 9/9
const FILLER = "core-016"; // Hit Job: a Spell to hold.
const DECK = [FILLER, FILLER, FILLER, FILLER] as const;

function played(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "cardPlayed" ? [event.defId] : []));
}

function ids(s: Scenario, ...defIds: string[]): string[] {
  return defIds.map((defId) => s.card(defId).id);
}

describe("C #56 Spell Tyrant", () => {
  it("declares its number `spells`; the base face picks, the Radiant face casts them all", () => {
    expect(def.id).toBe(TYRANT);
    expect(def.params).toEqual([{ key: "spells", base: 3, radiant: 3, better: "up", step: 1, min: 1 }]);
    expect(base.resume?.cast).toBeDefined();
    expect(radiant.resume).toBeUndefined();
  });

  describe("base", () => {
    it("Cry: offers you the Spells of your graveyard — no Field Spell, no Unit — up to 3", () => {
      const s = scenario({
        p1: { hand: [TYRANT, FILLER], graveyard: [STOCKPILE, VANILLA, MANA_WELL, SPEK, FRIENDS, LUNAR], library: DECK },
        p2: { hand: [FILLER] },
      });
      s.play(TYRANT);
      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p1");
      expect(pending?.kind).toBe("pick");
      expect(pending?.min).toBe(0);
      expect(pending?.max).toBe(3);
      expect(pending?.options.map((option) => option.selection)).toEqual(
        ids(s, STOCKPILE, SPEK, FRIENDS, LUNAR).map((instanceId) => ({ pick: "instance", instanceId })),
      );
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("legalActions offers only Spells, at most 3, and an answer naming anything else is refused", () => {
      const s = scenario({
        p1: { hand: [TYRANT, FILLER], graveyard: [STOCKPILE, VANILLA, MANA_WELL, SPEK, FRIENDS, LUNAR], library: DECK },
        p2: { hand: [FILLER] },
      });
      s.play(TYRANT);
      const spells = new Set(ids(s, STOCKPILE, SPEK, FRIENDS, LUNAR));
      const answers = legalActions(s.state, "p1").flatMap((action) => (action.type === "answer" ? [action.selection] : []));
      expect(answers.length).toBeGreaterThan(0);
      for (const selection of answers) {
        expect(selection.length).toBeLessThanOrEqual(3);
        for (const each of selection) expect(each.pick === "instance" && spells.has(each.instanceId)).toBe(true);
      }
      expect(() => s.answer(ids(s, VANILLA))).toThrow();
      expect(() => s.answer(ids(s, MANA_WELL))).toThrow();
      expect(() => s.answer(ids(s, STOCKPILE, SPEK, FRIENDS, LUNAR))).toThrow();
      expect(s.state.pending?.kind).toBe("pick");
    });

    it("R70 casts each pick in turn — free and counted as played — and exiles each after it resolves", () => {
      const s = scenario({
        p1: { hand: [TYRANT, FILLER], graveyard: [STOCKPILE, SPEK, FRIENDS], library: DECK, health: 20 },
        p2: { hand: [FILLER] },
      });
      s.play(TYRANT);
      s.answer(ids(s, STOCKPILE, FRIENDS));
      expect(played(s)).toEqual([TYRANT, STOCKPILE, FRIENDS]);
      expect(cardsPlayedThisTurn(s.state, "p1")).toBe(3);
      s.expectMana("p1", 0);
      // Stockpile drew 2 and healed 2; Friend of Felinors filled the other four zones.
      s.expectHealth("p1", 22);
      expect(s.hand("p1")).toHaveLength(3);
      expect([2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId)).toEqual(Array(4).fill("core-t-felinor"));
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([STOCKPILE, FRIENDS]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([SPEK]);
    });

    it("R221 the picks are cast in the order the graveyard holds them, oldest first, however the answer lists them", () => {
      const s = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: [SPEK, STOCKPILE, FRIENDS], library: DECK }, p2: { hand: [FILLER] } });
      s.play(TYRANT);
      s.answer(ids(s, FRIENDS, SPEK));
      expect(played(s)).toEqual([TYRANT, SPEK, FRIENDS]);
    });

    it("\"up to\": you may cast fewer, or none, and the rest stay in your graveyard", () => {
      const s = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: [STOCKPILE, SPEK], library: DECK }, p2: { hand: [FILLER] } });
      s.play(TYRANT);
      s.answer([]);
      expect(played(s)).toEqual([TYRANT]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([STOCKPILE, SPEK]);
    });

    it("fewer than 3 Spells there: the pick offers those there", () => {
      const s = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: [STOCKPILE, VANILLA], library: DECK }, p2: { hand: [FILLER] } });
      s.play(TYRANT);
      expect(s.state.pending?.max).toBe(1);
      s.answer(ids(s, STOCKPILE));
      s.expectInZone(STOCKPILE, "exile");
    });

    it("none there: nothing is asked and nothing is cast", () => {
      const s = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: [VANILLA, MANA_WELL] }, p2: { hand: [FILLER] } });
      s.play(TYRANT);
      expect(s.state.pending).toBeNull();
      expect(played(s)).toEqual([TYRANT]);
      s.expectInZone(TYRANT, "field");
    });

    it("R70 a cast Spell's choices are yours, asked as its cast begins; then the next pick is cast", () => {
      const s = scenario({
        p1: { hand: [TYRANT, FILLER], graveyard: [LUNAR, STOCKPILE], library: DECK },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.play(TYRANT);
      s.answer(ids(s, LUNAR, STOCKPILE));
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer(s.card(MENACE).id);
      s.expectStats(MENACE, { health: 6 });
      expect(played(s)).toEqual([TYRANT, LUNAR, STOCKPILE]);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([LUNAR, STOCKPILE]);
    });

    it("a cast paused on its choice survives a round trip: the frozen state answers to the same game", () => {
      const s = scenario({
        p1: { hand: [TYRANT, FILLER], graveyard: [LUNAR, STOCKPILE], library: DECK },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.play(TYRANT);
      s.answer(ids(s, LUNAR, STOCKPILE));
      const pending = s.state.pending;
      expect(pending).not.toBeNull();
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const answer = {
        type: "answer",
        choiceId: pending?.id ?? "",
        selection: [{ pick: "hero", player: "p2" }],
        playerId: "p1",
        nonce: "tyrant-roundtrip",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(frozen.events).toEqual(live.events);
      expect(live.state.players.p2.hero.health).toBe(27);
      expect(live.state.players.p1.exile.map((card) => card.defId)).toEqual([LUNAR, STOCKPILE]);
    });

    it("a cast it exiles never goes to the graveyard: C #60 Pile On's return to the deck does not apply", () => {
      // A full deck turns Pile On's return away (R80), which is how it reaches the graveyard to start
      // with; the cast's Recruit then makes room, so only the exile keeps it out of the deck.
      const full = [VANILLA, ...Array.from({ length: LIBRARY_CAP - 1 }, () => FILLER)];
      const s = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: [PILE_ON], library: full }, p2: { hand: [FILLER] } });
      s.expectInZone(PILE_ON, "graveyard");
      s.play(TYRANT);
      s.answer(ids(s, PILE_ON));
      expect(played(s)).toEqual([TYRANT, PILE_ON]);
      expect(s.unit("p1", 2)?.defId).toBe(VANILLA);
      s.expectInZone(PILE_ON, "exile");
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP - 1);
    });

    it("R386 an Upgrade offers 4 picks; a Degrade 2", () => {
      const grave = [STOCKPILE, SPEK, FRIENDS, LUNAR, STOCKPILE];
      const up = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: grave, library: DECK }, p2: { hand: [FILLER] } });
      stepParam(up.card(TYRANT), "spells", 1);
      up.play(TYRANT);
      expect(up.state.pending?.max).toBe(4);

      const down = scenario({ p1: { hand: [TYRANT, FILLER], graveyard: grave, library: DECK }, p2: { hand: [FILLER] } });
      stepParam(down.card(TYRANT), "spells", -1);
      down.play(TYRANT);
      expect(down.state.pending?.max).toBe(2);
    });
  });

  describe("radiant", () => {
    it("casts every Spell in your graveyard as the Cry begins, oldest first, with no choice, and exiles each", () => {
      const s = scenario({
        p1: { hand: [{ def: TYRANT, radiant: true }, FILLER], graveyard: [STOCKPILE, VANILLA, SPEK, MANA_WELL, FRIENDS], library: DECK },
        p2: { hand: [FILLER] },
      });
      s.play(TYRANT);
      expect(played(s)).toEqual([TYRANT, STOCKPILE, SPEK, FRIENDS]);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([STOCKPILE, SPEK, FRIENDS]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([VANILLA, MANA_WELL]);
      s.expectStats(TYRANT, { attack: 10, health: 10 });
      expect(cardsPlayedThisTurn(s.state, "p1")).toBe(4);
    });

    it("a Spell a cast puts in the graveyard is not cast: Zao Gao's discarded Lunar Eclipse stays there", () => {
      const s = scenario({
        p1: { hand: [{ def: TYRANT, radiant: true }, LUNAR, VANILLA], graveyard: [ZAO_GAO] },
        p2: { hand: [FILLER] },
      });
      s.play(TYRANT);
      expect(played(s)).toEqual([TYRANT, ZAO_GAO]);
      s.expectInZone(LUNAR, "graveyard").expectInZone(ZAO_GAO, "exile");
    });

    it("R70 a cast Spell's choices are still yours", () => {
      const s = scenario({
        p1: { hand: [{ def: TYRANT, radiant: true }, FILLER], graveyard: [LUNAR] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.play(TYRANT);
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer(s.card(MENACE).id);
      s.expectStats(MENACE, { health: 6 });
      s.expectInZone(LUNAR, "exile");
    });

    it("with no Spell in your graveyard it casts nothing", () => {
      const s = scenario({ p1: { hand: [{ def: TYRANT, radiant: true }, FILLER], graveyard: [VANILLA] }, p2: { hand: [FILLER] } });
      s.play(TYRANT);
      expect(played(s)).toEqual([TYRANT]);
      expect(s.state.pending).toBeNull();
    });
  });
});
