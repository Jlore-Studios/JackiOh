/**
 * The card patch history on disk (B4.2, R388, R635): `packages/cards/patches/`.
 *
 *   patches.json       every shipped patch in ship order: { version, date, title, source, notes, changes }
 *   <version>.json     the whole catalog as that patch left it (a snapshot, not a diff)
 *   index.json         GENERATED: for each card id, the versions in which it was added or changed
 *   shipped.json       every shipped patch's { version, commit, blob }: the commit that shipped it
 *                      and its snapshot file's git blob hash, so the workflow writes data, not source
 *   pending/           one fragment per patch being built: { version, title, sources, notes, cards }
 *
 * The order of patches is `patches.json`'s order and nothing else: a version is an opaque string,
 * compared for equality only and never parsed or sorted (R105, R388). `changes` and `index.json` are
 * derived from the snapshots by `rebuildDerived()`, so they can never disagree with them. Shipped
 * snapshots are never amended: branches add a fragment under `pending/` and `patches ship`
 * promotes it after it merges (R635).
 *
 * Tooling (fs lives in `scripts/`, CLAUDE.md rule 4), shared by `patches.ts` and `gen-loc.ts`;
 * `test/patches.test.ts` reads the same files through these functions.
 */

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const PATCHES_DIR = fileURLToPath(new URL("../patches/", import.meta.url));
export const PATCHES_JSON = `${PATCHES_DIR}patches.json`;
export const INDEX_JSON = `${PATCHES_DIR}index.json`;
/** Pending fragments: one file per patch being built, never edited after it ships (R635). */
export const PENDING_DIR = `${PATCHES_DIR}pending/`;
/** Every shipped patch's provenance: the commit that shipped it and its snapshot's blob (R635). */
export const SHIPPED_JSON = `${PATCHES_DIR}shipped.json`;

export type Catalog = Record<string, Record<string, unknown>>;

/** One card's line in a patch: added, changed (with the fields that moved) or removed. */
export type PatchChange =
  | { id: string; name: string; kind: "added" | "removed" }
  | { id: string; name: string; kind: "changed"; fields: string[] };

export type PatchEntry = {
  /** The catalog version this patch set, e.g. "v0.2.0" (R388). Opaque (R105). */
  version: string;
  /** The day it shipped, YYYY-MM-DD. */
  date: string;
  title: string;
  /** Where it came from: the issue, the PR, the commits. */
  source: string;
  /** What the patch did, in the designer's and the players' words. */
  notes: string;
  /** The first-parent commits that shipped this patch; new promotions carry one. */
  commits?: string[];
  /** Whether the entry was rebuilt from historical catalog data instead of promoted. */
  reconstructed?: boolean;
  /** GENERATED from the snapshots: every card the patch added, changed or removed, in catalog order. */
  changes: PatchChange[];
};

/** A snapshot's path. */
export function snapshotPath(version: string, dir: string = PATCHES_DIR): string {
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(version)) {
    throw new Error(`"${version}" is not a patch version (letters, digits, ".", "_" and "-")`);
  }
  return `${dir}${version}.json`;
}

export function readPatches(path: string = PATCHES_JSON): PatchEntry[] {
  return JSON.parse(readFileSync(path, "utf8")) as PatchEntry[];
}

export function readSnapshot(version: string, dir: string = PATCHES_DIR): Catalog {
  return JSON.parse(readFileSync(snapshotPath(version, dir), "utf8")) as Catalog;
}

export function writeJson(path: string, value: unknown): void {
  const temp = `${path}.${process.pid}.tmp`;
  writeFileSync(temp, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  renameSync(temp, path);
}

/** The catalog's key order for one entry (catalog.json's own), unknown keys kept at the end. */
const ENTRY_KEYS = [
  "id",
  "index",
  "name",
  "set",
  "type",
  "tags",
  "rarity",
  "printedRarity",
  "token",
  "cost",
  "refs",
  "params",
  "loc",
  "radiantFallback",
  "base",
  "radiant",
];

export function orderEntry(def: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const key of ENTRY_KEYS) if (def[key] !== undefined) out[key] = def[key];
  for (const key of Object.keys(def)) if (!(key in out) && def[key] !== undefined) out[key] = def[key];
  return out;
}

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

/** The fields of one entry that differ, one level into its faces ("base.text", "cost"). */
export function changedFields(before: Record<string, unknown>, after: Record<string, unknown>): string[] {
  const fields: string[] = [];
  const keys = [...new Set([...Object.keys(before), ...Object.keys(after)])];
  for (const key of keys) {
    const a = before[key];
    const b = after[key];
    if (same(a, b)) continue;
    const faceA = a as Record<string, unknown> | undefined;
    const faceB = b as Record<string, unknown> | undefined;
    if ((key === "base" || key === "radiant") && typeof faceA === "object" && typeof faceB === "object") {
      const inner = [...new Set([...Object.keys(faceA), ...Object.keys(faceB)])];
      for (const field of inner) if (!same(faceA[field], faceB[field])) fields.push(`${key}.${field}`);
    } else {
      fields.push(key);
    }
  }
  return fields;
}

