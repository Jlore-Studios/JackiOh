// R1295000: the card cosmetics a player's own cards wear: a foil over the art, a finish on the print and a
// border round the frame. R1953001: nothing sets the account-wide picks of those three
// (`COSMETICS_SETTINGS_KEY`, the account's `cosmetics` group, R633) and no face wears them; the group keeps
// its shape (R1655000). R1297000: the same store holds the deck slots, the playmat (`table`), the backdrop
// (`field`) and the card back (`cardBack`). Ids are the registry's (`crates/server/cosmetics.json`, generated
// into `wire/generated/cosmetics.ts`); an unknown id (a newer client's) is kept, never overwritten, and drawn
// as its kind's default. The picks never reach the engine, an action, `PlayerView` or the log; a face wears
// them only where the board says it is the viewer's own card in a game (`worn`, game/wearer.ts). An opponent's
// side, and every side of a watched or replayed online game, wears its seat's loadout frozen at match start
// (R1834002, `cards/loadouts.ts`); every other face draws the defaults (CLAUDE.md rule 7). Each slot is its own
// face attribute, apart from the rarity foil (`data-foil`, model.ts), so a decorative foil never blurs Mythic
// and Radiant. R1834000: a card's picks live on the account, this store is the device's cache
// (`cardPicksSync.ts`, `subscribeCardPickWrites`); it keeps what it is given, the wear path and `useCanEquip` apply ownership.

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
import type { SeatLoadout } from "./loadouts.ts";
import { foilFor, type FaceModel } from "./model.ts";

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

function defaultStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function load(): CardCosmetics {
  try {
    const text = defaultStorage()?.getItem(COSMETICS_SETTINGS_KEY) ?? null;
    return text === null ? { ...DEFAULT_CARD_COSMETICS } : normalizeCosmetics(JSON.parse(text));
  } catch {
    return { ...DEFAULT_CARD_COSMETICS };
  }
}

