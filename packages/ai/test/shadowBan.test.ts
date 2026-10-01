// The AI's shadow ban and the sweep that decides it (SPEC §9.9, R186; docs/polish/3-ai.md B24, B25).
//
// B24: the table itself. Every entry names a real non-token card of any set and a reason that starts
// with the sweep flags that put it there; the AI never deals itself a banned card; and enough
// cards stay unbanned to build a 30-card Hard deck. A banned card stays legal for every player.
//
// B25: `sweepFlags` raises each flag exactly at its AI_SWEEP threshold, tested on both sides of
// every bound with synthetic stats, and one real `sweepCard` run on a plain card flags no error.

import { describe, expect, it } from "vitest";
import { AI_DIFFICULTY, DECK_SIZE, createGame, createRng } from "@jackioh/engine";
import {
  AI_DECK,
  AI_SWEEP,
  SHADOW_BAN,
  SHADOW_BAN_IDS,
  buildAiDeck,
  sweepCard,
  sweepFlags,
  SHADOW_WATCH,
  atRiskIds,
  banFlags,
  halfFlags,
  pass2KeepOut,
  sweepAtRisk,
  sweepVerdict,
  type SweepPass2,
  type SweepResult,
  type SweepStats,
} from "../src/index";
import { aiPool } from "./_support";

const REASON = /^(error|timeout|neverPlayed|selfHarm)(, (error|timeout|neverPlayed|selfHarm))*: \S/;

// ---------------------------------------------------------------------------------------------
// B24
// ---------------------------------------------------------------------------------------------

describe("the shadow ban (B24)", () => {
  it("R186 B24: every entry is a real non-token card with a reason that starts with its sweep flags", () => {
    const pool = new Set(aiPool());
    for (const [defId, reason] of Object.entries(SHADOW_BAN)) {
      expect(pool.has(defId), `${defId} is not a non-token card`).toBe(true);
      expect(reason, defId).toMatch(REASON);
    }
  });

  it("R186 B24: SHADOW_BAN_IDS is exactly the table's keys, sorted", () => {
    expect([...SHADOW_BAN_IDS]).toEqual(Object.keys(SHADOW_BAN).sort());
    expect(new Set(SHADOW_BAN_IDS).size).toBe(SHADOW_BAN_IDS.length);
  });

  it("R186 B24: the unbanned pool holds at least AI_DECK.minPool cards, enough for a Hard deck", () => {
    const unbanned = aiPool().filter((id) => !SHADOW_BAN_IDS.includes(id));
    expect(unbanned.length).toBeGreaterThanOrEqual(AI_DECK.minPool);
    expect(unbanned.length).toBeGreaterThanOrEqual(AI_DIFFICULTY.hard.deckSize);
  });

  it("R186 B24: buildAiDeck never deals a banned card, over 200 seeds at every tier's size and cap", { timeout: 60_000 }, () => {
    const banned = new Set(SHADOW_BAN_IDS);
    for (const difficulty of ["easy", "medium", "hard"] as const) {
      const h = AI_DIFFICULTY[difficulty];
      for (let n = 1; n <= 200; n += 1) {
        const deck = buildAiDeck(createRng(`shadow-ban:${difficulty}:${n}`), h.deckSize, { manaCap: h.manaCap });
        for (const id of deck) expect(banned.has(id), `${difficulty} seed ${n}: ${id}`).toBe(false);
      }
    }
  });

  it("R186 B24: a banned card stays legal for a human's deck", () => {
    const others = aiPool().filter((id) => !SHADOW_BAN_IDS.includes(id));
    const deck = [...SHADOW_BAN_IDS.slice(0, DECK_SIZE), ...others].slice(0, DECK_SIZE);
    const opponent = buildAiDeck(createRng("shadow-ban-legal"), DECK_SIZE);
    expect(new Set(deck).size).toBe(DECK_SIZE);
    expect(() => createGame({ seed: "shadow-ban-legal", decks: [deck, opponent] })).not.toThrow();
  });
});

