// Cosmetics wearer binding (R1295000, cards/cosmetics.ts; CLAUDE.md rule 7).
// Viewer-controlled cards wear device card cosmetics; R1655000 card-specific picks beat account defaults.
// R1297000: deck cosmetics (playmat, backdrop, card back).
// R1834004, R1834002: opponent frozen loadouts on online boards (keyed on card owner, not controller, R1295000).

import { createContext, createElement, useContext, useMemo, type ReactElement, type ReactNode } from "react";

import type { PlayerId } from "@jackioh/shared";

import {
  resolveCosmetic,
  useCosmetics,
  useOwnsAccountCosmetic,
  wornDeckCosmetics,
  type CardCosmetics,
  type CosmeticId,
  type DeckCosmetics,
  type DeckSlotKind,
} from "../cards/cosmetics.ts";
import type { SeatLoadout, SeatLoadouts } from "../cards/loadouts.ts";
import { useCardSettings } from "../cards/settings.ts";

/** Everything the board says about who wears what (the props of `WearerProvider`). */
export type Wear = {
  /** The seat whose own cards wear the device's picks, or null where no card does. */
  seat: PlayerId | null;
  /** R1297000: the picks of the deck the wearer plays, or null where it carries none. */
  deck: DeckCosmetics | null;
  /** R1834004: both seats' frozen loadouts on an online board, or null. */
  loadouts: SeatLoadouts | null;
  /** R1834004: the seat the board is seen from (the view's viewer), which keeps its loadout with the setting off. */
  viewer: PlayerId | null;
};

const NO_WEAR: Wear = { seat: null, deck: null, loadouts: null, viewer: null };

const WearContext = createContext<Wear>(NO_WEAR);

export function WearerProvider({
  seat,
  deck = null,
  loadouts = null,
  viewer = null,
  children,
}: {
  seat: PlayerId | null;
  /** R1297000: the picks of the deck the viewer plays, or null where it carries none. */
  deck?: DeckCosmetics | null;
  /** R1834004: both seats' frozen loadouts, on an online board; absent elsewhere. */
  loadouts?: SeatLoadouts | null;
  /** R1834004: the view's viewer, the seat the board is seen from. */
  viewer?: PlayerId | null;
  children?: ReactNode;
}): ReactElement {
  const value = useMemo(() => ({ seat, deck, loadouts, viewer }), [seat, deck, loadouts, viewer]);
  return createElement(WearContext.Provider, { value }, children);
}

/** The seat whose own cards wear the device's cosmetics, or null when no card does. */
export function useWearerSeat(): PlayerId | null {
  return useContext(WearContext).seat;
}

/** R1834004: loadout for owner's cards (respects showOpponents setting). */
export function theirLoadout(owner: PlayerId | null | undefined, wear: Wear, showOpponents: boolean): SeatLoadout | null {
  if (owner === null || owner === undefined || wear.loadouts === null || owner === wear.seat) return null;
  if (!showOpponents && owner !== (wear.seat ?? wear.viewer)) return null;
  return wear.loadouts[owner];
}

/** R1834004: `theirLoadout` on this board, under the viewer's own setting. Its hooks run on every call. */
export function useTheirLoadout(owner: PlayerId | null | undefined): SeatLoadout | null {
  const wear = useContext(WearContext);
  return theirLoadout(owner, wear, useCardSettings().opponentCosmetics);
}

/**
 * R1297000, R1834004: resolved deck slots for seat; wearer uses deck/account picks (R1833000),
 * others use verified server loadouts (R1834001) or defaults.
 */
export function seatDeckCosmetics(
  seat: PlayerId | null,
  wear: Wear,
  account: CardCosmetics,
  showOpponents: boolean,
  owns: (cosmeticId: string) => boolean = () => true,
): Record<DeckSlotKind, CosmeticId> {
  const own = seat !== null && seat === wear.seat;
  const loadout = own ? null : theirLoadout(seat, wear, showOpponents);
  if (loadout === null) return wornDeckCosmetics(own, wear.deck, account, owns);
  return {
    table: resolveCosmetic("table", loadout.seat.table),
    field: resolveCosmetic("field", loadout.seat.field),
    cardBack: resolveCosmetic("cardBack", loadout.seat.cardBack),
  };
}

/** R1834004: `seatDeckCosmetics` on this board, under the viewer's own setting and ownership. Its hooks run on every call. */
export function useSeatDeckCosmetics(seat: PlayerId | null): Record<DeckSlotKind, CosmeticId> {
  const wear = useContext(WearContext);
  const account = useCosmetics();
  const owns = useOwnsAccountCosmetic();
  return seatDeckCosmetics(seat, wear, account, useCardSettings().opponentCosmetics, owns);
}
