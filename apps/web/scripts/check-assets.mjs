// #3183: refuses a built site Cloudflare Workers would not take, or that ships the engine uncompressed.
//
//   node apps/web/scripts/check-assets.mjs [dist]      (dist defaults to apps/web/dist)
//
// Workers serves no static asset over WORKERS_ASSET_MAX_BYTES (wrangler.jsonc `assets`); `wrangler
// deploy` stops at the first one ("Asset too large"), which is how staging's deploys failed while the
// engine's module grew to 25.5 MiB (2026-11-05). So no file may reach ASSET_REFUSED_BYTES, a margin
// under the cap, and no WebAssembly module may ship uncompressed: the page loads the engine as
// `jackioh_wasm_bg.wasm.br` (scripts/build-wasm.sh, apps/web/src/wasm/index.ts), which the site Worker
// serves with its Content-Encoding (apps/edge), and a `.wasm` in the build means something emitted the
// raw module again.
//
// check-production-bundle.mjs runs `assetProblems` before both deploys; a unit test runs this over the
// client CI builds for every pull request the web's jobs run on (src/net/check-assets.test.ts).
//
// Exits 1, listing every problem.

import { readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

/** Bytes in a mebibyte, as Cloudflare states its limits. */
const MIB = 1024 * 1024;
/** Cloudflare Workers' cap on one static asset, in MiB. */
const WORKERS_ASSET_MAX_MIB = 25;
export const WORKERS_ASSET_MAX_BYTES = WORKERS_ASSET_MAX_MIB * MIB;
/** How far under the cap a file is refused, so a build is stopped before a deploy is. */
const ASSET_MARGIN_BYTES = MIB;
export const ASSET_REFUSED_BYTES = WORKERS_ASSET_MAX_BYTES - ASSET_MARGIN_BYTES;
/** A WebAssembly module, which the build ships only brotli-compressed. */
const UNCOMPRESSED_MODULE = /\.wasm$/u;

/** Every file under `dir`, as its path. */
function files(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath, entry.name));
}

/** What in the built site at `dist` Workers would refuse, or ships the engine uncompressed: one sentence each. */
export function assetProblems(dist) {
  const problems = [];
  for (const path of files(dist)) {
    const name = relative(dist, path);
    const size = statSync(path).size;
    if (size >= ASSET_REFUSED_BYTES) {
      problems.push(
        `${name} is ${String(size)} B, at or over ${String(ASSET_REFUSED_BYTES)} B: Cloudflare Workers serves no file over ${String(WORKERS_ASSET_MAX_MIB)} MiB, so split or compress it`,
      );
    }
    if (UNCOMPRESSED_MODULE.test(name)) {
      problems.push(
        `${name} is an uncompressed WebAssembly module: the page loads the engine as jackioh_wasm_bg.wasm.br (scripts/build-wasm.sh), so nothing should emit a .wasm`,
      );
    }
  }
  return problems;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const dist = process.argv[2] ?? fileURLToPath(new URL("../dist", import.meta.url));
  const problems = assetProblems(dist);
  if (problems.length > 0) {
    console.error(`check-assets: ${dist} must not be deployed (${String(problems.length)} problem(s)):`);
    for (const problem of problems) console.error(`  - ${problem}`);
    process.exit(1);
  }
  console.log(`check-assets: every file in ${dist} is under ${String(ASSET_REFUSED_BYTES)} B, and no module ships uncompressed`);
}
