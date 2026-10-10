// Where a reference finds the card it names, and whether it may be opened (SPEC §10.10, R279).
// A reference names a card by id (`CardDef.refs`) and shows that card's printed face from the public
// catalog (§5.1): a `CardDefsProvider`'s, else the board's `CatalogContext`. With neither, the name
// is plain text: nothing is guessed (CLAUDE.md rule 7).
// Whether a reference is a control is the surface's call: inside a button, a hover preview or a small
// board face it only marks the name; `RefsInteractive` makes it focusable, with a tooltip.

import { createContext, useContext, useMemo, type ReactElement, type ReactNode } from "react";

import type { CardDef, CardDefs } from "@jackioh/shared";

import { CatalogContext } from "../game/catalog.ts";
import { glitchDef, isGlitch } from "./glitch.ts";

/** The printed definition of a catalog card, by id, or undefined when this screen has none. */
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

/** The closest catalog: a `CardDefsProvider`'s, else the board's `CatalogContext`, else none. */
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
