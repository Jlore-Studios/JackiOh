// R1973002: the patches `/stats` can show, as `GET /api/stats/cards` lists them, newest first. A patch is the
// window the cards share, a major or a patch with its micros and the patches no game was played on (R1973001).

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
 * The picker's choices, newest first: the `STATS_PATCHES_OFFERED` newest patches with enough games, and
 * the one shown when it is not among them.
 */
export function offeredPatches(patches: readonly PatchWindow[], shown: number): PatchWindow[] {
  const offered = patches.filter((patch) => patch.games >= STATS_PATCH_MIN_GAMES).slice(0, STATS_PATCHES_OFFERED);
  const current = patches[shown];
  if (current === undefined || offered.includes(current)) return offered;
  return patches.filter((patch) => patch === current || offered.includes(patch));
}

/** The patch before (older) and after (newer) the one at `shown`, where there is one. */
export function neighbours(
  patches: readonly PatchWindow[],
  shown: number,
): { previous: PatchWindow | null; next: PatchWindow | null } {
  if (shown < 0) return { previous: null, next: null };
  return { previous: patches[shown + 1] ?? null, next: shown > 0 ? (patches[shown - 1] ?? null) : null };
}
