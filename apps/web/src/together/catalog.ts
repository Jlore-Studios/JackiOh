// The card names a board needs when no engine runs it (a watched game, a shared replay, a Duos board):
// the catalog the client ships (`game/currentCatalog.ts`, §9.4, #1378), as on the match screen. No
// request is made for it, so none can fail.

import type { CardLookup } from "../game/catalog.ts";
import { currentCatalogLookup } from "../game/currentCatalog.ts";

export function useCatalogLookup(): CardLookup {
  return currentCatalogLookup();
}
