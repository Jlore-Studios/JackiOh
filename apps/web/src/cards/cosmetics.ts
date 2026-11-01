// R1295000: the card cosmetics a player's own cards wear — a foil over the art, a finish on the
// print and a border round the frame — whose account-wide picks are kept per browser under
// `COSMETICS_SETTINGS_KEY` and on the account as the `cosmetics` group (R633, `settings/groups.ts`).
// R1953001: nothing sets those three any more and no face wears them; the group keeps its shape
// (R1655000), and a card wears its own picks (below) or its kind's default. The ids are the
// registry's (`crates/server/cosmetics.json`, generated into `wire/generated/cosmetics.ts`). The
// server refuses a pick of the wrong kind or one the account does not own (R1834001).
//
// R1297000: the same store holds three deck slots beside them: the playmat (`table`), the ambient
// backdrop (`field`) and the back of the viewer's face-down cards and deck (`cardBack`). Each saved
// deck may carry its own picks (`DeckCosmetics`); a viewer's side wears the deck's, else the
// account's, else the kind's default (`wornDeckCosmetics`), and an id this client does not know
// draws as the default.
//
// R1833000: the account's playmats, backdrops and card backs are no longer all starters, so a pick
// of one is worn only where the viewer owns it (`ownsAccountCosmetic`: a default or a starter counts,
// and so does a row of `GET /api/cosmetics`' account list), or where every cosmetic counts as owned
// (practice, hotseat, R1310004's builds: no `AccountCosmeticsOwned` above it). A stored pick the
// account does not own, one synced from another device say, draws as its kind's default.
//
// The picks never reach the engine, an action, `PlayerView` or the log, and a face wears them only
// where the board says it is the viewer's own card in a game (`worn`, game/wearer.ts). R1834004: an
// opponent's side, and every side of a watched or replayed online game, wears instead the loadout
// its seat froze when the match began (R1834002, `cards/loadouts.ts`), as the server sent it, below
// a `TheirCopy`; every other face draws the defaults (CLAUDE.md rule 7). Each
// slot is its own attribute on the face, apart from the rarity foil (`data-foil`, model.ts), so a
// decorative foil never blurs the Mythic and Radiant signal: a Mythic face wears no cosmetic foil,
// and a Radiant face no foil, finish or border, whoever's loadout it wears. A default adds no
// attribute at all, so a face wearing nothing is the face it was before.
//
// R1655000: a card may also carry picks of its own (`jackioh.cosmetics.cards.v1`, never the
// `cosmetics` group above, so no client overwrites another's): a foil, a finish, a border and
// whether its full art prints. R1953001: a kind a card has no pick of wears its kind's default,
// never the account's pick above, which nothing sets any more (full art keeps the Full-art cards
// setting as its default). A card's pick is worn only where the viewer owns that
// cosmetic on that card (`ownsCosmetic`: a default or a starter counts, and so does a ledger row),
// or where every cosmetic counts as owned (practice, hotseat, R1310004's builds); the store keeps what
// it is given, the wear path and `useCanEquip` apply ownership. The face wearing them is still only
// the viewer's own copy (`worn`, or below an `OwnCopy`, fullArt.ts). R1834000: they live on the
// account now, and this store is its cache on the device: `cardPicksSync.ts` sends the device's own
// up once, then puts the account's in it and sends each later write (`subscribeCardPickWrites`).
//
// R1956000: from their first read, both stores listen for a window "storage" event on their key,
// re-read and notify, so a pick another tab makes shows on every face this tab draws at once, and on
// every face it mounts later, with no reload. R1956001: the
// viewer's own Created card (`FaceModel.createdBy`) wears the foil, finish and border of the card
// that generated it, as that card's own copy would, behind ownership on that card; its full art is
// its own (fullArt.ts), and its own rarity rules hold.
//
// An id this client does not know (one a newer client wrote to the account) is kept in the store,
// so this device never overwrites it, and drawn as its kind's default, never an error. A cosmetic
// foil moves only while one of COSMETIC_FOIL_ANIMATED_MAX slots is the face's; the others wear its
// still version, as every face does under reduced motion (cosmetics.css). R1833000: so does a moving
// finish or border (`MOVING_FINISHES_AND_BORDERS`), which shares the foil's slot. R1953001: every
// Legendary and Mythic one moves (`isChaseCosmetic`). R2715001: so does a full-art card's moving art
// (`useArtMotion`), which also holds still under the Reduce motion setting and the system's reduced
// motion, since its keyframes live inside the art's picture, where the page's motion gate cannot reach.

