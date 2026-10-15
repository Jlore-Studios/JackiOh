// The card names a board needs when no engine runs it: a watched game and a shared replay read
// `GET /api/catalog` once, as the match screen does (`routes/match.tsx`). Public, versioned data with
// no game state in it, so a failure only leaves the board showing def ids.

import { useEffect, useMemo, useState } from "react";

import type { CardDefs } from "@jackioh/shared";

import { lookupFromDefs, type CardLookup } from "../game/catalog.ts";
import { getCatalog } from "../net/api.ts";

export function useCatalogLookup(): CardLookup | null {
  const [defs, setDefs] = useState<CardDefs | null>(null);
  useEffect(() => {
    let cancelled = false;
    getCatalog().then(
      (response) => {
        if (!cancelled) setDefs(response.defs);
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
    };
  }, []);
  return useMemo(() => (defs === null ? null : lookupFromDefs(defs)), [defs]);
}
