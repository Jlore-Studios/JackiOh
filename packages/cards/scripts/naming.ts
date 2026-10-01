/**
 * The catalog-id <-> filename convention for card scripts and card tests
 * (BUILD M4-T2 `cards/src/scripts/NNN-slug.ts`, M4-T3 `cards/test/NNN-slug.test.ts`, SPEC §10.9,
 * and patch v0.2.0's set folders, B2.2).
 *
 * Pure string functions, no I/O and no catalog import, so `gen-registry.ts`, `missing-tests.ts`,
 * `gen-loc.ts` and `test/registry.test.ts` all read one copy of the rules.
 *
 * The convention, by example (the catalog id is the key of `packages/cards/catalog.json`, SPEC §5):
 *
 *   core-001             "Big D-fender"         src/scripts/001-big-d-fender.ts
 *   core-051-1           "KY's Empty Notebook"  src/scripts/051-1-kys-empty-notebook.ts
 *   core-t-rush          "Rush Token"           src/scripts/t-rush.ts
 *   classic-043          "Plague Nuke"          src/scripts/classic/043-plague-nuke.ts
 *   classicplus-012-1    "Devour"               src/scripts/classic-plus/012-1-devour.ts
 *   classicplus-t-ai-01  "Helpful Assistant"    src/scripts/classic-plus/t-ai-01-helpful-assistant.ts
 *
 * and the same path under `test/` with `.test.ts`. So a file's path is its set's folder (none for
 * Core) and a basename `<prefix>-<slug>`, where the prefix is the id minus its set segment, except for
 * Core's named tokens of SPEC §7 — the four shared ones, The Coin and the Ghoul Token — which are filed
 * under the bare prefix (`t-rush`, `t-coin`). An index repeats across sets ("43" is a Core, a Classic
 * and a Classic+ card), so the folder, never the basename alone, says which set a file belongs to.
 */

/** Every shipped id is `<set>-<prefix>`, the set segment holding no hyphen (`classicplus`, B2.2). */
const SET_SEGMENT_SEPARATOR = "-";

/**
 * The shipped sets' id segments and the folder each one's scripts and tests live in, in catalog
 * order: Core at the top of `src/scripts/` and `test/`, the others in a folder of their own.
 */
export const SET_FOLDERS = {
  core: "",
  classic: "classic",
  classicplus: "classic-plus",
} as const;

export type SetSegment = keyof typeof SET_FOLDERS;

const SET_SEGMENTS = Object.keys(SET_FOLDERS) as SetSegment[];

/** The set folders that are not the package root, for a tool that walks them. */
export const SET_SUBFOLDERS: readonly string[] = SET_SEGMENTS.map((segment) => SET_FOLDERS[segment]).filter(
  (folder) => folder !== "",
);

/** `classic-043` -> `classic`; `undefined` for an id whose set segment names no shipped set. */
export function setSegmentOf(id: string): SetSegment | undefined {
  const cut = id.indexOf(SET_SEGMENT_SEPARATOR);
  if (cut <= 0) return undefined;
  const segment = id.slice(0, cut);
  return (SET_SEGMENTS as readonly string[]).includes(segment) ? (segment as SetSegment) : undefined;
}

/** The folder a catalog id's script and test live in: `""` for Core, `classic`, `classic-plus`. */
export function folderOf(id: string): string {
  const segment = setSegmentOf(id);
  return segment === undefined ? "" : SET_FOLDERS[segment];
}

/** The set segment a folder holds (`""` -> `core`), or `undefined` for a folder that holds none. */
export function segmentOfFolder(folder: string): SetSegment | undefined {
  return SET_SEGMENTS.find((segment) => SET_FOLDERS[segment] === folder);
}

/** Catalog order of the sets (Core, Classic, Classic+), for sorting; an unknown set sorts last. */
export function setRank(id: string | undefined): number {
  const segment = id === undefined ? undefined : setSegmentOf(id);
  return segment === undefined ? SET_SEGMENTS.length : SET_SEGMENTS.indexOf(segment);
}

/**
 * The filename prefix of a catalog id: the id minus its leading set segment.
 * `core-001` -> `001`, `core-051-1` -> `051-1`, `core-t-rush` -> `t-rush`,
 * `classicplus-t-ai-01` -> `t-ai-01`.
 */
