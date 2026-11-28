// Static public patch data (§5.1) loads lazily one file at a time (R388).
// Patch order follows patches.json, never version-string sorting (R105 as B4.2 rewrites it).
// R5268000: a snapshot is the fold of its version's delta (`deltas/<version>.json`) and every one
// before it, its cards in `order.json`'s order; no version keeps a whole-catalog file.
// Keep the glob path-based: Vite rejects package specifiers.

import type { CardDefs } from "@jackioh/shared";
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
  /** R1369000: recorded by `patches ship` since #1366; the patches shipped before it have none. */
  readonly tier?: "major" | "patch" | "micro";
  /**
   * R3944000: on a major or a patch that rolls micros up (the daily patch `patches cut` writes, or a major
   * `patches ship` names after them), the versions of those micros, oldest first: exactly the micros
   * between the major or patch before it and itself. The entries shipped before it have none.
   */
  readonly includes?: readonly string[];
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

const PATCHES_FILE = "patches";
const INDEX_FILE = "index";
const TECHNICAL_FILE = "technical";
/** R5268000: every card id ever shipped, in the order each snapshot lists its own. */
const ORDER_FILE = "order";


import { applyDelta, readDelta, readOrder } from "./deltas.ts";
import { loadersByStem, type Loader } from "./loaders.ts";

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

/**
 * One cached loader per file; failures leave no cache entry so callers can retry. `deltas` are each
 * version's delta by version (R5268000), and each snapshot is folded once.
 */
export function sourceFromLoaders(loaders: Readonly<Record<string, Loader>>, deltas: Readonly<Record<string, Loader>> = {}): PatchSource {
  const cache = new Map<string, Promise<unknown>>();
  const loadFrom = (from: Readonly<Record<string, Loader>>, key: string, stem: string): Promise<unknown> | null => {
    const loader = from[stem];
    if (loader === undefined) return null;
    const cached = cache.get(key);
    if (cached !== undefined) return cached;
    const pending = loader().catch((error: unknown) => {
      cache.delete(key);
      throw error;
    });
    cache.set(key, pending);
    return pending;
  };
  const load = (stem: string): Promise<unknown> | null => loadFrom(loaders, stem, stem);
  const snapshots = new Map<string, Promise<Snapshot>>();
  const foldTo = (versions: readonly string[], at: number): Promise<Snapshot> => {
    const version = versions[at] ?? "";
    const cached = snapshots.get(version);
    if (cached !== undefined) return cached;
    const delta = loadFrom(deltas, `deltas/${version}`, version);
    const order = load(ORDER_FILE);
    if (delta === null || order === null) return Promise.reject(new Error(`${version} has no delta to fold`));
    const pending = Promise.all([at === 0 ? Promise.resolve({}) : foldTo(versions, at - 1), delta, order])
      .then(([before, value, ids]) => applyDelta(before, readDelta(value, version), readOrder(ids), version))
      .catch((error: unknown) => {
        snapshots.delete(version);
        throw error;
      });
    snapshots.set(version, pending);
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
      if (deltas[version] === undefined) return null;
      const pending = load(PATCHES_FILE);
      const versions = pending === null ? [] : readPatches(await pending).map((patch) => patch.version);
      const at = versions.indexOf(version);
      return at < 0 ? null : foldTo(versions, at);
    },
    technical: async () => {
      const pending = load(TECHNICAL_FILE);
      return pending === null ? [] : readTechnical(await pending);
    },
  };
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
const DELTA_FILES = import.meta.glob<unknown>("../../../../crates/cards/patches/deltas/*.json", { import: "default" });

export const realPatchSource: PatchSource = sourceFromLoaders(loadersByStem(PATCH_FILES), loadersByStem(DELTA_FILES));
