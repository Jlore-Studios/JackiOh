/**
 * Writes the card patch history, `packages/cards/patches/` (SPEC §10.10, R375):
 *
 *   patches.json              every patch, oldest first: { version, date, title, commits, sources,
 *                             reconstructed, notes }
 *   snapshots/<version>.json  catalog.json exactly as that version left it
 *   changes.json              the cards each version created, changed and removed (src/history.ts's
 *                             `patchCards` over the two above)
 *
 * Run it from anywhere (every path resolves from this file's URL):
 *
 *   pnpm --filter @jackioh/cards run patches
 *     Rewrites each shipped version's snapshot from git, the newest version's from catalog.json
 *     while it has not shipped, the entries `BACKFILL` records, and changes.json. Idempotent.
 *
 *   pnpm --filter @jackioh/cards run patches <version> <YYYY-MM-DD> "<title>"
 *     Names the next patch: appends it to patches.json (or re-dates and re-titles it while it is
 *     still the newest and unshipped) and snapshots catalog.json as it. Write its notes and sources
 *     in patches.json by hand; a rerun keeps them. Rerun after every change to catalog.json until
 *     the patch ships: `test/patches.test.ts` holds the newest snapshot equal to catalog.json.
 *
 * When a patch ships, put its last commit in its patches.json `commits` and add it to `SHIPPED`
 * with the blob id `git rev-parse <commit>:packages/cards/catalog.json` prints. From then on its
 * snapshot comes from git, and the test holds it byte-equal to that blob.
 *
 * A shipped snapshot is `git show <commit>:packages/cards/catalog.json`, which needs the commit:
 * on a shallow clone run `git fetch --unshallow` first (a shallow clone also stops
 * `git log --follow` at a later commit that changes no entry).
 *
 * fs and git live here and never in src/ (CLAUDE.md rule 4); the history itself is src/history.ts.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { CardDefs } from "@jackioh/shared";
import { patchCards, type Patch, type Snapshots } from "../src/history";

export const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));
export const CATALOG_PATH = fileURLToPath(new URL("../catalog.json", import.meta.url));
export const PATCHES_PATH = fileURLToPath(new URL("../patches/patches.json", import.meta.url));
export const CHANGES_PATH = fileURLToPath(new URL("../patches/changes.json", import.meta.url));
const SNAPSHOTS_DIR = fileURLToPath(new URL("../patches/snapshots/", import.meta.url));
/** catalog.json's path in the repository, as `git show <commit>:<path>` takes it. */
export const CATALOG_IN_REPO = "packages/cards/catalog.json";

/** A version that has shipped: the commit that left catalog.json as the version shipped it, and that file's git blob id. */
export type Shipped = { readonly version: string; readonly commit: string; readonly blob: string };

/**
 * Every shipped version, from `git log --follow packages/cards/catalog.json` on a full clone
 * (checked 2026-09-30). `blob` is git's own id for the file's exact bytes, so a snapshot can be
 * held byte-equal to its commit's catalog.json on a clone that does not have the commit.
 */
export const SHIPPED: readonly Shipped[] = [
  { version: "v0.1.0", commit: "46266903212386e2d73f7b1eee8c4b8b5d73fa9b", blob: "b0ee6660b9b65f7c81046eefbf0c460bc3f3a915" },
  { version: "v0.1.0-r1", commit: "cd780db96d226816d8051e756f9bda0314c10207", blob: "87b58bbd2df9e289ad74312bf17801c821a7543b" },
  { version: "v0.1.0-r2", commit: "a17a9e8ddc7ac1272861ce93a80e65e1e906f356", blob: "e139bead41bacf30b49c1eda9ad932e4a350da1b" },
  { version: "v0.1.0-r3", commit: "c219bb4ecbdc625d448fd558f68fc185c1fa1517", blob: "62af47ecbac2b3d499455019e1a67da1e73477ca" },
  { version: "v0.1.1", commit: "1005c50df56d244075879403230fa448259e7702", blob: "50171b44c1a14720e868ea68b5b917c45b4373fb" },
];

/**
 * The history as git has it, up to v0.1.1: patches.json's first entries, written from here on every
 * run. Only v0.1.1 was ever named; the versions before it are labels given on 2026-09-30 to states
 * git recorded, so each is `reconstructed`.
 */
