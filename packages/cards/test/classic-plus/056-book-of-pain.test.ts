// C+ #56 Book of Pain — SPEC §8.7 row 56, BUILD M9 Classic+ row C+ 56: "The opponent discards 2 cards
// of their choice (R16) through a hand prompt they hold on your turn, its options theirs alone and the
// caster seeing only that a prompt is open (§10.6); fewer cards, all of them; an empty hand, no prompt;
// the discards are public; the count reads through `param()`; radiant 4".

import { hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
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

function theirHandIds(s: Scenario): string[] {
  return s.hand("p2").map((card) => card.id);
}

describe("C+ #56 Book of Pain", () => {
  it("is a (1) Spell, Book; both faces run one script", () => {
    expect(def.id).toBe(BOOK);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R16 the opponent holds a hand prompt over their own hand on your turn, for 2 cards", () => {
      const s = pain();
      const ids = theirHandIds(s);
      s.play(BOOK);
      const pending = s.state.pending;
      expect(s.state.active).toBe("p1");
      expect(pending?.playerId).toBe("p2");
      expect(pending?.kind).toBe("hand");
      expect([pending?.min, pending?.max]).toEqual([2, 2]);
      expect(pending?.options.map((option) => (option.selection.pick === "instance" ? option.selection.instanceId : "")).sort()).toEqual([...ids].sort());
    });

    it("§10.6 R81 the options are theirs alone: the caster sees only that a prompt is open", () => {
      const s = pain();
      const ids = theirHandIds(s);
      s.play(BOOK);
      expect(s.view("p1").pending).toEqual({ forYou: false, pendingFor: "p2" });
      const mine = JSON.stringify(s.view("p1"));
      for (const id of ids) expect(mine).not.toContain(`"${id}"`);
      const theirs = s.view("p2").pending;
      expect(theirs?.forYou).toBe(true);
      expect(theirs !== null && "options" in theirs ? theirs.options : []).toHaveLength(3);
    });

    it("R16 their picks are discarded, and the discards are public", () => {
      const s = pain();
      const [first, second, kept] = s.hand("p2");
      s.play(BOOK);
      s.answer([first?.id ?? "", second?.id ?? ""]);
      expect(s.state.pending).toBeNull();
      s.expectInZone(first ?? "", "graveyard").expectInZone(second ?? "", "graveyard").expectInZone(kept ?? "", "hand");
      const seen = s.view("p1").events.filter((event) => event.type === "discarded");
      expect(seen.map((event) => (event.type === "discarded" ? event.defId : ""))).toEqual([first?.defId, second?.defId]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("fewer cards than asked: they discard all they have", () => {
      const s = pain({ theirs: [A] });
      s.play(BOOK);
      expect([s.state.pending?.min, s.state.pending?.max]).toEqual([1, 1]);
      s.answer([s.hand("p2")[0]?.id ?? ""]);
      expect(s.hand("p2")).toEqual([]);
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
      s.answer(s.hand("p2").map((card) => card.id));
      s.expectInZone(token ?? "", "gone");
    });

    it("R113 paused on their prompt, the state survives a JSON round trip and the answer replays the same", () => {
      const s = pain();
      s.play(BOOK);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const picks = s.hand("p2").slice(0, 2).map((card) => card.id);
      const resumed = reduce(revived, {
        type: "answer",
        playerId: "p2",
        choiceId: revived.pending?.id ?? "",
        selection: picks.map((instanceId) => ({ pick: "instance" as const, instanceId })),
        nonce: "pain-pause",
      });
      expect(resumed.error).toBeUndefined();
      s.answer(picks);
      expect(hashState(resumed.state)).toBe(hashState(s.state));
    });

    it("R386 an Upgrade asks for 3; a Degrade for 1", () => {
      const up = pain();
      stepParam(up.card(BOOK), "discards", 1);
      up.play(BOOK);
      expect(up.state.pending?.min).toBe(3);

      const down = pain();
      stepParam(down.card(BOOK), "discards", -1);
      down.play(BOOK);
      expect(down.state.pending?.min).toBe(1);
    });
  });

  describe("radiant", () => {
    it("the opponent discards 4 of their choice", () => {
      const s = pain({ radiant: true, theirs: [A, B, C, A, B] });
      s.play(BOOK);
      expect([s.state.pending?.min, s.state.pending?.max]).toEqual([4, 4]);
      const picks = s.hand("p2").slice(0, 4).map((card) => card.id);
      s.answer(picks);
      expect(s.hand("p2")).toHaveLength(1);
      expect(s.pile("p2", "graveyard").map((card) => card.id).sort()).toEqual([...picks].sort());
    });

    it("with 3 cards they discard all 3", () => {
      const s = pain({ radiant: true });
      s.play(BOOK);
      expect(s.state.pending?.min).toBe(3);
      s.answer(s.hand("p2").map((card) => card.id));
      expect(s.hand("p2")).toEqual([]);
    });
  });
});