import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useState,
  useSyncExternalStore,
  type ReactElement,
  type ReactNode,
} from "react";

import { COSMETIC_ID_MAX_LENGTH, COSMETIC_KINDS, COSMETICS } from "../wire/generated/cosmetics.ts";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { useSetting } from "../settings/store.ts";
import type { SeatLoadout } from "./loadouts.ts";
import { foilFor, type FaceModel } from "./model.ts";
import { getStored, localStorageOrNull, setStored } from "../net/storage.ts";

export type CosmeticKind = (typeof COSMETIC_KINDS)[number];
/** R1297000: the kinds a card wears, and the kinds a deck carries (the registry's order). */
export const CARD_SLOT_KINDS = ["foil", "finish", "border"] as const satisfies readonly CosmeticKind[];
export const DECK_SLOT_KINDS = ["table", "field", "cardBack"] as const satisfies readonly CosmeticKind[];
export type CardSlotKind = (typeof CARD_SLOT_KINDS)[number];
export type DeckSlotKind = (typeof DECK_SLOT_KINDS)[number];
export type Cosmetic = (typeof COSMETICS)[number];
export type CosmeticId = Cosmetic["id"];
/** A pick per kind. A stored pick is any short text, an id this client may not know among them. */
export type CardCosmetics = Record<CosmeticKind, string>;

export const COSMETICS_SETTINGS_KEY = "jackioh.cosmetics.v1";

/** The kind's entries, in the registry's order: its default first. */
export function cosmeticsOf(kind: CosmeticKind): readonly Cosmetic[] {
  return COSMETICS.filter((cosmetic) => cosmetic.kind === kind);
}

function defaultOf(kind: CosmeticKind): CosmeticId {
  const found = COSMETICS.find((cosmetic) => cosmetic.kind === kind && cosmetic.default);
  // The registry's check holds one default per kind (crates/server/src/cosmetics.rs).
  if (found === undefined) throw new Error(`the cosmetics registry has no default ${kind}`);
  return found.id;
}

/** Every kind's default: the three card slots and the three deck slots (R1297000). */
export const DEFAULT_CARD_COSMETICS: Readonly<Record<CosmeticKind, CosmeticId>> = Object.freeze({
  foil: defaultOf("foil"),
  finish: defaultOf("finish"),
  border: defaultOf("border"),
  table: defaultOf("table"),
  field: defaultOf("field"),
  cardBack: defaultOf("cardBack"),
});

/** `id` when it is one of `kind`'s, and the kind's default otherwise: never an error. */
export function resolveCosmetic(kind: CosmeticKind, id: unknown): CosmeticId {
  const found = COSMETICS.find((cosmetic) => cosmetic.kind === kind && cosmetic.id === id);
  return found === undefined ? DEFAULT_CARD_COSMETICS[kind] : found.id;
}

/** Kind by kind: a short text is kept, known or not; anything else is the kind's default. */
export function normalizeCosmetics(raw: unknown): CardCosmetics {
  const record = raw !== null && typeof raw === "object" && !Array.isArray(raw) ? (raw as Record<string, unknown>) : {};
  const out: CardCosmetics = { ...DEFAULT_CARD_COSMETICS };
  for (const kind of COSMETIC_KINDS) {
    const value = record[kind];
    if (typeof value === "string" && value !== "" && value.length <= COSMETIC_ID_MAX_LENGTH) out[kind] = value;
  }
  return out;
}

/** Absent, throwing or unparsable storage gives the defaults. */
function load(): CardCosmetics {
  try {
    const text = getStored(COSMETICS_SETTINGS_KEY);
    return text === null ? { ...DEFAULT_CARD_COSMETICS } : normalizeCosmetics(JSON.parse(text));
  } catch {
    return { ...DEFAULT_CARD_COSMETICS };
  }
}

function persist(picks: CardCosmetics): void {
  // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  setStored(COSMETICS_SETTINGS_KEY, JSON.stringify(picks));
}

let current: CardCosmetics | null = null;
const listeners = new Set<() => void>();
let storageListening = false;

/** The in-memory picks, loaded from storage once, on first read, and kept as storage holds them since. */
export function readCosmetics(): CardCosmetics {
  listenToStorage();
  current ??= load();
  return current;
}

/** Merges `patch` over the picks, normalizes, persists in try/catch and notifies synchronously. */
export function writeCosmetics(patch: Partial<Record<CosmeticKind, unknown>>): CardCosmetics {
  const merged: Record<string, unknown> = { ...readCosmetics() };
  for (const kind of COSMETIC_KINDS) {
    if (patch[kind] !== undefined) merged[kind] = patch[kind];
  }
  const next = normalizeCosmetics(merged);
  current = next;
  persist(next);
  for (const listener of [...listeners]) listener();
  return next;
}

