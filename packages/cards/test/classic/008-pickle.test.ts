// C #8 Pickle — SPEC §8.6 row 8, BUILD M9 Classic row C 8: "Three mode prompts held by the opponent
// during your turn, one after another, repeats allowed: they discard a card at random (R654),
// they exile the bottom card of their deck, or you draw 1; a mode that would do nothing is not
// offered (discard with an empty hand, exile with an empty deck), and "you draw" always is, fatigue
// included; each prompt runs its own clock and a timeout answers it with the AI policy (R79); your
// view names none of their remaining hand, their view never names your drawn card, and the mode
// options name no card; the paused prompts survive a JSON round trip; radiant: discard 2, exile the
// bottom 2, or draw 2; its tuned numbers (choices, discard, exile, draw) read through `param()`
// (R386)".

import { describe, expect, it } from "vitest";
import { reduce, stepParam, type GameState, type PendingChoice } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/008-pickle";

const PICKLE = "classic-008";
const FILLER = "core-005"; // (1) Spell, p1's spare card (§2.5).
// p1's library: distinct definitions, top first.
const MY_DECK = ["core-019", "core-025", "core-012", "core-020"] as const;
// p2's hand and library: definitions p1 holds nowhere.
const THEIR_HAND = ["core-008", "core-011", "core-015"] as const;
const THEIR_DECK = ["core-006", "core-035", "core-048", "core-039"] as const;

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the scenario has no ${what}`);
  return value;
}

function open(s: Scenario): PendingChoice {
  return must(s.state.pending, "open prompt");
}

function modes(s: Scenario): string[] {
  return open(s).options.map((option) => (option.selection.pick === "mode" ? option.selection.option : "?"));
}

function pickle(
  radiantFace = false,
  piles: { myDeck?: readonly string[]; theirHand?: readonly string[]; theirDeck?: readonly string[] } = {},
): Scenario {
  return scenario({
    p1: { hand: [{ def: PICKLE, radiant: radiantFace }, FILLER], library: [...(piles.myDeck ?? MY_DECK)] },
    p2: { hand: [...(piles.theirHand ?? THEIR_HAND)], library: [...(piles.theirDeck ?? THEIR_DECK)] },
  });
}

function defs(s: Scenario, player: "p1" | "p2", zone: "hand" | "library" | "graveyard" | "exile"): string[] {
  return s.pile(player, zone).map((card) => card.defId);
}

describe("C #8 Pickle", () => {
  it("declares its four numbers (R386): choices 3; discard, exile, draw 1 (Radiant 2)", () => {
    expect(def.params).toEqual([
      { key: "choices", base: 3, radiant: 3, better: "up", step: 1, min: 1 },
      { key: "discard", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "exile", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    // The same script on both faces: only the numbers differ.
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("E18 the first question is a mode prompt your opponent holds during your turn, offering all three", () => {
      const s = pickle();
      s.play(PICKLE);
      const pending = open(s);
      expect(pending.playerId).toBe("p2");
      expect(pending.kind).toBe("mode");
      expect(modes(s)).toEqual(["discard", "exile", "draw"]);
      expect(s.state.active).toBe("p1");
    });

    it("three questions, one after another, repeats allowed: three draws draw you three cards", () => {
      const s = pickle();
      s.play(PICKLE);
      for (let question = 0; question < 3; question += 1) {
        expect(open(s).playerId).toBe("p2");
        s.answer("draw");
      }
      expect(s.state.pending).toBeNull();
      expect(defs(s, "p1", "hand")).toEqual([FILLER, ...MY_DECK.slice(0, 3)]);
      s.expectInZone(PICKLE, "graveyard");
    });

    it("R654 discard discards a random card of theirs at once, with no hand pick", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("discard");
      // No follow-up pick: the random discard landed and the next question is open.
      expect(open(s).kind).toBe("mode");
      const grave = defs(s, "p2", "graveyard");
      expect(grave).toHaveLength(1);
      expect([...THEIR_HAND]).toContain(grave[0]);
      s.expectEvents("discarded");
    });

    it("exile takes the bottom card of their deck", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("exile");
      expect(defs(s, "p2", "exile")).toEqual([THEIR_DECK[3]]);
      expect(defs(s, "p2", "library")).toEqual(THEIR_DECK.slice(0, 3));
    });

    it("a mix: discard, exile, draw, in any order", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("exile");
      s.answer("discard");
      s.answer("draw");
      expect(s.state.pending).toBeNull();
      expect(defs(s, "p2", "exile")).toEqual([THEIR_DECK[3]]);
      const grave = defs(s, "p2", "graveyard");
      expect(grave).toHaveLength(1);
      expect([...THEIR_HAND]).toContain(grave[0]);
      expect(defs(s, "p1", "hand")).toEqual([FILLER, MY_DECK[0]]);
    });

    it("discard is not offered while their hand is empty, nor exile while their deck is", () => {
      const s = pickle(false, { theirHand: [], theirDeck: [] });
      s.play(PICKLE);
      expect(modes(s)).toEqual(["draw"]);
    });

    it("each question reads their piles as it is asked: an exile that empties their deck takes exile off the next", () => {
      const s = pickle(false, { theirDeck: ["core-006"] });
      s.play(PICKLE);
      expect(modes(s)).toEqual(["discard", "exile", "draw"]);
      s.answer("exile");
      expect(modes(s)).toEqual(["discard", "draw"]);
    });

    it("a discard that empties their hand takes discard off the next question", () => {
      const s = pickle(false, { theirHand: ["core-008"] });
      s.play(PICKLE);
      s.answer("discard");
      expect(defs(s, "p2", "graveyard")).toEqual(["core-008"]);
      expect(modes(s)).toEqual(["exile", "draw"]);
    });

    it("§2.4 you draw is always offered, and from an empty deck it is fatigue", () => {
      const s = pickle(false, { myDeck: [], theirHand: [], theirDeck: [] });
      s.play(PICKLE);
      s.answer("draw");
      s.answer("draw");
      s.answer("draw");
      expect(s.state.pending).toBeNull();
      // Three fatigue draws: 1 + 2 + 3.
      s.expectHealth("p1", 24);
    });

    it("R79 each question runs its opponent's own clock: a timeout answers the one open with the AI policy", () => {
      const s = pickle();
      s.play(PICKLE);
      const first = open(s);
      const result = reduce(s.state, { type: "timeout", playerId: "p2", nonce: "pickle-timeout" });
      expect(result.error).toBeUndefined();
      // The first question was answered; the next is open for p2 (or its discard pick), p1 still active.
      const next = must(result.state.pending, "a following prompt");
      expect(next.id).not.toBe(first.id);
      expect(next.playerId).toBe("p2");
      expect(result.state.active).toBe("p1");
      expect(result.events.filter((event) => event.type === "promptAnswered")).toHaveLength(1);
    });

    it("R177 your view names none of their remaining hand", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("discard");
      const mine = JSON.stringify(s.view("p1"));
      for (const card of s.hand("p2")) {
        expect(mine).not.toContain(`"${card.id}"`);
        expect(mine).not.toContain(card.defId);
      }
      // The random discard itself is public: the graveyard names it.
      for (const card of s.pile("p2", "graveyard")) expect(mine).toContain(card.id);
    });

    it("R97 their view never names the card you draw", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("draw");
      const drawn = s.card(MY_DECK[0]);
      s.expectInZone(drawn, "hand");
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(`"${drawn.id}"`);
      expect(theirs).not.toContain(MY_DECK[0]);
    });

    it("the mode options name no card: only what each would do", () => {
      const s = pickle();
      s.play(PICKLE);
      const theirs = must(s.view("p2").pending, "p2's view of the question");
      expect(theirs.forYou).toBe(true);
      const labels = open(s).options.map((option) => option.label);
      expect(labels).toEqual(["Discard 1 card", "Exile the bottom 1 card of your deck", "Your opponent draws 1 card"]);
      // p1 reads only that p2 is choosing.
      expect(s.view("p1").pending).toEqual({ forYou: false, pendingFor: "p2" });
    });

    it("§9.3 the paused questions survive a JSON round trip and resume through reduce", () => {
      const s = pickle();
      s.play(PICKLE);
      s.answer("draw");
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const second = must(revived.pending, "the second question");
      const answered = reduce(revived, {
        type: "answer",
        playerId: "p2",
        choiceId: second.id,
        selection: [{ pick: "mode", option: "exile" }],
        nonce: "pickle-round-trip-1",
      });
      expect(answered.error).toBeUndefined();
      const third = must(answered.state.pending, "the third question");
      expect(third.playerId).toBe("p2");
      // And the discard pick, paused in turn, round-trips too.
      const again = JSON.parse(JSON.stringify(answered.state)) as GameState;
      const picked = reduce(again, {
        type: "answer",
        playerId: "p2",
        choiceId: third.id,
        selection: [{ pick: "mode", option: "draw" }],
        nonce: "pickle-round-trip-2",
      });
      expect(picked.error).toBeUndefined();
      expect(picked.state.pending).toBeNull();
      expect(picked.state.work).toEqual([]);
    });

    it("R654 the random discard comes from the match rng: the same game discards the same card", () => {
      const first = pickle();
      first.play(PICKLE);
      first.answer("discard");
      const second = pickle();
      second.play(PICKLE);
      second.answer("discard");
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) => (event.type === "discarded" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("R386 an Upgrade of choices asks a fourth question", () => {
      const s = pickle();
      stepParam(s.card(PICKLE), "choices", 1);
      s.play(PICKLE);
      for (let question = 0; question < 4; question += 1) s.answer("draw");
      expect(s.state.pending).toBeNull();
      expect(defs(s, "p1", "hand")).toEqual([FILLER, ...MY_DECK]);
    });

    it("R386 an Upgrade of draw makes you draw 2", () => {
      const s = pickle();
      stepParam(s.card(PICKLE), "draw", 1);
      s.play(PICKLE);
      s.answer("draw");
      expect(defs(s, "p1", "hand")).toEqual([FILLER, MY_DECK[0], MY_DECK[1]]);
    });

    it("R386 an Upgrade of discard makes them discard 2, and of exile exiles 2", () => {
      const s = pickle();
      stepParam(s.card(PICKLE), "discard", 1);
      stepParam(s.card(PICKLE), "exile", 1);
      s.play(PICKLE);
      s.answer("discard");
      const grave = defs(s, "p2", "graveyard");
      expect(grave).toHaveLength(2);
      for (const defId of grave) expect([...THEIR_HAND]).toContain(defId);
      s.answer("exile");
      expect(defs(s, "p2", "exile")).toEqual([THEIR_DECK[3], THEIR_DECK[2]]);
    });

    it("R386 a Degrade of choices asks only two questions", () => {
      const s = pickle();
      stepParam(s.card(PICKLE), "choices", -1);
      s.play(PICKLE);
      s.answer("draw");
      s.answer("draw");
      expect(s.state.pending).toBeNull();
    });
  });

  describe("radiant", () => {
    it("R654 they discard 2 cards at random", () => {
      const s = pickle(true);
      s.play(PICKLE);
      expect(open(s).options.map((option) => option.label)).toEqual([
        "Discard 2 cards",
        "Exile the bottom 2 cards of your deck",
        "Your opponent draws 2 cards",
      ]);
      s.answer("discard");
      expect(open(s).kind).toBe("mode");
      const grave = defs(s, "p2", "graveyard");
      expect(grave).toHaveLength(2);
      for (const defId of grave) expect([...THEIR_HAND]).toContain(defId);
    });

    it("a hand of one card discards that one", () => {
      const s = pickle(true, { theirHand: ["core-008"] });
      s.play(PICKLE);
      s.answer("discard");
      expect(s.hand("p2")).toEqual([]);
      expect(defs(s, "p2", "graveyard")).toEqual(["core-008"]);
    });

    it("they exile the bottom 2 cards of their deck", () => {
      const s = pickle(true);
      s.play(PICKLE);
      s.answer("exile");
      expect(defs(s, "p2", "exile")).toEqual([THEIR_DECK[3], THEIR_DECK[2]]);
    });

    it("you draw 2", () => {
      const s = pickle(true);
      s.play(PICKLE);
      s.answer("draw");
      expect(defs(s, "p1", "hand")).toEqual([FILLER, MY_DECK[0], MY_DECK[1]]);
    });

    it("an exile of 2 that empties their deck takes exile off the next question", () => {
      const s = pickle(true, { theirDeck: ["core-006", "core-035"] });
      s.play(PICKLE);
      s.answer("exile");
      expect(modes(s)).toEqual(["discard", "draw"]);
    });
  });
});
