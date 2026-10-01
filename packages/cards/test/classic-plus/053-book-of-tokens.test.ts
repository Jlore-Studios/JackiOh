// C+ #53 Book of Tokens — SPEC §8.7 row 53, BUILD M9 Classic+ row C+ 53: "Summons 2 Rush Tokens (3/3
// Rush) into your leftmost open zones, one on a nearly full board, none on a full one; the count reads
// through `param()`; radiant 2 Radiant Rush Tokens (6/6 Rush, Cleave)".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/053-book-of-tokens";

const BOOK = "classicplus-053";
const RUSH = "core-t-rush";
const VANILLA = "core-008";
const FILLER = "core-005";

function book(opts: { radiant?: boolean; field?: readonly (string | { def: string; lane: number })[] } = {}): Scenario {
  return scenario({
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
    it("R64 summons 2 Rush Tokens (3/3 Rush) into your leftmost open zones", () => {
      const s = book({ field: [{ def: VANILLA, lane: 1 }, { def: VANILLA, lane: 3 }] });
      s.play(BOOK);
      expect(tokens(s)).toEqual([
        { lane: 2, radiant: false },
        { lane: 4, radiant: false },
      ]);
      const token = s.unit("p1", 2) ?? "";
      s.expectStats(token, { attack: 3, health: 3 });
      expect(s.stats(token).keywords.map((keyword) => keyword.kind)).toEqual(["Rush"]);
      expect(s.lastEvents.filter((event) => event.type === "summoned")).toHaveLength(2);
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

    it("R386 an Upgrade summons 3; a Degrade 1, never fewer", () => {
      const up = book();
      stepParam(up.card(BOOK), "tokens", 1);
      up.play(BOOK);
      expect(tokens(up)).toHaveLength(3);

      const down = book();
      stepParam(down.card(BOOK), "tokens", -4);
      down.play(BOOK);
      expect(tokens(down)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("§7 summons 2 Radiant Rush Tokens (6/6 Rush, Cleave)", () => {
      const s = book({ radiant: true });
      s.play(BOOK);
      expect(tokens(s)).toEqual([
        { lane: 1, radiant: true },
        { lane: 2, radiant: true },
      ]);
      const token = s.unit("p1", 1) ?? "";
      s.expectStats(token, { attack: 6, health: 6 });
      expect(s.stats(token).keywords.map((keyword) => keyword.kind)).toEqual(["Rush", "Cleave"]);
    });

    it("R386 the Radiant count steps the same way", () => {
      const s = book({ radiant: true });
      stepParam(s.card(BOOK), "tokens", 2);
      s.play(BOOK);
      expect(tokens(s)).toEqual([1, 2, 3, 4].map((lane) => ({ lane, radiant: true })));
    });
  });
});
