// R1295000: whose cards wear the device's card cosmetics on this board (cards/cosmetics.ts). A
// board the viewer plays (`wearCosmetics`: a match, a practice game, a hotseat game, a co-op seat's
// own duel) names the viewer, and only that seat's own cards wear them; a replay, a watched game, a
// teammate's duel and a bare board name nobody. Drawn state only (CLAUDE.md rule 7): nothing here
// reaches the engine or the view. R1655000: what that own card wears is the picks of its own card
// first (cards/cosmetics.ts), where the viewer owns them.
//
// R1297000: the same seat wears the deck slots (the playmat, the backdrop and the card back), and
// the provider also carries the picks of the deck the viewer plays, which beat the account's.
//
// R1834004: an online board also carries both seats' frozen loadouts (cards/loadouts.ts, R1834002).
// Every seat but the wearer wears its own: its cards (below a `TheirCopy`, keyed on each card's
// owner, never its controller, so a stolen unit keeps its owner's look, R1295000) and its deck
// slots; on a watched or replayed game, where nobody wears the device's picks, both seats do. With
// the viewer's "Show opponents' cosmetics" off, only the seat the board is seen from (the wearer, or
// else the view's viewer) keeps its look and every other draws the defaults. Practice, hotseat and
// Duos carry no loadouts, so there every other seat draws the defaults, as before.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { PlayerId } from "@jackioh/shared";

import {
  resolveCosmetic,
  useCosmetics,
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
  return createElement(WearContext.Provider, { value: { seat, deck, loadouts, viewer } }, children);
}

/** The seat whose own cards wear the device's cosmetics, or null when no card does. */
export function useWearerSeat(): PlayerId | null {
  return useContext(WearContext).seat;
}

/**
 * R1834004: the loadout a card owned by `owner` wears: none for the wearer's own (they wear the
 * device's picks), none without loadouts or an owner, and none for a seat other than the one the
 * board is seen from while `showOpponents` (the viewer's setting) is off.
 */
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
 * R1297000, R1834004: the deck slots `seat`'s half wears: the wearer's the deck's picks, else the
 * account's, else the defaults; every other seat its loadout's (`theirLoadout`), else the defaults.
 * A null seat (a back that names none) wears the defaults.
 */
export function seatDeckCosmetics(
  seat: PlayerId | null,
  wear: Wear,
  account: CardCosmetics,
  showOpponents: boolean,
): Record<DeckSlotKind, CosmeticId> {
  const own = seat !== null && seat === wear.seat;
  const loadout = own ? null : theirLoadout(seat, wear, showOpponents);
  if (loadout === null) return wornDeckCosmetics(own, wear.deck, account);
  return {
    table: resolveCosmetic("table", loadout.seat.table),
    field: resolveCosmetic("field", loadout.seat.field),
    cardBack: resolveCosmetic("cardBack", loadout.seat.cardBack),
  };
}

/** R1834004: `seatDeckCosmetics` on this board, under the viewer's own setting. Its hooks run on every call. */
export function useSeatDeckCosmetics(seat: PlayerId | null): Record<DeckSlotKind, CosmeticId> {
  const wear = useContext(WearContext);
  const account = useCosmetics();
  return seatDeckCosmetics(seat, wear, account, useCardSettings().opponentCosmetics);
}
