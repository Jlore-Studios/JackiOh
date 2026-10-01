// C+ #25 Soul Shot — SPEC §8.7 row 25, BUILD M9 Classic+ row C+ 25: "Destroys one random enemy Unit
// (R60), an Indestructible one knocked to Attack Position instead (R46); no enemy Unit, nothing and no
// random draw (R129); an Immune to Spells Unit is never destroyed by it; radiant Lucky 1: two picks, the
// better destroyed, better being the higher attack plus current health, then the higher cost, then the
// lower lane (R414)".

import { addStep, createRng, hashState, reduce, tuningOf, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/025-soul-shot";

const SHOT = "classicplus-025";
const SMALL = "core-012"; // 3/4, (2)
const BIG = "core-019"; // 9/9, (3)
const ROCK = "core-066"; // 10/10 Indestructible
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const MOTHS = "core-009"; // Moths to the Flame, (2) 1/14
const VANILLA = "core-008"; // Mr. Vanilla, (1) 4/4
const SILAS = "core-052"; // Silly Silas, (3) 4/4
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

    /**
     * R414 by the rng itself: the two picks are the next two draws of the match rng over the enemy Units
     * in lane order, so a test can name them. Of two different picks `winner` must die; of one Unit
     * picked twice, that one. Both cases must come up across the seeds.
     */
    function expectLuckyKeeps(p2: SideSetup, lanes: readonly number[], winnerLane: number): void {
      const seen = { split: 0, same: 0 };
      for (let seed = 1; seed <= 24; seed += 1) {
        const s = shot(p2, true, `r414-${seed}`);
        const pool = lanes.map((lane) => s.unit("p2", lane)?.id ?? "");
        const winner = s.unit("p2", winnerLane)?.id ?? "";
        const rng = createRng(s.state.seed, s.state.rngCursor);
        const first = rng.pick(pool);
        const second = rng.pick(pool);
        s.play(SHOT);
        if (first === second) seen.same += 1;
        else seen.split += 1;
        expect(destroyedIds(s)).toEqual([first === second ? first : winner]);
      }
      expect(seen.split).toBeGreaterThan(0);
      expect(seen.same).toBeGreaterThan(0);
    }

    it("R414 the better is the higher attack plus current health: a 1/14 damaged to 1/4 loses to a 4/4", () => {
      expectLuckyKeeps({ field: [{ def: MOTHS, lane: 1, damage: 10 }, { def: VANILLA, lane: 2 }] }, [1, 2], 2);
    });

    it("R414 a tie on attack plus health goes to the higher cost: a (3) 4/4 over a (1) 4/4", () => {
      expectLuckyKeeps({ field: [{ def: VANILLA, lane: 1 }, { def: SILAS, lane: 3 }] }, [1, 3], 3);
    });

    it("R414 then to the lower lane: of two (1) 4/4s, the one in lane 2 over lane 4", () => {
      expectLuckyKeeps({ field: [{ def: VANILLA, lane: 2 }, { def: VANILLA, lane: 4 }] }, [2, 4], 2);
    });

    it("§9.3 the two Lucky picks replay from a JSON copy to the same hash", () => {
      const s = shot({ field: [{ def: SMALL, lane: 1 }, { def: BIG, lane: 2 }, { def: VANILLA, lane: 4 }] }, true, "shot-replay");
      const action = { type: "play", instanceId: s.card(SHOT).id, playerId: "p1", nonce: "shot-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
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
