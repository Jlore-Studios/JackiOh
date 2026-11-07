// R1973002: the patches `/stats` can show, as `GET /api/stats/cards` lists them, newest first. A patch is the
// window the cards share, a major or a patch with its micros and the patches no game was played on (R1973001).
// R3324000: each offered patch is a tab, the newest always among them, and the newest short of games is
// collecting. R3324001: the current patch is the newest version listed.

import { CATALOG_VERSION } from "@jackioh/cards";
import type { PublicStatsCardsResponse } from "../net/api.ts";
import { STATS_PATCH_MIN_GAMES, STATS_PATCHES_OFFERED } from "./config.ts";

export type PatchWindow = NonNullable<PublicStatsCardsResponse["patches"]>[number];

/** The version that opened the patch: its name. */
export function patchName(patch: PatchWindow): string {
  return patch.versions[0] ?? "";
}

/** The version to ask the patch's figures at: its newest, whose window holds all of it. */
export function patchTarget(patch: PatchWindow): string {
  return patch.versions.at(-1) ?? "";
}

export function patchIndex(patches: readonly PatchWindow[], version: string): number {
  return patches.findIndex((patch) => patch.versions.includes(version));
}

/**
 * The patch tabs, newest first: the newest patch whatever its games (R3324000), the `STATS_PATCHES_OFFERED`
 * newest patches with enough games, and the one shown when it is not among them.
 */
export function offeredPatches(patches: readonly PatchWindow[], shown: number): PatchWindow[] {
  const enough = patches.filter((patch) => patch.games >= STATS_PATCH_MIN_GAMES).slice(0, STATS_PATCHES_OFFERED);
  return patches.filter((patch, index) => index === 0 || index === shown || enough.includes(patch));
}

/**
 * R3324000: whether the patch at `index` is still collecting its data: the newest patch, while it has fewer
 * than `STATS_PATCH_MIN_GAMES` games. An older patch short of games has stopped collecting.
 */
export function isCollecting(patches: readonly PatchWindow[], index: number): boolean {
  const patch = patches[index];
  return index === 0 && patch !== undefined && patch.games < STATS_PATCH_MIN_GAMES;
}

/** R3324001: the current patch: the newest version the server lists, else the client's own catalog version. */
export function currentVersion(patches: readonly PatchWindow[]): string {
  const newest = patches[0];
  return newest === undefined ? CATALOG_VERSION : patchTarget(newest) || CATALOG_VERSION;
}

/** The patch before (older) and after (newer) the one at `shown`, where there is one. */
export function neighbours(
  patches: readonly PatchWindow[],
  shown: number,
): { previous: PatchWindow | null; next: PatchWindow | null } {
  if (shown < 0) return { previous: null, next: null };
  return { previous: patches[shown + 1] ?? null, next: shown > 0 ? (patches[shown - 1] ?? null) : null };
}
