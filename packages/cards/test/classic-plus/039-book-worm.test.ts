// C+ #39 Book Worm — SPEC §8.7 row 39, BUILD M9 Classic+ row C+ 39: "Its count N is 1 as it arrives
// and rises by 1 at each start of its controller's turn (`counters.books`); Death adds N random
// non-token Books of any set (R380, repeats allowed), reading N last-known (R78), a full hand burning
// the rest; bounced and played again it starts at 1; its preview is N (R280); the growth reads through
// `param()`; radiant the Books are Radiant".
//
// N is kept in the instance's memory (§10.1), which R78 clears as the card leaves the field, exactly
// as it would clear a counter; the preview proofs (R280) are below, in this file.

import { HAND_CAP, defOf, stepParam } from "@jackioh/engine";
import type { CardView } from "@jackioh/shared";
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

function previewOf(view: CardView | null | undefined): number | undefined {
  return view?.preview?.find((entry) => entry.label === "N")?.value;
}

describe("C+ #39 Book Worm", () => {
  describe("base", () => {
    it("N is 1 as it arrives: dying at once it adds 1 Book", () => {
      const s = onField();
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      s.expectInZone(WORM, "graveyard");
      const added = booksIn(s, before);
      expect(added).toHaveLength(1);
    });

    it("R380 the Books are random non-token Book cards of any set, never the Worm", () => {
      const sets = new Set<string>();
      for (const seed of ["bw-1", "bw-2", "bw-3", "bw-4", "bw-5", "bw-6", "bw-7", "bw-8"]) {
        const s = scenario({
          seed,
          p1: { hand: [FILLER], field: [WORM], library: [FILLER] },
          p2: { hand: [FILLER], field: [MENACE] },
        });
        const before = s.hand("p1").map((card) => card.id);
        s.attack(WORM, MENACE);
        for (const id of booksIn(s, before)) {
          const def = defOf(s.state, id);
          expect(def.tags).toContain("Book");
          expect(def.token).toBe(false);
          expect(id).not.toBe(WORM);
          sets.add(def.set);
        }
      }
      expect(sets.size).toBeGreaterThan(1);
    });

    it("N rises by 1 at each start of its controller's turn, never the opponent's", () => {
      const s = onField(false, [FILLER, FILLER]);
      s.endTurn(); // p2's start of turn: no growth.
      expect(previewOf(s.view("p1").you.units[0])).toBe(1);
      s.endTurn(); // p1's: N = 2.
      expect(previewOf(s.view("p1").you.units[0])).toBe(2);
      nextOwnTurn(s); // N = 3.
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(3);
    });

    it("R78 Death reads N last-known: the count it had as it died", () => {
      const s = onField(false, [FILLER, FILLER]);
      nextOwnTurn(s);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(2);
      expect(s.card(WORM).memory).toEqual({});
    });

    it("§2.4 a full hand burns the Books that do not fit", () => {
      const hand = Array.from({ length: HAND_CAP - 1 }, () => FILLER);
      const s = onField(false, hand);
      nextOwnTurn(s); // N = 2; the turn's draw fills the hand to the cap.
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      s.attack(WORM, MENACE);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(2);
    });

    it("R78 bounced and played again it starts at 1", () => {
      const s = onField(false, [FLOOD, FILLER, FILLER]);
      nextOwnTurn(s); // N = 2.
      expect(previewOf(s.view("p1").you.units[0])).toBe(2);
      s.play(FLOOD);
      s.expectInZone(WORM, "hand");
      expect(s.card(WORM).memory).toEqual({});
      // With no mana left the turn may end by itself (§2.5); come round to p1's next turn either way.
      const turn = s.state.turn;
      while (s.state.active !== "p1" || s.state.turn === turn) s.endTurn();
      s.play(WORM);
      expect(previewOf(s.view("p1").you.units[0])).toBe(1);
    });

    it("R280 its preview is N, on the field, to both players; in hand it reads 1", () => {
      const s = scenario({
        p1: { hand: [WORM, FILLER], field: [WORM], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER] },
      });
      nextOwnTurn(s);
      const field = s.view("p1").you.units[0];
      expect(previewOf(field)).toBe(2);
      expect(previewOf(s.view("p2").opponent.units[0])).toBe(2);
      const inHand = s.view("p1").you.hand;
      if (!Array.isArray(inHand)) throw new Error("own hand in full");
      expect(previewOf(inHand.find((card) => card.defId === WORM))).toBe(1);
      // The label is a substring of the face's text (R280).
      expect(defOf(s.state, WORM).base.text).toContain("N");
      // What the preview says is what the Death then adds.
      const before = s.hand("p1").map((card) => card.id);
      s.attack(s.unit("p1", 1)?.id ?? "", MENACE);
      expect(booksIn(s, before)).toHaveLength(2);
    });

    it("R97 the Books it adds are hidden from the opponent", () => {
      const s = onField();
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      const added = s.hand("p1").filter((card) => !before.includes(card.id));
      const shown = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(shown.length).toBeGreaterThan(0);
      for (const event of shown) {
        if (event.type !== "addedToHand") continue;
        expect(added.map((card) => card.defId)).not.toContain(event.defId);
      }
    });

    it("R386 the growth reads through param(): an Upgrade grows N by 2 a turn", () => {
      const s = onField(false, [FILLER, FILLER]);
      stepParam(s.card(WORM), "growth", 1);
      nextOwnTurn(s);
      expect(previewOf(s.view("p1").you.units[0])).toBe(3);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      expect(booksIn(s, before)).toHaveLength(3);
    });
  });

  describe("radiant", () => {
    it("the Books it adds are Radiant, N of them", () => {
      const s = onField(true, [FILLER, FILLER]);
      nextOwnTurn(s);
      const before = s.hand("p1").map((card) => card.id);
      s.attack(WORM, MENACE);
      const added = s.hand("p1").filter((card) => !before.includes(card.id));
      expect(added).toHaveLength(2);
      expect(added.every((card) => card.radiant && defOf(s.state, card.defId).tags.includes("Book"))).toBe(true);
    });

    it("R280 its preview is N on the Radiant face too", () => {
      const s = onField(true, [FILLER, FILLER]);
      nextOwnTurn(s);
      expect(previewOf(s.view("p1").you.units[0])).toBe(2);
    });
  });
});