export function subscribeCosmetics(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useCosmetics(): CardCosmetics {
  return useSyncExternalStore(subscribeCosmetics, readCosmetics, readCosmetics);
}

// ---------------------------------------------------------------------------------------------
// R1655000: the picks of each card, on the device alone. They sit in a store of their own, never in
// the `cosmetics` group the account syncs (settings/groups.ts, R633, R634), so an old client and a
// new one never overwrite each other's picks, and no settings reset touches them (R1579000).
// ---------------------------------------------------------------------------------------------

export const CARD_PICKS_SETTINGS_KEY = "jackioh.cosmetics.cards.v1";

/**
 * What one card wears of its own. A kind it has no pick of is absent and wears its kind's default
 * (R1953001; the Full-art cards setting, for `fullArt`); a pick of a kind's default id is a pick. An
 * id this client does not know reads as its kind's default. R2655000: `fullArt` is `false`, `true`
 * (the card's Epic full art) or a full art's id, kept as written; fullArt.ts reads which tier it names.
 */
export type CardPicks = { foil?: CosmeticId; finish?: CosmeticId; border?: CosmeticId; fullArt?: boolean | string };

/** One card's picks as stored: a short text is kept for each kind, known or not (as `normalizeCosmetics` keeps one). */
export type StoredCardPicks = { foil?: string; finish?: string; border?: string; fullArt?: boolean | string };

/** R1834000: what one write changed of a card's picks, field by field: the new pick, or null where it was cleared. */
export type CardPickChange = { foil?: string | null; finish?: string | null; border?: string | null; fullArt?: boolean | string | null };

const CARD_PICK_FIELDS = [...CARD_SLOT_KINDS, "fullArt"] as const;
const NO_PICKS: CardPicks = Object.freeze({});

function isPickText(value: unknown): value is string {
  return typeof value === "string" && value !== "" && value.length <= COSMETIC_ID_MAX_LENGTH;
}

/** R2655000: what every full art's registry id starts with, as the account's store holds it to (migration 0072). */
const FULL_ART_ID_PREFIX = "full-art-";

/** R2715000: what an Animated art's registry id starts with, a `fullArt` entry too (migration 0074). */
export const ANIMATED_ART_ID_PREFIX = "animated-";

/** R2655000: a card's full-art pick as stored: a boolean, or a full art's id (R2715000: an Animated art's among them), known or not. */
function isFullArtPick(value: unknown): value is boolean | string {
  return typeof value === "boolean" || (isPickText(value) && (value.startsWith(FULL_ART_ID_PREFIX) || value.startsWith(ANIMATED_ART_ID_PREFIX)));
}

function normalizeStoredPicks(raw: unknown): StoredCardPicks {
  const record = raw !== null && typeof raw === "object" && !Array.isArray(raw) ? (raw as Record<string, unknown>) : {};
  const out: StoredCardPicks = {};
  for (const kind of CARD_SLOT_KINDS) {
    const value = record[kind];
    if (isPickText(value)) out[kind] = value;
  }
  const fullArt = record["fullArt"];
  if (isFullArtPick(fullArt)) out.fullArt = fullArt;
  return out;
}

/** What a face reads of `stored`: each id known, an unknown one the kind's default (the card's word is final, as a deck's is). */
function resolveCardPicks(stored: StoredCardPicks): CardPicks {
  const out: CardPicks = {};
  for (const kind of CARD_SLOT_KINDS) {
    const id = stored[kind];
    if (id !== undefined) out[kind] = resolveCosmetic(kind, id);
  }
  if (stored.fullArt !== undefined) out.fullArt = stored.fullArt;
  return out;
}

/** The stored picks by card id, and what each reads as: one object per card, kept while the card's picks stand. */
type CardPicksState = { stored: Map<string, StoredCardPicks>; resolved: Map<string, CardPicks> };

/** What storage holds of every card's picks, as written: absent, throwing or unparsable storage holds none. */
function readStoredCardPicks(): Record<string, unknown> {
  try {
    const text = getStored(CARD_PICKS_SETTINGS_KEY);
    const parsed: unknown = text === null ? null : JSON.parse(text);
    if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)) return parsed as Record<string, unknown>;
  } catch {
    // Unparsable or blocked storage: no card has a pick of its own.
  }
  return {};
}