// ---------------------------------------------------------------------------------------------
// B25
// ---------------------------------------------------------------------------------------------

function stats(overrides: Partial<SweepStats> = {}): SweepStats {
  return {
    defId: "core-011",
    games: AI_SWEEP.seedsPerCard,
    drawnGames: AI_SWEEP.seedsPerCard,
    affordableTurns: 0,
    plays: 0,
    errors: 0,
    timeouts: 0,
    evalDeltaSum: 0,
    evalDeltaCount: 0,
    ...overrides,
  };
}

describe("sweepFlags (B25)", () => {
  it("B25: clean stats raise no flag", () => {
    expect(sweepFlags(stats())).toEqual([]);
    expect(sweepFlags(stats({ affordableTurns: 10, plays: 3, evalDeltaSum: 30, evalDeltaCount: 3 }))).toEqual([]);
  });

  it("B25: error is raised by the first error and not before", () => {
    expect(sweepFlags(stats({ errors: 0 }))).not.toContain("error");
    expect(sweepFlags(stats({ errors: 1 }))).toEqual(["error"]);
  });

  it("B25: timeout is raised by the first timeout and not before", () => {
    expect(sweepFlags(stats({ timeouts: 0 }))).not.toContain("timeout");
    expect(sweepFlags(stats({ timeouts: 1 }))).toEqual(["timeout"]);
  });

  it("B25: neverPlayed needs minAffordableTurns affordable turns and no play", () => {
    const at = AI_SWEEP.minAffordableTurns;
    expect(sweepFlags(stats({ affordableTurns: at, plays: 0 }))).toEqual(["neverPlayed"]);
    expect(sweepFlags(stats({ affordableTurns: at - 1, plays: 0 }))).toEqual([]);
    expect(sweepFlags(stats({ affordableTurns: at, plays: 1, evalDeltaSum: 0, evalDeltaCount: 1 }))).toEqual([]);
  });

  it("B25: selfHarm needs a mean evaluation delta strictly below selfHarmDelta over at least minHarmPlays plays", () => {
    const bound = AI_SWEEP.selfHarmDelta;
    const plays = AI_SWEEP.minHarmPlays;
    const base = { affordableTurns: 5, plays };
    expect(sweepFlags(stats({ ...base, evalDeltaSum: bound * plays, evalDeltaCount: plays }))).toEqual([]);
    expect(sweepFlags(stats({ ...base, evalDeltaSum: (bound - 1) * plays, evalDeltaCount: plays }))).toEqual(["selfHarm"]);
    // One play fewer than minHarmPlays is not enough, however bad the plays were.
    const fewer = plays - 1;
    expect(sweepFlags(stats({ ...base, plays: fewer, evalDeltaSum: (bound - 100) * fewer, evalDeltaCount: fewer }))).toEqual([]);
    expect(sweepFlags(stats({ ...base, evalDeltaSum: bound * 10, evalDeltaCount: 0 }))).toEqual([]);
  });

  it("B25: several flags come out in the order error, timeout, neverPlayed, selfHarm", () => {
    const all = stats({
      errors: 2,
      timeouts: 1,
      affordableTurns: AI_SWEEP.minAffordableTurns,
      plays: 0,
      evalDeltaSum: (AI_SWEEP.selfHarmDelta - 10) * AI_SWEEP.minHarmPlays,
      evalDeltaCount: AI_SWEEP.minHarmPlays,
    });
    expect(sweepFlags(all)).toEqual(["error", "timeout", "neverPlayed", "selfHarm"]);
  });

  it("B25: sweepCard on Tempo Timmy over 2 seeds plays 2 games and flags no error", { timeout: 300_000 }, () => {
    const result = sweepCard("core-011", { seeds: 2 });
    expect(result.defId).toBe("core-011");
    expect(result.games).toBe(2);
    expect(result.flags).not.toContain("error");
    expect(result.errors).toBe(0);
    expect(result.drawnGames).toBeLessThanOrEqual(result.games);
    const { flags, ...rest } = result;
    expect(sweepFlags(rest)).toEqual(flags);
  });
});

