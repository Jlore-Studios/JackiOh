/**
 * Makes a card patch (B4.2, R388): snapshots the catalog, records the patch in `patches.json`,
 * regenerates each patch's card-by-card `changes` and the per-card `index.json`, and — for the
 * newest patch — bumps `CATALOG_VERSION` everywhere the string lives.
 *
 *   pnpm --filter @jackioh/cards run patch <version> "<title>" --date <YYYY-MM-DD> \
 *     [--source "<issue, PR, commits>"] [--notes "<what changed>"] [--from-git <rev>]
 *
 * - The snapshot is `packages/cards/catalog.json` as it stands, or with `--from-git <rev>` the file
 *   as commit `<rev>` left it (`git show <rev>:packages/cards/catalog.json`), which is how the
 *   patches before v0.2.0 were rebuilt from history.
 * - A version already in `patches.json` is amended in place (its snapshot and whatever fields are
 *   given); a new one is appended, so `patches.json`'s order is the order patches were made — the
 *   only order a version has (R105: never parsed, never sorted).
 * - When the patch is the newest and was taken from the working catalog, `CATALOG_VERSION` is
 *   rewritten in `src/catalog-data.ts`, `apps/server/.env.example`, `render.yaml` and the server's
 *   end-to-end default (`apps/server/src/index.ts`). `test/patches.test.ts` holds all of them, and
 *   the catalog, to the newest snapshot. A deployment then reseeds: `db:seed-catalog` stamps every
 *   `cards` row and `app.settings.catalog_version` with the new version.
 *
 * `<version>` may be a micro patch's `vA.B.Y`, which ships as the newest version plus the next
 * letter (`versions.ts`, R650).
 *
 * There is no clock in this package (CLAUDE.md rule 4), so the date is always given.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  rebuildDerived,
  readPatches,
  snapshotPath,
  writeJson,
  PATCHES_JSON,
  type Catalog,
  type PatchEntry,
} from "./patches-io";
import { resolveVersion } from "./versions";

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));
const CATALOG_PATH = fileURLToPath(new URL("../catalog.json", import.meta.url));

/** Every file that carries the catalog version, and how to rewrite it there. */
export const VERSION_SITES: readonly { file: string; pattern: RegExp; render: (version: string) => string }[] = [
  {
    file: "packages/cards/src/catalog-data.ts",
    pattern: /export const CATALOG_VERSION = "[^"]*";/,
    render: (v) => `export const CATALOG_VERSION = "${v}";`,
  },
  {
    file: "apps/server/.env.example",
    pattern: /^CATALOG_VERSION=.*$/m,
    render: (v) => `CATALOG_VERSION=${v}`,
  },
  {
    file: "render.yaml",
    pattern: /(- key: CATALOG_VERSION\n\s+value: )\S+/,
    render: (v) => `$1${v}`,
  },
  {
    file: "apps/server/src/index.ts",
    pattern: /(\n\s+CATALOG_VERSION: )"[^"]*"(,)/,
    render: (v) => `$1"${v}"$2`,
  },
];

/** The version each site carries now, or `undefined` where the pattern finds none. */
export function versionsAtSites(): { file: string; version: string | undefined }[] {
  return VERSION_SITES.map((site) => {
    const text = readFileSync(`${REPO_ROOT}${site.file}`, "utf8");
    const match = /"([^"]+)"|=(\S+)|value: (\S+)/.exec(text.match(site.pattern)?.[0] ?? "");
    return { file: site.file, version: match?.[1] ?? match?.[2] ?? match?.[3] };
  });
}

function bumpSites(version: string): void {
  for (const site of VERSION_SITES) {
    const path = `${REPO_ROOT}${site.file}`;
    const text = readFileSync(path, "utf8");
    if (!site.pattern.test(text)) throw new Error(`${site.file}: no CATALOG_VERSION to rewrite`);
    writeFileSync(path, text.replace(site.pattern, site.render(version)), "utf8");
  }
}

type Args = { version: string; title: string; date?: string; source?: string; notes?: string; fromGit?: string };

function parseArgs(argv: readonly string[]): Args {
  const positional: string[] = [];
  const flags: Record<string, string> = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i] ?? "";
    if (arg.startsWith("--")) {
      const value = argv[i + 1];
      if (value === undefined) throw new Error(`${arg} needs a value`);
      flags[arg.slice(2)] = value;
      i += 1;
    } else positional.push(arg);
  }
  const [version, title] = positional;
  if (version === undefined || title === undefined) {
    throw new Error('usage: patch <version> "<title>" --date <YYYY-MM-DD> [--source …] [--notes …] [--from-git <rev>]');
  }
  const args: Args = { version, title };
  if (flags["date"] !== undefined) args.date = flags["date"];
  if (flags["source"] !== undefined) args.source = flags["source"];
  if (flags["notes"] !== undefined) args.notes = flags["notes"];
  if (flags["from-git"] !== undefined) args.fromGit = flags["from-git"];
  return args;
}

function catalogAt(rev: string | undefined): Catalog {
  const text =
    rev === undefined
      ? readFileSync(CATALOG_PATH, "utf8")
      : execFileSync("git", ["show", `${rev}:packages/cards/catalog.json`], { cwd: REPO_ROOT, encoding: "utf8" });
  return JSON.parse(text) as Catalog;
}

export function makePatch(args: Args): PatchEntry[] {
  const patches = readPatches();
  const at = patches.findIndex((patch) => patch.version === args.version);
  const existing = at >= 0 ? patches[at] : undefined;
  const date = args.date ?? existing?.date;
  if (date === undefined || !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    throw new Error("--date <YYYY-MM-DD> is required for a new patch (this package has no clock)");
  }
  const entry: PatchEntry = {
    version: args.version,
    date,
    title: args.title,
    source: args.source ?? existing?.source ?? "",
    notes: args.notes ?? existing?.notes ?? "",
    changes: existing?.changes ?? [],
  };
  if (existing === undefined) patches.push(entry);
  else patches[at] = entry;

  writeJson(snapshotPath(args.version), catalogAt(args.fromGit));
  writeJson(PATCHES_JSON, patches);
  const written = rebuildDerived();

  const newest = written[written.length - 1];
  if (args.fromGit === undefined && newest?.version === args.version) bumpSites(args.version);
  return written;
}

function main(): void {
  const asked = parseArgs(process.argv.slice(2));
  // R650: a micro patch (`vA.B.Y`) is named after the newest version in patches.json.
  const shipped = readPatches().map((p) => p.version);
  const args = { ...asked, version: resolveVersion(asked.version, shipped) };
  const written = makePatch(args);
  const patch = written.find((p) => p.version === args.version);
  const counts = { added: 0, changed: 0, removed: 0 };
  for (const change of patch?.changes ?? []) counts[change.kind] += 1;
  console.log(
    `patch ${args.version}: ${counts.added} added, ${counts.changed} changed, ${counts.removed} removed ` +
      `(${written.length} patches in patches.json)`,
  );
}

const entry = process.argv[1];
if (entry !== undefined && pathToFileURL(entry).href === import.meta.url) main();
