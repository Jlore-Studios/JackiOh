// The newest set's "New" ribbon (R1383): the newest shipped set's cards wear it in the Almanac and the
// deck builder for the patch that ships the set and the one after. Presentation only (CLAUDE.md rule 7);
// the window is counted in patches.json's own order (R105, as source.ts keeps it), never a version or
// date. A set the catalog holds before it ships (R1420) wears it only once a patch adds its cards.

import patchesJson from "../../../../crates/cards/patches/patches.json";
import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { newestShippedSet, type CardDefs, type SetName } from "@jackioh/shared";

import type { Patch, PatchChange } from "./source.ts";

/** R1383: how many patches wear it. */
export const NEW_RIBBON_PATCHES = 2;

/** What the ribbon reads of a patches.json entry. */
export type RibbonPatch = Pick<Patch, "version"> & { readonly changes: readonly Pick<PatchChange, "id" | "kind">[] };

/** The patch history, oldest first; `cargo jackioh patches check` proves the file. */
export const PATCH_LOG: readonly RibbonPatch[] = patchesJson as unknown as readonly RibbonPatch[];

/**
 * R1383: `newest` while `version` is the patch that added its first card or the patch after, else
 * null. A version the history does not list is in no window.
 */
export function ribbonSet(
  patches: readonly RibbonPatch[],
  version: string,
  cards: CardDefs,
  newest: SetName = newestShippedSet(),
): SetName | null {
  const shippedAt = patches.findIndex((patch) =>
    patch.changes.some((change) => change.kind === "added" && cards[change.id]?.set === newest),
  );
  const now = patches.findIndex((patch) => patch.version === version);
  if (shippedAt < 0 || now < shippedAt || now - shippedAt >= NEW_RIBBON_PATCHES) return null;
  return newest;
}

/** The set whose cards wear the ribbon in this build, or null. */
export const NEW_RIBBON_SET: SetName | null = ribbonSet(PATCH_LOG, CATALOG_VERSION, CATALOG);