// ---------------------------------------------------------------------------------------------
// every tier: a ban holds at every difficulty, so a card is judged at the fewest resources and
// the most, and a card no tier could afford is reported rather than passed
// ---------------------------------------------------------------------------------------------

function result(overrides: Partial<SweepResult> = {}): SweepResult {
  const base = stats(overrides);
  return { ...base, tier: "easy", flags: sweepFlags(base), unswept: base.affordableTurns === 0, ...overrides };
}

/**
 * A pass-2 result of `forced` at `tier` (fixture numbers, no game played): `cards[0]` is the forced
 * card's entry unless it names another defId; each entry defaults to seedsPerCardAtRisk games.
 */
function pass2Of(
  forced: string,
  tier: SweepPass2["tier"],
  cards: Partial<SweepStats>[],
  suspects: SweepPass2["suspects"] = [],
): SweepPass2 {
  return {
    forced,
    tier,
    games: AI_SWEEP.seedsPerCardAtRisk,
    cards: cards.map((card) => stats({ defId: forced, games: AI_SWEEP.seedsPerCardAtRisk, drawnGames: 0, ...card })),
    suspects,
  };
}

describe("the sweep judges a card at every tier (R186)", () => {
  it("R186 AI_SWEEP sweeps at Easy and at Hard", () => {
    expect([...AI_SWEEP.tiers]).toEqual(["easy", "hard"]);
  });

  it("R186 a flag at any tier bans the card, and the reason names every tier that flagged it", () => {
    const at = AI_SWEEP.banAffordableTurns;
    const easy = result({ defId: "core-078", tier: "easy", affordableTurns: 4, plays: 0 });
    const hard = result({ defId: "core-078", tier: "hard", affordableTurns: 5, plays: 0 });
    const verdict = sweepVerdict(
      [easy, hard],
      [pass2Of("core-078", "easy", [{ affordableTurns: at + 2 }]), pass2Of("core-078", "hard", [{ affordableTurns: at + 5 }])],
    );
    expect(verdict.flags).toEqual(["neverPlayed"]);
    expect(verdict.unswept).toBe(false);
    expect(verdict.reason).toMatch(REASON);
    expect(verdict.reason).toContain(`easy: affordable in hand on ${String(at + 2)} turns over 24 pass-2 games, never played`);
    expect(verdict.reason).toContain(`hard: affordable in hand on ${String(at + 5)} turns over 24 pass-2 games, never played`);

    const onlyHard = sweepVerdict([
      result({ defId: "core-078", tier: "easy", affordableTurns: 6, plays: 2, evalDeltaSum: 4, evalDeltaCount: 2 }),
      result({ defId: "core-078", tier: "hard", errors: 1, affordableTurns: 4, plays: 1, evalDeltaCount: 1 }),
    ]);
    expect(onlyHard.flags).toEqual(["error"]);
    expect(onlyHard.reason).toMatch(/^error: hard: 1 engine or search error/);
    expect(onlyHard.reason).not.toContain("easy:");
  });

  it("R186 a card clean at every tier has no reason; one never affordable anywhere is unswept, not clean", () => {
    const clean = sweepVerdict([
      result({ tier: "easy", affordableTurns: 5, plays: 3, evalDeltaSum: 9, evalDeltaCount: 3 }),
      result({ tier: "hard", affordableTurns: 7, plays: 4, evalDeltaSum: 8, evalDeltaCount: 4 }),
    ]);
    expect(clean).toMatchObject({ flags: [], unswept: false, reason: null });

    const neverAffordable = sweepVerdict([result({ tier: "easy" }), result({ tier: "hard" })]);
    expect(neverAffordable).toMatchObject({ flags: [], unswept: true, reason: null });

    const affordableOnlyAtHard = sweepVerdict([
      result({ tier: "easy" }),
      result({ tier: "hard", affordableTurns: 4, plays: 2, evalDeltaCount: 2 }),
    ]);
    expect(affordableOnlyAtHard.unswept).toBe(false);
  });

  it(
    "R186 GIGA Glowy Jelly Bean (6 mana) is unswept at Easy's four crystals and judged at Hard's seven",
    { timeout: 300_000 },
    () => {
      const easy = sweepCard("core-029", { seeds: 2, tier: "easy" });
      expect(easy.tier).toBe("easy");
      expect(easy.affordableTurns).toBe(0);
      expect(easy.unswept).toBe(true);
      expect(easy.flags).toEqual([]);

      const hard = sweepCard("core-029", { seeds: 2, tier: "hard" });
      expect(hard.tier).toBe("hard");
      expect(hard.errors).toBe(0);
      expect(hard.affordableTurns).toBeGreaterThan(0);
      expect(hard.unswept).toBe(false);
    },
  );
});

