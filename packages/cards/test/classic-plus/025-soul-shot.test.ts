// C+ #25 Soul Shot — SPEC §8.7 row 25, BUILD M9 Classic+ row C+ 25: "Destroys one random enemy Unit
// (R60), an Indestructible one knocked to Attack Position instead (R46); no enemy Unit, nothing and no
// random draw (R129); an Immune to Spells Unit is never destroyed by it; radiant Lucky 1: two picks, the
// better destroyed, better being the higher attack plus current health, then the higher cost, then the
// lower lane (R414)".

import { addStep, tuningOf } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/025-soul-shot";

const SHOT = "classicplus-025";
const SMALL = "core-012"; // 3/4, (2)
const BIG = "core-019"; // 9/9, (3)
const ROCK = "core-066"; // 10/10 Indestructible
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const FILLER = "core-005";

function shot(p2: SideSetup, radiantFace = false, seed = "soul-shot"): Scenario {
  return scenario({ seed, p1: { hand: [{ def: SHOT, radiant: radiantFace }, FILLER], field: [{ def: BIG, lane: 3 }] }, p2: { hand: [FILLER], ...p2 } });
}

function destroyedIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "destroyed" ? [event.instanceId] : []));
}

describe("C+ #25 Soul Shot", () => {
  it("is a (2) Spell with no play-time choice; the Radiant face prints Lucky 1", () => {
    expect(def.cost).toBe(2);
    expect(base.targets).toBeUndefined();
    expect(radiant).toBe(base);
    expect(def.radiant.keywords).toEqual([{ kind: "Lucky", n: 1 }]);
  });

  describe("base", () => {
    it("R60 destroys exactly one enemy Unit, never one of yours, and the pick varies by seed", () => {
      const picked = new Set<string>();
      for (let seed = 1; seed <= 16; seed += 1) {
        const s = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }, { def: SMALL, lane: 4 }] }, false, `shot-${seed}`);
        const enemies = [1, 2, 4].map((lane) => s.unit("p2", lane)?.id);
        s.play(SHOT);
        const dead = destroyedIds(s);
        expect(dead).toHaveLength(1);
        expect(enemies).toContain(dead[0]);
        expect(s.unit("p1", 3)?.defId).toBe(BIG);
        picked.add(String(enemies.indexOf(dead[0])));
      }
      expect(picked.size).toBeGreaterThan(1);
    });

    it("R129 with no enemy Unit nothing happens and no random draw is made", () => {
      const s = shot({});
      const cursor = s.state.rngCursor;
      s.play(SHOT);
      expect(destroyedIds(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R46 an Indestructible pick survives, knocked to Attack Position", () => {
      const s = shot({ field: [{ def: ROCK, lane: 1, position: "DEF" }] });
      s.play(SHOT);
      const rock = s.unit("p2", 1);
      expect(rock?.defId).toBe(ROCK);
      expect(s.stats(rock ?? "").position).toBe("ATK");
    });

    it("§6.1 an Immune to Spells Unit is never picked", () => {
      for (let seed = 1; seed <= 12; seed += 1) {
        const s = shot({ field: [{ def: TOP_LOSER, lane: 1, radiant: true }, { def: SMALL, lane: 2 }] }, false, `shot-immune-${seed}`);
        s.play(SHOT);
        expect(s.unit("p2", 1)?.defId).toBe(TOP_LOSER);
        expect(s.unit("p2", 2)).toBeNull();
      }
    });

    it("§6.1 with only an Immune to Spells Unit, nothing is destroyed and nothing drawn", () => {
      const s = shot({ field: [{ def: TOP_LOSER, lane: 1, radiant: true }] });
      const cursor = s.state.rngCursor;
      s.play(SHOT);
      expect(destroyedIds(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });
  });

  describe("radiant", () => {
    it("R414 Lucky 1: of two picks the better dies — with a 9/9 and a 3/4, the 9/9 dies far more often than one pick would", () => {
      let big = 0;
      const tries = 24;
      for (let seed = 1; seed <= tries; seed += 1) {
        const s = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }] }, true, `lucky-${seed}`);
        s.play(SHOT);
        if (s.unit("p2", 2) === null) big += 1;
        expect(destroyedIds(s)).toHaveLength(1);
      }
      // Two picks keep the 9/9 unless both land on the 3/4: three in four on average.
      expect(big).toBeGreaterThan(tries / 2);
    });

    it("R414 each Lucky pick is a draw: two draws where the base face makes one", () => {
      const once = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }] }, false);
      const c1 = once.state.rngCursor;
      once.play(SHOT);
      const twice = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }] }, true);
      const c2 = twice.state.rngCursor;
      twice.play(SHOT);
      expect(twice.state.rngCursor - c2).toBe(2 * (once.state.rngCursor - c1));
    });

    it("R414 ties on attack plus health go to the higher cost, then the lower lane", () => {
      // Two copies of the 3/4 tie on everything but their lane: whichever two picks land, lane 1 dies
      // whenever lane 1 was picked at all.
      for (let seed = 1; seed <= 16; seed += 1) {
        const s = shot({ field: [{ def: SMALL, lane: 1 }, { def: SMALL, lane: 4 }] }, true, `tie-${seed}`);
        s.play(SHOT);
        const left = s.unit("p2", 1);
        const right = s.unit("p2", 4);
        expect([left, right].filter((unit) => unit === null)).toHaveLength(1);
      }
    });

    it("R386 Lucky is tuned like any numbered keyword: Lucky 2 makes three picks", () => {
      const s = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }] }, true);
      const card = s.card(SHOT);
      const tuning = tuningOf(card);
      tuning.x = addStep(tuning.x, "Lucky", 1);
      const cursor = s.state.rngCursor;
      s.play(SHOT);
      expect(s.state.rngCursor - cursor).toBe(3);
    });
  });
});
