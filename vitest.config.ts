import { defineConfig } from "vitest/config";

import WeightedShardSequencer from "./apps/web/src/test/shardSequencer.ts";

export default defineConfig({
  test: {
    // The web's unit tests (apps/web/vitest.config.ts, project `web`), the Worker's
    // (apps/edge/vitest.config.ts, project `edge`) and the dev dashboard's (apps/admin/vitest.config.ts,
    // project `admin`, #3074): the rules, the cards, the AI and the server are tested by `cargo test`.
    projects: ["apps/admin", "apps/edge", "apps/web"],
    // `--shard=k/K` (ci.yml's `web (unit k/K)`) deals the files by their measured time, by the table in
    // apps/web/src/test/shardWeights.ts, not by count (#3666).
    sequence: { sequencer: WeightedShardSequencer },
  },
});
