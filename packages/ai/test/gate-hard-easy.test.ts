// Quality gate: the same AI with Hard's handicap against itself on Easy (docs/polish/3-ai.md B30, B31).
//
// R180: the tiers differ in resources alone, so the only thing separating the two seats here is
// the handicap. The Hard seat (the subject) must win at least `gateNeeded("hard-vs-easy", n)` of its
// n games, seats alternating, both AIs at AI_GATE_BUDGET (gate-random.test.ts holds the rule that
// count comes from). `pnpm test` plays the smoke size (AI_GATE.smokeSeeds); `pnpm ai:gate` sets
// JACKIOH_AI_GATE=full and plays AI_GATE.fullSeeds["hard-vs-easy"]. A failure names the seeds the
// Hard AI did not win, which replay exactly through `gameConfig("hard-vs-easy", n)`.
// Every game also has to be clean (B31): nothing rejected, nothing thrown, no fallback, a real
// result, and a log that folds back to the live hash.

import { describe, expect, it } from "vitest";
import { AI_DIFFICULTY, HUMAN_HANDICAP, createRng, type Handicap } from "@jackioh/engine";
import {
  AI_GATE,
  AI_GATE_BUDGET,
  buildAiDeck,
  gameConfig,
  gateNeeded,
  runGateGames,
  type GateReport,
  type Matchup,
} from "../src/index";
import { gamesToPlay, gateShard, writeShard } from "./_shard";

const MATCHUP: Matchup = "hard-vs-easy";
const FULL = process.env["JACKIOH_AI_GATE"] === "full";
const GAMES = FULL ? AI_GATE.fullSeeds[MATCHUP] : AI_GATE.smokeSeeds;
const NEEDED = gateNeeded(MATCHUP, GAMES);
const SHARD = gateShard();
/** The games this process plays: all of them, or its shard's when CI splits the run (./_shard.ts). */
const PLAYED = gamesToPlay(GAMES, SHARD);
const SHARD_LABEL = SHARD === undefined ? "" : `, shard ${String(SHARD.index)}/${String(SHARD.count)}: ${String(PLAYED.length)} played`;
/** Per-game allowance under load (the machine is shared), plus a fixed margin. */
const TIMEOUT = 60_000 + PLAYED.length * 45_000;

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
  it("R180 B30: gameConfig seats Hard's handicap against Easy's, the same AI and budget on both, alternating seats", () => {
    for (const n of [1, 2, 3]) {
      const config = gameConfig(MATCHUP, n, AI_GATE_BUDGET);
      const seed = `${AI_GATE.seedSeries}:${MATCHUP}:${n}`;
      const subject = n % 2 === 1 ? "p1" : "p2";
      const other = subject === "p1" ? "p2" : "p1";
      const hard: Handicap = AI_DIFFICULTY.hard;
      const easy: Handicap = AI_DIFFICULTY.easy;

      expect(config.seed).toBe(seed);
      expect(config.controllers[subject]).toEqual({ kind: "ai", budget: AI_GATE_BUDGET });
      expect(config.controllers[other]).toEqual({ kind: "ai", budget: AI_GATE_BUDGET });
      expect(config.handicaps?.[subject] ?? HUMAN_HANDICAP).toEqual(hard);
      expect(config.handicaps?.[other] ?? HUMAN_HANDICAP).toEqual(easy);

      const at = (seat: "p1" | "p2"): number => (seat === "p1" ? 0 : 1);
      expect(config.decks[at(subject)]).toHaveLength(hard.deckSize);
      expect(config.decks[at(other)]).toHaveLength(easy.deckSize);
      expect(config.decks[at(subject)]).toEqual(
        buildAiDeck(createRng(`${seed}:deck:${subject}`), hard.deckSize, { manaCap: hard.manaCap }),
      );
      expect(config.decks[at(other)]).toEqual(
        buildAiDeck(createRng(`${seed}:deck:${other}`), easy.deckSize, { manaCap: easy.manaCap }),
      );
    }
  });

  it(`B30: the Hard AI wins at least ${NEEDED} of ${GAMES} games against itself on Easy`, { timeout: TIMEOUT }, () => {
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
