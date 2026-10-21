// Static public patch data (§5.1) loads lazily one file at a time (R388); failed loads can retry.
// Patch order follows patches.json, never version-string sorting (R105 as B4.2 rewrites it).
// Keep the glob path-based: Vite rejects package specifiers.

import type { CardDef, CardDefs } from "@jackioh/shared";
import { isRecord } from "../wire/json.ts";

export type PatchChange = {
  readonly id: string;
  readonly name: string;
  readonly kind: "added" | "changed" | "removed";
  readonly fields?: readonly string[];
};

/** R724001: what players read of a patch, apart from its record: a title and one fact per line. */
export type PlayerNote = {
  readonly title: string;
  readonly new: readonly string[];
  readonly balance: readonly string[];
  readonly changes: readonly string[];
  readonly text: readonly string[];
  readonly fixes: readonly string[];
};

export type Patch = {
  readonly version: string;
  readonly date: string;
  readonly title: string;
  readonly source: string;
  readonly notes: string;
  /** R724001: every shipped patch has one (`cargo jackioh patches check`); a fixture may not. */
  readonly player?: PlayerNote;
  /** R1369000: recorded by `patches ship` from #1366 on; absent on the patches shipped before it. */
  readonly tier?: "major" | "patch" | "micro";
  readonly changes: readonly PatchChange[];
};

/**
 * R1369000: one line of the technical log, `technical.json` (`cargo jackioh patches log`): a change
 * that shipped no card data, logged on the X that was live when it reached players.
 */
export type TechnicalEntry = {
  readonly version: string;
  readonly commit: string;
  readonly pr?: number;
  readonly title: string;
  readonly kind: "app" | "ai" | "internal";
  /** The line players read; the page shows only App and AI entries that have one. */
  readonly player?: string;
};

export type Snapshot = CardDefs;

export type HistoryIndex = Readonly<Record<string, readonly string[]>>;

export type PatchSource = {
  patches(): Promise<readonly Patch[]>;
  index(): Promise<HistoryIndex>;
  snapshot(version: string): Promise<Snapshot | null>;
  technical(): Promise<readonly TechnicalEntry[]>;
};

export type Loader = () => Promise<unknown>;

/** R646: files that are not snapshots. */
const PATCHES_FILE = "patches";
const INDEX_FILE = "index";
const SHIPPED_FILE = "shipped";
const TECHNICAL_FILE = "technical";

export function fileStem(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.endsWith(".json") ? name.slice(0, -".json".length) : name;
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

function readTechnical(value: unknown): readonly TechnicalEntry[] {
  if (!Array.isArray(value)) throw new Error("technical.json is not a list of entries");
  for (const entry of value) {
    if (!isRecord(entry) || typeof entry.version !== "string" || typeof entry.kind !== "string") {
      throw new Error("technical.json holds an entry that is not one");
    }
  }
  return value as readonly TechnicalEntry[];
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
      if (version === PATCHES_FILE || version === INDEX_FILE || version === SHIPPED_FILE || version === TECHNICAL_FILE) return null;
      const pending = load(version);
      return pending === null ? null : readSnapshot(await pending, version);
    },
    technical: async () => {
      const pending = load(TECHNICAL_FILE);
      return pending === null ? [] : readTechnical(await pending);
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
  technical?: readonly TechnicalEntry[];
}): PatchSource {
  return {
    patches: () => Promise.resolve(data.patches),
    index: () => Promise.resolve(data.index),
    snapshot: (version) => Promise.resolve(data.snapshots[version] ?? null),
    technical: () => Promise.resolve(data.technical ?? []),
  };
}

export const EMPTY_PATCH_SOURCE: PatchSource = sourceFromData({ patches: [], index: {}, snapshots: {} });

const PATCH_FILES = import.meta.glob<unknown>("../../../../crates/cards/patches/*.json", { import: "default" });

export const realPatchSource: PatchSource = sourceFromLoaders(loadersByStem(PATCH_FILES));
