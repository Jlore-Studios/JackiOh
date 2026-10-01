// CI plays the full gates (`pnpm ai:gate`) in shards so no job runs for long (.github/workflows/ci.yml).
// `JACKIOH_AI_GATE_SHARD=k/K` makes each gate file play only its share of the games
// (`gateShardGames`): every game it plays must still be clean (B31) and the perf gate still times
// every decision of its games, but a win-rate threshold is a property of the whole run, so a shard
// writes its games to `JACKIOH_AI_GATE_OUT` and `pnpm ai:gate:merge` holds the wins of all shards
// together against `gateNeeded`. Unsharded, every gate plays and judges its whole run as before.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { gateShardGames, type GateReport, type Matchup } from "../src/index";

export type GateShard = { index: number; count: number };

/** The shard this process plays, from `JACKIOH_AI_GATE_SHARD` ("k/K", 1 ≤ k ≤ K), or none. */
export function gateShard(): GateShard | undefined {
  const raw = process.env["JACKIOH_AI_GATE_SHARD"];
  if (raw === undefined || raw.trim() === "") return undefined;
  const match = /^(\d+)\/(\d+)$/.exec(raw.trim());
  const index = Number(match?.[1]);
  const count = Number(match?.[2]);
  if (match === null || index < 1 || count < 1 || index > count) {
    throw new Error(`JACKIOH_AI_GATE_SHARD must be "k/K" with 1 ≤ k ≤ K, not ${JSON.stringify(raw)}`);
  }
  return { index, count };
}

/** The game numbers (1-based) this process plays out of a run of `total`. */
export function gamesToPlay(total: number, shard: GateShard | undefined): number[] {
  return shard === undefined ? gateShardGames(total, 1, 1) : gateShardGames(total, shard.index, shard.count);
}

/** One shard's games, as `scripts/gate-merge.ts` reads them. */
export type ShardFile = {
  matchup: Matchup;
  total: number;
  shard: string;
  games: { n: number; seed: string; subjectSeat: string; won: boolean; turnCapDraw: boolean; result: unknown }[];
};

/** Writes a shard's games to `JACKIOH_AI_GATE_OUT` (default `ai-gate-shards/` in the cwd). */
export function writeShard(gate: GateReport, total: number, shard: GateShard, played: readonly number[]): string {
  const dir = process.env["JACKIOH_AI_GATE_OUT"] ?? "ai-gate-shards";
  mkdirSync(dir, { recursive: true });
  const file: ShardFile = {
    matchup: gate.matchup,
    total,
    shard: `${String(shard.index)}/${String(shard.count)}`,
    games: gate.games.map((game, at) => ({
      n: played[at] ?? Number.NaN,
      seed: game.seed,
      subjectSeat: game.subjectSeat,
      won: game.won,
      turnCapDraw: game.record.result?.winner === "draw" && game.record.result.reason === "turn-cap",
      result: game.record.result,
    })),
  };
  const path = join(dir, `${gate.matchup}-${String(shard.index)}-of-${String(shard.count)}.json`);
  writeFileSync(path, `${JSON.stringify(file, null, 2)}\n`);
  return path;
}