// ---------------------------------------------------------------------------------------------
// R390: two passes. Pass 1 finds the cards at risk, pass 2 sweeps them again with more games and
// deals them more often as filler, and a judgement ban needs pass 2's numbers. Fixture results
// throughout, so no rule here waits on a whole sweep; one real pass-2 game at the end.
// ---------------------------------------------------------------------------------------------

describe("the two-pass sweep (R390)", () => {
  it("R390 pass 2's numbers: 24 games per at-risk card and tier, at-risk filler ×4, bans at 6 affordable turns and 8 plays", () => {
    expect(AI_SWEEP.seedsPerCardAtRisk).toBe(24);
    expect(AI_SWEEP.atRiskBoost).toBe(4);
    expect(AI_SWEEP.banAffordableTurns).toBe(2 * AI_SWEEP.minAffordableTurns);
    expect(AI_SWEEP.banHarmPlays).toBe(2 * AI_SWEEP.minHarmPlays);
  });

  it("R390 at risk is a flag at half strength: affordable on minAffordableTurns turns and played at most once, or a mean below half of selfHarmDelta", () => {
    const at = AI_SWEEP.minAffordableTurns;
    expect(halfFlags(stats({ affordableTurns: at, plays: 0 }))).toEqual(["neverPlayed"]);
    expect(halfFlags(stats({ affordableTurns: at, plays: 1, evalDeltaCount: 1 }))).toEqual(["neverPlayed"]);
    expect(halfFlags(stats({ affordableTurns: at, plays: 2, evalDeltaCount: 2 }))).toEqual([]);
    expect(halfFlags(stats({ affordableTurns: at - 1, plays: 0 }))).toEqual([]);
    const half = AI_SWEEP.selfHarmDelta / 2;
    expect(halfFlags(stats({ affordableTurns: 9, plays: 2, evalDeltaSum: (half - 1) * 2, evalDeltaCount: 2 }))).toEqual(["selfHarm"]);
    expect(halfFlags(stats({ affordableTurns: 9, plays: 2, evalDeltaSum: half * 2, evalDeltaCount: 2 }))).toEqual([]);
    // One play is enough to put a card at risk (to ban it takes banHarmPlays).
    expect(halfFlags(stats({ affordableTurns: 9, plays: 1, evalDeltaSum: half - 1, evalDeltaCount: 1 }))).toEqual(["neverPlayed", "selfHarm"]);
  });

  it("R390 the at-risk list is a pure function of pass 1's results, the ban and the watch list, sorted", () => {
    const pass1 = [
      result({ defId: "core-030", tier: "easy", affordableTurns: 5, plays: 1, evalDeltaCount: 1 }),
      result({ defId: "core-030", tier: "hard", affordableTurns: 9, plays: 6, evalDeltaCount: 6 }),
      result({ defId: "core-011", tier: "easy", affordableTurns: 9, plays: 6, evalDeltaCount: 6 }),
      result({ defId: "core-012", tier: "hard", affordableTurns: 9, plays: 3, evalDeltaSum: -90, evalDeltaCount: 3 }),
    ];
    const ban = { "core-099": "neverPlayed: easy: affordable in hand on 21 turns, never played" };
    const watch = { "classic-020": "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s)" };
    expect(atRiskIds(pass1, ban, watch)).toEqual(["classic-020", "core-012", "core-030", "core-099"]);
    expect(atRiskIds([...pass1].reverse(), ban, watch)).toEqual(atRiskIds(pass1, ban, watch));
    expect(atRiskIds([], {}, {})).toEqual([]);
    // By default today's tables count: every banned or watched card is at risk from the start.
    const atRisk = atRiskIds([]);
    for (const id of [...SHADOW_BAN_IDS, ...Object.keys(SHADOW_WATCH)]) expect(atRisk).toContain(id);
  });

  it("R390 pass 2's filler keeps out the cards banned for error or timeout and lifts the ban for neverPlayed and selfHarm", () => {
    const ban = {
      "core-042": "neverPlayed: hard: affordable in hand on 21 turns, never played",
      "core-051": "error, neverPlayed: easy: 1 engine or search error(s) over 8 games, affordable in hand on 4 turns, never played",
      "core-055": "timeout: hard: 1 decision(s) over 2000 ms or game(s) past 600 actions",
      "core-057": "selfHarm: easy: mean evaluate change -60.0 over 9 play(s) over 24 pass-2 games",
    };
    expect(banFlags(ban["core-051"])).toEqual(["error", "neverPlayed"]);
    const pass1 = [result({ defId: "classic-003", errors: 2 }), result({ defId: "classic-004", timeouts: 1 }), result({ defId: "classic-005", affordableTurns: 5 })];
    expect(pass2KeepOut(pass1, ban)).toEqual(["classic-003", "classic-004", "core-051", "core-055"]);
  });

  it("R390 pass 1 alone never bans for neverPlayed or selfHarm, however strong its numbers; error and timeout ban as before", () => {
    const strong = result({
      defId: "core-078",
      affordableTurns: 30,
      plays: 0,
      evalDeltaSum: AI_SWEEP.selfHarmDelta * 100,
      evalDeltaCount: 10,
    });
    expect(strong.flags).toEqual(["neverPlayed", "selfHarm"]);
    expect(sweepVerdict([strong])).toMatchObject({ flags: [], reason: null });
    expect(sweepVerdict([result({ defId: "core-078", errors: 1 })]).flags).toEqual(["error"]);
    expect(sweepVerdict([result({ defId: "core-078", timeouts: 1 })]).flags).toEqual(["timeout"]);
  });

  it("R390 R601 neverPlayed needs 6 affordable turns and no play over pass 2's games at that tier, forced and filler games summed", () => {
    const at = AI_SWEEP.banAffordableTurns;
    const p1 = [result({ defId: "core-078", affordableTurns: 4 })];
    const verdict = (pass2: SweepPass2[]): string | null => sweepVerdict(p1, pass2).reason;
    expect(verdict([pass2Of("core-078", "easy", [{ affordableTurns: at }])])).toMatch(/^neverPlayed: easy: /);
    expect(verdict([pass2Of("core-078", "easy", [{ affordableTurns: at - 1 }])])).toBeNull();
    expect(verdict([pass2Of("core-078", "easy", [{ affordableTurns: at + 9, plays: 1, evalDeltaCount: 1 }])])).toBeNull();
    // Its own pass 2 saw it affordable on 3 turns; another card's pass 2 dealt it as filler for 3 more.
    const own = pass2Of("core-078", "easy", [{ affordableTurns: at / 2 }]);
    const filler = pass2Of("core-030", "easy", [{ defId: "core-030", affordableTurns: 9, plays: 4, evalDeltaCount: 4 }, { defId: "core-078", games: 5, affordableTurns: at / 2 }]);
    expect(verdict([own])).toBeNull();
    expect(verdict([own, filler])).toBe(`neverPlayed: easy: affordable in hand on ${String(at)} turns over 29 pass-2 games, never played`);
    // A play in any pass-2 game at that tier clears it, a forced one or a filler one.
    const played = pass2Of("core-030", "easy", [{ defId: "core-030" }, { defId: "core-078", games: 1, plays: 1, evalDeltaCount: 1 }]);
    expect(verdict([own, filler, played])).toBeNull();
    // Pass 2 at the other tier is no evidence for this one.
    expect(verdict([pass2Of("core-078", "hard", [{ affordableTurns: at - 1 }]), pass2Of("core-078", "easy", [{ affordableTurns: at - 1 }])])).toBeNull();
  });

  it("R390 R601 selfHarm needs 8 plays over pass 2's games, averaging below selfHarmDelta", () => {
    const plays = AI_SWEEP.banHarmPlays;
    const bound = AI_SWEEP.selfHarmDelta;
    const p1 = [result({ defId: "core-078", affordableTurns: 4, plays: 2, evalDeltaSum: bound * 2, evalDeltaCount: 2 })];
    const harm = (count: number, each: number): string | null =>
      sweepVerdict(p1, [pass2Of("core-078", "hard", [{ affordableTurns: 20, plays: count, evalDeltaSum: each * count, evalDeltaCount: count }])]).reason;
    expect(harm(plays, bound - 1)).toMatch(/^selfHarm: hard: mean evaluate change -41\.0 over 8 play\(s\) over 24 pass-2 games$/);
    expect(harm(plays - 1, bound - 100)).toBeNull();
    expect(harm(plays, bound)).toBeNull();
  });

  it("R390 an error or timeout bans only its game's forced card; at-risk filler of that game is a suspect, banned only if its own games repeat it", () => {
    const seed = "sweep2:easy:core-030:7";
    const suspect = { defId: "core-078", seed, forced: "core-030", errors: 1, timeouts: 0 };
    const forced = pass2Of("core-030", "easy", [{ errors: 1, affordableTurns: 9, plays: 5, evalDeltaCount: 5 }, { defId: "core-078", games: 3, affordableTurns: 2, plays: 1, evalDeltaCount: 1 }], [suspect]);
    expect(sweepVerdict([result({ defId: "core-030", affordableTurns: 4, plays: 2, evalDeltaCount: 2 })], [forced]).flags).toEqual(["error"]);
    const fillerOwn = pass2Of("core-078", "easy", [{ affordableTurns: 9, plays: 4, evalDeltaCount: 4 }]);
    const p1 = [result({ defId: "core-078", affordableTurns: 4, plays: 1, evalDeltaCount: 1 })];
    expect(sweepVerdict(p1, [forced, fillerOwn]).flags).toEqual([]);
    const repeated = pass2Of("core-078", "easy", [{ errors: 1, affordableTurns: 9, plays: 4, evalDeltaCount: 4 }]);
    expect(sweepVerdict(p1, [forced, repeated])).toMatchObject({ flags: ["error"] });
    expect(sweepVerdict(p1, [forced, repeated]).reason).toMatch(/^error: easy: 1 engine or search error\(s\) over 35 games$/);
  });

  it("R390 R600 the watch list holds the cards at risk by their own numbers that were not banned, with those numbers", () => {
    // At risk by pass 1, cleared in pass 2: watched, its numbers named.
    const p1 = [result({ defId: "core-078", affordableTurns: 4, plays: 1, evalDeltaCount: 1 })];
    const cleared = sweepVerdict(p1, [pass2Of("core-078", "easy", [{ affordableTurns: 20, plays: 7, evalDeltaCount: 7 }])]);
    expect(cleared.reason).toBeNull();
    expect(cleared.watch).toBe("at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 0.0");
    // At risk only because it was banned or watched before, and clean in both passes now: off the list.
    const clean = sweepVerdict(
      [result({ defId: "core-099", affordableTurns: 6, plays: 4, evalDeltaCount: 4 })],
      [pass2Of("core-099", "easy", [{ affordableTurns: 20, plays: 9, evalDeltaCount: 9 }])],
    );
    expect(clean).toMatchObject({ reason: null, watch: null });
    // At risk by pass 2's own numbers: watched.
    const second = sweepVerdict(
      [result({ defId: "core-099", affordableTurns: 6, plays: 4, evalDeltaCount: 4 })],
      [pass2Of("core-099", "easy", [{ affordableTurns: 5, plays: 1, evalDeltaCount: 1 }])],
    );
    expect(second.watch).toMatch(/^at risk: easy: pass 2 affordable on 5 turns over 24 games, played 1 time\(s\)/);
    // Banned: never watched.
    expect(sweepVerdict(p1, [pass2Of("core-078", "easy", [{ affordableTurns: 9 }])])).toMatchObject({ flags: ["neverPlayed"], watch: null });
  });

  it("R390 a pass-2 result comes through JSON whole, so slices join in `--report`", () => {
    const fixture = pass2Of("core-078", "easy", [{ affordableTurns: 3 }], [{ defId: "core-030", seed: "sweep2:easy:core-078:1", forced: "core-078", errors: 0, timeouts: 1 }]);
    expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
  });

  it(
    "R390 a real pass-2 game: named seed, boosted at-risk filler, the ban lifted for judgement bans and kept for bugs, timeouts charged to the forced card and listed against the filler",
    { timeout: 600_000 },
    () => {
      // Every card on today's ban is at risk and lifted; ten more at-risk cards are kept out as if a
      // pass 1 had flagged them `error`. A fake clock makes every AI decision 2.5 s long.
      const keepOut = aiPool().filter((id) => !SHADOW_BAN_IDS.includes(id) && id !== "core-011").slice(0, 10);
      const atRisk = [...SHADOW_BAN_IDS, ...keepOut, "core-011"].sort();
      let clock = 0;
      const timed = sweepAtRisk("core-011", atRisk, keepOut, { seeds: 1, tier: "easy", now: () => (clock += 2500) });
      expect(timed.forced).toBe("core-011");
      expect(timed.games).toBe(1);
      const ids = timed.cards.map((card) => card.defId);
      expect(ids[0]).toBe("core-011");
      for (const id of ids) expect(atRisk).toContain(id);
      for (const id of keepOut) expect(ids).not.toContain(id);
      expect(ids.some((id) => SHADOW_BAN_IDS.includes(id)), `dealt: ${ids.join(", ")}`).toBe(true);

      const [forced, ...filler] = timed.cards;
      expect(forced?.timeouts).toBeGreaterThan(0);
      for (const card of filler) expect(card).toMatchObject({ games: 1, errors: 0, timeouts: 0 });
      expect(timed.suspects.map((entry) => entry.defId)).toEqual(filler.map((card) => card.defId));
      for (const entry of timed.suspects) {
        expect(entry).toMatchObject({ seed: "sweep2:easy:core-011:1", forced: "core-011", errors: 0, timeouts: forced?.timeouts });
      }

      // The same game without a clock: the clock only measures, so the deal and the play are the same.
      const plain = sweepAtRisk("core-011", atRisk, keepOut, { seeds: 1, tier: "easy" });
      expect(plain.suspects).toEqual([]);
      expect(plain.cards.map((card) => ({ ...card, timeouts: 0 }))).toEqual(timed.cards.map((card) => ({ ...card, timeouts: 0 })));
    },
  );
});
