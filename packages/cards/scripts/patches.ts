/**
 * Card patches with several in flight at once (R635): pending fragments, the check that proves
 * them, and the promotion that ships them, in ship order.
 *
 *   pnpm --filter @jackioh/cards run patches <version> <date> "<title>" \
 *     [--source "<issue, PR, commits>"] [--notes "<what changed>"] [--cards <id,...>]
 *   pnpm --filter @jackioh/cards run patches check
 *   pnpm --filter @jackioh/cards run patches ship
 *
 * - A branch changes `catalog.json` and adds one fragment, `pending/<version>.json`: `{ version,
 *   title, sources, notes, cards }`, where `cards` lists the catalog ids the patch creates,
 *   changes or removes (`--cards` says them; otherwise they are diffed from the working catalog
 *   against the newest shipped snapshot). Branches never edit `patches.json`, the snapshots,
 *   `index.json` or `shipped.json`. The optional `date` is checked and not stored: promotion
 *   dates the patch by the UTC date of the commit that added the fragment.
 * - `check` fails naming the card when a catalog entry differs from the newest shipped snapshot
 *   without exactly one fragment claiming it, when a claimed card does not differ, or when a
 *   fragment's version is not a bare patch number (`^v\d+\.\d+\.\d+$`). CI runs it in the
 *   `validate:catalog` step.
 * - `ship` promotes every fragment on main, oldest first-parent commit that added one first: it
 *   appends the patch (its version, or `<version>b`, then `c`, …, when that name already shipped;
 *   a shipped version never reopens), snapshots `catalog.json` as that commit left it, records
 *   `{ version, commit, blob }` in `shipped.json`, deletes the fragment, regenerates the derived
 *   files and bumps `CATALOG_VERSION` to the newest patch. With no fragments it changes nothing,
 *   so running it twice is running it once. It needs full history (`fetch-depth: 0`).
 *
 * There is no clock in this package (CLAUDE.md rule 4): dates come from the git history, and the
 * UTC conversion below is arithmetic, never `Date`.
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { bumpSites } from "./patch";
import {
  checkFragments,
  diffCatalogs,
  FRAGMENT_VERSION,
  gitBlobHash,
  nextShipName,
  pendingPath,
  readFragments,
  readPatches,
  readShipped,
  readSnapshot,
  rebuildDerived,
  removeFile,
  snapshotPath,
  writeJson,
  type Catalog,
  type PatchEntry,
  type PendingFragment,
} from "./patches-io";

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));
const CATALOG_REL = "packages/cards/catalog.json";
const PATCHES_REL = "packages/cards/patches";

/** Every patches path under one root, so tests promote a fixture repo instead of this one. */
export function patchPaths(repoRoot: string): {
  dir: string;
  patchesJson: string;
  shippedJson: string;
  pendingDir: string;
  catalog: string;
} {
  const dir = `${repoRoot}/${PATCHES_REL}/`;
  return {
    dir,
    patchesJson: `${dir}patches.json`,
    shippedJson: `${dir}shipped.json`,
    pendingDir: `${dir}pending/`,
    catalog: `${repoRoot}/${CATALOG_REL}`,
  };
}

function readCatalog(path: string): Catalog {
  return JSON.parse(readFileSync(path, "utf8")) as Catalog;
}

type FragmentArgs = {
  version: string;
  title: string;
  date?: string;
  /** `--source`: where the patch came from. Stored on the fragment as `sources`. */
  sources?: string;
  notes?: string;
  cards?: string[];
};

/**
 * Writes (or updates) the pending fragment for a patch being built. `cards` defaults to every
 * catalog id that differs from the newest shipped snapshot, in catalog order.
 */
export function writeFragment(repoRoot: string, args: FragmentArgs): PendingFragment {
  const paths = patchPaths(repoRoot);
  const patches = readPatches(paths.patchesJson);
  const newest = patches[patches.length - 1];
  if (newest === undefined) throw new Error("patches.json holds no shipped patch");
  const catalog = readCatalog(paths.catalog);
  const newestSnapshot = readSnapshot(newest.version, paths.dir);
  const cards = args.cards ?? diffCatalogs(newestSnapshot, catalog).map((change) => change.id);
  const fragment: PendingFragment = {
    version: args.version,
    title: args.title,
    sources: args.sources ?? "",
    notes: args.notes ?? "",
    cards,
  };
  mkdirSync(paths.pendingDir, { recursive: true });
  writeJson(pendingPath(args.version, paths.pendingDir), fragment);
  return fragment;
}

