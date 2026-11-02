// Where a reference finds the card it names, and whether it may be opened (SPEC §10.10, R279): the
// printed face from the public catalog (§5.1), a `CardDefsProvider`'s, else the board's
// `CatalogContext`; with neither, the name is plain text, nothing guessed (CLAUDE.md rule 7). Only
// `RefsInteractive` makes it a control; inside a button or a hover preview it only marks the name.

import { createContext, useContext, useMemo, type ReactElement, type ReactNode } from "react";

import type { CardDef, CardDefs } from "@jackioh/shared";

import { CatalogContext } from "../game/catalog.ts";
import { glitchDef, isGlitch } from "./glitch.ts";

export type DefResolver = (id: string) => CardDef | undefined;

const DefsContext = createContext<DefResolver | null>(null);
const InteractiveContext = createContext(false);

/** Makes a catalog's definitions the ones the references below it show. */
export function CardDefsProvider({ defs, children }: { defs: CardDefs; children: ReactNode }): ReactElement {
  // Glitch's words are corrupted wherever a reference or a motif reads them (glitch.ts).
  const resolve = useMemo<DefResolver>(
    () => (id) => {
      const def = defs[id];
      return def !== undefined && isGlitch(id) ? glitchDef(def) : def;
    },
    [defs],
  );
  return <DefsContext.Provider value={resolve}>{children}</DefsContext.Provider>;
}

/** The references below it are controls: focusable, and each opens the card it names (R279). */
export function RefsInteractive({ enabled = true, children }: { enabled?: boolean; children: ReactNode }): ReactElement {
  return <InteractiveContext.Provider value={enabled}>{children}</InteractiveContext.Provider>;
}

export function useDefResolver(): DefResolver | null {
  const direct = useContext(DefsContext);
  const lookup = useContext(CatalogContext);
  return useMemo<DefResolver | null>(() => {
    if (direct !== null) return direct;
    if (lookup === null) return null;
    return (id) => lookup(id, false)?.def;
  }, [direct, lookup]);
}

export function useRefsInteractive(): boolean {
  return useContext(InteractiveContext);
}