/** What `after` changed against `before`, in `after`'s order, removals last. */
export function diffCatalogs(before: Catalog | null, after: Catalog): PatchChange[] {
  const out: PatchChange[] = [];
  const nameOf = (def: Record<string, unknown> | undefined, id: string): string =>
    typeof def?.["name"] === "string" ? def["name"] : id;
  for (const [id, def] of Object.entries(after)) {
    const prev = before?.[id];
    if (prev === undefined) out.push({ id, name: nameOf(def, id), kind: "added" });
    else {
      const fields = changedFields(prev, def);
      if (fields.length > 0) out.push({ id, name: nameOf(def, id), kind: "changed", fields });
    }
  }
  for (const [id, def] of Object.entries(before ?? {})) {
    if (after[id] === undefined) out.push({ id, name: nameOf(def, id), kind: "removed" });
  }
  return out;
}

/** For each card id, the versions in which it was added or changed, in patch order. */
export function buildIndex(patches: readonly PatchEntry[]): Record<string, string[]> {
  const index: Record<string, string[]> = {};
  for (const patch of patches) {
    for (const change of patch.changes) {
      if (change.kind === "removed") continue;
      (index[change.id] ??= []).push(patch.version);
    }
  }
  return index;
}

/**
 * Recomputes every patch's `changes` from the snapshots, in patches.json's order, and rewrites
 * `index.json`. Returns the patches as written.
 */
export function rebuildDerived(dir: string = PATCHES_DIR): PatchEntry[] {
  const patches = readPatches(`${dir}patches.json`);
  let previous: Catalog | null = null;
  const next = patches.map((patch) => {
    const snapshot = readSnapshot(patch.version, dir);
    const changes = diffCatalogs(previous, snapshot);
    previous = snapshot;
    return { ...patch, changes };
  });
  writeJson(`${dir}patches.json`, next);
  writeJson(`${dir}index.json`, buildIndex(next));
  return next;
}

/** `rebuildDerived`, under the name gen-loc reads. */
export function rewriteIndex(): void {
  rebuildDerived();
}

/**
 * Pending fragments and the shipped list (R635): several card patches are built on separate
 * branches at once, so branches never edit `patches.json`, the snapshots or the shipped list.
 * A branch changes `catalog.json` and adds one fragment, `pending/<version>.json`, claiming the
 * catalog ids its patch touches; `patches check` proves the claims against the newest shipped
 * snapshot, and `patches ship` promotes each fragment to a patch in ship order after it merges.
 */

/** A patch not yet shipped: the designer's label, what it does, and the catalog ids it touches. */
export type PendingFragment = {
  /** The designer's label, a bare patch number: "v0.2.5", never a revision ("v0.2.0b"). */
  version: string;
  title: string;
  /** Where the patch came from: the issue, the PR, the commits. One string, as `source` below. */
  sources: string;
  /** What the patch does, in the designer's and the players' words. */
  notes: string;
  /** The catalog ids the patch creates, changes or removes, in catalog order. */
  cards: string[];
};

/** One shipped patch's provenance: the commit that shipped it and its snapshot's blob. */
export type ShippedEntry = {
  /** The patch's version as `patches.json` lists it, revisions ("v0.2.0b") included. */
  version: string;
  /** The first-parent commit on main that shipped it (for old patches, the one that holds them). */
  commit: string;
  /** The git blob hash (`git hash-object`) of its snapshot file's bytes. */
  blob: string;
};

/** A fragment's version is a bare patch number (R635): "v0.2.5", never "v0.2.0b" or "v0.2.0-r1". */
export const FRAGMENT_VERSION = /^v\d+\.\d+\.\d+$/;

/** A pending fragment's path. */
export function pendingPath(version: string, dir: string = PENDING_DIR): string {
  if (!FRAGMENT_VERSION.test(version)) {
    throw new Error(`"${version}" is not a pending fragment (a bare patch number like "v0.2.5")`);
  }
  return `${dir}${version}.json`;
}

function isFragment(value: unknown): value is PendingFragment {
  const fragment = value as Record<string, unknown>;
  return (
    typeof fragment === "object" &&
    fragment !== null &&
    typeof fragment["version"] === "string" &&
    typeof fragment["title"] === "string" &&
    typeof fragment["sources"] === "string" &&
    typeof fragment["notes"] === "string" &&
    Array.isArray(fragment["cards"]) &&
    fragment["cards"].every((id) => typeof id === "string")
  );
}

/** Every pending fragment by file name, or [] when `pending/` does not exist yet. */
export function readFragments(dir: string = PENDING_DIR): { name: string; fragment: PendingFragment }[] {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return [];
  }
  const out: { name: string; fragment: PendingFragment }[] = [];
  for (const name of names.sort()) {
    if (!name.endsWith(".json") || name.startsWith(".")) continue;
    const fragment: unknown = JSON.parse(readFileSync(`${dir}${name}`, "utf8"));
    if (!isFragment(fragment)) throw new Error(`pending/${name} is not a fragment ({ version, title, sources, notes, cards })`);
    out.push({ name, fragment });
  }
  return out;
}

