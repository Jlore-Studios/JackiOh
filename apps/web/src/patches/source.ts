// Where the patch history comes from (brief B4.2, R388): the one seam between the client and the
// catalog workstream's `packages/cards/patches/`, which holds
//
//   - `patches.json`: every patch in patch order, `{ version, date, title, source, notes, changes }`,
//     each change `{ id, name, kind: "added" | "changed" | "removed", fields? }`;
//   - `<version>.json`: the whole catalog as that patch left it (id -> CardDef, catalog.json's shape);
//   - `index.json`: card id -> the versions in which that card changed, in patch order.
//
// LAZY BY FILE. Vite's `import.meta.glob` turns every file into its own chunk behind a loader, so
// no page bundles a snapshot and a page that shows two patches downloads two snapshots. Each file is
// loaded at most once per page (a failed load is forgotten, so "Try again" asks again). The data is
// public (§5.1) and static, so the Patch notes page needs no server and no account.
//
// The glob names the directory by path: Vite resolves a glob that starts with `./`, `../` or `/` (or
// an alias), never a package specifier, and `@jackioh/cards/patches/*.json` fails with "Invalid glob".
// The package's `"./patches/*"` export is the same directory.
//
// THE ORDER OF PATCHES IS THE FILE'S (R105 as B4.2 rewrites it): `patches()` returns patches.json's
// array as it stands. Nothing here or downstream parses, compares or sorts a version string.
//
// Components never import this module's real source directly: they read `usePatchSource()`
// (context.tsx), which tests fill with `sourceFromData` fixtures.

import type { CardDef, CardDefs } from "@jackioh/shared";

/** One card a patch touched, as patches.json records it. */
export type PatchChange = {
  readonly id: string;
  readonly name: string;
  readonly kind: "added" | "changed" | "removed";
  /** The fields that changed ("cost", "base.text", …), on a "changed" entry. */
  readonly fields?: readonly string[];
};

/** One patch, as patches.json records it. */
export type Patch = {
  readonly version: string;
  /** ISO date, "2026-09-30"; shown as written, never parsed. */
  readonly date: string;
  readonly title: string;
  /** Where the patch came from: the issue, PR or commits. */
  readonly source: string;
  readonly notes: string;
  readonly changes: readonly PatchChange[];
};

/** The whole catalog as one patch left it: id -> definition. */
export type Snapshot = CardDefs;

/** Card id -> the versions in which it changed, in patch order (its first is the one that added it). */
export type HistoryIndex = Readonly<Record<string, readonly string[]>>;

/** The seam: everything the Patch notes page and the History section know about patches. */
export type PatchSource = {
  /** Every patch in patch order, oldest first, as patches.json lists them; [] when there are none. */
  patches(): Promise<readonly Patch[]>;
  /** Card id -> the versions in which it changed; {} when there are none. */
  index(): Promise<HistoryIndex>;
  /** The catalog as `version` left it, or null when no snapshot of that version exists. */
  snapshot(version: string): Promise<Snapshot | null>;
};

/** A file's loader, as `import.meta.glob` hands them out: resolves to the file's parsed JSON. */
export type Loader = () => Promise<unknown>;

/** The files that are not snapshots: the list, the index, and the shipping ledger (R632). */
const PATCHES_FILE = "patches";
const INDEX_FILE = "index";
const SHIPPED_FILE = "shipped";

/** "../../../../packages/cards/patches/v0.1.1.json" -> "v0.1.1". */
export function fileStem(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.endsWith(".json") ? name.slice(0, -".json".length) : name;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** patches.json, checked for the fields every screen reads; anything else is a broken file. */
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

/**
 * A source over one loader per file, keyed by file stem ("patches", "index", "v0.1.1"). Each file
 * is loaded once; a load that fails is dropped from the cache, so asking again retries it.
 */
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

/** `import.meta.glob`'s map (path -> loader) keyed by file stem instead. */
export function loadersByStem(glob: Readonly<Record<string, Loader>>): Record<string, Loader> {
  const out: Record<string, Loader> = {};
  for (const [path, loader] of Object.entries(glob)) out[fileStem(path)] = loader;
  return out;
}

/** A source over data already in hand: the fixtures tests feed through the context. */
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

/** No patch data at all: the page's empty state. */
export const EMPTY_PATCH_SOURCE: PatchSource = sourceFromData({ patches: [], index: {}, snapshots: {} });

/** The catalog workstream's files, one lazy chunk each. */
const PATCH_FILES = import.meta.glob<unknown>("../../../../packages/cards/patches/*.json", { import: "default" });

/** The real patch history, from `packages/cards/patches/`. */
export const realPatchSource: PatchSource = sourceFromLoaders(loadersByStem(PATCH_FILES));
