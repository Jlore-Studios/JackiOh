/**
 * The card patch history on disk (B4.2, R388): `packages/cards/patches/`.
 *
 *   patches.json       every patch in order: { version, date, title, source, notes, changes }
 *   <version>.json     the whole catalog as that patch left it (a snapshot, not a diff)
 *   index.json         GENERATED: for each card id, the versions in which it was added or changed
 *
 * The order of patches is `patches.json`'s order and nothing else: a version is an opaque string,
 * compared for equality only and never parsed or sorted (R105, R388). `changes` and `index.json` are
 * derived from the snapshots by `rebuildDerived()`, so they can never disagree with them.
 *
 * Tooling (fs lives in `scripts/`, CLAUDE.md rule 4), shared by `patch.ts` and `gen-loc.ts`;
 * `test/patches.test.ts` reads the same files through these functions.
 */

import { readFileSync, renameSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const PATCHES_DIR = fileURLToPath(new URL("../patches/", import.meta.url));
export const PATCHES_JSON = `${PATCHES_DIR}patches.json`;
export const INDEX_JSON = `${PATCHES_DIR}index.json`;

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
  /** GENERATED from the snapshots: every card the patch added, changed or removed, in catalog order. */
  changes: PatchChange[];
};

/** A snapshot's path. */
export function snapshotPath(version: string): string {
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(version)) {
    throw new Error(`"${version}" is not a patch version (letters, digits, ".", "_" and "-")`);
  }
  return `${PATCHES_DIR}${version}.json`;
}

export function readPatches(): PatchEntry[] {
  return JSON.parse(readFileSync(PATCHES_JSON, "utf8")) as PatchEntry[];
}

export function readSnapshot(version: string): Catalog {
  return JSON.parse(readFileSync(snapshotPath(version), "utf8")) as Catalog;
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
export function rebuildDerived(): PatchEntry[] {
  const patches = readPatches();
  let previous: Catalog | null = null;
  const next = patches.map((patch) => {
    const snapshot = readSnapshot(patch.version);
    const changes = diffCatalogs(previous, snapshot);
    previous = snapshot;
    return { ...patch, changes };
  });
  writeJson(PATCHES_JSON, next);
  writeJson(INDEX_JSON, buildIndex(next));
  return next;
}

/** `rebuildDerived`, under the name gen-loc reads. */
export function rewriteIndex(): void {
  rebuildDerived();
}
