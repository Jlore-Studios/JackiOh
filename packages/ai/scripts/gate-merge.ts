// `pnpm ai:gate:merge [dir]`: the full quality gates' verdict when CI has played them in shards
// (.github/workflows/ci.yml, test/_shard.ts). Each shard wrote the games it played, one file per
// matchup, to `dir` (default `ai-gate-shards/`); this reads them all and, per matchup, holds the run
// to exactly what one unsharded `pnpm ai:gate` would: every game 1..AI_GATE.fullSeeds[matchup]
// played exactly once, and at least `gateNeeded(matchup, games)` won. The shards have already
// checked each game is clean (B31) and timed every perf decision (B42), which need no merging.
// Exit 1 names the first problem: a missing or doubled game, or too few wins with the seeds lost.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { AI_GATE, gateNeeded, type Matchup } from "../src/index";
import type { ShardFile } from "../test/_shard";

const MATCHUPS: readonly Matchup[] = ["ai-vs-random", "ai-vs-greedy", "hard-vs-easy"];

function main(): number {
  const dir = process.argv[2] ?? "ai-gate-shards";
  const files = readdirSync(dir, { recursive: true })
    .map(String)
    .filter((name) => name.endsWith(".json"))
    .map((name) => JSON.parse(readFileSync(join(dir, name), "utf8")) as ShardFile);

  let failed = false;
  for (const matchup of MATCHUPS) {
    const total = AI_GATE.fullSeeds[matchup];
    const needed = gateNeeded(matchup, total);
    const games = files.filter((file) => file.matchup === matchup).flatMap((file) => {
      if (file.total !== total) {
        throw new Error(`${matchup}: shard ${file.shard} played a run of ${String(file.total)}, not ${String(total)}`);
      }
      return file.games;
    });

    const seen = new Map<number, number>();
    for (const game of games) seen.set(game.n, (seen.get(game.n) ?? 0) + 1);
    const missing: number[] = [];
    const doubled: number[] = [];
    for (let n = 1; n <= total; n += 1) {
      const count = seen.get(n) ?? 0;
      if (count === 0) missing.push(n);
      if (count > 1) doubled.push(n);
    }
    const strays = [...seen.keys()].filter((n) => !(Number.isInteger(n) && n >= 1 && n <= total));

    const wins = games.filter((game) => game.won).length;
    const draws = games.filter((game) => game.turnCapDraw).length;
    process.stdout.write(
      `[gate ${matchup}] ${String(wins)} wins and ${String(draws)} turn-cap draws of ${String(total)}; ${String(needed)} wins needed\n`,
    );

    if (missing.length > 0 || doubled.length > 0 || strays.length > 0) {
      failed = true;
      process.stderr.write(
        `${matchup}: games missing [${missing.join(", ")}], played twice [${doubled.join(", ")}], ` +
          `out of range [${strays.join(", ")}]; every shard must upload its file\n`,
      );
    } else if (wins < needed) {
      failed = true;
      const lost = games
        .filter((game) => !game.won)
        .sort((a, b) => a.n - b.n)
        .map((game) => `${game.seed} (${game.subjectSeat}, ${JSON.stringify(game.result)})`)
        .join(", ");
      process.stderr.write(`${matchup}: ${String(wins)} wins, ${String(needed)} needed; not won: ${lost}\n`);
    }
  }
  return failed ? 1 : 0;
}

process.exitCode = main();
