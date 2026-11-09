// Stryker's vitest configuration (apps/web/stryker.config.mjs, #3490): the web's own, without the test
// files that read the repository's sources as text. Stryker compiles every mutant of a layer into its
// files, in place, so a test that reads those files' text (counting key handlers across src/, a file's
// first line, a convention walked over the tree) fails before any mutant runs, and could only ever
// kill a mutant for its altered text. Those are checks on the source, not on what it does, and every
// one of them still runs in CI. Found by what they call, so a new one is left out too.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { configDefaults, defineConfig, mergeConfig } from "vitest/config";

import base from "./vitest.config.ts";

const SRC = fileURLToPath(new URL("./src/", import.meta.url));

/** A test file that walks the tree, or reads a TypeScript file, as text. */
const READS_SOURCES = /readdirSync\(|import\.meta\.glob\(|readFileSync\([^)]*\.tsx?\b/;

const sourceReaders = readdirSync(SRC, { recursive: true, encoding: "utf8" })
  .filter((file) => /\.test\.tsx?$/.test(file))
  .filter((file) => READS_SOURCES.test(readFileSync(join(SRC, file), "utf8")))
  .map((file) => `src/${file}`);

export default mergeConfig(
  base,
  defineConfig({ test: { exclude: [...configDefaults.exclude, ...sourceReaders] } }),
);