export function slugPrefixOf(id: string): string {
  const cut = id.indexOf(SET_SEGMENT_SEPARATOR);
  if (cut <= 0 || cut === id.length - 1) {
    throw new Error(`"${id}" is not a catalog id (expected "<set>-<prefix>", e.g. "core-043")`);
  }
  return id.slice(cut + 1);
}

/** `051-1` and `t-rush` are token prefixes; `051` is a card prefix. Nothing nests deeper. */
export function isTokenPrefix(prefix: string): boolean {
  return /^t-/.test(prefix) || /^\d+-\d+$/.test(prefix);
}

/**
 * A shared token's prefix (`t-rush`, `t-ai-01`): a token no one card defines. Core's (SPEC §7's
 * four shared ones, The Coin and the Ghoul Token) are filed under the bare prefix (`t-rush.ts`);
 * Classic+'s AI generated cards carry their slug like any card (`t-ai-01-helpful-assistant.ts`).
 */
export function isSharedTokenPrefix(prefix: string): boolean {
  return /^t-/.test(prefix);
}

/**
 * A card name as a filename slug: lowercase, apostrophes dropped, every other run of
 * non-alphanumerics collapsed to one `-`.
 *
 * Checked against every name, so: "Big D-fender" -> `big-d-fender`, "KY's Empty Notebook" ->
 * `kys-empty-notebook` (the apostrophe vanishes rather than becoming a dash), "CN-Virus" ->
 * `cn-virus`, "/fullsend" -> `fullsend`, "Call to Chaos (Core Edition)" ->
 * `call-to-chaos-core-edition`, `"Miss" Mrow` -> `miss-mrow`, "4-mana 7/7" -> `4-mana-7-7`,
 * "Forever&" -> `forever`, "BOOM! Big Max" -> `boom-big-max`.
 */
