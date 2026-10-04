// Quality gate: the AI against the one-ply greedy baseline (docs/polish/3-ai.md B29, B31).
//
// The AI on Easy at AI_GATE_BUDGET must win at least `gateNeeded("ai-vs-greedy", n)` of its n games
// against `greedyAction` at equal (Easy) resources, seats alternating (gate-random.test.ts holds the
// rule that count comes from). `pnpm test` plays the smoke size (AI_GATE.smokeSeeds); `pnpm ai:gate`
// sets JACKIOH_AI_GATE=full and plays AI_GATE.fullSeeds["ai-vs-greedy"]. A failure names the seeds
// the AI did not win, which replay exactly through `gameConfig("ai-vs-greedy", n)`.
// Every game also has to be clean (B31): nothing rejected, nothing thrown, no fallback, a real
// result, and a log that folds back to the live hash.

import { describe, expect, it } from "vitest";
import { AI_DIFFICULTY, HUMAN_HANDICAP, createRng, type GameState, type Handicap } from "@jackioh/engine";
import {
  AI_EVAL,
  AI_GATE,
  AI_GATE_BUDGET,
  AI_TUNING_SERIES,
  GREEDY_EVAL,
  SHADOW_BAN_IDS,
  buildAiDeck,
  greedyAction,
  playMatch,
  gameConfig,
  gateNeeded,
  runGateGames,
  type GateReport,
  type Matchup,
} from "../src/index";
import { gamesToPlay, gateShard, writeShard } from "./_shard";

const MATCHUP: Matchup = "ai-vs-greedy";
const FULL = process.env["JACKIOH_AI_GATE"] === "full";
const GAMES = FULL ? AI_GATE.fullSeeds[MATCHUP] : AI_GATE.smokeSeeds;
const NEEDED = gateNeeded(MATCHUP, GAMES);
const SHARD = gateShard();
/** The games this process plays: all of them, or its shard's when CI splits the run (./_shard.ts). */
const PLAYED = gamesToPlay(GAMES, SHARD);
const SHARD_LABEL = SHARD === undefined ? "" : `, shard ${String(SHARD.index)}/${String(SHARD.count)}: ${String(PLAYED.length)} played`;
/** Per-game allowance under load (the machine is shared), plus a fixed margin. */
const TIMEOUT = 60_000 + GAMES * 45_000;

let cached: GateReport | undefined;
function report(): GateReport {
  cached ??= runGateGames(MATCHUP, PLAYED, AI_GATE_BUDGET);
  return cached;
}

function losingSeeds(gate: GateReport): string {
  return gate.games
    .filter((game) => !game.won)
    .map((game) => `${game.seed} (${game.subjectSeat}, ${JSON.stringify(game.record.result)})`)
    .join(", ");
}

