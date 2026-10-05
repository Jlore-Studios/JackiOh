// C+ #39 Book Worm — SPEC §8.7 row 39, BUILD M9 Classic+ row C+ 39: "It starts with no Plague Tokens
// (balance patch 1: no N counter — the tokens stacked on itself are the count); at each start of its
// controller's turn one Plague Token is placed on it; Death adds one random non-token Book of any set
// (R380, repeats allowed) per token, reading the tokens last-known (R78, R89), a full hand burning the
// rest; bounced and played again it starts at none; the public token count is the preview both players
// see; no tunable; radiant the Books are Radiant".

import { HAND_CAP, defOf, stepParam, type CardInstance } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const WORM = "classicplus-039";
const MENACE = "core-019"; // 9/9: the worm dies attacking it.
const FLOOD = "core-017"; // (4) Spell: bounce all Units.
const FILLER = "core-005";

function onField(radiant = false, hand: readonly string[] = [FILLER]): Scenario {
  return scenario({
    p1: { hand, field: [{ def: WORM, radiant }], library: [FILLER, FILLER, FILLER, FILLER] },
    p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER, FILLER, FILLER] },
  });
}

/** Two `endTurn`s: the opponent's turn and back to the start of ours. */
function nextOwnTurn(s: Scenario): Scenario {
  return s.endTurn().endTurn();
}

function booksIn(s: Scenario, before: readonly string[]): string[] {
  return s
    .hand("p1")
    .filter((card) => !before.includes(card.id))
    .map((card) => card.defId);
}

/** The Plague Tokens stacked on the Worm (§10.1, public in every view). */
function tokensOf(s: Scenario, card: CardInstance): number {
  return s.card(card).counters.plague ?? 0;
}

describe("C+ #39 Book Worm", () => {
  describe("base", () => {
    it("it starts with no Plague Tokens: dying at once adds no Book", () => {
      const s = onField();
      expect(tokensOf(s, s.card(WORM))).toBe(0);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      s.expectInZone(WORM, "graveyard");
      expect(booksIn(s, before)).toHaveLength(0);
    });

    it("R380 the Books are random non-token Book cards of any set, never the Worm", () => {
      const sets = new Set<string>();
      for (const seed of ["bw-1", "bw-2", "bw-3", "bw-4", "bw-5", "bw-6", "bw-7", "bw-8"]) {
        const s = scenario({
          seed,
          p1: { hand: [FILLER], field: [WORM], library: [FILLER] },
          p2: { hand: [FILLER], field: [MENACE] },
        });
        nextOwnTurn(s); // One token: one Book.
        expect(tokensOf(s, s.card(WORM))).toBe(1);
        const before = s.hand("p1").map((card) => card.id);
        s.attack(WORM, MENACE);
        const added = booksIn(s, before);
        expect(added).toHaveLength(1);
        for (const id of added) {
          const def = defOf(s.state, id);
          expect(def.tags).toContain("Book");
          expect(def.token).toBe(false);
          expect(id).not.toBe(WORM);
          sets.add(def.set);
        }
      }
      expect(sets.size).toBeGreaterThan(1);
    });

    it("one token lands at each start of its controller's turn, never the opponent's", () => {
      const s = onField(false, [FILLER, FILLER]);
      const worm = s.card(WORM);
      s.endTurn(); // p2's start of turn: no token.
      expect(tokensOf(s, worm)).toBe(0);
      s.endTurn(); // p1's: the first token.
      expect(tokensOf(s, worm)).toBe(1);
      nextOwnTurn(s); // The second.
      expect(tokensOf(s, worm)).toBe(2);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(2);
    });

    it("R78 R89 Death reads the tokens last-known: the count it had as it died", () => {
      const s = onField(false, [FILLER, FILLER]);
      nextOwnTurn(s);
      nextOwnTurn(s);
      expect(tokensOf(s, s.card(WORM))).toBe(2);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(2);
    });

    it("§2.4 a full hand burns the Books that do not fit", () => {
      const hand = Array.from({ length: HAND_CAP - 1 }, () => FILLER);
      const s = onField(false, hand);
      nextOwnTurn(s); // One token; the turn's draw fills the hand to the cap.
      expect(tokensOf(s, s.card(WORM))).toBe(1);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      s.attack(WORM, MENACE);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R78 bounced and played again it starts at no tokens", () => {
      const s = onField(false, [FLOOD, FILLER, FILLER]);
      nextOwnTurn(s); // One token.
      expect(tokensOf(s, s.card(WORM))).toBe(1);
      s.play(FLOOD);
      s.expectInZone(WORM, "hand");
      // With no mana left the turn may end by itself (§2.5); come round to p1's next turn either way.
      const turn = s.state.turn;
      while (s.state.active !== "p1" || s.state.turn === turn) s.endTurn();
      s.play(WORM);
      expect(tokensOf(s, s.card(WORM))).toBe(0);
    });

    it("R280 the token count is public, on the field, to both players; in hand it reads no tokens", () => {
      const s = scenario({
        p1: { hand: [WORM, FILLER], field: [WORM], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER] },
      });
      nextOwnTurn(s);
      expect(s.view("p1").you.units[0]?.counters?.plague).toBe(1);
      expect(s.view("p2").opponent.units[0]?.counters?.plague).toBe(1);
      const inHand = s.view("p1").you.hand;
      if (!Array.isArray(inHand)) throw new Error("own hand in full");
      const wormInHand = inHand.find((card) => card.defId === WORM);
      expect(wormInHand === undefined || !("counters" in wormInHand)).toBe(true);
      // The text names what the counters are.
      expect(defOf(s.state, WORM).base.text).toContain("Plague Token");
      // What the counters say is what the Death then adds.
      const before = s.hand("p1").map((card) => card.id);
      s.attack(s.unit("p1", 1)?.id ?? "", MENACE);
      expect(booksIn(s, before)).toHaveLength(1);
    });

    it("R97 the Books it adds are hidden from the opponent", () => {
      const s = onField();
      nextOwnTurn(s);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      const added = s.hand("p1").filter((card) => !before.includes(card.id));
      expect(added).toHaveLength(1);
      const shown = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(shown.length).toBeGreaterThan(0);
      for (const event of shown) {
        if (event.type !== "addedToHand") continue;
        expect(added.map((card) => card.defId)).not.toContain(event.defId);
      }
    });

    it("R386 there is no tunable growth: an Upgrade still stacks one token a turn", () => {
      const s = onField(false, [FILLER, FILLER]);
      stepParam(s.card(WORM), "growth", 1);
      nextOwnTurn(s);
      expect(tokensOf(s, s.card(WORM))).toBe(1);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("the Books it adds are Radiant, one per token", () => {
      const s = onField(true, [FILLER, FILLER]);
      nextOwnTurn(s);
      nextOwnTurn(s);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      const added = s.hand("p1").filter((card) => !before.includes(card.id));
      expect(added).toHaveLength(2);
      expect(added.every((card) => card.radiant && defOf(s.state, card.defId).tags.includes("Book"))).toBe(true);
    });

    it("R280 its token count is public on the Radiant face too", () => {
      const s = onField(true, [FILLER, FILLER]);
      nextOwnTurn(s);
      expect(s.view("p1").you.units[0]?.counters?.plague).toBe(1);
    });
  });
});