export function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/['‘’]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/**
 * The canonical basename (no extension, no folder) for a catalog entry: what `gen-registry.ts`
 * expects to import and what `missing-tests.ts` prints as the file a card still needs.
 */
export function expectedBasename(id: string, name: string): string {
  const prefix = slugPrefixOf(id);
  // Core's SPEC §7 shared tokens are one word already ("Rush Token" under `t-rush`), so the prefix
  // is the whole filename; anything else carries its slug.
  if (isSharedTokenPrefix(prefix) && setSegmentOf(id) === "core") return prefix;
  return `${prefix}-${slugify(name)}`;
}

/** The path under `src/scripts/` or `test/`, without an extension: `classic/043-plague-nuke`. */
export function expectedRelPath(id: string, name: string): string {
  const folder = folderOf(id);
  const basename = expectedBasename(id, name);
  return folder === "" ? basename : `${folder}/${basename}`;
}

/** `001-big-d-fender.ts` -> `001-big-d-fender`; also strips `.test.ts`. */
export function basenameOf(filename: string): string {
  return filename.replace(/\.test\.ts$/, "").replace(/\.ts$/, "");
}

/**
 * Whether `basename` (no extension) is the file of catalog id `id`, in that id's own set folder.
 *
 * The slug itself is not checked: the prefix decides which card a file belongs to, so a misspelled
 * slug still lands on the right card (and `expectedBasename` is what reports the misspelling).
 * The one hard case is the prefix boundary: `051-1-kys-empty-notebook` belongs to `core-051-1`
 * (KY's Empty Notebook) and must NOT be read as a slug of `core-051` (KY's Private Tutor).
 *
 * With `allIds` (every catalog id) the answer is exact: the longest id prefix of that set the
 * basename carries wins, which is what `gen-registry.ts` and `missing-tests.ts` do. Without it the
 * rule is the documented heuristic "a lone digit segment right after a card prefix is a token
 * sub-index", which is right for every shipped id except #25 "4-mana 7/7", whose slug itself opens
 * with a digit segment (`025-4-mana-7-7`) — pass `allIds` when the exact answer matters.
 */
export function matchesCard(basename: string, id: string, allIds?: readonly string[]): boolean {
  if (allIds !== undefined) return resolveBasename(basename, allIds, folderOf(id)) === id;

  const prefix = slugPrefixOf(id);
  if (basename === prefix) return true; // the slugless form: `t-rush.ts`
  if (!basename.startsWith(`${prefix}-`)) return false;
  const rest = basename.slice(prefix.length + 1);
  if (rest === "") return false;
  if (isTokenPrefix(prefix)) return true; // a token id has no sub-token, so the rest is a slug
  return !/^\d(-|$)/.test(rest);
}

/**
 * Which catalog id a basename in `folder` (`""`, Core's, by default) belongs to, by longest prefix
 * among that folder's set: `051-1-kys-empty-notebook` matches the prefixes `051` and `051-1`, and
 * the longer one is the card that owns the file. `undefined` means the filename names no card of
 * that set (a typo'd or unpadded prefix), or the folder holds no set.
 */
export function resolveBasename(basename: string, allIds: readonly string[], folder = ""): string | undefined {
  const segment = segmentOfFolder(folder);
  if (segment === undefined) return undefined;
  let best: string | undefined;
  let bestLength = -1;
  for (const id of allIds) {
    if (setSegmentOf(id) !== segment) continue;
    const prefix = slugPrefixOf(id);
    if (basename !== prefix && !basename.startsWith(`${prefix}-`)) continue;
    if (prefix.length > bestLength) {
      best = id;
      bestLength = prefix.length;
    }
  }
  return best;
}

/** `classic/043-plague-nuke` -> its folder and basename; a Core path has the folder `""`. */
export function splitRelPath(relPath: string): { folder: string; basename: string } {
  const cut = relPath.lastIndexOf("/");
  return cut < 0
    ? { folder: "", basename: relPath }
    : { folder: relPath.slice(0, cut), basename: relPath.slice(cut + 1) };
}

/** Which catalog id a path under `src/scripts/` or `test/` (no extension) belongs to. */
export function resolveRelPath(relPath: string, allIds: readonly string[]): string | undefined {
  const { folder, basename } = splitRelPath(relPath);
  return resolveBasename(basename, allIds, folder);
}

/**
 * A filename prefix as a sortable number, mirroring the engine's index ranking
 * (`engine/src/catalog.ts` `indexRank`): `001` -> 1, `051-1` -> 51.1 (so a card-defined token sorts
 * straight after its card), `t-rush` -> +Infinity (shared tokens sort last within their set).
 */
export function prefixRank(prefix: string): number {
  const parts = /^(\d+)(?:-(\d+))?$/.exec(prefix);
  if (parts === null) return Number.POSITIVE_INFINITY;
  const [, main, sub] = parts;
  const rank = Number.parseFloat(sub === undefined ? `${main}` : `${main}.${sub}`);
  return Number.isFinite(rank) ? rank : Number.POSITIVE_INFINITY;
}

/** A sort key: `[named-no-card tier, set, §5 index rank, path]`. */
export type SortKey = readonly [number, number, number, string];

/**
 * The deterministic ordering every tool uses: catalog order — set (Core, Classic, Classic+), then
 * SPEC §5 index ascending, card-defined tokens after their card, shared tokens last within the set —
 * and anything that names no catalog card after all of it; then the path, so the order never
 * depends on the order the filesystem listed a directory.
 */
export function sortKey(path: string, id: string | undefined): SortKey {
  const tier = id === undefined ? 1 : 0;
  const set = setRank(id);
  const rank = id === undefined ? Number.POSITIVE_INFINITY : prefixRank(slugPrefixOf(id));
  return [tier, set, rank, path];
}

/** Compares two `sortKey`s. */
export function compareSortKeys(a: SortKey, b: SortKey): number {
  if (a[0] !== b[0]) return a[0] - b[0];
  if (a[1] !== b[1]) return a[1] - b[1];
  if (a[2] !== b[2]) return a[2] < b[2] ? -1 : 1; // Infinity - Infinity is NaN, so compare, don't subtract
  if (a[3] === b[3]) return 0;
  return a[3] < b[3] ? -1 : 1;
}

/**
 * A legal, unique JS identifier for the namespace import of a script file: paths are unique within
 * `src/scripts/`, the folder is part of the alias (`classic/043-…` -> `mclassic_043_…`), and the `m`
 * guard keeps `001-…` from starting an identifier with a digit.
 */
export function moduleAliasOf(relPath: string): string {
  return `m${relPath.replace(/[^a-zA-Z0-9]+/g, "_")}`;
}
