// Card names and rules text for the board.
//
// `CardView` carries no name, type, tags or rules text (SPEC §10.8), which BUILD M5-T1 requires on a
// card's face. The catalog is public (§5.1; §9.4 checks `catalogVersion` on both sides), so the client
// holds it; with none loaded every component shows the `defId`.
//
// A match makes cards no catalog holds (R77, R102, R179), and the view carries their definitions
// (`PlayerView.defs`, R243). `MatchCardsContext` holds them, with the power each Heroic Power on the
// field rolled (`HeroView.powers`), so a `Card` that holds only its `CardView` can still read what it
// is: `useCardInfo` looks in the catalog first and in the match's definitions next.

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
  /** The §8 text for the face being shown (base or radiant). */
  text: string;
  tags: readonly Tag[];
  /** Printed attack and health for the face being shown; spells have neither. */
  attack?: number;
  health?: number;
  /** The whole catalog def, for the card faces (apps/web/src/cards, faceModel). */
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

/** What a match holds beyond the catalog, as one view says it (R243). */
export type MatchCards = {
  /** `PlayerView.defs`: the match-made definitions the view names. */
  defs: CardDefs;
  /** Each Heroic Power on the field by instance id: the power it rolled and its X (`HeroView.powers`). */
  powers: ReadonlyMap<string, RolledPower>;
};

const NO_MATCH_CARDS: MatchCards = { defs: {}, powers: new Map() };

export const MatchCardsContext = createContext<MatchCards>(NO_MATCH_CARDS);

/** The match's own cards as `view` names them: its definitions, and both seats' field powers. */
export function matchCardsOf(view: PlayerView): MatchCards {
  const powers = new Map<string, RolledPower>();
  for (const seat of [view.you, view.opponent]) {
    for (const power of seat.hero.powers ?? []) powers.set(power.instanceId, { name: power.name });
  }
  return { defs: view.defs ?? {}, powers };
}

/** Puts `view`'s match cards in context for everything under it. */
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

/** The power a Heroic Power on the field rolled, when the view names one for this instance. */
export function useFieldPower(instanceId: string | undefined): RolledPower | undefined {
  const match = useContext(MatchCardsContext);
  return instanceId === undefined ? undefined : match.powers.get(instanceId);
}
