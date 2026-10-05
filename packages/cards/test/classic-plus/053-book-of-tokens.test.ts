// C+ #53 Book of Tokens — SPEC §8.7 row 53, BUILD M9 Classic+ row C+ 53: "Summons 1-2 Rush Tokens (3/3
// Rush), the count random, into your leftmost open zones, one on a nearly full board, none on a full
// one; the count is no declared number, so tuning moves nothing; radiant 1-2 Radiant Rush Tokens
// (6/6 Rush, Cleave)".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/053-book-of-tokens";

const BOOK = "classicplus-053";
const RUSH = "core-t-rush";
const VANILLA = "core-008";
const FILLER = "core-005";

function book(
  opts: { radiant?: boolean; field?: readonly (string | { def: string; lane: number })[]; seed?: string } = {},
): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: [{ def: BOOK, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER], field: opts.field ?? [] },
    p2: { hand: [FILLER] },
  });
}

function tokens(s: Scenario): { lane: number; radiant: boolean }[] {
  const out: { lane: number; radiant: boolean }[] = [];
  for (let lane = 1; lane <= 5; lane += 1) {
    const unit = s.unit("p1", lane);
    if (unit?.defId === RUSH) out.push({ lane, radiant: unit.radiant });
  }
  return out;
}

describe("C+ #53 Book of Tokens", () => {
  it("is a (1) Spell, Book", () => {
    expect(def.id).toBe(BOOK);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["Book"]);
    expect(def.refs).toEqual([RUSH]);
  });

  describe("base", () => {
    it("R64 summons 1-2 Rush Tokens (3/3 Rush) into your leftmost open zones, the count random but never above the curve", () => {
      const counts = new Set<number>();
      for (let n = 0; n < 20; n += 1) {
        const s = book({ field: [{ def: VANILLA, lane: 1 }, { def: VANILLA, lane: 3 }], seed: `book-of-tokens-${n}` });
        s.play(BOOK);
        const found = tokens(s);
        expect(found.length).toBeGreaterThanOrEqual(1);
        expect(found.length).toBeLessThanOrEqual(2);
        expect(found.map((token) => token.lane)).toEqual(found.length === 1 ? [2] : [2, 4]);
        expect(found.every((token) => token.radiant === false)).toBe(true);
        const first = s.unit("p1", found[0]?.lane ?? 0) ?? "";
        s.expectStats(first, { attack: 3, health: 3 });
        expect(s.stats(first).keywords.map((keyword) => keyword.kind)).toEqual(["Rush"]);
        counts.add(found.length);
      }
      // Both faces of the roll come up across seeds: genuinely 1-2, never 3.
      expect(counts).toEqual(new Set([1, 2]));
    });

    it("§3.2 one on a nearly full board", () => {
      const s = book({ field: [VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(BOOK);
      expect(tokens(s)).toEqual([{ lane: 5, radiant: false }]);
    });

    it("§3.2 none on a full board, and the Spell still resolves", () => {
      const s = book({ field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(BOOK);
      expect(tokens(s)).toEqual([]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R386 the count is no declared number: a tokens tuning moves nothing, the roll stays 1-2", () => {
      const s = book({ seed: "book-of-tokens-tune" });
      stepParam(s.card(BOOK), "tokens", 2);
      s.play(BOOK);
      expect(tokens(s).length).toBeGreaterThanOrEqual(1);
      expect(tokens(s).length).toBeLessThanOrEqual(2);
    });
  });

  describe("radiant", () => {
    it("§7 summons 1-2 Radiant Rush Tokens (6/6 Rush, Cleave)", () => {
      const counts = new Set<number>();
      for (let n = 0; n < 20; n += 1) {
        const s = book({ radiant: true, seed: `book-of-tokens-radiant-${n}` });
        s.play(BOOK);
        const found = tokens(s);
        expect(found.length).toBeGreaterThanOrEqual(1);
        expect(found.length).toBeLessThanOrEqual(2);
        expect(found.map((token) => token.lane)).toEqual(found.length === 1 ? [1] : [1, 2]);
        expect(found.every((token) => token.radiant === true)).toBe(true);
        const first = s.unit("p1", 1) ?? "";
        s.expectStats(first, { attack: 6, health: 6 });
        expect(s.stats(first).keywords.map((keyword) => keyword.kind)).toEqual(["Rush", "Cleave"]);
        counts.add(found.length);
      }
      expect(counts).toEqual(new Set([1, 2]));
    });

    it("§6.1 Lucky 1: the Radiant face rolls twice and keeps the most, so it summons 2 more often", () => {
      expect(def.radiant.keywords).toEqual([{ kind: "Lucky", n: 1 }]);
      expect(def.base.keywords).toEqual([]);
      const twos = (radiant: boolean): number => {
        let found = 0;
        for (let n = 0; n < 40; n += 1) {
          const s = book({ radiant, seed: `book-of-tokens-lucky-${n}` });
          s.play(BOOK);
          if (tokens(s).length === 2) found += 1;
        }
        return found;
      };
      expect(twos(true)).toBeGreaterThan(twos(false));
    });

    it("R386 the Radiant roll is untunable the same way", () => {
      const s = book({ radiant: true, seed: "book-of-tokens-radiant-tune" });
      stepParam(s.card(BOOK), "tokens", 2);
      s.play(BOOK);
      expect(tokens(s).length).toBeGreaterThanOrEqual(1);
      expect(tokens(s).length).toBeLessThanOrEqual(2);
    });
  });
});
