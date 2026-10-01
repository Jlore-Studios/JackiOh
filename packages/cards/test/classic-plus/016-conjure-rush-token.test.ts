// C+ #16 Conjure Rush Token+ — SPEC §8.7 row 16, BUILD M9 Classic+ row C+ 16: "As C+ #15 with 3 different random
// keywords the token lacks; the keyword count reads through `param()`; radiant 3 tokens
// with 3 each".

import { RANDOM_KEYWORD_POOL, stepParam } from "@jackioh/engine";
import { keywordKey } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/016-conjure-rush-token";

const CARD = "classicplus-016";
const RUSH_TOKEN = "core-t-rush";
const KEYWORDS = 3;
const FILLER = "core-005"; // a card in hand, so a turn never auto-ends under the test
const BODY = "core-019"; // Midrange Menace, a unit to fill a zone

function tokens(s: Scenario): ReturnType<Scenario["card"]>[] {
  return [1, 2, 3, 4, 5].flatMap((lane) => {
    const unit = s.unit("p1", lane);
    return unit !== null && unit.defId === RUSH_TOKEN ? [unit] : [];
  });
}

/** The keywords a token gained beyond its printed Rush: all from R21's pool, all different, never Rush. */
function gained(s: Scenario, token: Parameters<Scenario["stats"]>[0]): string[] {
  const keys = s.stats(token).keywords.map(keywordKey).filter((key) => key !== "Rush");
  for (const key of keys) expect(RANDOM_KEYWORD_POOL as readonly string[]).toContain(key);
  expect(new Set(keys).size).toBe(keys.length);
  return keys;
}

describe("C+ #16 Conjure Rush Token+", () => {
  it("is a (2) Spell that declares no play-time choice", () => {
    expect(def.id).toBe(CARD);
    expect(def.cost).toBe(2);
    expect(def.type).toBe("Spell");
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("R21 summons one Rush Token, 3/3 Rush, with 3 different random keywords it lacks, never Rush again", () => {
      const seen = new Set<string>();
      for (let seed = 1; seed <= 12; seed += 1) {
        const s = scenario({ seed: `c16-${seed}`, p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
        s.play(CARD);
        const made = tokens(s);
        expect(made).toHaveLength(1);
        const [token] = made;
        if (token === undefined) throw new Error("no token");
        expect(s.unit("p1", 1)?.id).toBe(token.id);
        s.expectStats(token, { attack: 3, health: 3 });
        expect(s.stats(token).keywords.map(keywordKey)).toContain("Rush");
        const keys = gained(s, token);
        expect(keys).toHaveLength(KEYWORDS);
        for (const key of keys) seen.add(key);
      }
      expect(seen.size).toBeGreaterThan(KEYWORDS);
    });

    it("R64 R129 a full board summons nothing and draws nothing from the rng", () => {
      const s = scenario({ p1: { hand: [CARD, FILLER], field: [BODY, BODY, BODY, BODY, BODY] }, p2: { hand: [FILLER] } });
      const cursor = s.state.rngCursor;
      s.play(CARD);
      expect(tokens(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
      s.expectInZone(CARD, "graveyard");
    });

    it("R386 the keyword count reads through param: an Upgrade gives one more, a Degrade one fewer", () => {
      const up = scenario({ p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(up.card(CARD), "keywords", 1);
      up.play(CARD);
      expect(gained(up, tokens(up)[0] ?? "")).toHaveLength(KEYWORDS + 1);

      const down = scenario({ p1: { hand: [CARD, FILLER] }, p2: { hand: [FILLER] } });
      stepParam(down.card(CARD), "keywords", -1);
      down.play(CARD);
      expect(gained(down, tokens(down)[0] ?? "")).toHaveLength(KEYWORDS - 1);
    });
  });

  describe("radiant", () => {
    it("summons three Rush Tokens into the leftmost open zones, each with its own 3 random keywords", () => {
      const s = scenario({ p1: { hand: [{ def: CARD, radiant: true }, FILLER], field: [{ def: BODY, lane: 2 }] }, p2: { hand: [FILLER] } });
      s.play(CARD);
      const made = tokens(s);
      expect(made.map((token) => s.card(token).zone)).toEqual([1, 3, 4].map((lane) => ({ z: "field", player: "p1", row: "units", lane })));
      for (const token of made) {
        expect(token.radiant).toBe(false);
        expect(gained(s, token)).toHaveLength(KEYWORDS);
      }
    });

    it("R64 a nearly full board summons fewer: one open zone, one token", () => {
      const s = scenario({ p1: { hand: [{ def: CARD, radiant: true }, FILLER], field: [BODY, BODY, BODY, BODY] }, p2: { hand: [FILLER] } });
      s.play(CARD);
      expect(tokens(s)).toHaveLength(1);
      expect(s.unit("p1", 5)?.defId).toBe(RUSH_TOKEN);
    });
  });
});
