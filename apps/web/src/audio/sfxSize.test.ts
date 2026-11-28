// @vitest-environment node
// #5190: the recipes' gzipped chunk (sfx.ts and the leaf modules it imports, #4428), held to a cap.

import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { build } from "vite";
import { describe, expect, it } from "vitest";

/**
 * The most the recipes' chunk may weigh gzipped: `sfx.ts` bundled and minified by Vite as a library,
 * as the lazy chunk `sfxLoader.ts` loads on first sound is (#4428). #5190 measured 29,695 bytes before
 * the spell verbs' nine recipes and 31,128 with them, and rounded the second up to the next 512. Raise it
 * only on purpose, with the new measurement and the recipes that need it here; a recipe that does not fit
 * shares a helper with the others rather than moving the cap.
 */
export const SFX_CHUNK_GZIP_CAP_BYTES = 31_232;

/** The recipes module's gzipped size, bundled in memory so that no build has to have run. */
async function gzippedChunkBytes(): Promise<number> {
  const result = await build({
    configFile: false,
    logLevel: "silent",
    build: {
      write: false,
      minify: true,
      lib: { entry: fileURLToPath(new URL("./sfx.ts", import.meta.url)), formats: ["es"] },
    },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((r) => ("output" in r ? r.output : []));
  const chunks = outputs.flatMap((o) => (o.type === "chunk" ? [o.code] : []));
  return gzipSync(chunks.join("\n")).length;
}

describe("#4428 the recipes' chunk", () => {
  it("#5190 gzips to no more than SFX_CHUNK_GZIP_CAP_BYTES", async () => {
    const bytes = await gzippedChunkBytes();
    expect(bytes, `the recipes gzip to ${bytes} bytes`).toBeLessThanOrEqual(SFX_CHUNK_GZIP_CAP_BYTES);
  }, 60_000);
});
