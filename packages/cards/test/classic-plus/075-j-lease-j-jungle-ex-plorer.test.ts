// C+ #75 J-lease J-Jungle EX-plorer — SPEC §8.7 row 75, BUILD M9 Classic+ row C+ 75: "Cry shuffles a
// J-lease J-Jungle EX-plorer Pack (C+ #75.1) into your deck at a random position (R80's cap), shown in
// your library list; no Cry from a copy (R1); the count reads through `param()`; radiant the Pack is
// Radiant".
//
// "No Cry from a copy": R1 is the engine's — a summoned card never fires its Cry — and no card in the
// preview pool can summon a copy of a non-Human 2-Cost Unit; R1 itself is proved in the engine's summon
// tests and through C+ #67 Pear (067-pear.test.ts), which summons a Cry Unit.

import { LIBRARY_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/075-j-lease-j-jungle-ex-plorer";

const EXPLORER = "classicplus-075";
const PACK = "classicplus-075-1";
const FILLER = "core-005";

function packs(s: Scenario): ReturnType<Scenario["card"]>[] {
  return s.pile("p1", "library").filter((card) => card.defId === PACK);
}

function explorer(opts: { radiant?: boolean; library?: number } = {}): Scenario {
  return scenario({
    seed: "explorer",
    p1: {
      hand: [{ def: EXPLORER, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER],
      library: Array.from({ length: opts.library ?? 4 }, () => FILLER),
    },
    p2: { hand: [FILLER] },
  });
}

describe("C+ #75 J-lease J-Jungle EX-plorer", () => {
  it("is a (2) 5/5 Legendary Unit (10/10 Radiant) that names its Pack", () => {
    expect(def.id).toBe(EXPLORER);
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([2, 5, 5, 10, 10]);
    expect(def.refs).toContain(PACK);
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  describe("base", () => {
    it("§6.3 its Cry shuffles one Pack into your deck at a random position", () => {
      const s = explorer();
      s.play(EXPLORER);
      expect(packs(s)).toHaveLength(1);
      expect(packs(s)[0]?.radiant).toBe(false);
      expect(s.pile("p1", "library")).toHaveLength(5);
      s.expectEvents("cardPlayed", "shuffledIn");
    });

    it("R311 the Pack is shown in your own library list, never its place; the opponent sees a count", () => {
      const s = explorer();
      s.play(EXPLORER);
      const own = s.view("p1").you.ownLibrary;
      expect(own?.cards.some((entry) => entry.defId === PACK)).toBe(true);
      const theirs = s.view("p2");
      expect(theirs.opponent.libraryCount).toBe(5);
      expect(JSON.stringify(theirs)).not.toContain(PACK);
    });

    it("§10.7 the position is the match rng's: some seed puts it on top, another below", () => {
      const positions = new Set<number>();
      for (let i = 0; i < 20; i += 1) {
        const s = scenario({ seed: `explorer-${i}`, p1: { hand: [EXPLORER, FILLER], library: [FILLER, FILLER, FILLER] }, p2: { hand: [FILLER] } });
        s.play(EXPLORER);
        positions.add(s.pile("p1", "library").findIndex((card) => card.defId === PACK));
      }
      expect(positions.size).toBeGreaterThan(1);
    });

    it("R80 a full library turns the Pack away: no Pack is made", () => {
      const s = explorer({ library: LIBRARY_CAP });
      s.play(EXPLORER);
      expect(packs(s)).toEqual([]);
      expect(s.lastEvents.some((event) => event.type === "libraryOverflow" && event.outcome === "notCreated")).toBe(true);
    });

    it("R386 an Upgrade of its count shuffles 2 Packs; a Degrade never takes it below 1", () => {
      const up = explorer();
      stepParam(up.card(EXPLORER), "packs", 1);
      up.play(EXPLORER);
      expect(packs(up)).toHaveLength(2);

      const down = explorer();
      stepParam(down.card(EXPLORER), "packs", -1);
      down.play(EXPLORER);
      expect(packs(down)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R275 the Radiant face shuffles a Radiant Pack, and is a 10/10", () => {
      const s = explorer({ radiant: true });
      s.play(EXPLORER);
      expect(packs(s)).toHaveLength(1);
      expect(packs(s)[0]?.radiant).toBe(true);
      s.expectStats(EXPLORER, { attack: 10, maxHealth: 10 });
    });

    it("R311 the Radiant Pack is listed Radiant in your library list", () => {
      const s = explorer({ radiant: true });
      s.play(EXPLORER);
      const own = s.view("p1").you.ownLibrary;
      expect(own?.cards.some((entry) => entry.defId === PACK && entry.radiant === true)).toBe(true);
    });
  });
});
