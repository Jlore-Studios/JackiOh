// The two facts the first load needs of the patch history (§5.1): the newest version and each
// patch's added card ids, which is all `wire/cards.ts` (`CATALOG_VERSION`) and `patches/ribbon.ts`
// (`PATCH_LOG`) read. The client imports the `#jackioh-patch-meta` specifier. In every Vite
// pipeline — `vite build`, `vite dev`, vitest, the component tests' server — the `patchMeta()`
// plugin below resolves it to a virtual module holding just these values, so the 650 KB history
// and `node:fs` never reach a browser. Where no plugin runs — loading `vite.config.ts`, whose graph
// reaches `wire/cards.ts` through `static-pages.ts`, bundled by Node's loader that rejects a
// `virtual:` scheme — package.json's `imports` maps the specifier to this file, whose exports
// come from disk (`PATCH_VERSION`, `PATCH_ADDED`); a browser never loads it. Lives here, not
// under `src/wire/`, because cargo tests rewrite `src/wire/generated/*`, `engineConfig.ts` and
// `serverConfig.ts`.

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

const newest = readPatches().at(-1);
if (newest === undefined) throw new Error(`${PATCHES} holds no patches`);

/** The newest version, the history's last entry. */
export const PATCH_VERSION: string = newest.version;

/** Each patch's version and the card ids it added, oldest first. */
export const PATCH_ADDED: readonly PatchMeta[] = readPatches();

const SPECIFIER = "#jackioh-patch-meta";
const VIRTUAL = `\0virtual:jackioh-patch-meta`;

export function patchMeta(): Plugin {
  return {
    name: "jackioh-patch-meta",
    // Before Vite's own resolver, which would map the specifier to the real file.
    enforce: "pre",
    resolveId(source) {
      return source === SPECIFIER ? VIRTUAL : null;
    },
    load(id) {
      if (id !== VIRTUAL) return null;
      // Read fresh, so a build or a test run after `patches ship` sees the new history.
      const patches = readPatches();
      const last = patches.at(-1);
      if (last === undefined) throw new Error(`${PATCHES} holds no patches`);
      return `export const PATCH_VERSION = ${JSON.stringify(last.version)};\nexport const PATCH_ADDED = ${JSON.stringify(patches)};\n`;
    },
  };
}