function cardPicksFrom(stored: Record<string, unknown>): CardPicksState {
  const state: CardPicksState = { stored: new Map(), resolved: new Map() };
  for (const [defId, raw] of Object.entries(stored)) {
    const picks = normalizeStoredPicks(raw);
    if (Object.keys(picks).length > 0) {
      state.stored.set(defId, picks);
      state.resolved.set(defId, resolveCardPicks(picks));
    }
  }
  return state;
}

function loadCardPicks(): CardPicksState {
  return cardPicksFrom(readStoredCardPicks());
}

let cardPicksState: CardPicksState | null = null;
/** Whether a write failed, so the picks in memory are newer than storage's. */
let cardPicksUnsaved = false;
const cardPicksListeners = new Set<() => void>();
/** R1834000: told of each write `updateCardPicks` makes, never of `replaceAllCardPicks`. */
const cardPickWriteListeners = new Set<(defId: string, change: CardPickChange) => void>();

function cardPicks(): CardPicksState {
  listenToStorage();
  cardPicksState ??= loadCardPicks();
  return cardPicksState;
}

/** `defId`'s own picks, as a face reads them: none for a card that has none. The same object until its picks change. */
export function readCardPicks(defId: string): CardPicks {
  return cardPicks().resolved.get(defId) ?? NO_PICKS;
}

function subscribeCardPicks(listener: () => void): () => void {
  cardPicksListeners.add(listener);
  return () => {
    cardPicksListeners.delete(listener);
  };
}

/**
 * R1956000: another tab wrote the account's picks or the cards' (a null key is a `localStorage.clear()`
 * there): re-read what it wrote and notify, so every face here wears it at once. What this page could
 * not save is dropped for what storage now holds, which is what every other tab draws.
 */
function onCosmeticsStorage(event: StorageEvent): void {
  const key = event.key;
  if (key === null || key === COSMETICS_SETTINGS_KEY) {
    current = load();
    for (const listener of [...listeners]) listener();
  }
  if (key === null || key === CARD_PICKS_SETTINGS_KEY) {
    cardPicksState = loadCardPicks();
    cardPicksUnsaved = false;
    for (const listener of [...cardPicksListeners]) listener();
  }
}

/**
 * The first read of either store installs the one window "storage" listener, for the page's life:
 * the stores keep what storage holds while no face is mounted too, so a face mounted later draws a
 * pick another tab made meanwhile.
 */
function listenToStorage(): void {
  if (storageListening || typeof window === "undefined") return;
  window.addEventListener("storage", onCosmeticsStorage);
  storageListening = true;
}

export function useCardPicks(defId: string): CardPicks {
  return useSyncExternalStore(subscribeCardPicks, () => readCardPicks(defId), () => readCardPicks(defId));
}

/**
 * What storage holds now, so a pick another tab wrote since this page read stands: as written, and as
 * read. After a failed write the picks live in memory alone, and what storage holds would forget them.
 */
function currentCardPicks(): { written: Record<string, unknown>; state: CardPicksState } {
  if (localStorageOrNull() !== null && !cardPicksUnsaved) {
    const written = readStoredCardPicks();
    return { written, state: cardPicksFrom(written) };
  }
  return { written: Object.fromEntries(cardPicks().stored) as Record<string, unknown>, state: cardPicks() };
}

/**
 * Changes `defId`'s stored picks by `change`, persists in try/catch and notifies synchronously, unless
 * nothing changed. It starts from what storage holds now, so a pick another tab wrote since this page
 * read stands, and it rewrites only the fields this client knows of `defId`'s entry, so a newer
 * client's kinds are kept, as `normalizeCosmetics` keeps its ids.
 */
function updateCardPicks(defId: string, change: (picks: StoredCardPicks) => void): void {
  const { written, state } = currentCardPicks();
  cardPicksState = state;
  const before = state.stored.get(defId) ?? {};
  const picks: StoredCardPicks = { ...before };
  change(picks);
  const changed = CARD_PICK_FIELDS.filter((field) => before[field] !== picks[field]);
  if (changed.length === 0) return;
  if (Object.keys(picks).length === 0) {
    state.stored.delete(defId);
    state.resolved.delete(defId);
  } else {
    state.stored.set(defId, picks);
    state.resolved.set(defId, resolveCardPicks(picks));
  }
  const raw = written[defId];
  const entry: Record<string, unknown> = raw !== null && typeof raw === "object" && !Array.isArray(raw) ? { ...raw } : {};
  for (const field of CARD_PICK_FIELDS) {
    if (picks[field] === undefined) delete entry[field];
    else entry[field] = picks[field];
  }
  if (Object.keys(entry).length === 0) delete written[defId];
  else written[defId] = entry;
  // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  cardPicksUnsaved = !setStored(CARD_PICKS_SETTINGS_KEY, JSON.stringify(written));
  for (const listener of [...cardPicksListeners]) listener();
  const sent: Record<string, string | boolean | null> = {};
  for (const field of changed) sent[field] = picks[field] ?? null;
  for (const listener of [...cardPickWriteListeners]) listener(defId, sent as CardPickChange);
}

