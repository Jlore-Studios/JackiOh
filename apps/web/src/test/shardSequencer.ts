// The Vitest sequencer of the root vitest.config.ts (#3666): `--shard=k/K` runs the k-th of the K shares
// that shardWeights.ts's `dealShards` makes, in place of Vitest's own slices of equal count. Vitest's
// `sort()` still orders the files within a shard, as ever.
//
// A file is known to the table by its path from the repository root with `/` separators; CI runs from
// the root, so the keys match. Run from another directory, the paths differ, every file counts
// DEFAULT_WEIGHT_S and the deal is still a partition (each file in exactly one shard), only by count.

import { relative, sep } from "node:path";

import { BaseSequencer, type TestSpecification } from "vitest/node";

import { dealShards } from "./shardWeights.ts";

export default class WeightedShardSequencer extends BaseSequencer {
  override async shard(files: TestSpecification[]): Promise<TestSpecification[]> {
    const { index, count } = this.ctx.config.shard ?? { index: 1, count: 1 };
    // A file two projects both run has one path and, here, one entry holding both specifications.
    const byPath = new Map<string, TestSpecification[]>();
    for (const spec of files) {
      const path = relative(this.ctx.config.root, spec.moduleId).split(sep).join("/");
      byPath.set(path, [...(byPath.get(path) ?? []), spec]);
    }
    const mine = dealShards([...byPath.keys()], count)[index - 1] ?? [];
    return mine.flatMap((path) => byPath.get(path) ?? []);
  }
}
