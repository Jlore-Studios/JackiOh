// C+ #15 Conjure Rush Token — SPEC §8.7 row 15, BUILD M9 Classic+ row C+ 15: "Summons Core's Rush Token
// (3/3 Rush) into your leftmost open zone with one random keyword from R21's pool it lacks (never Rush
// again); a full board summons nothing and draws nothing (R129); the keyword count reads through
// `param()`; radiant 3 Rush Tokens, fewer on a nearly full board, each with its own random keyword".

import { RANDOM_KEYWORD_POOL, hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
import { keywordKey, type Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/015-conjure-rush-token";

const CARD = "classicplus-015";
const RUSH_TOKEN = "core-t-rush";
const FILLER = "core-005"; // a card in hand, so a turn never auto-ends under the test
const BODY = "core-019"; // Midrange Menace, 9/9 Taunt: a unit to fill a zone

/** The Rush Tokens on p1's side, lane order. */
function tokens(s: Scenario): ReturnType<Scenario["card"]>[] {
  return [1, 2, 3, 4, 5].flatMap((lane) => {
    const unit = s.unit("p1", lane);
    return unit !== null && unit.defId === RUSH_TOKEN ? [unit] : [];
  });
}

/** The keywords a token gained beyond its printed Rush, as R21's pool writes them. */
function gained(s: Scenario, token: ReturnType<Scenario["card"]>): string[] {
  return s
    .stats(token)
    .keywords.map((keyword) => keywordKey(keyword))
    .filter((key) => key !== "Rush");
}

function expectFromPool(keys: readonly string[]): void {
  for (const key of keys) expect(RANDOM_KEYWORD_POOL as readonly string[]).toContain(key);
  expect(new Set(keys).size).toBe(keys.length);
}

describe("C+ #15 Conjure Rush Token", () => {
  it("is a (1) Spell that declares no play-time choice", () => {
    expect(def.id).toBe(CARD);
    expect(def.cost).toBe(1);
    expect(def.type).toBe("Spell");
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("summons Core's Rush Token, 3/3 Rush on its base face, into the leftmost open zone", () => {
      const s = scenario({ p1: { hand: [CARD, FILLER], field: [{ def: BODY, lane: 1 }] }, p2: { hand: [FILLER] } });
      s.play(CARD);
      const [token] = tokens(s);
      expect(token).toBeDefined();
      if (token === undefined) return;
      expect(s.unit("p1", 2)?.id).toBe(token.id);
      expect(token.radiant).toBe(false);
      s.expectStats(token, { attack: 3, health: 3 });
      expect(s.stats(token).keywords.map(keywordKey)).toContain("Rush");
    });

    it("R21 it gains one random keyword from the pool, never Rush again, across seeds", () => {
      const seen = new Set<string>();
      for (let seed = 1; seed <= 24; seed += 1) {
        const s = scenario({ seed: `c15-${seed}`, p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
        s.play(CARD);
        const [token] = tokens(s);
        if (token === undefined) throw new Error("no token");
        const keys = gained(s, token);
        expect(keys).toHaveLength(1);
        expectFromPool(keys);
        expect(keys).not.toContain("Rush");
        for (const key of keys) seen.add(key);
      }
      // The roll is random: two dozen seeds land on more than one keyword.
      expect(seen.size).toBeGreaterThan(1);
    });

    it("R64 R129 a full board summons nothing and draws nothing from the rng", () => {
      const s = scenario({
        p1: { hand: [CARD, FILLER], field: [BODY, BODY, BODY, BODY, BODY] },
        p2: { hand: [FILLER] },
      });
      const cursor = s.state.rngCursor;
      s.play(CARD);
      expect(tokens(s)).toEqual([]);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      expect(s.state.rngCursor).toBe(cursor);
      s.expectInZone(CARD, "graveyard");
    });

    it("R386 the keyword count reads through param: an Upgrade gives two, and a Degrade never goes below one", () => {
      const up = scenario({ p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(up.card(CARD), "keywords", 1);
      up.play(CARD);
      const [upToken] = tokens(up);
      if (upToken === undefined) throw new Error("no token");
      const keys = gained(up, upToken);
      expect(keys).toHaveLength(2);
      expectFromPool(keys);

      const down = scenario({ p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(down.card(CARD), "keywords", -1);
      down.play(CARD);
      const [downToken] = tokens(down);
      if (downToken === undefined) throw new Error("no token");
      expect(gained(down, downToken)).toHaveLength(1);
    });

    it("§9.3 the rolled keyword replays from a JSON copy to the same hash", () => {
      const s = scenario({ seed: "c15-replay", p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
      const action = { type: "play", instanceId: s.card(CARD).id, playerId: "p1", nonce: "c15-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
    });
  });

  describe("radiant", () => {
    it("summons three Rush Tokens, each with its own random keyword", () => {
      const s = scenario({ p1: { hand: [{ def: CARD, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      s.play(CARD);
      const made = tokens(s);
      expect(made).toHaveLength(3);
      expect(made.map((token) => s.card(token).zone)).toEqual([
        { z: "field", player: "p1", row: "units", lane: 1 },
        { z: "field", player: "p1", row: "units", lane: 2 },
        { z: "field", player: "p1", row: "units", lane: 3 },
      ]);
      for (const token of made) {
        expect(token.radiant).toBe(false);
        const keys = gained(s, token);
        expect(keys).toHaveLength(1);
        expectFromPool(keys);
      }
    });

    it("R21 each token rolls its own keyword: across seeds the three do not always share one", () => {
      let differ = 0;
      for (let seed = 1; seed <= 8; seed += 1) {
        const s = scenario({ seed: `c15-own-${seed}`, p1: { hand: [{ def: CARD, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
        s.play(CARD);
        const rolled = tokens(s).map((token) => gained(s, token).join());
        expect(rolled).toHaveLength(3);
        if (new Set(rolled).size > 1) differ += 1;
      }
      expect(differ).toBeGreaterThan(0);
    });

    it("R64 a nearly full board summons fewer: one open zone, one token", () => {
      const s = scenario({
        p1: { hand: [{ def: CARD, radiant: true }, FILLER], field: [BODY, BODY, { def: BODY, lane: 4 }, BODY] },
        p2: { hand: [FILLER] },
      });
      s.play(CARD);
      const made = tokens(s);
      expect(made).toHaveLength(1);
      expect(s.unit("p1", 5)?.defId).toBe(RUSH_TOKEN);
    });

    it("R386 an Upgrade gives each of the three two keywords", () => {
      const s = scenario({ p1: { hand: [{ def: CARD, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(s.card(CARD), "keywords", 1);
      s.play(CARD);
      const made = tokens(s);
      expect(made).toHaveLength(3);
      for (const token of made) expect(gained(s, token)).toHaveLength(2);
    });
  });
});
