// Prints the catalog version this checkout ships: the newest entry of packages/cards/patches/patches.json.
//
//   node scripts/catalog-version.mjs [path/to/patches.json]
//
// R388 makes the catalog version the newest patch, and `pnpm --filter @jackioh/cards run patch` writes
// it into `packages/cards/src/catalog-data.ts`, `render.yaml`, `apps/server/.env.example` and the
// server's end-to-end default in the same commit, which a test holds in step. render.yaml's
// `startCommand` runs this and exports the result as CATALOG_VERSION before `release` stamps the
// database and before the server starts, so the version a deploy serves comes from the code it
// deployed and a stale value in Render's dashboard cannot be served. The version is read as data,
// the way `loadCurrentPatch` in apps/server/src/api/catalog.ts reads it: the list's order is the
// order of versions, never a comparison of strings (R105).
//
// Exits non-zero, printing nothing on stdout, when the list names no newest version, so the start
// command stops there and Render keeps the previous deploy serving.

import { readFileSync } from "node:fs";

const path = process.argv[2] ?? new URL("../packages/cards/patches/patches.json", import.meta.url);

let newest;
try {
  const list = JSON.parse(readFileSync(path, "utf8"));
  newest = Array.isArray(list) ? list[list.length - 1]?.version : undefined;
} catch (cause) {
  console.error(`catalog-version: the patch list could not be read from ${String(path)}: ${String(cause)}`);
  process.exit(1);
}

if (typeof newest !== "string" || newest.length === 0) {
  console.error(`catalog-version: the patch list at ${String(path)} names no newest version (R388)`);
  process.exit(1);
}

process.stdout.write(newest);
