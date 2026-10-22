// R1295000: whose cards wear the device's card cosmetics on this board (cards/cosmetics.ts). A
// board the viewer plays (`wearCosmetics`: a match, a practice game, a hotseat game, a co-op seat's
// own duel) names the viewer, and only that seat's own cards wear them; a replay, a watched game, a
// teammate's duel and a bare board name nobody, and every card there draws the defaults. Drawn
// state only (CLAUDE.md rule 7): nothing here reaches the engine or the view. R1655000: what that
// own card wears is the picks of its own card first (cards/cosmetics.ts), where the viewer owns them.
//
// R1297000: the same seat wears the deck slots (the playmat, the backdrop and the card back), and
// the provider also carries the picks of the deck the viewer plays, which beat the account's.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { PlayerId } from "@jackioh/shared";

import { useCosmetics, wornDeckCosmetics, type CosmeticId, type DeckCosmetics, type DeckSlotKind } from "../cards/cosmetics.ts";

const WearerContext = createContext<PlayerId | null>(null);
const DeckCosmeticsContext = createContext<DeckCosmetics | null>(null);

export function WearerProvider({
  seat,
  deck = null,
  children,
}: {
  seat: PlayerId | null;
  /** R1297000: the picks of the deck the viewer plays, or null where it carries none. */
  deck?: DeckCosmetics | null;
  children?: ReactNode;
}): ReactElement {
  return createElement(
    WearerContext.Provider,
    { value: seat },
    createElement(DeckCosmeticsContext.Provider, { value: deck }, children),
  );
}

/** The seat whose own cards wear the device's cosmetics, or null when no card does. */
export function useWearerSeat(): PlayerId | null {
  return useContext(WearerContext);
}

/**
 * R1297000: the deck slots a side wears: the deck's picks, else the account's, else the defaults
 * for `own` (the viewer's own side) on a board that has a wearer, and the defaults for every other
 * side. Its hooks run on every call.
 */
export function useWornDeckCosmetics(own: boolean): Record<DeckSlotKind, CosmeticId> {
  const seat = useContext(WearerContext);
  const deck = useContext(DeckCosmeticsContext);
  return wornDeckCosmetics(own && seat !== null, deck, useCosmetics());
}
