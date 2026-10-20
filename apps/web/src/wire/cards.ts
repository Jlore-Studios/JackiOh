// Catalog data and version are shared with Rust (SURFACE §10.4, §11.3; R668).
// Read their sources relatively for component tests; saves must use Rust's compiled version.

import catalogJson from "../../../../crates/cards/catalog.json";
import aliasesJson from "../../../../crates/cards/patches/aliases.json";
import patchesJson from "../../../../crates/cards/patches/patches.json";

import type { CardDefs } from "./index.ts";

export type { CardFlavour } from "../cards/flavour.ts";

/** JSON widening is trusted only after `cargo jackioh catalog check` proves the catalog values. */
export const CATALOG: CardDefs = catalogJson as unknown as CardDefs;

function newestVersion(patches: unknown): string {
  if (!Array.isArray(patches)) throw new Error("crates/cards/patches/patches.json is not a list of patches");
  const last: unknown = patches.at(-1);
  const version = typeof last === "object" && last !== null ? (last as { version?: unknown }).version : undefined;
  if (typeof version !== "string") throw new Error("crates/cards/patches/patches.json's newest patch has no version");
  return version;
}

/** Catalog version from the newest card-data patch (§9.4; R388). */
export const CATALOG_VERSION: string = newestVersion(patchesJson);

const VERSION_ALIASES: Readonly<Record<string, string>> = aliasesJson.aliases;

/**
 * R1371000: the name a version read from an unmarked file goes by now: its new name when the version
 * retcon (#1371) renamed it (`crates/cards/patches/aliases.json`), else the name itself. Only a reader
 * of a file or a stored record without the `versions` marker asks; old and new names overlap, so a
 * marked one is never mapped.
 */
export function currentVersionName(unmarked: string): string {
  return Object.hasOwn(VERSION_ALIASES, unmarked) ? (VERSION_ALIASES[unmarked] ?? unmarked) : unmarked;
}
