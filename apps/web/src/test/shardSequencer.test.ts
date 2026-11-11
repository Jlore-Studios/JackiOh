// @vitest-environment node
// The weighted shards of `web (unit k/8)` (#3666): shardWeights.ts's deal and the sequencer that hands it to
// Vitest's `--shard`. The deal works on synthetic paths and weights, so the real table's timings never
// decide a test; the table itself is held only to name files that exist.

import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import WeightedShardSequencer from "./shardSequencer.ts";
import { DEFAULT_WEIGHT_S, WEIGHTS, dealShards } from "./shardWeights.ts";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../../../..");

const PATHS = Array.from({ length: 60 }, (_, i) => `apps/web/src/f${i}.test.ts`);
const COUNTS = [1, 2, 3, 4, 5, 6, 7, 8];

/** A few heavy paths among many light ones. */
const WEIGHTED: Record<string, number> = {
  "apps/web/src/f3.test.ts": 30,
  "apps/web/src/f17.test.ts": 20,
  "apps/web/src/f40.test.ts": 12,
  "apps/web/src/f41.test.ts": 5,
};

const load = (shard: readonly string[], weights: Readonly<Record<string, number>>): number =>
  shard.reduce((sum, path) => sum + (weights[path] ?? DEFAULT_WEIGHT_S), 0);

describe("#3666 the web unit tests' weighted shards", () => {
  it("#3666 every path lands in exactly one shard, for each count from 1 to 8", () => {
    for (const count of COUNTS) {
      const shards = dealShards(PATHS, count, WEIGHTED);
      expect(shards).toHaveLength(count);
      expect(shards.flat().sort()).toEqual([...PATHS].sort());
    }
  });

  it("#3666 the shards are the same whatever order the paths arrive in", () => {
    for (const count of COUNTS) {
      expect(dealShards([...PATHS].reverse(), count, WEIGHTED)).toEqual(dealShards(PATHS, count, WEIGHTED));
    }
  });

  it("#3666 no two shards differ by more than the heaviest path", () => {
    for (const count of COUNTS.slice(1)) {
      const loads = dealShards(PATHS, count, WEIGHTED).map((shard) => load(shard, WEIGHTED));
      expect(Math.max(...loads) - Math.min(...loads)).toBeLessThanOrEqual(30);
    }
  });

  it("#3666 a path with no weight counts DEFAULT_WEIGHT_S", () => {
    const big = "apps/web/src/big.test.ts";
    const others = Array.from({ length: 10 }, (_, i) => `apps/web/src/other${i}.test.ts`);
    const shards = dealShards([big, ...others], 2, { [big]: 10 * DEFAULT_WEIGHT_S });
    // The ten others, unlisted, weigh as much together as `big` alone: it stands alone in its shard.
    expect(shards.find((shard) => shard.includes(big))).toEqual([big]);
    expect(shards.flat().sort()).toEqual([big, ...others].sort());
  });

  it("#3666 WEIGHTS names only files that exist, in whole seconds", () => {
    for (const [path, seconds] of Object.entries(WEIGHTS)) {
      expect(existsSync(join(ROOT, path)), path).toBe(true);
      expect(path, path).toMatch(/\.test\.tsx?$/);
      expect(Number.isInteger(seconds) && seconds > 0, path).toBe(true);
    }
  });

  it("#3666 WeightedShardSequencer.shard runs every file once across a count's shards", async () => {
    const count = 3;
    const specs = Array.from({ length: 20 }, (_, i) => ({ moduleId: join(ROOT, `apps/web/src/a${i}.test.ts`) }));
    const ran: string[] = [];
    for (let index = 1; index <= count; index++) {
      const ctx = { config: { root: ROOT, shard: { index, count } } };
      const sequencer = new WeightedShardSequencer(ctx as unknown as ConstructorParameters<typeof WeightedShardSequencer>[0]);
      const mine = await sequencer.shard(specs as unknown as Parameters<WeightedShardSequencer["shard"]>[0]);
      ran.push(...mine.map((spec) => spec.moduleId));
    }
    expect(ran.sort()).toEqual(specs.map((spec) => spec.moduleId).sort());
  });
});
