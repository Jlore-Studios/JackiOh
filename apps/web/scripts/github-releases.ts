// Prints every shipped version's GitHub release as JSON, `{ latest, releases: [{ tag, target, name,
// prerelease, body }] }`, written by src/patches/github.ts from crates/cards/patches/; writes nothing.
// .github/workflows/releases.yml publishes what it prints.
//
//   pnpm --dir apps/web exec tsx scripts/github-releases.ts [--production <patches.json>]
//   pnpm --dir apps/web exec tsx scripts/github-releases.ts --production <patches.json> | jq -r '.releases[-1].body'
//
// `--production` is the patches.json production holds (`git show origin/production:crates/cards/patches/patches.json`):
// a version it lacks is a pre-release, on staging. Without it every version reads as on production.
// The hosts are the web's own: production's from .env.production, staging's `SITE_ORIGIN`.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { loadEnv } from "vite";

import { SITE_ORIGIN, originOf, paths } from "../src/net/navigate.ts";
import { githubReleases, type ShippedRecord } from "../src/patches/github.ts";
import type { Patch, Snapshot, TechnicalEntry } from "../src/patches/source.ts";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "..");
const PATCHES = join(WEB, "../../crates/cards/patches");

function read<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

const { values } = parseArgs({ options: { production: { type: "string" } } });
const patches = read<Patch[]>(join(PATCHES, "patches.json"));
const production = values.production === undefined ? patches : read<Patch[]>(values.production);

const plan = githubReleases({
  patches,
  shipped: read<ShippedRecord[]>(join(PATCHES, "shipped.json")),
  technical: read<TechnicalEntry[]>(join(PATCHES, "technical.json")),
  snapshot: (version) => read<Snapshot>(join(PATCHES, `${version}.json`)),
  live: new Set(production.map((patch) => patch.version)),
  sites: {
    production: originOf(loadEnv("production", WEB, "VITE_").VITE_SITE_ORIGIN),
    staging: SITE_ORIGIN,
    patchNotes: paths.patchNotes,
  },
});
process.stdout.write(`${JSON.stringify(plan, null, 2)}\n`);