/**
 * Every way the tree is not shippable, each naming the card (and the fragment) at fault, or []
 * when `ship` would go through. Pure files in, strings out: `main` prints and exits on these.
 */
export function checkPatches(repoRoot: string): string[] {
  const paths = patchPaths(repoRoot);
  const patches = readPatches(paths.patchesJson);
  const newest = patches[patches.length - 1];
  if (newest === undefined) throw new Error("patches.json holds no shipped patch");
  return checkFragments({
    files: readFragments(paths.pendingDir),
    catalog: readCatalog(paths.catalog),
    newest: readSnapshot(newest.version, paths.dir),
  });
}

function git(repoRoot: string, args: readonly string[], extraEnv: Record<string, string> = {}): string {
  return execFileSync("git", [...args], { cwd: repoRoot, encoding: "utf8", env: { ...process.env, ...extraEnv } });
}

/** The first-parent commit on main that added a file, or "" when history does not hold it. */
export function addingCommit(repoRoot: string, relPath: string): string {
  return git(repoRoot, ["log", "--first-parent", "--format=%H", "--diff-filter=A", "--", relPath])
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "")[0] ?? "";
}

/**
 * A unix timestamp as a UTC YYYY-MM-DD. Arithmetic over days (Howard Hinnant's civil_from_days),
 * never `Date`: the clock ban (CLAUDE.md rule 4) covers this package, and the timestamp comes
 * from the git history, not from now.
 */
export function utcDateOf(unixSeconds: number): string {
  const days = Math.floor(unixSeconds / 86400) + 719468;
  const era = Math.floor(days / 146097);
  const dayOfEra = days - era * 146097;
  const yearOfEra = Math.floor((dayOfEra - Math.floor(dayOfEra / 1460) + Math.floor(dayOfEra / 36524) - Math.floor(dayOfEra / 146096)) / 365);
  const year = yearOfEra + era * 400;
  const dayOfYear = dayOfEra - (365 * yearOfEra + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100));
  const monthPrime = Math.floor((5 * dayOfYear + 2) / 153);
  const day = dayOfYear - Math.floor((153 * monthPrime + 2) / 5) + 1;
  const month = monthPrime + (monthPrime < 10 ? 3 : -9);
  const fullYear = month <= 2 ? year + 1 : year;
  const pad = (n: number): string => String(n).padStart(2, "0");
  return `${fullYear}-${pad(month)}-${pad(day)}`;
}

export type ShipResult = { shipped: string[] };

/**
 * Promotes every pending fragment to a shipped patch, in the order of the first-parent commit
 * that added it (ship order, R635). One commit per fragment is assumed — the squash-merge shape
 * the bot's pull requests land in — so the snapshot is `catalog.json` as that commit left it.
 * Idempotent: with no fragments it changes nothing.
 */
