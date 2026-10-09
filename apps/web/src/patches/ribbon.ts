// The newest set's "New" ribbon (R1383): the cards of the newest shipped set wear a small ribbon in
// the Almanac and the deck builder for the patch that ships the set and the one after, then lose it.
//
// Presentation only (CLAUDE.md rule 7). The window is counted in patch order, which is
// patches.json's own (R105, as source.ts keeps it): no version string is parsed or compared and no
// date is read. A lettered patch ("v0.2.10b") is a patch like any other. The patch that ships a set
// is the first whose changes add one of its cards; a set the catalog holds before it ships (R1420)
// has no such patch yet, so no card wears the ribbon until it does.

import patchesJson from "../../../../crates/cards/patches/patches.json";
import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { newestShippedSet, type CardDefs, type SetName } from "@jackioh/shared";

import type { Patch, PatchChange } from "./source.ts";

/** R1383: how many patches wear it: the one that ships the set and the one after. */
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
