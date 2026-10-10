// Static public patch data (§5.1) loads lazily one file at a time (R388); failed loads can retry.
// Patch order follows patches.json, never version-string sorting (R105 as B4.2 rewrites it).
// Keep the glob path-based: Vite rejects package specifiers.

import type { CardDef, CardDefs } from "@jackioh/shared";

export type PatchChange = {
  readonly id: string;
  readonly name: string;
  readonly kind: "added" | "changed" | "removed";
  readonly fields?: readonly string[];
};

export type Patch = {
  readonly version: string;
  readonly date: string;
  readonly title: string;
  readonly source: string;
  readonly notes: string;
  readonly changes: readonly PatchChange[];
};

export type Snapshot = CardDefs;

export type HistoryIndex = Readonly<Record<string, readonly string[]>>;

export type PatchSource = {
  patches(): Promise<readonly Patch[]>;
  index(): Promise<HistoryIndex>;
  snapshot(version: string): Promise<Snapshot | null>;
};

export type Loader = () => Promise<unknown>;

/** R646: files that are not snapshots. */
const PATCHES_FILE = "patches";
const INDEX_FILE = "index";
const SHIPPED_FILE = "shipped";

export function fileStem(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.endsWith(".json") ? name.slice(0, -".json".length) : name;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readPatches(value: unknown): readonly Patch[] {
  if (!Array.isArray(value)) throw new Error("patches.json is not a list of patches");
  for (const patch of value) {
    if (!isRecord(patch) || typeof patch.version !== "string" || !Array.isArray(patch.changes)) {
      throw new Error("patches.json holds an entry that is not a patch");
    }
  }
  return value as readonly Patch[];
}

function readIndex(value: unknown): HistoryIndex {
  if (!isRecord(value)) throw new Error("index.json is not a map of card ids");
  return value as HistoryIndex;
}

function readSnapshot(value: unknown, version: string): Snapshot {
  if (!isRecord(value)) throw new Error(`${version}.json is not a catalog snapshot`);
  return value as Record<string, CardDef>;
}

/** One cached loader per file; failures leave no cache entry so callers can retry. */
export function sourceFromLoaders(loaders: Readonly<Record<string, Loader>>): PatchSource {
  const cache = new Map<string, Promise<unknown>>();
  const load = (stem: string): Promise<unknown> | null => {
    const loader = loaders[stem];
    if (loader === undefined) return null;
    const cached = cache.get(stem);
    if (cached !== undefined) return cached;
    const pending = loader().catch((error: unknown) => {
      cache.delete(stem);
      throw error;
    });
    cache.set(stem, pending);
    return pending;
  };
  return {
    patches: async () => {
      const pending = load(PATCHES_FILE);
      return pending === null ? [] : readPatches(await pending);
    },
    index: async () => {
      const pending = load(INDEX_FILE);
      return pending === null ? {} : readIndex(await pending);
    },
    snapshot: async (version) => {
      if (version === PATCHES_FILE || version === INDEX_FILE || version === SHIPPED_FILE) return null;
      const pending = load(version);
      return pending === null ? null : readSnapshot(await pending, version);
    },
  };
}

export function loadersByStem(glob: Readonly<Record<string, Loader>>): Record<string, Loader> {
  const out: Record<string, Loader> = {};
  for (const [path, loader] of Object.entries(glob)) out[fileStem(path)] = loader;
  return out;
}

export function sourceFromData(data: {
  patches: readonly Patch[];
  index: HistoryIndex;
  snapshots: Readonly<Record<string, Snapshot>>;
}): PatchSource {
  return {
    patches: () => Promise.resolve(data.patches),
    index: () => Promise.resolve(data.index),
    snapshot: (version) => Promise.resolve(data.snapshots[version] ?? null),
  };
}

export const EMPTY_PATCH_SOURCE: PatchSource = sourceFromData({ patches: [], index: {}, snapshots: {} });

const PATCH_FILES = import.meta.glob<unknown>("../../../../crates/cards/patches/*.json", { import: "default" });

export const realPatchSource: PatchSource = sourceFromLoaders(loadersByStem(PATCH_FILES));