/** Sets `defId`'s `kind` pick to `id`, or clears it with null, which leaves it to the kind's default. */
export function writeCardPick(defId: string, kind: CardSlotKind, id: CosmeticId | null): void {
  updateCardPicks(defId, (picks) => {
    if (id !== null && isPickText(id)) picks[kind] = id;
    else delete picks[kind];
  });
}

/**
 * Sets `defId`'s full-art pick: `false` for none, `true` for its Epic, or a full art's id (R2655000,
 * `fullArtPickOf`); null clears the pick, which leaves it to the Full-art cards setting.
 */
export function writeCardFullArt(defId: string, pick: boolean | string | null): void {
  updateCardPicks(defId, (picks) => {
    if (isFullArtPick(pick)) picks.fullArt = pick;
    else delete picks.fullArt;
  });
}

/** R1834000: every card's stored picks by card id, as the device holds them now, an id this client does not know kept. */
export function readAllCardPicks(): Record<string, StoredCardPicks> {
  return Object.fromEntries(currentCardPicks().state.stored);
}

/**
 * R1834000: puts `picks` (card id to its picks, the account's copy) in place of every card's, persists
 * in try/catch and notifies the faces synchronously. A card whose picks did not change keeps the
 * object a face read. It tells no write listener: the account's answer is never sent back to it.
 */
export function replaceAllCardPicks(picks: Readonly<Record<string, unknown>>): void {
  const before = cardPicks();
  const next = cardPicksFrom(picks as Record<string, unknown>);
  for (const [defId, stored] of next.stored) {
    const held = before.stored.get(defId);
    const kept = before.resolved.get(defId);
    if (held !== undefined && kept !== undefined && CARD_PICK_FIELDS.every((field) => held[field] === stored[field])) {
      next.resolved.set(defId, kept);
    }
  }
  cardPicksState = next;
  // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  cardPicksUnsaved = !setStored(CARD_PICKS_SETTINGS_KEY, JSON.stringify(Object.fromEntries(next.stored)));
  for (const listener of [...cardPicksListeners]) listener();
}

/** R1834000: calls `listener` with the card and what changed after every write of a card's picks on this page. */
export function subscribeCardPickWrites(listener: (defId: string, change: CardPickChange) => void): () => void {
  cardPickWriteListeners.add(listener);
  return () => {
    cardPickWriteListeners.delete(listener);
  };
}

/** The cosmetic ids the viewer owns on each card, past the defaults and starters that every account owns on every card. */
export type OwnedByCard = ReadonlyMap<string, ReadonlySet<string>>;

/** The cosmetics the faces below a `FullArtOwned` may wear: `"every"` (practice, hotseat, R1310004's builds) or the ledger's. */
export type CardOwnership = "every" | OwnedByCard;

/** R1655000: what the faces below it own; `"every"` with none above it. Provided by `FullArtOwned` (fullArt.ts). */
export const OwnershipContext = createContext<CardOwnership>("every");

/**
 * R1834004: the loadout of the seat that owns the copy drawn below it, which its faces wear where
 * they are not the viewer's own (`worn`): null with none above it, and wherever the board says the
 * copy draws the defaults (game/wearer.ts). Provided by `TheirCopy` (loadouts.ts).
 */
export const TheirCopyContext = createContext<SeatLoadout | null>(null);

/**
 * Whether `cosmeticId` is the viewer's on the card `defId`: every one is where `owned` is `"every"`;
 * else a default or a starter is (every account owns them on every card, §14.2), and so is one the
 * ledger lists on that card. An id the registry does not know, or names another card, never is.
 */
export function ownsCosmetic(owned: CardOwnership, defId: string, cosmeticId: string): boolean {
  if (owned === "every") return true;
  const found = COSMETICS.find((cosmetic) => cosmetic.id === cosmeticId);
  if (found === undefined || ("card" in found && found.card !== defId)) return false;
  return found.default || found.starter || owned.get(defId)?.has(cosmeticId) === true;
}

