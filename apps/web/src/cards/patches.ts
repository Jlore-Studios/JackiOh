// The card patch history as the client reads it (SPEC §10.10, R375): the patch list, what each
// patch did to which card, and the catalog snapshots, which load on demand.
//
// `patches.json` and `changes.json` are small and imported statically: the site footer names the
// current version on every screen, a card's History is labelled with its version count before it
// opens, and the Patch notes page lists each patch's cards, all without a snapshot. The snapshots are
// the whole catalog five times over (about 290 KB), so each is its own dynamic import, and the main
// bundle carries none of them: they load when a History section opens (`loadSnapshots`), once per
// page.
//
// Presentation only (CLAUDE.md rule 7): no game, replay or server reads any of this, and the history
// logic itself is `@jackioh/cards/history`, pure and shared with the tests.

import changesJson from "@jackioh/cards/patches/changes.json";
import patchesJson from "@jackioh/cards/patches/patches.json";
import { historyVersions, newestPatch, type Patch, type PatchCards, type PatchSource, type Snapshots } from "@jackioh/cards/history";
import type { CardDefs } from "@jackioh/shared";

/** Every patch, oldest first. JSON widens a source's `kind` to a string; packages/cards proves the data. */
export const PATCHES: readonly Patch[] = patchesJson as unknown as readonly Patch[];

/** The cards each patch created, changed and removed, by version (`patchCards` over the snapshots). */
export const PATCH_CARDS: Readonly<Record<string, PatchCards>> = changesJson;

/** The current version: the newest patch's. */
export const CURRENT_VERSION: string = newestPatch(PATCHES).version;

/** Where the patches came from. */
export const REPOSITORY_URL = "https://github.com/jgoetzmann/JackiOh";

/** R375: what a reconstructed version's badge says about it, in full. */
export const RECONSTRUCTED_NOTE =
  "The versions before v0.1.1 were never named at the time: their labels were given later, to states reconstructed from the repository's history.";

/** An issue's or a pull request's page. */
export function sourceUrl(source: PatchSource): string {
  return `${REPOSITORY_URL}/${source.kind === "pr" ? "pull" : "issues"}/${String(source.number)}`;
}

/** How a source is named on the page: "Issue #27", "PR #28". */
export function sourceLabel(source: PatchSource): string {
  return `${source.kind === "pr" ? "PR" : "Issue"} #${String(source.number)}`;
}

/** A commit's page. */
export function commitUrl(commit: string): string {
  return `${REPOSITORY_URL}/commit/${commit}`;
}

/** How many versions a card's history has, from `changes.json`; 0 for a card the history does not know. */
export function versionCount(id: string): number {
  return historyVersions(id, PATCHES, PATCH_CARDS).length;
}

/**
 * One dynamic import per snapshot, by version. A bare specifier cannot be a template in a dynamic
 * import, so each is written out; `patches.test.ts` fails when this list and `patches.json` part.
 */
export const SNAPSHOT_LOADERS: Readonly<Record<string, () => Promise<{ default: unknown }>>> = {
  "v0.1.0": () => import("@jackioh/cards/patches/snapshots/v0.1.0.json"),
  "v0.1.0-r1": () => import("@jackioh/cards/patches/snapshots/v0.1.0-r1.json"),
  "v0.1.0-r2": () => import("@jackioh/cards/patches/snapshots/v0.1.0-r2.json"),
  "v0.1.0-r3": () => import("@jackioh/cards/patches/snapshots/v0.1.0-r3.json"),
  "v0.1.1": () => import("@jackioh/cards/patches/snapshots/v0.1.1.json"),
};

let loading: Promise<Snapshots> | null = null;

/** Every patch's snapshot, loaded once per page; a failed load is tried again on the next call. */
export function loadSnapshots(): Promise<Snapshots> {
  if (loading !== null) return loading;
  const load = Promise.all(
    PATCHES.map(async (patch) => {
      const loader = SNAPSHOT_LOADERS[patch.version];
      if (loader === undefined) throw new Error(`no snapshot loader for ${patch.version} (cards/patches.ts)`);
      const module = await loader();
      return [patch.version, module.default as CardDefs] as const;
    }),
  ).then((entries): Snapshots => Object.fromEntries(entries));
  loading = load;
  load.catch(() => {
    if (loading === load) loading = null;
  });
  return load;
}