export function shipPatches(repoRoot: string): ShipResult {
  const paths = patchPaths(repoRoot);
  const files = readFragments(paths.pendingDir);
  if (files.length === 0) return { shipped: [] };

  const problems = checkPatches(repoRoot);
  if (problems.length > 0) throw new Error(`cannot ship pending fragments:\n${problems.join("\n")}`);

  const order = new Map(
    git(repoRoot, ["log", "--first-parent", "--format=%H"])
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "")
      .map((sha, index) => [sha, index] as const),
  );
  const queued = files.map(({ name, fragment }) => {
    const rel = `${PATCHES_REL}/pending/${name}`;
    const commit = addingCommit(repoRoot, rel);
    if (commit === "") {
      throw new Error(`pending/${name} was not added on this history's first-parent line (ship needs full history)`);
    }
    const at = order.get(commit);
    if (at === undefined) {
      throw new Error(`pending/${name} was added by ${commit}, which is not on this history's first-parent line`);
    }
    return { name, fragment, commit, at };
  });
  queued.sort((a, b) => b.at - a.at);

  const patches = readPatches(paths.patchesJson);
  const taken = new Set(patches.map((patch) => patch.version));
  const shipped = readShipped(paths.shippedJson);
  const names: string[] = [];
  for (const { name, fragment, commit } of queued) {
    const version = nextShipName(taken, fragment.version);
    const raw = git(repoRoot, ["show", `${commit}:${CATALOG_REL}`]);
    writeFileSync(snapshotPath(version, paths.dir), raw, "utf8");
    taken.add(version);
    const entry: PatchEntry = {
      version,
      date: utcDateOf(Number(git(repoRoot, ["log", "-1", "--format=%ct", commit]).trim())),
      title: fragment.title,
      source: fragment.sources,
      notes: fragment.notes,
      commits: [commit],
      reconstructed: false,
      changes: [],
    };
    patches.push(entry);
    shipped.push({ version, commit, blob: gitBlobHash(readFileSync(snapshotPath(version, paths.dir), "utf8")) });
    removeFile(`${paths.pendingDir}${name}`);
    names.push(version);
  }
  writeJson(paths.patchesJson, patches);
  writeJson(paths.shippedJson, shipped);
  const written = rebuildDerived(paths.dir);

  const newest = written[written.length - 1];
  if (newest === undefined) throw new Error("patches.json holds no shipped patch after promotion");
  bumpSites(newest.version, repoRoot);
  return { shipped: names };
}

function parseFragmentArgs(argv: readonly string[]): FragmentArgs {
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
  const [version, second, third] = positional;
  if (version === undefined || second === undefined || positional.length > 3) {
    throw new Error('usage: patches <version> [date] "<title>" [--source …] [--notes …] [--cards <id,...>]');
  }
  if (!FRAGMENT_VERSION.test(version)) {
    throw new Error(`"${version}" is not a bare patch number (^v\\d+\\.\\d+\\.\\d+$), the only shape a fragment takes`);
  }
  // The old calling shape kept the date between the version and the title; promotion dates the
  // patch by its merge commit instead, so a given date is validated and not stored.
  const [title, date] = third === undefined ? [second, flags["date"]] : [third, second];
  if (title === undefined) throw new Error('usage: patches <version> [date] "<title>" [--source …] [--notes …] [--cards <id,...>]');
  if (date !== undefined && !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    throw new Error(`"${date}" is not a YYYY-MM-DD date`);
  }
  const args: FragmentArgs = { version, title };
  if (date !== undefined) args.date = date;
  if (flags["source"] !== undefined) args.sources = flags["source"];
  if (flags["notes"] !== undefined) args.notes = flags["notes"];
  if (flags["cards"] !== undefined) {
    args.cards = flags["cards"].split(",").map((id) => id.trim()).filter((id) => id !== "");
  }
  return args;
}

function main(): void {
  const [command, ...rest] = process.argv.slice(2);
  if (command === "check") {
    const problems = checkPatches(REPO_ROOT);
    if (problems.length > 0) {
      for (const problem of problems) console.error(`patches check: ${problem}`);
      process.exit(1);
    }
    console.log("patches check: every catalog change is claimed by exactly one pending fragment");
    return;
  }
  if (command === "ship") {
    const result = shipPatches(REPO_ROOT);
    if (result.shipped.length === 0) console.log("patches ship: no pending fragments, nothing changed");
    else for (const version of result.shipped) console.log(`patches ship: shipped ${version}`);
    return;
  }
  if (command === undefined) {
    throw new Error('usage: patches <version> [date] "<title>" [--source …] [--notes …] [--cards <id,...>] | patches check | patches ship');
  }
  const args = parseFragmentArgs([command, ...rest]);
  const fragment = writeFragment(REPO_ROOT, args);
  console.log(
    `fragment ${fragment.version}: claims ${fragment.cards.length} card(s)` +
      (fragment.cards.length > 0 ? ` (${fragment.cards.join(", ")})` : " (the catalog matches the newest snapshot)"),
  );
}

const entry = process.argv[1];
if (entry !== undefined && pathToFileURL(entry).href === import.meta.url) {
  try {
    main();
  } catch (error) {
    console.error(`patches: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