function persist(picks: CardCosmetics): void {
  try {
    defaultStorage()?.setItem(COSMETICS_SETTINGS_KEY, JSON.stringify(picks));
  } catch {
    // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  }
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

// R1655000: the picks of each card sit in a store of their own, never in the `cosmetics` group the
// account syncs (settings/groups.ts, R633, R634), so an old client and a new one never overwrite each
// other's picks, and no settings reset touches them (R1579000).

export const CARD_PICKS_SETTINGS_KEY = "jackioh.cosmetics.cards.v1";

/**
 * What one card wears of its own. A kind it has no pick of is absent and wears its kind's default (R1953001;
 * the Full-art cards setting, for `fullArt`); a pick of a default id is a pick; an unknown id reads as the
 * default. R2655000: `fullArt` is `false`, `true` (the Epic full art) or a full art's id, kept as written.
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

/** R2655000: a card's full-art pick as stored: a boolean, or a full art's id, known or not. */
function isFullArtPick(value: unknown): value is boolean | string {
  return typeof value === "boolean" || (isPickText(value) && value.startsWith(FULL_ART_ID_PREFIX));
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
    const text = defaultStorage()?.getItem(CARD_PICKS_SETTINGS_KEY) ?? null;
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
 * R1956000: another tab wrote the account's or the cards' picks (a null key is a `localStorage.clear()`):
 * re-read and notify so every face wears it at once; what this page could not save gives way to storage.
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
 * The first read of either store installs the one window "storage" listener for the page's life: the stores
 * keep what storage holds even with no face mounted, so a later face draws a pick another tab made meanwhile.
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
  if (defaultStorage() !== null && !cardPicksUnsaved) {
    const written = readStoredCardPicks();
    return { written, state: cardPicksFrom(written) };
  }
  return { written: Object.fromEntries(cardPicks().stored) as Record<string, unknown>, state: cardPicks() };
}

/**
 * Changes `defId`'s stored picks by `change`, from what storage holds now. It rewrites only the
 * fields this client knows of `defId`'s entry, so a newer client's kinds are kept.
 */
function updateCardPicks(defId: string, change: (picks: StoredCardPicks) => void): void {
  const storage = defaultStorage();
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
  try {
    storage?.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify(written));
    cardPicksUnsaved = storage === null;
  } catch {
    cardPicksUnsaved = true;
  }
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
 * R1834000: puts `picks` (the account's copy) in place of every card's. A card whose picks did not
 * change keeps the object a face read. It tells no write listener: the account's answer is never sent back to it.
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
  const storage = defaultStorage();
  try {
    storage?.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify(Object.fromEntries(next.stored)));
    cardPicksUnsaved = storage === null;
  } catch {
    cardPicksUnsaved = true;
  }
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
 * R1834004: the loadout of the seat owning the copy drawn below, which its not-`worn` faces wear; null with
 * none above or where the board says the copy draws the defaults (game/wearer.ts). Provided by `TheirCopy` (loadouts.ts).
 */
export const TheirCopyContext = createContext<SeatLoadout | null>(null);

/**
 * Whether `cosmeticId` is the viewer's on the card `defId`: a default or a starter always is (every
 * account owns them on every card, §14.2), and so is one the ledger lists on that card.
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
 * What `face` wears, each kind a known id: the defaults where `picks` is null (not the viewer's own), the
 * default foil with a rarity foil (Mythic, Radiant), the default finish and border on a Radiant face (its
 * gold is its own). R1655000: a kind `card` picks that `owns` takes is the card's pick, else `picks` (the
 * kind defaults on a face, R1953001; a cosmetic sample as that cosmetic).
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

// R1297000: the deck slots.

/** The picks a saved deck carries, as the server and the deckbuilder's mirror hold them. */
export type DeckCosmeticFields = {
  tableCosmetic?: string | null;
  fieldCosmetic?: string | null;
  cardBackCosmetic?: string | null;
};

/** A deck's picks by slot: `null` where it has none, which the account's choice fills. */
export type DeckCosmetics = Readonly<Record<DeckSlotKind, string | null>>;

export function deckCosmeticsOf(fields: DeckCosmeticFields): DeckCosmetics | null {
  const picks: DeckCosmetics = {
    table: fields.tableCosmetic ?? null,
    field: fields.fieldCosmetic ?? null,
    cardBack: fields.cardBackCosmetic ?? null,
  };
  return DECK_SLOT_KINDS.every((kind) => typeof picks[kind] !== "string") ? null : picks;
}

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
 * `kind`'s id to draw: the deck's pick (even an unknown id, which draws as the default), else the account's,
 * else the default. R1833000: a pick `owns` refuses draws as the default too.
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

// R1833000: the account cosmetics the viewer owns. While ownership decides (`FULL_ART_GATED`),
// ownedFullArt.ts mounts `AccountCosmeticsOwned` over every gated screen from `GET /api/cosmetics`'
// account list; with none above it (practice, hotseat, R1310004's builds) every one counts as owned.

/** What the pickers and sides below an `AccountCosmeticsOwned` own: `"every"`, or the account cosmetic ids its ledger lists. */
export type AccountOwnership = "every" | ReadonlySet<string>;

const AccountOwnershipContext = createContext<AccountOwnership>("every");

/** R1833000: the pickers and sides below it own the account cosmetics `owned` holds; `"every"` with none above it. */
export function AccountCosmeticsOwned({ owned, children }: { owned: AccountOwnership; children?: ReactNode }): ReactElement {
  return createElement(AccountOwnershipContext.Provider, { value: owned }, children);
}

/** Whether the viewer owns the account cosmetic `id`: a default or a starter always does (every account owns them, §14.2). */
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

// The cap on moving foils. A face that wants its foil to move claims a slot when it mounts and
// gives it back when it unmounts or stops wanting it; past COSMETIC_FOIL_ANIMATED_MAX it waits in
// line, first come first served, and moves once a slot passes to it.

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

/** The attributes a face wears for its cosmetics: none at all for the defaults. */
export type CosmeticAttributes = {
  "data-cosmetic-foil"?: CosmeticId;
  "data-cosmetic-motion"?: "animated" | "static";
  "data-finish"?: CosmeticId;
  "data-frame"?: CosmeticId;
};

/**
 * A face's cosmetic attributes: when `worn`, the card's own picks (R1655000) where owned, over its kinds'
 * defaults (R1953001); a Created card's are its generator's (R1956001). Below a `TheirCopy` and not `worn`:
 * that seat's picks, all owned (R1834001, R1834004). Rarity rules hold over both; `animatedFoil` stills a
 * foil and a moving finish or border (R1833000). Its hooks run on every face, worn or not.
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
