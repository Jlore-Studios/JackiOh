// `pnpm ai:stats` (SPEC §9.11, R378): an internal AI development run of the build in this checkout.
//
// Plays AI-against-AI All Random games (`devGameRecord`, packages/ai/src/devRun.ts), writes each
// finished one as a game record, one JSON line per game, and prints the run's card win rates. The
// records are tagged `source: "dev"` and filed under the patch the run tests, so once they are
// loaded with `pnpm --filter @jackioh/server stats:import <file>`, `stats:cards --source=dev
// --patch=<version>` reads them beside that patch's live games, and no live figure ever counts them
// unless asked.
//
//   pnpm ai:stats                                  AI_DEV_RUN.games games of the "dev" series,
//                                                  tagged with the newest patch in patches.json
//   pnpm ai:stats --games=50 --from=51             games 51-100, so slices run in parallel
//   pnpm ai:stats --patch=v0.2.5 --out=v0.2.5.jsonl
//                                                  a pre-release run of v0.2.5, kept in a file
//   pnpm ai:stats --series=tuning-a                another seed series: other games
//
// A game takes seconds at the browser's budget, so a full run takes a while; progress goes to
// stderr. `--out` is resolved against the directory the command was started in, and the file is
// written a line per game as the run goes, so a run that is stopped keeps the games it finished.
//
// Node tooling, so it may read files, the clock and the console; src/ stays pure.

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG, registerAll } from "@jackioh/cards";
import { newestPatch, type Patch } from "@jackioh/cards/history";
import { cardStats, formatCardStats, type GameRecord } from "@jackioh/shared";
import { AI_DEV_RUN, devGameRecord } from "../src/index";

const MS_PER_SECOND = 1000;
/** Seconds print with this many decimals. */
const SECONDS_DECIMALS = 1;

const PATCHES_PATH = fileURLToPath(new URL("../../cards/patches/patches.json", import.meta.url));

const USAGE = `Usage: pnpm ai:stats [options]

  --games=N           How many games. Default ${String(AI_DEV_RUN.games)}.
  --from=N            The first game's number, so slices of a run can go in parallel. Default 1.
  --patch=<version>   The patch the run tests. Default: the newest in patches.json.
  --series=<name>     The seed series. Default "${AI_DEV_RUN.series}".
  --out=<file>        Write the records here, one JSON line per game (stats:import reads it).`;

export type DevRunArgs = { games: number; from: number; patch: string | null; series: string; out: string | null };

function count(name: string, raw: string): number {
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`--${name} must be a positive integer (got ${JSON.stringify(raw)}).\n\n${USAGE}`);
  }
  return value;
}

/** `--flag=value` arguments, refusing anything else. pnpm's own `--` is skipped. */
export function parseDevRunArgs(argv: readonly string[]): DevRunArgs {
  const args: DevRunArgs = { games: AI_DEV_RUN.games, from: 1, patch: null, series: AI_DEV_RUN.series, out: null };
  for (const arg of argv) {
    if (arg === "--") continue;
    const match = /^--(?<name>[a-z]+)=(?<value>.+)$/u.exec(arg);
    const name = match?.groups?.["name"];
    const raw = match?.groups?.["value"];
    if (name === undefined || raw === undefined) throw new Error(`Unrecognised argument ${JSON.stringify(arg)}.\n\n${USAGE}`);
    if (name === "games") args.games = count(name, raw);
    else if (name === "from") args.from = count(name, raw);
    else if (name === "patch") args.patch = raw;
    else if (name === "series") args.series = raw;
    else if (name === "out") args.out = raw;
    else throw new Error(`Unrecognised option --${name}.\n\n${USAGE}`);
  }
  return args;
}

function main(): void {
  const args = parseDevRunArgs(process.argv.slice(2));
  registerAll();

  const patch = args.patch ?? newestPatch(JSON.parse(readFileSync(PATCHES_PATH, "utf8")) as Patch[]).version;
  // pnpm runs this in packages/ai; a path the user typed means the directory they typed it in.
  const out = args.out === null ? null : resolve(process.env["INIT_CWD"] ?? process.cwd(), args.out);
  if (out !== null) writeFileSync(out, "");

  const records: GameRecord[] = [];
  const started = performance.now();
  for (let n = args.from; n < args.from + args.games; n += 1) {
    const gameStarted = performance.now();
    const record = devGameRecord(n, { series: args.series, patch });
    const seconds = ((performance.now() - gameStarted) / MS_PER_SECOND).toFixed(SECONDS_DECIMALS);
    if (record === null) {
      process.stderr.write(`[ai:stats] ${args.series}:${String(n)} did not finish; no record (${seconds}s)\n`);
      continue;
    }
    records.push(record);
    if (out !== null) appendFileSync(out, `${JSON.stringify(record)}\n`);
    process.stderr.write(
      `[ai:stats] ${args.series}:${String(n)} ${record.game.winner === "draw" ? "draw" : `${record.game.winner} won`} ` +
        `by ${record.game.reason} in ${String(record.game.turns)} turns (${seconds}s)\n`,
    );
  }

  const elapsed = ((performance.now() - started) / MS_PER_SECOND).toFixed(SECONDS_DECIMALS);
  process.stderr.write(
    `[ai:stats] ${String(records.length)} of ${String(args.games)} games recorded in ${elapsed}s` +
      `${out === null ? "" : `, written to ${out}`}\n`,
  );
  const report = cardStats(records, { source: "dev", mode: "random", patch, pilot: "ai" });
  process.stdout.write(`${formatCardStats(report, (card) => CATALOG[card]?.name)}\n`);
}

// Only run when invoked directly, so a test can import `parseDevRunArgs` without playing a run.
if (process.argv[1] !== undefined && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/"))) {
  main();
}
