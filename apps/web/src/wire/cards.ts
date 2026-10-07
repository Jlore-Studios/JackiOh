// `@jackioh/cards` for the web client (docs/v0.3.0/SURFACE.md §10.4): the shipped catalog as data,
// its version, and the flavour sidecar's entry type. The card scripts are Rust now
// (`crates/cards`), compiled into the WebAssembly module and registered by its `init`, so nothing
// here registers anything; the TypeScript `registerAll()` calls went with them.
//
// The data is `crates/cards/catalog.json` itself, read by relative path so this module needs no
// alias of its own (the component tests' Vite server imports it too). The version is the newest entry
// of `crates/cards/patches/patches.json`, which is the very value the Rust side compiles in
// (`jackioh_cards::catalog_version()`, SURFACE §11.3): a practice save stamped with one is read
// against the other (R668).

import catalogJson from "../../../../crates/cards/catalog.json";
import patchesJson from "../../../../crates/cards/patches/patches.json";

import type { CardDefs } from "./index.ts";

export type { CardFlavour } from "../cards/flavour.ts";

/**
 * JSON widens `type`, `rarity`, `set` and the keyword unions to `string`, so the data and the
 * `CardDef` type meet here once. The Rust catalog checks (`cargo jackioh catalog check`) are what
 * prove the values; this cast carries no other trust.
 */
export const CATALOG: CardDefs = catalogJson as unknown as CardDefs;

/** Every catalog id, in catalog.json order. */
export const CATALOG_IDS: readonly string[] = Object.keys(CATALOG);

function newestVersion(patches: unknown): string {
  if (!Array.isArray(patches)) throw new Error("crates/cards/patches/patches.json is not a list of patches");
  const last: unknown = patches.at(-1);
  const version = typeof last === "object" && last !== null ? (last as { version?: unknown }).version : undefined;
  if (typeof version !== "string") throw new Error("crates/cards/patches/patches.json's newest patch has no version");
  return version;
}

/** §9.4: the catalog version, bumped when card data changes (the newest patch, R388). */
export const CATALOG_VERSION: string = newestVersion(patchesJson);
