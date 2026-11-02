// R1834002: the loadout each seat of an online match wears, frozen by the server when the match began:
// its seat's six picks (its deck's slots over its account's; foil, finish and border are their kinds'
// defaults, R1953001) and the picks of each of its cards the receiver has been shown. R1834003: the seats'
// picks come in a `loadouts` frame on every join and reconnect, before the view, the cards' in a
// `cardLoadouts` frame after it, once per socket; a replay's page carries both (`net/api.ts`). Read as
// untrusted input: an unknown id, or a kind a card's entry lacks, draws as its kind's default.
// R1834004: the server checked every pick against the seat's own account (R1834001), so nothing here
// asks whether the viewer owns it; game/wearer.ts decides which copies wear one. Drawn state only
// (CLAUDE.md rule 7): no loadout reaches the engine, an action or the view.

import { createElement, type ReactElement, type ReactNode } from "react";

import type { PlayerId } from "@jackioh/shared";

import { COSMETIC_KINDS } from "../wire/generated/cosmetics.ts";
import { isRecord } from "../wire/json.ts";
import {
  CARD_SLOT_KINDS,
  DEFAULT_CARD_COSMETICS,
  TheirCopyContext,
  resolveCosmetic,
  type CardCosmetics,
  type CardPicks,
  type CosmeticKind,
} from "./cosmetics.ts";

/** R1834002: one seat's loadout: its six picks, and the picks of each card of its the server has sent, by card id. */
export type SeatLoadout = { seat: CardCosmetics; cards: Readonly<Record<string, CardPicks>> };

/** R1834002: both seats' loadouts, by engine seat. */
export type SeatLoadouts = Readonly<Record<PlayerId, SeatLoadout>>;

/** R1834003: one `cardLoadouts` payload: the picks of cards owned by `seat`, by card id. */
export type CardLoadouts = { seat: PlayerId; cards: Readonly<Record<string, CardPicks>> };

/** Every kind's default, for a seat a replay from before R1834002 names nothing of. */
const DEFAULT_SEAT: CardCosmetics = Object.freeze({ ...DEFAULT_CARD_COSMETICS });

const isSeat = (value: unknown): value is PlayerId => value === "p1" || value === "p2";

/** One seat's six picks, each a registry id of its kind (an unknown one its default), or null when a key is missing or not a text. */
function seatPicksOf(raw: unknown): CardCosmetics | null {
  if (!isRecord(raw)) return null;
  const out: Partial<Record<CosmeticKind, string>> = {};
  for (const kind of COSMETIC_KINDS) {
    const id = raw[kind];
    if (typeof id !== "string") return null;
    out[kind] = resolveCosmetic(kind, id);
  }
  return out as CardCosmetics;
}

/** R1834003: a `loadouts` frame's two seats, or null when either is malformed (the board must not crash on it). */
export function parseSeatPicks(raw: unknown): Record<PlayerId, CardCosmetics> | null {
  if (!isRecord(raw)) return null;
  const p1 = seatPicksOf(raw.p1);
  const p2 = seatPicksOf(raw.p2);
  return p1 === null || p2 === null ? null : { p1, p2 };
}

/**
 * One card entry's picks: each kind it names resolved (its word is final), and full art only when it
 * says `true` (its Epic) or names a full art by its id (R2655000), which fullArt.ts reads only when it
 * is that card's.
 */
function cardPicksOf(raw: Record<string, unknown>): CardPicks {
  const out: CardPicks = {};
  for (const kind of CARD_SLOT_KINDS) {
    const id = raw[kind];
    if (typeof id === "string") out[kind] = resolveCosmetic(kind, id);
  }
  if (raw.fullArt === true || (typeof raw.fullArt === "string" && raw.fullArt !== "")) out.fullArt = raw.fullArt;
  return out;
}

/** R1834003: a `cardLoadouts` payload, or null when its seat or its list is malformed; an entry with no card id is skipped. */
export function parseCardLoadouts(raw: unknown): CardLoadouts | null {
  if (!isRecord(raw) || !isSeat(raw.seat) || !Array.isArray(raw.cards)) return null;
  const cards: Record<string, CardPicks> = {};
  for (const entry of raw.cards as unknown[]) {
    if (isRecord(entry) && typeof entry.cardId === "string") cards[entry.cardId] = cardPicksOf(entry);
  }
  return { seat: raw.seat, cards };
}

/**
 * R1834003: what one socket, or one replay's pages, have sent so far: the seats' picks, the newest
 * standing, and every card's picks merged by seat and card id, a card sent again replacing its entry. A
 * `cardLoadouts` before any `loadouts` is kept for it. Each verb answers what `current()` then holds,
 * null until the seats' picks have come.
 */
export type LoadoutsHolder = {
  seats(picks: Record<PlayerId, CardCosmetics>): SeatLoadouts;
  cards(update: CardLoadouts): SeatLoadouts | null;
  /**
   * R1834002, R1834003: a replay page's two fields, read leniently: its seats' picks (every kind's
   * default where the page names none, as a match from before R1834002's does) and its cards'.
   */
  page(page: { loadouts?: unknown; cardLoadouts?: unknown }): SeatLoadouts;
  current(): SeatLoadouts | null;
};

export function createLoadoutsHolder(): LoadoutsHolder {
  let seats: Record<PlayerId, CardCosmetics> | null = null;
  const cards: Record<PlayerId, Readonly<Record<string, CardPicks>>> = { p1: {}, p2: {} };
  let held: SeatLoadouts | null = null;
  const build = (picks: Record<PlayerId, CardCosmetics>): SeatLoadouts => {
    seats = picks;
    held = { p1: { seat: picks.p1, cards: cards.p1 }, p2: { seat: picks.p2, cards: cards.p2 } };
    return held;
  };
  const merge = (update: CardLoadouts): void => {
    cards[update.seat] = { ...cards[update.seat], ...update.cards };
  };
  return {
    seats: build,
    cards(update) {
      merge(update);
      return seats === null ? null : build(seats);
    },
    page(page) {
      if (Array.isArray(page.cardLoadouts)) {
        for (const raw of page.cardLoadouts as unknown[]) {
          const update = parseCardLoadouts(raw);
          if (update !== null) merge(update);
        }
      }
      return build(parseSeatPicks(page.loadouts) ?? { p1: DEFAULT_SEAT, p2: DEFAULT_SEAT });
    },
    current: () => held,
  };
}

/** R1834004: the faces below it wear `loadout` where they are not the viewer's own; none with null. */
export function TheirCopy({ loadout, children }: { loadout: SeatLoadout | null; children?: ReactNode }): ReactElement {
  return createElement(TheirCopyContext.Provider, { value: loadout }, children);
}
