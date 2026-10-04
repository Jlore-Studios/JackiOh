// C+ #56 Book of Pain — SPEC §8.7 row 56, BUILD M9 Classic+ row C+ 56: "The opponent discards 2 cards
// at random (R641), with no prompt; the opponent's remaining hand stays hidden (§10.6) while the
// discards are public; fewer cards, all of them; an empty hand, nothing; the count reads through
// `param()`; radiant 4".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/056-book-of-pain";

const BOOK = "classicplus-056";
const FILLER = "core-005";
const A = "core-008"; // Mr. Vanilla
const B = "core-011"; // Tempo Timmy
const C = "core-020"; // Pointmaster
const RUSH = "core-t-rush"; // a unit-token card (R11)

function pain(opts: { radiant?: boolean; theirs?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: BOOK, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER] },
    p2: { hand: opts.theirs ?? [A, B, C] },
  });
}

describe("C+ #56 Book of Pain", () => {
  it("is a (1) Spell, Book; both faces run one script", () => {
    expect(def.id).toBe(BOOK);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R641 no prompt opens: 2 random cards of theirs go at once", () => {
      const s = pain();
      s.play(BOOK);
      expect(s.state.active).toBe("p1");
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toHaveLength(1);
      const grave = s.pile("p2", "graveyard").map((card) => card.defId);
      expect(grave).toHaveLength(2);
      for (const defId of grave) expect([A, B, C]).toContain(defId);
      s.expectInZone(BOOK, "graveyard");
    });

    it("§10.6 R641 the opponent's remaining hand stays hidden, while the discards are public", () => {
      const s = pain();
      s.play(BOOK);
      expect(s.view("p1").pending).toBeNull();
      expect(s.view("p2").pending).toBeNull();
      const mine = JSON.stringify(s.view("p1"));
      for (const card of s.hand("p2")) {
        expect(mine).not.toContain(`"${card.id}"`);
        expect(mine).not.toContain(card.defId);
      }
      const seen = s.view("p1").events.filter((event) => event.type === "discarded");
      expect(seen).toHaveLength(2);
      for (const card of s.pile("p2", "graveyard")) expect(mine).toContain(card.id);
    });

    it("R641 the random discards come from the match rng: the same game discards the same cards", () => {
      const first = pain();
      first.play(BOOK);
      const second = pain();
      second.play(BOOK);
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) => (event.type === "discarded" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("fewer cards than asked: they discard all they have", () => {
      const s = pain({ theirs: [A] });
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toEqual([]);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([A]);
    });

    it("an empty hand opens no prompt and the Spell still resolves", () => {
      const s = pain({ theirs: [] });
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      expect(s.lastEvents.some((event) => event.type === "promptOpened")).toBe(false);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R11 a discarded unit-token card ceases to exist", () => {
      const s = pain({ theirs: [RUSH, A] });
      const token = s.hand("p2").find((card) => card.defId === RUSH);
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      s.expectInZone(token ?? "", "gone");
      expect(s.hand("p2")).toEqual([]);
    });

    it("R113 nothing pauses on the opponent: no prompt, no work owed", () => {
      const s = pain();
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      expect(s.state.work).toEqual([]);
    });

    it("R386 an Upgrade asks for 3; a Degrade for 1", () => {
      const up = pain();
      stepParam(up.card(BOOK), "discards", 1);
      up.play(BOOK);
      expect(up.state.pending).toBeNull();
      expect(up.pile("p2", "graveyard")).toHaveLength(3);
      expect(up.hand("p2")).toHaveLength(0);

      const down = pain();
      stepParam(down.card(BOOK), "discards", -1);
      down.play(BOOK);
      expect(down.state.pending).toBeNull();
      expect(down.pile("p2", "graveyard")).toHaveLength(1);
      expect(down.hand("p2")).toHaveLength(2);
    });
  });

  describe("radiant", () => {
    it("R641 the opponent discards 4 at random", () => {
      const s = pain({ radiant: true, theirs: [A, B, C, A, B] });
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toHaveLength(1);
      expect(s.pile("p2", "graveyard")).toHaveLength(4);
    });

    it("with 3 cards they discard all 3", () => {
      const s = pain({ radiant: true });
      s.play(BOOK);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toEqual([]);
      expect(s.pile("p2", "graveyard")).toHaveLength(3);
    });
  });
});
