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

function gzipped(file: string): number {
  return gzipSync(readFileSync(join(PKG, file)), { level: constants.Z_BEST_COMPRESSION }).length;
}

describe("the browser module's size (#442 L12)", () => {
  it("is within 1 MiB of main's at the branch point, module and glue gzipped", () => {
    const bytes = gzipped("jackioh_wasm.wasm") + gzipped("jackioh_wasm.js");
    expect(bytes).toBeLessThanOrEqual(MAIN_AT_BRANCH_POINT_BYTES + L12_HEADROOM_BYTES);
  });
});