/** Whether the viewer may equip `cosmeticId` on the card `defId`: true wherever every cosmetic counts as owned. */
export function useCanEquip(): (defId: string, cosmeticId: CosmeticId) => boolean {
  const owned = useContext(OwnershipContext);
  return useCallback((defId, cosmeticId) => ownsCosmetic(owned, defId, cosmeticId), [owned]);
}

/**
 * What `face` wears, each kind resolved to a known id: the defaults where `picks` is null (a face
 * that is not the viewer's own), the default foil on a face with its rarity foil (Mythic, Radiant)
 * and the default finish and border on a Radiant face, whose gold is its own. R1655000: a kind
 * the card has a pick of (`card`) that `owns` takes the card's pick; `picks` otherwise, which a face
 * passes as the kind defaults (R1953001) and a sample of a cosmetic as that cosmetic.
 */
export function wornCosmetics(
  face: FaceModel,
  picks: CardCosmetics | null,
  card: CardPicks = NO_PICKS,
  owns: (cosmeticId: string) => boolean = () => true,
): Record<CardSlotKind, CosmeticId> {
  if (picks === null) {
    return { foil: DEFAULT_CARD_COSMETICS.foil, finish: DEFAULT_CARD_COSMETICS.finish, border: DEFAULT_CARD_COSMETICS.border };
  }
  const pickOf = (kind: CardSlotKind): string => {
    const own = card[kind];
    if (own !== undefined && owns(own)) return own;
    // R1833000: what lies under the card's pick is worn only where it is owned on this card.
    return owns(picks[kind]) ? picks[kind] : DEFAULT_CARD_COSMETICS[kind];
  };
  return {
    foil: foilFor(face, true) === "none" ? resolveCosmetic("foil", pickOf("foil")) : DEFAULT_CARD_COSMETICS.foil,
    finish: face.radiant ? DEFAULT_CARD_COSMETICS.finish : resolveCosmetic("finish", pickOf("finish")),
    border: face.radiant ? DEFAULT_CARD_COSMETICS.border : resolveCosmetic("border", pickOf("border")),
  };
}

// ---------------------------------------------------------------------------------------------
// R1297000: the deck slots. A saved deck carries its own picks, each a registry id or absent; the
// board resolves them with the account's picks beneath and the kind's default under those.
// ---------------------------------------------------------------------------------------------

/** The picks a saved deck carries, as the server and the deckbuilder's mirror hold them. */
export type DeckCosmeticFields = {
  tableCosmetic?: string | null;
  fieldCosmetic?: string | null;
  cardBackCosmetic?: string | null;
};

/** A deck's picks by slot: `null` where it has none, which the account's choice fills. */
export type DeckCosmetics = Readonly<Record<DeckSlotKind, string | null>>;

/** The deck's picks by slot, or null when it has none at all. */
export function deckCosmeticsOf(fields: DeckCosmeticFields): DeckCosmetics | null {
  const picks: DeckCosmetics = {
    table: fields.tableCosmetic ?? null,
    field: fields.fieldCosmetic ?? null,
    cardBack: fields.cardBackCosmetic ?? null,
  };
  return DECK_SLOT_KINDS.every((kind) => typeof picks[kind] !== "string") ? null : picks;
}

/** The deck fields of `raw` that hold a pick: a text of 1 to COSMETIC_ID_MAX_LENGTH characters. */
export function pickedDeckCosmetics(raw: unknown): DeckCosmeticFields {
  const record = raw !== null && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const keep = (value: unknown): string | undefined =>
    typeof value === "string" && value !== "" && value.length <= COSMETIC_ID_MAX_LENGTH ? value : undefined;
  const table = keep(record["tableCosmetic"]);
  const field = keep(record["fieldCosmetic"]);
  const cardBack = keep(record["cardBackCosmetic"]);
  return {
    ...(table === undefined ? {} : { tableCosmetic: table }),
    ...(field === undefined ? {} : { fieldCosmetic: field }),
    ...(cardBack === undefined ? {} : { cardBackCosmetic: cardBack }),
  };
}

/** The deck-field patch that sets `kind`'s pick to `id`, or clears it with null. */
export function deckCosmeticPatch(kind: DeckSlotKind, id: string | null): DeckCosmeticFields {
  switch (kind) {
    case "table":
      return { tableCosmetic: id };
    case "field":
      return { fieldCosmetic: id };
    case "cardBack":
      return { cardBackCosmetic: id };
  }
}

/**
 * `kind`'s id to draw: the deck's pick when it has one (even an unknown id, which then draws as
 * the default: the deck's word is final), else the account's, else the default. R1833000: a pick
 * `owns` refuses (one the account does not own) draws as the default too.
 */
