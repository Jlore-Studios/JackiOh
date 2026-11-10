// The two facts the first load needs of the patch history (§5.1): the newest version and each
// patch's added card ids, which is all `wire/cards.ts` (`CATALOG_VERSION`) and `patches/ribbon.ts`
// (`PATCH_LOG`) read. The client imports this package's `#jackioh-patch-meta` specifier, resolved
// here through package.json's `imports`. Where a Vite pipeline runs — `vite build`, `vite dev`,
// vitest, the component tests' server — the `patchMeta()` plugin below inlines just these values
// with `define`, so the 650 KB history and `node:fs` never enter a browser bundle. Where no
// pipeline runs — loading `vite.config.ts`, whose graph reaches `wire/cards.ts` through
// `static-pages.ts` — the same values come from disk below. Lives here, not under `src/wire/`,
// because cargo tests rewrite `src/wire/generated/*`, `engineConfig.ts` and `serverConfig.ts`.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Plugin } from "vite";

const PATCHES = "crates/cards/patches/patches.json";

type PatchChange = { readonly id?: unknown; readonly kind?: unknown };
type PatchEntry = { readonly version?: unknown; readonly changes?: unknown };

/** One patch's version and the card ids it added — the shape the client inlines. */
export type PatchMeta = { readonly version: string; readonly added: readonly string[] };

function readPatches(): readonly PatchMeta[] {
  const patches = JSON.parse(readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", PATCHES), "utf8")) as unknown;
  if (!Array.isArray(patches)) throw new Error(`${PATCHES} is not a list of patches`);
  return (patches as readonly PatchEntry[]).map((patch) => {
    if (typeof patch.version !== "string") throw new Error(`${PATCHES} holds an entry with no version`);
    const changes = Array.isArray(patch.changes) ? (patch.changes as readonly PatchChange[]) : [];
    return {
      version: patch.version,
      added: changes
        .filter((change) => change.kind === "added" && typeof change.id === "string")
        .map((change) => change.id as string),
    };
  });
}

/**
 * The values a Vite pipeline inlines (see `patchMeta`). Declared, never assigned here: bare Node
 * has no `define`, so the `typeof` below falls through to disk.
 */
declare const __JACKIOH_PATCH_META__: readonly PatchMeta[] | undefined;

const META: readonly PatchMeta[] =
  typeof __JACKIOH_PATCH_META__ !== "undefined" ? __JACKIOH_PATCH_META__ : readPatches();
const newest = META.at(-1);
if (newest === undefined) throw new Error(`${PATCHES} holds no patches`);

/** The newest version, the history's last entry. */
export const PATCH_VERSION: string = newest.version;

/** Each patch's version and the card ids it added, oldest first. */
export const PATCH_ADDED: readonly PatchMeta[] = META;

export function patchMeta(): Plugin {
  return {
    name: "jackioh-patch-meta",
    config() {
      // Inline the values at compile time in every pipeline this plugin joins — the page, its
      // workers, vitest and the component server alike — so the patch history and `node:fs`
      // never enter a browser bundle. Bare Node (no `define`) reads them from disk above.
      return { define: { __JACKIOH_PATCH_META__: JSON.stringify(META) } };
    },
  };
}
