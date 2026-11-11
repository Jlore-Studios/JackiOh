// The same two facts `patch-meta.ts` inlines (§5.1): the newest version and each patch's
// added card ids, for bundlers without its `patchMeta()` plugin. Cypress's e2e specs are bundled
// by its webpack preprocessor, which has no Vite plugin: `package.json`'s `imports` maps the
// `#jackioh-patch-meta` specifier to this file under the `browser` condition, and to the disk
// reader by default. Every Vite pipeline keeps resolving through `patchMeta()` first
// (`enforce: "pre"`), so the 650 KB history never reaches the production bundle through here;
// only the e2e spec bundle carries it, which never ships.

import patchesJson from "../../crates/cards/patches/patches.json";

type PatchChange = { readonly id?: unknown; readonly kind?: unknown };
type PatchEntry = { readonly version?: unknown; readonly changes?: unknown };

/** One patch's version and the card ids it added — the shape the client inlines. */
export type PatchMeta = { readonly version: string; readonly added: readonly string[] };

function readPatches(): readonly PatchMeta[] {
  const patches: unknown = patchesJson;
  if (!Array.isArray(patches)) throw new Error("patches.json is not a list of patches");
  return (patches as readonly PatchEntry[]).map((patch) => {
    if (typeof patch.version !== "string") throw new Error("patches.json holds a patch with no version string");
    const changes = Array.isArray(patch.changes) ? (patch.changes as readonly PatchChange[]) : [];
    return {
      version: patch.version,
      added: changes
        .filter((change) => change.kind === "added" && typeof change.id === "string")
        .map((change) => change.id as string),
    };
  });
}

const metas = readPatches();
const newest = metas.at(-1);
if (newest === undefined) throw new Error("patches.json holds no patches");

/** The newest version, the history's last entry. */
export const PATCH_VERSION: string = newest.version;

/** Each patch's version and the card ids it added, oldest first. */
export const PATCH_ADDED: readonly PatchMeta[] = metas;
