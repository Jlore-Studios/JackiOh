// The Luau module's bytes for a test (R915009, #919): jsdom cannot fetch the module from a file URL as a
// page or the practice worker does, so a test that runs custom cards reads them from disk (written by
// `scripts/build-wasm.sh`, which `pretest` runs), once per test file.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

let bytes: ArrayBuffer | null = null;

export function luauBytes(): ArrayBuffer {
  // A path, not `new URL(…, import.meta.url)`, as `setup.ts` reads the normal module's.
  if (bytes === null) {
    const file = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../wasm/pkg/luau/jackioh_wasm_luau.wasm"));
    bytes = file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength) as ArrayBuffer;
  }
  return bytes;
}
