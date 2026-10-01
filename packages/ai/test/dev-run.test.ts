// SPEC §9.11, R378: the AI's development run. A development game is an All Random game with two AI
// pilots on this spec's resources, dealt exactly as the server deals a live one, and it is filed as
// a record of its own source, which no live figure counts unless it is asked for. The games here run
// at a small budget so the file stays fast; `scripts/stats.ts` plays at the browser's.

import { describe, expect, it } from "vitest";
import { DECK_SIZE, createRng, summarizeGame } from "@jackioh/engine";
import { cardStats, DEFAULT_CARD_STATS_FILTER } from "@jackioh/shared";
import { AI_BUDGET, AI_DEV_RUN, buildAiDeck, devGameConfig, devGameRecord, playMatch, type SearchBudget } from "../src/index";
import { parseDevRunArgs } from "../scripts/stats";

const QUICK: SearchBudget = {
  nodes: 30,
  lethalNodes: 10,
  determinizations: 1,
  beamWidth: 1,
  rootBranching: 6,
  branching: 2,
  maxDepth: 2,
  finalists: 1,
};

describe("the AI's development run (§9.11)", () => {
  it("R378 deals game n as All Random deals it, to two AI seats on this spec's resources", () => {
    const config = devGameConfig(7, { series: AI_DEV_RUN.series });
    expect(config.seed).toBe(`${AI_DEV_RUN.series}:7`);
    // R258: the server's deal, `${seed}:p1-deck` and `${seed}:p2-deck`, nothing banned.
    expect(config.decks).toEqual([
      buildAiDeck(createRng(`${config.seed}:p1-deck`), DECK_SIZE, { banned: [] }),
      buildAiDeck(createRng(`${config.seed}:p2-deck`), DECK_SIZE, { banned: [] }),
    ]);
    expect(config.handicaps).toBeUndefined();
    expect(config.controllers).toEqual({ p1: { kind: "ai", budget: AI_BUDGET }, p2: { kind: "ai", budget: AI_BUDGET } });
    expect(devGameConfig(7, { series: "other" }).decks).not.toEqual(config.decks);
  });

  it("R378 files each finished game as a development record of the patch it tests", { timeout: 120_000 }, () => {
    const options = { series: "dev-test", patch: "v0.2.5", budget: QUICK };
    const records = [1, 2].map((n) => devGameRecord(n, options));

    records.forEach((record, at) => {
      const n = at + 1;
      if (record === null) throw new Error(`dev-test:${String(n)} did not finish`);
      const config = devGameConfig(n, options);
      expect(record).toMatchObject({
        id: `dev:dev-test:${String(n)}`,
        source: "dev",
        mode: "random",
        patch: "v0.2.5",
        pilots: { p1: "ai", p2: "ai" },
      });
      // The game half is the engine's own reading of the game the AI played.
      const played = playMatch(config);
      expect(record.game).toEqual(summarizeGame({ seed: config.seed, decks: config.decks, log: played.log }));
      expect(record.game.winner).toBe(played.result?.winner);
      expect(record.game.seats.p1.deck).toEqual(config.decks[0]);
    });
    // Seeded: the same game is the same record.
    expect(devGameRecord(1, options)).toEqual(records[0]);

    // R378: a live query reads none of them; a development query reads both.
    const finished = records.filter((record): record is NonNullable<typeof record> => record !== null);
    expect(cardStats(finished, DEFAULT_CARD_STATS_FILTER).games).toBe(0);
    expect(cardStats(finished, { ...DEFAULT_CARD_STATS_FILTER, source: "dev" }).games).toBe(2);
    expect(cardStats(finished, { ...DEFAULT_CARD_STATS_FILTER, source: "dev", pilot: "human" }).games).toBe(0);
  });

  it("reads the run's options, and refuses one it does not know", () => {
    expect(parseDevRunArgs([])).toEqual({ games: AI_DEV_RUN.games, from: 1, patch: null, series: AI_DEV_RUN.series, out: null });
    expect(parseDevRunArgs(["--", "--games=50", "--from=51", "--patch=v0.2.5", "--series=a", "--out=run.jsonl"])).toEqual({
      games: 50,
      from: 51,
      patch: "v0.2.5",
      series: "a",
      out: "run.jsonl",
    });
    expect(() => parseDevRunArgs(["--games=0"])).toThrow(/--games must be a positive integer/);
    expect(() => parseDevRunArgs(["--from=1.5"])).toThrow(/--from must be a positive integer/);
    expect(() => parseDevRunArgs(["--budget=9"])).toThrow(/Unrecognised option --budget/);
    expect(() => parseDevRunArgs(["50"])).toThrow(/Unrecognised argument/);
  });
});