function isShippedEntry(value: unknown): value is ShippedEntry {
  const entry = value as Record<string, unknown>;
  return (
    typeof entry === "object" &&
    entry !== null &&
    typeof entry["version"] === "string" &&
    typeof entry["commit"] === "string" &&
    typeof entry["blob"] === "string"
  );
}

/** The shipped list, oldest first, in `patches.json`'s order. */
export function readShipped(path: string = SHIPPED_JSON): ShippedEntry[] {
  const entries: unknown = JSON.parse(readFileSync(path, "utf8"));
  if (!Array.isArray(entries) || !entries.every(isShippedEntry)) {
    throw new Error("shipped.json is not a list of { version, commit, blob }");
  }
  return entries;
}

/** Removes a file that is there, and never complains about one that is not. */
export function removeFile(path: string): void {
  try {
    unlinkSync(path);
  } catch {
    // Already gone: promotion deletes each fragment once, and a second `ship` finds none.
  }
}

/**
 * The git blob hash of a file's text: the sha1 of `blob <bytes>\0<text>`, as `git hash-object`
 * prints it. `shipped.json` carries one per snapshot, so a rewritten snapshot fails the proof.
 */
export function gitBlobHash(text: string): string {
  const bytes = Buffer.byteLength(text, "utf8");
  return createHash("sha1").update(`blob ${bytes}\0${text}`, "utf8").digest("hex");
}

/**
 * The name a fragment ships under (R635): its own version, unless that version already shipped,
 * in which case the next revision letter — the first revision of v0.2.0 is "v0.2.0b", then
 * "v0.2.0c" (`docs/issues-and-patches.md`). A shipped version never reopens.
 */
export function nextShipName(taken: ReadonlySet<string>, version: string): string {
  if (!taken.has(version)) return version;
  for (const letter of "bcdefghijklmnopqrstuvwxyz") {
    const revision = `${version}${letter}`;
    if (!taken.has(revision)) return revision;
  }
  throw new Error(`no free revision letter for "${version}" (b through z are all shipped)`);
}

/** Two catalogs hold the same entries, key order aside. */
export function sameCatalog(a: Catalog, b: Catalog): boolean {
  const keysA = Object.keys(a);
  const keysB = new Set(Object.keys(b));
  if (keysA.length !== keysB.size) return false;
  return keysA.every((id) => keysB.has(id) && JSON.stringify(a[id]) === JSON.stringify(b[id]));
}

/**
 * The working catalog with every pending-claimed entry reverted to the newest shipped snapshot:
 * entries the snapshots hold come back, created ones go away. With no fragments it is the catalog
 * as it stands.
 */
export function revertPending(catalog: Catalog, newest: Catalog, claimed: ReadonlySet<string>): Catalog {
  const out: Catalog = {};
  for (const [id, def] of Object.entries(catalog)) {
    if (!claimed.has(id)) out[id] = def;
  }
  for (const [id, def] of Object.entries(newest)) {
    if (claimed.has(id)) out[id] = def;
  }
  return out;
}

/**
 * Every way the pending fragments disagree with the newest shipped snapshot, each naming the
 * card (and the fragment) at fault, or [] when the tree is shippable: every catalog entry that
 * differs from the newest snapshot is claimed by exactly one fragment, every claimed card
 * differs, every fragment names a bare patch number, and every fragment file holds the version
 * its name says. Pure: `patches check` reads the files and prints what this returns.
 */
export function checkFragments(args: {
  files: readonly { name: string; fragment: PendingFragment }[];
  catalog: Catalog;
  newest: Catalog;
}): string[] {
  const problems: string[] = [];
  for (const { name, fragment } of args.files) {
    if (!FRAGMENT_VERSION.test(fragment.version)) {
      problems.push(`pending/${name} names version "${fragment.version}", not a bare patch number (^v\\d+\\.\\d+\\.\\d+$)`);
    }
    if (name !== `${fragment.version}.json`) {
      problems.push(`pending/${name} holds version "${fragment.version}", not the version its file names`);
    }
  }
  const differed = new Set(diffCatalogs(args.newest, args.catalog).map((change) => change.id));
  const claimants = new Map<string, string[]>();
  for (const { fragment } of args.files) {
    for (const id of fragment.cards) {
      const versions = claimants.get(id) ?? [];
      if (!versions.includes(fragment.version)) versions.push(fragment.version);
      claimants.set(id, versions);
    }
  }
  for (const id of [...differed].sort()) {
    const versions = claimants.get(id) ?? [];
    if (versions.length === 0) problems.push(`"${id}" differs from the newest shipped snapshot but no pending fragment claims it`);
    else if (versions.length > 1) {
      problems.push(`"${id}" is claimed by ${versions.map((version) => `"${version}"`).join(" and ")}, but one card ships in one patch`);
    }
  }
  for (const [id, versions] of [...claimants].sort(([a], [b]) => (a < b ? -1 : 1))) {
    if (!differed.has(id)) {
      problems.push(`"${id}" is claimed by "${versions[0]}" but identical to the newest shipped snapshot`);
    }
  }
  return problems;
}
