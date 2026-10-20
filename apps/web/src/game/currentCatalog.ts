// The current catalog (SPEC §9.4: "static, versioned, shipped with the client"; R163, #1378): the
// definitions the engine's module registers and its compiled-in version, the one a deck save sends.
// Every screen with no game of its own reads it here and asks the server for none: the module is in
// before such a screen renders (`main.tsx`'s EngineGate). Call it from a render or an effect, never at
// import: before the module loads it throws.

import type { CatalogSnapshot } from "@jackioh/validator";

import { catalogVersion, registeredCatalog } from "../wasm/index.ts";
import { lookupFromDefs, type CardLookup } from "./catalog.ts";

let snapshot: CatalogSnapshot | null = null;
let lookup: CardLookup | null = null;

/** The registered catalog and its version, read once: the same object on every call. Never mutate it. */
export function currentCatalog(): CatalogSnapshot {
  snapshot ??= { version: catalogVersion(), cards: registeredCatalog() };
  return snapshot;
}

/** The board's names over `currentCatalog()` (`lookupFromDefs`), built once: the same function on every call. */
export function currentCatalogLookup(): CardLookup {
  lookup ??= lookupFromDefs(currentCatalog().cards);
  return lookup;
}
