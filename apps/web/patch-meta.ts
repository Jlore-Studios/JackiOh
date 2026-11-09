// The two facts the first load needs of the patch history (§5.1): the newest version and each
// patch's added card ids, which is all `wire/cards.ts` (`CATALOG_VERSION`) and `patches/ribbon.ts`
// (`PATCH_LOG`) read. The client imports this package's `#jackioh-patch-meta` specifier, which the
// `patchMeta()` plugin below serves as a virtual module carrying just these values, so the 650 KB
// history never enters the bundle. Where no plugin runs — loading `vite.config.ts`,
// `vitest.config.ts`, or the component tests' Vite server, whose graph reaches `wire/cards.ts`
// through `static-pages.ts` — the specifier resolves here through package.json's `imports`, and the
// same values come from disk. Lives here, not under `src/wire/`, because cargo tests rewrite
// `src/wire/generated/*`, `engineConfig.ts` and `serverConfig.ts`.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Plugin } from "vite";

/** The specifier the client imports: the plugin serves it, Node resolves it to this file. */
export const PATCH_META_SPECIFIER = "#jackioh-patch-meta";

/** The plugin's own module id, never imported in source. */
const MODULE_ID = "virtual:jackioh-patch-meta";

const PATCHES = "crates/cards/patches/patches.json";

type PatchChange = { readonly id?: unknown; readonly kind?: unknown };
type PatchEntry = { readonly version?: unknown; readonly changes?: unknown };

function readPatches(): { readonly version: string; readonly added: readonly string[] }[] {
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

const META = readPatches();
const newest = META.at(-1);
if (newest === undefined) throw new Error(`${PATCHES} holds no patches`);

/** The newest version, the history's last entry. */
export const PATCH_VERSION: string = newest.version;

/** Each patch's version and the card ids it added, oldest first. */
export const PATCH_ADDED: readonly { readonly version: string; readonly added: readonly string[] }[] = META;

export function patchMeta(): Plugin {
  return {
    name: "jackioh-patch-meta",
    resolveId(source) {
      return source === PATCH_META_SPECIFIER ? `\0${MODULE_ID}` : null;
    },
    load(id) {
      if (id !== `\0${MODULE_ID}`) return null;
      return (
        `export const PATCH_VERSION = ${JSON.stringify(PATCH_VERSION)};\n` +
        `export const PATCH_ADDED = ${JSON.stringify(PATCH_ADDED)};\n`
      );
    },
  };
}
