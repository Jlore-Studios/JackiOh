// Stryker over the Worker (#3490): whether its tests fail when its code is wrong. `sh
// scripts/mutants.sh edge` runs it and mutants.yml runs it every week; it reports and gates nothing
// yet (`break: null`). apps/web/stryker.config.mjs says why each setting is what it is.

/** @type {import("@stryker-mutator/api/core").PartialStrykerOptions} */
export default {
  testRunner: "vitest",
  plugins: ["@stryker-mutator/vitest-runner"],
  vitest: { configFile: "vitest.config.ts", related: true },
  // config.test.ts reads wrangler.jsonc and the server's routes from the repository's root.
  inPlace: true,
  mutate: ["src/**/*.ts", "!src/**/*.test.ts"],
  ignoreStatic: true,
  // No `// @ts-nocheck` written into the source files: vitest strips the types and checks none, and
  // in place the line lands in every file under src/, which a test reading a file's first line sees.
  disableTypeChecks: false,
  reporters: ["clear-text", "progress", "json", "html"],
  jsonReporter: { fileName: `${process.env.MUTANTS_OUT ?? "reports/mutation"}/mutation.json` },
  htmlReporter: { fileName: `${process.env.MUTANTS_OUT ?? "reports/mutation"}/mutation.html` },
  thresholds: { high: 80, low: 60, break: null },
  tempDirName: ".stryker-tmp",
};