describe(`gate ${MATCHUP} (${FULL ? "full" : "smoke"}: ${GAMES} games${SHARD_LABEL})`, () => {
  it("B29: gameConfig seats the Easy AI against the greedy baseline at equal resources, alternating seats", () => {
    for (const n of [1, 2, 3]) {
      const config = gameConfig(MATCHUP, n, AI_GATE_BUDGET);
      const seed = `${AI_GATE.seedSeries}:${MATCHUP}:${n}`;
      const subject = n % 2 === 1 ? "p1" : "p2";
      const other = subject === "p1" ? "p2" : "p1";
      const easy: Handicap = AI_DIFFICULTY.easy;

      expect(config.seed).toBe(seed);
      expect(config.controllers[subject]).toEqual({ kind: "ai", budget: AI_GATE_BUDGET });
      expect(config.controllers[other]).toEqual({ kind: "greedy" });
      expect(config.handicaps?.[subject] ?? HUMAN_HANDICAP).toEqual(easy);
      expect(config.handicaps?.[other] ?? HUMAN_HANDICAP).toEqual(easy);

      const at = (seat: "p1" | "p2"): number => (seat === "p1" ? 0 : 1);
      expect(config.decks[at(subject)]).toEqual(
        buildAiDeck(createRng(`${seed}:deck:${subject}`), easy.deckSize, { manaCap: easy.manaCap }),
      );
      expect(config.decks[at(other)]).toEqual(
        buildAiDeck(createRng(`${seed}:deck:${other}`), easy.deckSize, { manaCap: easy.manaCap }),
      );
    }
  });

  it("B29: the gate plays its own frozen seed series, never the one tuning plays, and deals both seats by one deck rule", () => {
    expect(AI_GATE.seedSeries).toBe("gate:v3");
    expect(AI_TUNING_SERIES).not.toBe(AI_GATE.seedSeries);
    expect(gameConfig(MATCHUP, 1).seed).toBe(`${AI_GATE.seedSeries}:${MATCHUP}:1`);
    expect(gameConfig(MATCHUP, 1, AI_GATE_BUDGET, AI_TUNING_SERIES).seed).toBe(`${AI_TUNING_SERIES}:${MATCHUP}:1`);
    // Neither side is dealt a card the AI's shadow ban keeps out of the other (R186).
    const banned = new Set(SHADOW_BAN_IDS);
    for (let n = 1; n <= 40; n += 1) {
      for (const deck of gameConfig(MATCHUP, n).decks) {
        for (const id of deck) expect(banned.has(id), `game ${String(n)}: ${id}`).toBe(false);
      }
    }
  });

  it("B29: the greedy baseline keeps its own frozen weights, so tuning the AI's never moves it", { timeout: 60_000 }, () => {
    expect(GREEDY_EVAL).not.toBe(AI_EVAL);
    // Every greedy decision of a game, made again with the AI's weights turned upside down, is the
    // same decision: greedy reads GREEDY_EVAL and nothing else. Greedy plays both seats here, so
    // the states come quickly.
    const config = gameConfig(MATCHUP, 2);
    const greedySeat = "p1";
    expect(config.controllers[greedySeat]).toEqual({ kind: "greedy" });
    const states: GameState[] = [];
    playMatch({ ...config, controllers: { p1: { kind: "greedy" }, p2: { kind: "greedy" } }, maxActions: 120 }, {
      afterAction(before, _after, seat) {
        if (seat === greedySeat && before.pending === null) states.push(before);
      },
    });
    expect(states.length, "greedy acted in its main phase").toBeGreaterThan(3);
    const decisions = (): unknown[] => states.map((state, i) => greedyAction(state, greedySeat, createRng(`greedy-probe:${String(i)}`)));
    const before = decisions();
    const saved = JSON.parse(JSON.stringify(AI_EVAL)) as Record<string, unknown>;
    try {
      Object.assign(AI_EVAL as unknown as Record<string, unknown>, {
        heroHealth: -5,
        attack: -3,
        health: -3,
        handCard: 40,
        threatPerDamage: -10,
        pressurePerDamage: -10,
      });
      expect(decisions()).toEqual(before);
    } finally {
      Object.assign(AI_EVAL as unknown as Record<string, unknown>, saved);
    }
  });

  it(`B29: the Easy AI wins at least ${NEEDED} of ${GAMES} games against the greedy baseline`, { timeout: TIMEOUT }, () => {
    const gate = report();
    expect(gate.matchup).toBe(MATCHUP);
    expect(gate.games).toHaveLength(PLAYED.length);
    gate.games.forEach((game, at) => {
      expect(game.seed).toBe(`${AI_GATE.seedSeries}:${MATCHUP}:${String(PLAYED[at])}`);
      expect(game.subjectSeat).toBe((PLAYED[at] ?? 0) % 2 === 1 ? "p1" : "p2");
      expect(game.won).toBe(game.record.result?.winner === game.subjectSeat);
    });
    // Only wins count: a draw at the turn cap is reported beside them and is a game the AI did not close.
    expect(gate.wins).toBe(gate.games.filter((game) => game.won).length);
    expect(gate.rate).toBeCloseTo(gate.wins / PLAYED.length, 10);
    // A shard's wins are a share of the run's, so `pnpm ai:gate:merge` holds them against NEEDED
    // together with the other shards'. The games' cleanliness is still checked here (B31).
    if (SHARD !== undefined) {
      const path = writeShard(gate, GAMES, SHARD, PLAYED);
      process.stdout.write(
        `[gate ${MATCHUP}${SHARD_LABEL}] ${String(gate.wins)} wins and ${String(gate.turnCapDraws)} turn-cap draws, written to ${path}\n`,
      );
      return;
    }
    // Written to stdout, as the fuzz suite writes its numbers: vitest's default reporter swallows
    // console output from a passing test, and a green gate should still show how it passed.
    process.stdout.write(
      `[gate ${MATCHUP}] ${String(gate.wins)} wins and ${String(gate.turnCapDraws)} turn-cap draws of ${String(GAMES)}; ${String(NEEDED)} wins needed\n`,
    );
    expect(
      gate.wins,
      `${String(gate.wins)} wins, ${String(gate.turnCapDraws)} turn-cap draws; not won: ${losingSeeds(gate)}`,
    ).toBeGreaterThanOrEqual(NEEDED);
  });

  it("B31: every game is clean: nothing rejected or thrown, no fallback, a result, and a replay that matches", { timeout: TIMEOUT }, () => {
    const gate = report();
    expect(gate.games).toHaveLength(PLAYED.length);
    for (const game of gate.games) {
      const label = `${game.seed} (${game.subjectSeat})`;
      expect(game.record.rejected, label).toEqual([]);
      expect(game.record.thrown, label).toEqual([]);
      expect(game.record.fallbacks, label).toBe(0);
      expect(game.record.result, label).not.toBeNull();
      expect(game.replayErrors, label).toBe(0);
      expect(game.replayHash, label).toBe(game.record.hash);
    }
  });
});
