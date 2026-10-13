// `CardView` omits names, types, tags and text (SPEC §10.8; BUILD M5-T1); the public catalog (§5.1,
// §9.4 `catalogVersion`) is the client's fallback.
// Match-made cards (R77, R102, R179) and rolled Heroic Powers come from `PlayerView.defs` and
// `HeroView.powers` (R243), after the catalog in `useCardInfo`.

import { createContext, createElement, useContext, useMemo, type ReactElement, type ReactNode } from "react";

import {
  fillParams,
  type CardDef,
  type CardDefs,
  type CardType,
  type CardView,
  type PlayerView,
  type Rarity,
  type Tag,
} from "@jackioh/shared";

import { glitchInfo, isGlitch } from "../cards/glitch.ts";
import type { RolledPower } from "../cards/index.ts";

export type CardInfo = {
  name: string;
  type: CardType;
  /** §8 text for the face being shown. */
  text: string;
  tags: readonly Tag[];
  /** Spells have neither printed attack nor health. */
  attack?: number;
  health?: number;
  def?: CardDef;
  /** The catalog rarity (public, §5.1); the effects layer's Legendary and Mythic entrances read it. */
  rarity?: Rarity;
};

export type CardLookup = (defId: string, radiant: boolean) => CardInfo | undefined;

export const CatalogContext = createContext<CardLookup | null>(null);

export function lookupFromDefs(defs: CardDefs): CardLookup {
  return (defId, radiant) => {
    const def = defs[defId];
    if (def === undefined) return undefined;
    const face = radiant ? def.radiant : def.base;
    const info: CardInfo = {
      name: def.name,
      // B2.7: a face with its own type is that type (Classic+ #22's Radiant Field Trap).
      type: face.type ?? def.type,
      // B3.4: the face's `{key}` numbers filled in with its printed values; a raw placeholder never shows.
      text: fillParams(def, radiant ? "radiant" : "base"),
      tags: def.tags,
      attack: face.attack,
      health: face.health,
      def,
      rarity: def.rarity,
    };
    // Glitch's name and text are corrupted wherever a lookup answers for it (cards/glitch.ts).
    return isGlitch(defId) ? glitchInfo(info) : info;
  };
}

/** What to show when no catalog is loaded: the def id, never a guess at a name. */
export function unknownCard(defId: string): CardInfo {
  return { name: defId, type: "Unit", text: "", tags: [] };
}

/** Match-made definitions and rolled powers (R243). */
export type MatchCards = {
  defs: CardDefs;
  powers: ReadonlyMap<string, RolledPower>;
};

const NO_MATCH_CARDS: MatchCards = { defs: {}, powers: new Map() };

export const MatchCardsContext = createContext<MatchCards>(NO_MATCH_CARDS);

export function matchCardsOf(view: PlayerView): MatchCards {
  const powers = new Map<string, RolledPower>();
  for (const seat of [view.you, view.opponent]) {
    for (const power of seat.hero.powers ?? []) powers.set(power.instanceId, { name: power.name });
  }
  return { defs: view.defs ?? {}, powers };
}

export function MatchCardsProvider({ view, children }: { view: PlayerView; children?: ReactNode }): ReactElement {
  const value = useMemo(() => matchCardsOf(view), [view]);
  return createElement(MatchCardsContext.Provider, { value }, children);
}

/** A lookup that reads the catalog first and the match's own definitions next (R243). */
export function withMatchDefs(lookup: CardLookup | null, defs: CardDefs | undefined): CardLookup | null {
  if (defs === undefined || Object.keys(defs).length === 0) return lookup;
  const fromMatch = lookupFromDefs(defs);
  return (defId, radiant) => lookup?.(defId, radiant) ?? fromMatch(defId, radiant);
}

export function useCardInfo(defId: string, radiant: boolean): CardInfo {
  const lookup = useContext(CatalogContext);
  const match = useContext(MatchCardsContext);
  return lookup?.(defId, radiant) ?? lookupFromDefs(match.defs)(defId, radiant) ?? unknownCard(defId);
}

/** B5 E14: the definition of the Spell a copier has the text of (`CardView.copies`), when it copies one. */
export function useCopiedDef(card: CardView | null | undefined): CardDef | undefined {
  const copies = card?.copies;
  const info = useCardInfo(copies?.defId ?? "", copies?.radiant ?? false);
  return copies === undefined ? undefined : info.def;
}

export function useFieldPower(instanceId: string | undefined): RolledPower | undefined {
  const match = useContext(MatchCardsContext);
  return instanceId === undefined ? undefined : match.powers.get(instanceId);
}