export function resolveDeckCosmetic(
  kind: DeckSlotKind,
  deck: DeckCosmetics | null | undefined,
  account: CardCosmetics,
  owns: (cosmeticId: string) => boolean = () => true,
): CosmeticId {
  const id = resolveCosmetic(kind, deck?.[kind] ?? account[kind]);
  return owns(id) ? id : DEFAULT_CARD_COSMETICS[kind];
}

/** The deck slots a side wears: the defaults when `worn` is false (a side with no loadout to wear, game/wearer.ts), and each pick `owns` refuses. */
export function wornDeckCosmetics(
  worn: boolean,
  deck: DeckCosmetics | null | undefined,
  account: CardCosmetics,
  owns: (cosmeticId: string) => boolean = () => true,
): Record<DeckSlotKind, CosmeticId> {
  if (!worn) {
    return { table: DEFAULT_CARD_COSMETICS.table, field: DEFAULT_CARD_COSMETICS.field, cardBack: DEFAULT_CARD_COSMETICS.cardBack };
  }
  return {
    table: resolveDeckCosmetic("table", deck, account, owns),
    field: resolveDeckCosmetic("field", deck, account, owns),
    cardBack: resolveDeckCosmetic("cardBack", deck, account, owns),
  };
}

// ---------------------------------------------------------------------------------------------
// R1833000: the account cosmetics the viewer owns. While ownership decides (`FULL_ART_GATED`),
// ownedFullArt.ts mounts `AccountCosmeticsOwned` over every gated screen from `GET /api/cosmetics`'
// account list; with none above it (practice, hotseat, R1310004's builds) every one counts as owned.
// ---------------------------------------------------------------------------------------------

/** What the pickers and sides below an `AccountCosmeticsOwned` own: `"every"`, or the account cosmetic ids its ledger lists. */
export type AccountOwnership = "every" | ReadonlySet<string>;

const AccountOwnershipContext = createContext<AccountOwnership>("every");

/** R1833000: the pickers and sides below it own the account cosmetics `owned` holds; `"every"` with none above it. */
export function AccountCosmeticsOwned({ owned, children }: { owned: AccountOwnership; children?: ReactNode }): ReactElement {
  return createElement(AccountOwnershipContext.Provider, { value: owned }, children);
}

/**
 * Whether the viewer owns the account cosmetic `id`: every one does where `owned` is `"every"`; else
 * a default or a starter does (every account owns them, §14.2), and so does one `owned` lists. An id
 * the registry does not know never does.
 */
export function ownsAccountCosmetic(owned: AccountOwnership, id: string): boolean {
  if (owned === "every") return true;
  const found = COSMETICS.find((cosmetic) => cosmetic.id === id);
  if (found === undefined) return false;
  return found.default || found.starter || owned.has(id);
}

/** R1833000: whether the viewer owns an account cosmetic, as the `AccountCosmeticsOwned` above it says. */
export function useOwnsAccountCosmetic(): (id: string) => boolean {
  const owned = useContext(AccountOwnershipContext);
  return useCallback((id) => ownsAccountCosmetic(owned, id), [owned]);
}

// ---------------------------------------------------------------------------------------------
// The cap on moving foils. A face that wants its foil to move claims a slot when it mounts and
// gives it back when it unmounts or stops wanting it; past COSMETIC_FOIL_ANIMATED_MAX it waits in
// line, first come first served, and moves once a slot passes to it.
// ---------------------------------------------------------------------------------------------

const moving = new Set<symbol>();
const queued: symbol[] = [];
const slotListeners = new Set<() => void>();

function notifySlots(): void {
  for (const listener of [...slotListeners]) listener();
}

function claimSlot(token: symbol): void {
  if (moving.size < COSMETIC_FOIL_ANIMATED_MAX) moving.add(token);
  else queued.push(token);
  notifySlots();
}

function releaseSlot(token: symbol): void {
  moving.delete(token);
  const waiting = queued.indexOf(token);
  if (waiting >= 0) queued.splice(waiting, 1);
  while (moving.size < COSMETIC_FOIL_ANIMATED_MAX && queued.length > 0) {
    const next = queued.shift();
    if (next !== undefined) moving.add(next);
  }
  notifySlots();
}

function subscribeSlots(listener: () => void): () => void {
  slotListeners.add(listener);
  return () => {
    slotListeners.delete(listener);
  };
}

/**
 * R1833000: the finishes and borders that move (cosmetics.css), each taking a foil's slot: the
 * Legendary and Mythic ones (R1953001: Gilded edge among them).
 */
