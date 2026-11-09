// Stryker over the web's layers (#3490): whether a unit test fails when the code it covers is wrong,
// which coverage cannot say. `sh scripts/mutants.sh web-<layer> [k]` runs one layer's shard and
// mutants.yml runs one of each layer every week; `pnpm --dir apps/web exec stryker run
// stryker.config.mjs` runs all of them at once. It reports and gates nothing yet (`break: null`).

/** @type {import("@stryker-mutator/api/core").PartialStrykerOptions} */
export default {
  testRunner: "vitest",
  // Named, not found by Stryker's `@stryker-mutator/*` glob, which pnpm's layout hides from it.
  plugins: ["@stryker-mutator/vitest-runner"],
  // `related`: each mutant runs only the test files that import what it changed.
  vitest: { configFile: "vitest.config.ts", related: true },
  // In place, not in a sandbox copy of apps/web: the tests read files by their path from the
  // repository's root (the spec's notes, the stylesheets, the voice and music manifests). Stryker
  // puts each file back when it ends.
  inPlace: true,
  // Game.tsx's layers (CLAUDE.md, Architecture: animations, effects, audio, cards, drag, settings)
  // and the client's two adapters, the wire and the WebAssembly module's, without their tests, the
  // generated (ts-rs's types and the engine's and the server's constants) or the built module.
  mutate: [
    "src/game/animations.ts",
    "src/fx/**/*.{ts,tsx}",
    "src/audio/**/*.{ts,tsx}",
    "src/cards/**/*.{ts,tsx}",
    "src/game/drag/**/*.{ts,tsx}",
    "src/settings/**/*.{ts,tsx}",
    "src/wire/**/*.ts",
    "src/wasm/**/*.{ts,tsx}",
    "!src/**/*.test.{ts,tsx}",
    "!src/**/test/**",
    "!src/wire/generated/**",
    "!src/wire/engineConfig.ts",
    "!src/wire/serverConfig.ts",
    "!src/wasm/pkg/**",
  ],
  // A module-level value runs once for a whole test file, so each such mutant would run every test
  // again; left out (and counted as Ignored).
  ignoreStatic: true,
  reporters: ["clear-text", "progress", "json", "html"],
  jsonReporter: { fileName: `${process.env.MUTANTS_OUT ?? "reports/mutation"}/mutation.json` },
  htmlReporter: { fileName: `${process.env.MUTANTS_OUT ?? "reports/mutation"}/mutation.html` },
  thresholds: { high: 80, low: 60, break: null },
  tempDirName: ".stryker-tmp",
};