export const BACKFILL: readonly Patch[] = [
  {
    version: "v0.1.0",
    date: "2026-09-18",
    title: "Core as first built",
    commits: ["46266903212386e2d73f7b1eee8c4b8b5d73fa9b"],
    sources: [],
    reconstructed: true,
    notes: "The Core set as the repository's first commit built it: 100 cards and 9 tokens.",
  },
  {
    version: "v0.1.0-r1",
    date: "2026-09-22",
    title: "Core Set balance changes",
    commits: ["1539fa7e7b4f0279b0395db97e1b1bff982aeaba", "cd780db96d226816d8051e756f9bda0314c10207"],
    sources: [
      { kind: "issue", number: 1 },
      { kind: "pr", number: 2 },
    ],
    reconstructed: true,
    notes:
      "The designer's balance changes (issue #1), whose notes landed first on 2026-09-20 (PR #2): " +
      "#3 Right-house defender gains Taunt, #68 “Twisted Sourcerer” becomes “Twisted Sorcerer”, " +
      "#81 Radiant Saintess changes twice, the Rush, Sheep and Felinor Tokens' Radiant faces grow, and the " +
      "Bread Token gets a Radiant face of its own.",
  },
  {
    version: "v0.1.0-r2",
    date: "2026-09-24",
    title: "Call to Chaos and The Coin",
    commits: ["f5b94bce13b4c967425ce67b6f8d1d34b66ec650", "a17a9e8ddc7ac1272861ce93a80e65e1e906f356"],
    sources: [
      { kind: "pr", number: 11 },
      { kind: "pr", number: 14 },
    ],
    reconstructed: true,
    notes: "#95 Call to Chaos's text changes (PR #11), and The Coin is added for the player going second (PR #14).",
  },
  {
    version: "v0.1.0-r3",
    date: "2026-09-25",
    title: "The Radiant pass",
    commits: ["c219bb4ecbdc625d448fd558f68fc185c1fa1517"],
    sources: [{ kind: "pr", number: 18 }],
    reconstructed: true,
    notes:
      "99 entries change (PR #18). Every Radiant text is written out in full instead of in shorthand, " +
      "Radiant stats and keywords are raised to one standard, #13 and #14 get the Jlockeed tag, and each " +
      "card lists the cards its text names.",
  },
  {
    version: "v0.1.1",
    date: "2026-09-27",
    title: "Patch v0.1.1",
    commits: ["1005c50df56d244075879403230fa448259e7702"],
    sources: [
      { kind: "issue", number: 27 },
      { kind: "pr", number: 28 },
    ],
    reconstructed: false,
    notes:
      "The Ghoul Token is added and 105 entries change: card text is reworded (Deck and Tribute), Pierce " +
      "arrives on #44 True Strike, #50 “Kpop Fanatic” becomes “K-Pop Fanatic”, #85 and #90 cost 2 instead " +
      "of 1, #99 costs 4 instead of 3, #1, #8, #20 and #77 change their stats, #8, #55 and #81 lose " +
      "keywords, #25, #56, #86 and #92 change their Radiant keywords, and #30, #50, #62 and #80 gain a tag.",
  },
];

/** Where a version's snapshot lives. */
export function snapshotPath(version: string): string {
  return `${SNAPSHOTS_DIR}${version}.json`;
}

/** Git's id for a file with exactly these bytes: the SHA-1 of `blob <length>\0<bytes>`. */
export function gitBlobId(bytes: Buffer): string {
  return createHash("sha1").update(`blob ${String(bytes.length)}\0`).update(bytes).digest("hex");
}