export const MOVING_FINISHES_AND_BORDERS: ReadonlySet<string> = new Set([
  "finish-gilded-edge",
  "finish-obsidian",
  "finish-starlit",
  "border-filigree",
  "border-celestial",
]);

/** R1953001: whether `id` is a chase, a Legendary or Mythic cosmetic (the registry's mark, R1333000), every one of which moves. */
export function isChaseCosmetic(id: string): boolean {
  return COSMETICS.find((cosmetic) => cosmetic.id === id)?.chase === true;
}

/** Whether this face's foil may move now: it wants to, and holds one of the slots. */
function useFoilSlot(wanted: boolean): boolean {
  const [token] = useState(() => Symbol("cosmetic-foil"));
  useEffect(() => {
    if (!wanted) return undefined;
    claimSlot(token);
    return () => {
      releaseSlot(token);
    };
  }, [wanted, token]);
  const holds = useSyncExternalStore(
    subscribeSlots,
    () => moving.has(token),
    () => false,
  );
  return wanted && holds;
}

/**
 * R2715001: whether a face's moving art may move now: it wants to (CardFace: an Animated, Legendary or
 * Mythic face, full layout, Animated foil on, not a lazy grid face), neither the Reduce motion setting
 * nor the system asks for reduced motion, and it holds one of the foil's slots. Its hooks run on every
 * face.
 */
export function useArtMotion(wanted: boolean): boolean {
  const panel = useSetting("reduceMotion");
  const system = useOsReducedMotion();
  return useFoilSlot(wanted && !panel && !system);
}

/** The attributes a face wears for its cosmetics: none at all for the defaults. */
export type CosmeticAttributes = {
  "data-cosmetic-foil"?: CosmeticId;
  "data-cosmetic-motion"?: "animated" | "static";
  "data-finish"?: CosmeticId;
  "data-frame"?: CosmeticId;
};

/**
 * The cosmetic attributes of a face: when `worn`, the card's own picks (R1655000) where the viewer
 * owns them and its kinds' defaults under them (R1953001: never the account's picks); the defaults
 * otherwise. R1956001: a worn Created card's are its generator's (`createdBy`), owned on the
 * generator, worn by its own face's rarity rules. R1834004: a face that is not `worn` below a
 * `TheirCopy` wears that seat's picks of its card, every one owned (the server checked, R1834001),
 * and its kinds' defaults under them, as its owner sees it (R1953001: never the seat's account foil,
 * finish and border), under the same rarity rules. `animatedFoil` is the card settings' switch,
 * which stills a cosmetic foil as it does the rarity foil, and a moving finish or border with it
 * (R1833000). Its hooks run on every face, worn or not.
 */
export function useCosmeticAttributes(face: FaceModel, worn: boolean, animatedFoil: boolean): CosmeticAttributes {
  const source = face.createdBy ?? face.defId;
  const card = useCardPicks(source);
  const owned = useContext(OwnershipContext);
  const theirs = useContext(TheirCopyContext);
  const wears =
    !worn && theirs !== null
      ? wornCosmetics(face, DEFAULT_CARD_COSMETICS, theirs.cards[face.defId])
      : wornCosmetics(face, worn ? DEFAULT_CARD_COSMETICS : null, card, (id) => ownsCosmetic(owned, source, id));
  const foiled = wears.foil !== DEFAULT_CARD_COSMETICS.foil;
  const animates = foiled || MOVING_FINISHES_AND_BORDERS.has(wears.finish) || MOVING_FINISHES_AND_BORDERS.has(wears.border);
  const moves = useFoilSlot(animates && animatedFoil);
  return {
    ...(foiled ? { "data-cosmetic-foil": wears.foil } : {}),
    ...(animates ? { "data-cosmetic-motion": moves ? "animated" : "static" } : {}),
    ...(wears.finish === DEFAULT_CARD_COSMETICS.finish ? {} : { "data-finish": wears.finish }),
    ...(wears.border === DEFAULT_CARD_COSMETICS.border ? {} : { "data-frame": wears.border }),
  };
}

/** Test seam: drops the in-memory picks, the cards' among them, the listeners, the storage listener and every foil slot. */
export function resetCosmeticsForTests(): void {
  current = null;
  listeners.clear();
  cardPicksState = null;
  cardPicksUnsaved = false;
  cardPicksListeners.clear();
  cardPickWriteListeners.clear();
  if (storageListening) window.removeEventListener("storage", onCosmeticsStorage);
  storageListening = false;
  moving.clear();
  queued.length = 0;
  slotListeners.clear();
}
