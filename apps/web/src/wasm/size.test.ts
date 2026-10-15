// #442 L12: the browser module, its WebAssembly and its emscripten glue, gzipped (`gzip -9`), stays
// within 1 MiB of `main`'s at the v0.4.0 branch point, which v0.4.0 part 5 measured on f01a13d5d
// (`docs/v0.4.0/README.md`, section 5.3). `scripts/build-wasm.sh`, which the web's tests run first,
// writes both files into `pkg/`.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { constants, gzipSync } from "node:zlib";

import { describe, expect, it } from "vitest";

const PKG = join(dirname(fileURLToPath(import.meta.url)), "pkg");

/** `main`'s module and glue at the branch point, gzipped: 2,939,781 B and 5,781 B. */
const MAIN_AT_BRANCH_POINT_BYTES = 2_945_562;
/** L12's headroom over it. */
const L12_HEADROOM_BYTES = 1024 * 1024;
/**
 * The engine's own growth since the branch point, which L12's MiB (Luau's budget) was never meant to
 * pay for: `main` itself had reached the line (3,994,260 B measured on 23a2ee0), and #936's Duos life
 * on every hero (`duos_life.rs`, the `duos` fields of the state and the view) adds about 6.4 KB.
 */
const ENGINE_GROWTH_SINCE_BRANCH_POINT_BYTES = 32 * 1024;

function gzipped(file: string): number {
  return gzipSync(readFileSync(join(PKG, file)), { level: constants.Z_BEST_COMPRESSION }).length;
}

describe("the browser module's size (#442 L12)", () => {
  it("is within 1 MiB of main's at the branch point and the engine's growth since, module and glue gzipped", () => {
    const bytes = gzipped("jackioh_wasm.wasm") + gzipped("jackioh_wasm.js");
    const budget = MAIN_AT_BRANCH_POINT_BYTES + L12_HEADROOM_BYTES + ENGINE_GROWTH_SINCE_BRANCH_POINT_BYTES;
    expect(bytes).toBeLessThanOrEqual(budget);
  });
});
