// Quality gate: the AI against §10.7's random policy (docs/polish/3-ai.md B28, B31).
//
// The AI on Easy at AI_GATE_BUDGET must win at least `gateNeeded("ai-vs-random", n)` of its n games,
// seats alternating. Only wins count, here as in every gate: a draw at the turn cap is reported
// beside the wins (`turnCapDraws`) and counts for nothing. `pnpm test` plays the smoke size
// (AI_GATE.smokeSeeds); `pnpm ai:gate` sets JACKIOH_AI_GATE=full and plays
// AI_GATE.fullSeeds["ai-vs-random"]. A failure names the seeds the AI did not win, which replay
// exactly through `gameConfig("ai-vs-random", n)`. Every game also has to be clean (B31): nothing
// rejected, nothing thrown, no fallback, a real result, and a log that folds back to the live hash.
//
// This file also holds the rule every gate's count comes from (`gateNeeded`, SPEC §9.9).

import { describe, expect, it } from "vitest";
import { AI_DIFFICULTY, HUMAN_HANDICAP, createRng, type Handicap } from "@jackioh/engine";
import {
  AI_GATE,
  AI_GATE_BUDGET,
  binomialTail,
  buildAiDeck,
  gameConfig,
  gateNeeded,
  runGateGames,
  type GateReport,
  type Matchup,
} from "../src/index";
import { gamesToPlay, gateShard, writeShard } from "./_shard";

const MATCHUP: Matchup = "ai-vs-random";
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

const MATCHUPS: readonly Matchup[] = ["ai-vs-random", "ai-vs-greedy", "hard-vs-easy"];

describe(`gate ${MATCHUP} (${FULL ? "full" : "smoke"}: ${GAMES} games${SHARD_LABEL})`, () => {
  it("B28: gameConfig seats the Easy AI against the random policy, alternating seats, with the specified decks", () => {
    for (const n of [1, 2, 3]) {
      const config = gameConfig(MATCHUP, n, AI_GATE_BUDGET);
      const seed = `${AI_GATE.seedSeries}:${MATCHUP}:${n}`;
      const subject = n % 2 === 1 ? "p1" : "p2";
      const other = subject === "p1" ? "p2" : "p1";
      const easy: Handicap = AI_DIFFICULTY.easy;

      expect(config.seed).toBe(seed);
      expect(config.controllers[subject]).toEqual({ kind: "ai", budget: AI_GATE_BUDGET });
      expect(config.controllers[other]).toEqual({ kind: "random" });
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

  it("binomialTail is P(X >= k) for X ~ Binomial(n, p)", () => {
    expect(binomialTail(2, 0.5, 0)).toBeCloseTo(1, 12);
    expect(binomialTail(2, 0.5, 1)).toBeCloseTo(0.75, 12);
    expect(binomialTail(2, 0.5, 2)).toBeCloseTo(0.25, 12);
    expect(binomialTail(3, 0.2, 3)).toBeCloseTo(0.008, 12);
    expect(binomialTail(20, 0.68, 21)).toBe(0);
    expect(binomialTail(20, 1, 20)).toBeCloseTo(1, 12);
    expect(binomialTail(20, 0, 1)).toBe(0);
  });

  it("B28-B30: each gate needs the brief's share of its games, or the count an AI as strong as measured reaches in 1 - falseAlarm of runs of that size, whichever is lower (SPEC §9.9)", () => {
    for (const matchup of MATCHUPS) {
      const measured = AI_GATE.measuredRate[matchup];
      for (const games of [AI_GATE.smokeSeeds, AI_GATE.fullSeeds[matchup]]) {
        const needed = gateNeeded(matchup, games);
        const label = `${matchup}, ${String(games)} games: ${String(needed)}`;
        // Never more than the brief asks for.
        expect(needed, label).toBeLessThanOrEqual(Math.ceil(AI_GATE.briefRate[matchup] * games));
        // An AI exactly as strong as the one measured passes at least 1 - falseAlarm of the time...
        expect(binomialTail(games, measured, needed), label).toBeGreaterThanOrEqual(1 - AI_GATE.falseAlarm);
        // ...and one more win would either break that or ask for more than the brief.
        const tighter = binomialTail(games, measured, needed + 1) < 1 - AI_GATE.falseAlarm;
        expect(tighter || needed === Math.ceil(AI_GATE.briefRate[matchup] * games), label).toBe(true);
      }
    }
  });

  it(`B28: the Easy AI wins at least ${NEEDED} of ${GAMES} games against the random policy`, { timeout: TIMEOUT }, () => {
    const gate = report();
    expect(gate.matchup).toBe(MATCHUP);
    expect(gate.games).toHaveLength(PLAYED.length);
    gate.games.forEach((game, at) => {
      expect(game.seed).toBe(`${AI_GATE.seedSeries}:${MATCHUP}:${String(PLAYED[at])}`);
      expect(game.subjectSeat).toBe((PLAYED[at] ?? 0) % 2 === 1 ? "p1" : "p2");
      expect(game.won).toBe(game.record.result?.winner === game.subjectSeat);
    });
    expect(gate.wins).toBe(gate.games.filter((game) => game.won).length);
    const capped = gate.games.filter((game) => game.record.result?.winner === "draw" && game.record.result.reason === "turn-cap");
    expect(gate.turnCapDraws).toBe(capped.length);
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
