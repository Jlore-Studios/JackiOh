// C #30 Recycle — SPEC §8.6 row 30, BUILD M9 Classic row C 30: "Shuffles every card in your graveyard
// into your deck at random positions (this Spell is resolving, not there, and lands in the graveyard
// after); R80's cap: cards that don't fit stay in the graveyard; the positions are blank in both views
// and your deck list names the cards (R311); then draw 1; an empty graveyard only draws; radiant: each
// shuffled card costs (1) less (`costMod`, kept, R78); its name is a rules word, and C #64's
// "Recycler" is no reference to it (R381); its tuned numbers (draw, radiant discount) read through
// `param()` (R386)".

import { LIBRARY_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { CATALOG } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/030-recycle";

const RECYCLE = "classic-030";
const FILLER = "core-005";
const X = "core-008";
const Y = "core-011";
const A = "core-019";
const B = "core-001";
const C = "core-020";

function ids(cards: readonly { id: string }[]): string[] {
  return cards.map((card) => card.id);
}

function defs(cards: readonly { defId: string }[]): string[] {
  return cards.map((card) => card.defId);
}

function libraryAndHand(s: Scenario): string[] {
  return [...ids(s.pile("p1", "library")), ...ids(s.hand("p1"))].sort();
}

describe("C #30 Recycle", () => {
  it("has a script per face", () => {
    expect(def.id).toBe(RECYCLE);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  it("R381 its name is a rules word: C #64 Malzahar's Recycler names no card, least of all this one", () => {
    expect(CATALOG["classic-064"]?.refs ?? []).not.toContain(RECYCLE);
    expect(Object.values(CATALOG).filter((card) => card.name === "Recycle").map((card) => card.id)).toEqual([RECYCLE]);
  });

  describe("base", () => {
    it("shuffles every graveyard card into the deck, then draws 1; the Spell itself lands in the graveyard after", () => {
      const s = scenario({ p1: { hand: [RECYCLE], library: [X, Y], graveyard: [A, B, C] }, p2: { hand: [FILLER] } });
      const before = [...ids(s.pile("p1", "library")), ...ids(s.pile("p1", "graveyard"))].sort();

      s.play(RECYCLE);

      expect(defs(s.pile("p1", "graveyard"))).toEqual([RECYCLE]);
      expect(s.pile("p1", "library")).toHaveLength(4);
      expect(s.hand("p1")).toHaveLength(1);
      expect(libraryAndHand(s)).toEqual(before);
      expect(s.events.filter((event) => event.type === "shuffledIn")).toHaveLength(3);
      s.expectEvents("shuffledIn", "shuffledIn", "shuffledIn", "drawn");
    });

    it("R78 each card keeps its id and its costMod", () => {
      const s = scenario({ p1: { hand: [RECYCLE], library: [X], graveyard: [{ def: A, costMod: 1 }] }, p2: { hand: [FILLER] } });
      const card = s.card(A);

      s.play(RECYCLE);

      expect(s.card(card.id).costMod).toBe(1);
      expect(["library", "hand"]).toContain(s.card(card.id).zone.z);
    });

    it("an empty graveyard only draws", () => {
      const s = scenario({ p1: { hand: [RECYCLE], library: [X, Y] }, p2: { hand: [FILLER] } });

      s.play(RECYCLE);

      expect(defs(s.hand("p1"))).toEqual([X]);
      expect(defs(s.pile("p1", "library"))).toEqual([Y]);
      expect(s.events.filter((event) => event.type === "shuffledIn")).toHaveLength(0);
    });

    it("R80 R316 a card the full deck turns away stays in the graveyard, reported by libraryOverflow", () => {
      const s = scenario({
        p1: { hand: [RECYCLE], library: Array.from({ length: LIBRARY_CAP - 1 }, () => X), graveyard: [A, B] },
        p2: { hand: [FILLER] },
      });
      const turnedAway = s.card(B);

      s.play(RECYCLE);

      expect(defs(s.pile("p1", "graveyard"))).toEqual([B, RECYCLE]);
      expect(s.pile("p1", "graveyard")[0]?.id).toBe(turnedAway.id);
      const overflow = s.events.filter((event) => event.type === "libraryOverflow");
      expect(overflow).toEqual([{ type: "libraryOverflow", player: "p1", instanceId: turnedAway.id, defId: B, outcome: "graveyard" }]);
      expect(s.events.filter((event) => event.type === "enteredGraveyard" && event.instanceId === turnedAway.id)).toHaveLength(0);
      // One card went in (60), then the draw took one out.
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP - 1);
    });

    it("R97 R311 the positions are blank in both views, and your own deck list names the cards", () => {
      const s = scenario({ p1: { hand: [RECYCLE], library: [X], graveyard: [A, B] }, p2: { hand: [FILLER] } });

      s.play(RECYCLE);

      for (const viewer of ["p1", "p2"] as const) {
        const shuffled = s.view(viewer).events?.filter((event) => event.type === "shuffledIn") ?? [];
        expect(shuffled.length).toBeGreaterThan(0);
        for (const event of shuffled) expect(event.type === "shuffledIn" && event.position).toBe(-1);
      }
      const listed = (s.view("p1").you.ownLibrary?.cards ?? []).map((entry) => entry.defId);
      const inDeck = defs(s.pile("p1", "library"));
      for (const defId of inDeck) expect(listed).toContain(defId);
      expect(s.view("p2").opponent.ownLibrary).toBeUndefined();
    });

    it("R386 an Upgrade of draw makes it draw 2", () => {
      const s = scenario({ p1: { hand: [RECYCLE], library: [X, Y, A] }, p2: { hand: [FILLER] } });
      stepParam(s.card(RECYCLE), "draw", 1);

      s.play(RECYCLE);

      expect(defs(s.hand("p1"))).toEqual([X, Y]);
    });
  });

  describe("radiant", () => {
    it("R78 each shuffled card costs (1) less, a costMod it keeps", () => {
      const s = scenario({ p1: { hand: [{ def: RECYCLE, radiant: true }], library: [X], graveyard: [A, B] }, p2: { hand: [FILLER] } });
      const a = s.card(A);
      const b = s.card(B);

      s.play(RECYCLE);

      expect(s.card(a.id).costMod).toBe(-1);
      expect(s.card(b.id).costMod).toBe(-1);
      expect(s.card(X).costMod).toBe(0);
    });

    it("R65 R78 the discount adds to what the card already carries: a card at +1 ends at 0, and one at 0 at −1", () => {
      const s = scenario({
        p1: { hand: [{ def: RECYCLE, radiant: true }], library: [X], graveyard: [{ def: A, costMod: 1 }, B] },
        p2: { hand: [FILLER] },
      });
      const a = s.card(A);
      const b = s.card(B);

      s.play(RECYCLE);

      expect(s.card(a.id).costMod).toBe(0);
      expect(s.card(b.id).costMod).toBe(-1);
    });

    it("R177 the cost change inside the deck is read by neither player", () => {
      const s = scenario({ p1: { hand: [{ def: RECYCLE, radiant: true }], library: [X, Y, C], graveyard: [A] }, p2: { hand: [FILLER] } });
      const a = s.card(A);

      s.play(RECYCLE);

      expect(s.events.some((event) => event.type === "costChanged" && event.instanceId === a.id)).toBe(true);
      for (const viewer of ["p1", "p2"] as const) {
        const changes = s.view(viewer).events?.filter((event) => event.type === "costChanged") ?? [];
        for (const event of changes) expect(event.type === "costChanged" && event.instanceId).not.toBe(a.id);
      }
    });

    it("R80 a card the cap left in the graveyard is not discounted", () => {
      const s = scenario({
        p1: { hand: [{ def: RECYCLE, radiant: true }], library: Array.from({ length: LIBRARY_CAP - 1 }, () => X), graveyard: [A, B] },
        p2: { hand: [FILLER] },
      });
      const left = s.card(B);

      s.play(RECYCLE);

      expect(s.card(left.id).zone.z).toBe("graveyard");
      expect(s.card(left.id).costMod).toBe(0);
    });

    it("then draws 1", () => {
      const s = scenario({ p1: { hand: [{ def: RECYCLE, radiant: true }], library: [X] }, p2: { hand: [FILLER] } });

      s.play(RECYCLE);

      expect(defs(s.hand("p1"))).toEqual([X]);
    });

    it("R386 an Upgrade of the discount makes them cost (2) less", () => {
      const s = scenario({ p1: { hand: [{ def: RECYCLE, radiant: true }], library: [X, Y], graveyard: [A] }, p2: { hand: [FILLER] } });
      const a = s.card(A);
      stepParam(s.card(RECYCLE), "discount", 1);

      s.play(RECYCLE);

      expect(s.card(a.id).costMod).toBe(-2);
    });
  });
});
