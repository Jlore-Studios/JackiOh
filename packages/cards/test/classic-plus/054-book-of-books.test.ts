// C+ #54 Book of Books — SPEC §8.7 row 54, BUILD M9 Classic+ row C+ 54: "Adds 2 random non-token Books
// of any set, never Book of Books (R387), which cost (0) (`costOverride` 0); repeats allowed; a full
// hand burns; hidden from the opponent (R97); the count reads through `param()`; radiant the Books are
// Radiant". R637: each Book is Temporary, in both faces, and is discarded at the end of the turn.

import { defOf, query, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/054-book-of-books";

const BOOK = "classicplus-054";
const FILLER = "core-005";

function book(opts: { radiant?: boolean; seed?: string; fillers?: number } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "book-of-books",
    p1: {
      hand: [{ def: BOOK, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)],
    },
    p2: { hand: [FILLER] },
  });
}

function added(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.filter((event): event is Extract<GameEvent, { type: "addedToHand" }> => event.type === "addedToHand" && event.player === "p1");
}

describe("C+ #54 Book of Books", () => {
  it("is a (1) Spell, Book", () => {
    expect(def.id).toBe(BOOK);
    expect(def.tags).toEqual(["Book"]);
  });

  describe("base", () => {
    it("adds 2 random Books to your hand, each costing (0)", () => {
      const s = book();
      s.play(BOOK);
      const books = added(s).map((event) => s.card(event.instanceId));
      expect(books).toHaveLength(2);
      for (const card of books) {
        expect(card.zone.z).toBe("hand");
        expect(defOf(s.state, card.defId).tags).toContain("Book");
        expect(card.costOverride).toBe(0);
        expect(card.radiant).toBe(false);
      }
      const mine = s.view("p1").you.hand;
      expect(Array.isArray(mine) && books.every((card) => mine.some((view) => view.instanceId === card.id && view.cost === 0))).toBe(true);
    });

    it("R380 R387 over many seeds: Books of several sets, never a token, never Book of Books", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 80; i += 1) {
        const s = book({ seed: `books-${i}` });
        s.play(BOOK);
        for (const event of added(s)) seen.add(event.defId);
      }
      expect(seen.has(BOOK)).toBe(false);
      const pool = query({ tags: ["Book"] }).map((entry) => entry.id);
      for (const id of seen) expect(pool).toContain(id);
      const sets = new Set([...seen].map((id) => defOf(book().state, id).set));
      expect(sets.has("Classic")).toBe(true);
      expect(sets.has("Classic+")).toBe(true);
      expect([...seen].every((id) => !defOf(book().state, id).token)).toBe(true);
    });

    it("R60 repeats are allowed", () => {
      let repeated = false;
      for (let i = 0; i < 80 && !repeated; i += 1) {
        const s = book({ seed: `repeat-${i}` });
        s.play(BOOK);
        const [a, b] = added(s);
        repeated = a !== undefined && a.defId === b?.defId;
      }
      expect(repeated).toBe(true);
    });

    it("§2.4 R4 a full hand burns the Books, which keep no (0) price", () => {
      const s = book({ fillers: 9 });
      s.play(BOOK);
      const burned = s.lastEvents.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
      const lost = s.card(burned[0]?.type === "burned" ? burned[0].instanceId : "");
      expect(lost.zone.z).toBe("graveyard");
      expect(lost.costOverride).toBeUndefined();
      // R637: Temporary is for the card's stay in a hand, so a burned Book carries none.
      expect(lost.grantedKeywords).toEqual([]);
      expect(added(s)).toHaveLength(1);
    });

    it("R637 each Book is Temporary: discarded from your hand at the end of your turn, the filler kept", () => {
      const s = book();
      s.play(BOOK);
      const books = added(s).map((event) => s.card(event.instanceId));
      expect(books).toHaveLength(2);
      for (const card of books) expect(card.grantedKeywords).toEqual([{ kind: "Temporary" }]);
      const view = s.view("p1").you.hand;
      expect(Array.isArray(view) && books.every((card) => view.some((entry) => entry.instanceId === card.id && (entry.keywords ?? []).some((k) => k.kind === "Temporary")))).toBe(true);

      s.endTurn();
      for (const card of books) expect(s.card(card.id).zone.z).toBe("graveyard");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
    });

    it("R97 the opponent sees the adds under the sentinel", () => {
      const s = book();
      s.play(BOOK);
      const ids = added(s).map((event) => event.instanceId);
      const theirs = s.view("p2");
      const events = theirs.events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(events).toHaveLength(2);
      for (const event of events) expect(event).toMatchObject({ instanceId: "hidden", defId: "hidden" });
      for (const id of ids) expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
    });

    it("R386 an Upgrade adds 3; a Degrade 1, never fewer", () => {
      const up = book();
      stepParam(up.card(BOOK), "books", 1);
      up.play(BOOK);
      expect(added(up)).toHaveLength(3);

      const down = book();
      stepParam(down.card(BOOK), "books", -3);
      down.play(BOOK);
      expect(added(down)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R74 the Books are Radiant and cost (0), never Book of Books", () => {
      for (let i = 0; i < 30; i += 1) {
        const s = book({ radiant: true, seed: `rbooks-${i}` });
        s.play(BOOK);
        const books = added(s).map((event) => s.card(event.instanceId));
        expect(books).toHaveLength(2);
        for (const card of books) {
          expect(card.radiant).toBe(true);
          expect(card.costOverride).toBe(0);
          expect(card.defId).not.toBe(BOOK);
        }
      }
    });

    it("R637 the Radiant Books are Temporary too", () => {
      const s = book({ radiant: true, seed: "rtemporary" });
      s.play(BOOK);
      const books = added(s).map((event) => s.card(event.instanceId));
      expect(books).toHaveLength(2);
      for (const card of books) expect(card.grantedKeywords).toEqual([{ kind: "Temporary" }]);

      s.endTurn();
      for (const card of books) expect(s.card(card.id).zone.z).toBe("graveyard");
    });

    it("§9.3 the same seed adds the same Books", () => {
      const a = book({ radiant: true, seed: "same" });
      const b = book({ radiant: true, seed: "same" });
      a.play(BOOK);
      b.play(BOOK);
      expect(added(a).map((event) => event.defId)).toEqual(added(b).map((event) => event.defId));
    });
  });
});