/** catalog.json exactly as `commit` left it, or null when this clone does not have the commit. */
export function catalogAt(commit: string): Buffer | null {
  try {
    return execFileSync("git", ["show", `${commit}:${CATALOG_IN_REPO}`], {
      cwd: REPO_ROOT,
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch {
    return null;
  }
}

/** The patch list on disk, or none before the first run. */
export function readPatches(): Patch[] {
  let text: string;
  try {
    text = readFileSync(PATCHES_PATH, "utf8");
  } catch {
    return [];
  }
  return JSON.parse(text) as Patch[];
}

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

/** The next patch as the command line names it, or null for a plain rerun. */
type Naming = { version: string; date: string; title: string } | null;

const DATE = /^\d{4}-\d{2}-\d{2}$/u;

function namingOf(args: readonly string[]): Naming {
  if (args.length === 0) return null;
  const [version, date, title] = args;
  if (args.length !== 3 || version === undefined || date === undefined || title === undefined || !DATE.test(date)) {
    throw new Error('usage: patches [<version> <YYYY-MM-DD> "<title>"]');
  }
  return { version, date, title };
}

/** BACKFILL, then the entries patches.json holds after it, then the named patch. */
function patchList(onDisk: readonly Patch[], naming: Naming): Patch[] {
  const recorded = new Set(BACKFILL.map((patch) => patch.version));
  const list = [...BACKFILL, ...onDisk.filter((patch) => !recorded.has(patch.version))];
  if (naming === null) return list;

  const shipped = new Set(SHIPPED.map((entry) => entry.version));
  if (shipped.has(naming.version)) {
    throw new Error(`${naming.version} has shipped: its snapshot is git's, and a change to catalog.json is the next patch`);
  }
  const newest = list[list.length - 1];
  if (newest !== undefined && newest.version === naming.version) {
    list[list.length - 1] = { ...newest, date: naming.date, title: naming.title };
    return list;
  }
  if (list.some((patch) => patch.version === naming.version)) {
    throw new Error(`${naming.version} is already in patches.json and is not the newest patch`);
  }
  if (newest !== undefined && !shipped.has(newest.version)) {
    throw new Error(
      `${newest.version} has not shipped. Once it has, record its last commit in patches.json and in SHIPPED ` +
        `(scripts/patches.ts), with the blob \`git rev-parse <commit>:${CATALOG_IN_REPO}\` prints, then name the next patch`,
    );
  }
  list.push({ version: naming.version, date: naming.date, title: naming.title, commits: [], sources: [], reconstructed: false, notes: "" });
  return list;
}

/** The bytes a version's snapshot holds: its commit's catalog.json once shipped, else catalog.json now. */
function snapshotBytes(patch: Patch, newest: boolean): Buffer {
  const shipped = SHIPPED.find((entry) => entry.version === patch.version);
  if (shipped === undefined) {
    if (!newest) throw new Error(`${patch.version} is not the newest patch and has not shipped: add it to SHIPPED`);
    return readFileSync(CATALOG_PATH);
  }
  const bytes = catalogAt(shipped.commit);
  if (bytes === null) {
    throw new Error(`this clone does not have ${shipped.commit} (${patch.version}): run \`git fetch --unshallow\` first`);
  }
  const blob = gitBlobId(bytes);
  if (blob !== shipped.blob) {
    throw new Error(`${patch.version}: ${shipped.commit}:${CATALOG_IN_REPO} is blob ${blob}, but SHIPPED records ${shipped.blob}`);
  }
  return bytes;
}

/** Writes every file under patches/, or nothing when a check fails. Returns the patch list written. */
export function writePatches(args: readonly string[]): Patch[] {
  const patches = patchList(readPatches(), namingOf(args));
  const files = patches.map((patch, at) => ({ patch, bytes: snapshotBytes(patch, at === patches.length - 1) }));

  const newest = files[files.length - 1];
  if (newest !== undefined && !newest.bytes.equals(readFileSync(CATALOG_PATH))) {
    throw new Error(
      `catalog.json has changed since ${newest.patch.version} shipped: name the next patch ` +
        '(`pnpm --filter @jackioh/cards run patches <version> <YYYY-MM-DD> "<title>"`)',
    );
  }

  mkdirSync(SNAPSHOTS_DIR, { recursive: true });
  const snapshots: Record<string, CardDefs> = {};
  for (const { patch, bytes } of files) {
    writeFileSync(snapshotPath(patch.version), bytes);
    snapshots[patch.version] = JSON.parse(bytes.toString("utf8")) as CardDefs;
  }
  writeJson(PATCHES_PATH, patches);
  writeJson(CHANGES_PATH, patchCards(patches, snapshots as Snapshots));
  return patches;
}

function main(): void {
  try {
    const patches = writePatches(process.argv.slice(2));
    const newest = patches[patches.length - 1];
    console.log(`patches: wrote ${String(patches.length)} versions; the newest is ${newest?.version ?? "none"}`);
  } catch (error) {
    console.error(`patches: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

// Run as a CLI; stay silent when imported (test/patches.test.ts reads SHIPPED and BACKFILL).
const entry = process.argv[1];
if (entry !== undefined && pathToFileURL(entry).href === import.meta.url) main();
