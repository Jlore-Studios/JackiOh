// R1295000: whose cards wear the device's card cosmetics on this board (cards/cosmetics.ts). A
// board the viewer plays (`wearCosmetics`) names the viewer, and only that seat's own cards wear
// them; a replay, a watched game, a teammate's duel and a bare board name nobody. Drawn state only
// (CLAUDE.md rule 7): nothing here reaches the engine or the view. R1655000: that own card wears the
// picks of its own card first, where the viewer owns them.
// R1297000: the same seat wears the deck slots (the playmat, the backdrop and the card back), and
// the provider also carries the picks of the deck the viewer plays, which beat the account's.
// R1834004: an online board also carries both seats' frozen loadouts (cards/loadouts.ts, R1834002).
// Every seat but the wearer wears its own (keyed on each card's owner, never its controller, so a
// stolen unit keeps its owner's look); on a watched or replayed game both seats do. Practice,
// hotseat and Duos carry no loadouts, so there every other seat draws the defaults.

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

export type Wear = {
  /** The seat whose own cards wear the device's picks, or null where no card does. */
  seat: PlayerId | null;
  deck: DeckCosmetics | null;
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
  deck?: DeckCosmetics | null;
  loadouts?: SeatLoadouts | null;
  viewer?: PlayerId | null;
  children?: ReactNode;
}): ReactElement {
  const value = useMemo(() => ({ seat, deck, loadouts, viewer }), [seat, deck, loadouts, viewer]);
  return createElement(WearContext.Provider, { value }, children);
}

export function useWearerSeat(): PlayerId | null {
  return useContext(WearContext).seat;
}

/**
 * The loadout a card owned by `owner` wears: none for the wearer's own (they wear the device's
 * picks), and none for a seat other than the one the board is seen from while `showOpponents` is off.
 */
export function theirLoadout(owner: PlayerId | null | undefined, wear: Wear, showOpponents: boolean): SeatLoadout | null {
  if (owner === null || owner === undefined || wear.loadouts === null || owner === wear.seat) return null;
  if (!showOpponents && owner !== (wear.seat ?? wear.viewer)) return null;
  return wear.loadouts[owner];
}

/** `theirLoadout` under the viewer's own setting. Its hooks run on every call. */
export function useTheirLoadout(owner: PlayerId | null | undefined): SeatLoadout | null {
  const wear = useContext(WearContext);
  return theirLoadout(owner, wear, useCardSettings().opponentCosmetics);
}

/**
 * The deck slots `seat`'s half wears: the wearer's the deck's picks, else the account's, else the
 * defaults, each one `owns` refuses (one the viewer does not own, R1833000) the default; every other
 * seat its loadout's, which the server judged (R1834001), else the defaults. A null seat wears the defaults.
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

/** `seatDeckCosmetics` under the viewer's own setting and ownership. Its hooks run on every call. */
export function useSeatDeckCosmetics(seat: PlayerId | null): Record<DeckSlotKind, CosmeticId> {
  const wear = useContext(WearContext);
  const account = useCosmetics();
  const owns = useOwnsAccountCosmetic();
  return seatDeckCosmetics(seat, wear, account, useCardSettings().opponentCosmetics, owns);
}
