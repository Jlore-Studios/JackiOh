// R1295000: the card cosmetics a player's own cards wear — a foil over the art, a finish on the
// print and a border round the frame — picked in the collection's Cosmetics section
// (`collection/CosmeticsPanel.tsx`, R1579000), kept per browser under `COSMETICS_SETTINGS_KEY` and
// on the account as the `cosmetics` group (R633, `settings/groups.ts`). The ids are the registry's
// (`crates/server/cosmetics.json`, generated into `wire/generated/cosmetics.ts`), every one unlocked.
//
// R1297000: the same store holds three deck slots beside them: the playmat (`table`), the ambient
// backdrop (`field`) and the back of the viewer's face-down cards and deck (`cardBack`). Each saved
// deck may carry its own picks (`DeckCosmetics`); a viewer's side wears the deck's, else the
// account's, else the kind's default (`wornDeckCosmetics`), and an id this client does not know
// draws as the default. The opponent's side draws the defaults, as every card cosmetic does.
//
// The picks are the device's own: nothing here reaches the engine, an action, `PlayerView`, the
// log or a replay, and a face wears them only where the board says it is the viewer's own card in
// a game (`worn`, game/wearer.ts); every other face draws the defaults (CLAUDE.md rule 7). Each
// slot is its own attribute on the face, apart from the rarity foil (`data-foil`, model.ts), so a
// decorative foil never blurs the Mythic and Radiant signal: a Mythic face wears no cosmetic foil,
// and a Radiant face no foil, finish or border. A default adds no attribute at all, so a face
// wearing nothing is the face it was before.
//
// R1655000: a card may also carry picks of its own, on the device alone (`jackioh.cosmetics.cards.v1`,
// never the `cosmetics` group above, so no client overwrites another's): a foil, a finish, a border
// and whether its full art prints. A kind a card has no pick of wears the account's pick above (the
// Full-art cards setting, for full art). A card's pick is worn only where the viewer owns that
// cosmetic on that card (`ownsCosmetic`: a default or a starter counts, and so does a ledger row),
// or where every cosmetic counts as owned (practice, hotseat, R1310004's builds); the store keeps what
// it is given, the wear path and `useCanEquip` apply ownership. The face wearing them is still only
// the viewer's own copy (`worn`, or below an `OwnCopy`, fullArt.ts).
//
// An id this client does not know (one a newer client wrote to the account) is kept in the store,
// so this device never overwrites it, and drawn as its kind's default, never an error. A cosmetic
// foil moves only while one of COSMETIC_FOIL_ANIMATED_MAX slots is the face's; the others wear its
// still version, as every face does under reduced motion (cosmetics.css).

import { createContext, useCallback, useContext, useEffect, useState, useSyncExternalStore } from "react";

import { COSMETIC_ID_MAX_LENGTH, COSMETIC_KINDS, COSMETICS } from "../wire/generated/cosmetics.ts";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
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

function defaultStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Absent, throwing or unparsable storage gives the defaults. */
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

/** The in-memory picks, loaded from storage once, on first read. */
export function readCosmetics(): CardCosmetics {
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
 * What one card wears of its own. A kind it has no pick of is absent and wears the account's pick
 * (the Full-art cards setting, for `fullArt`); a pick of a kind's default id is a pick, which beats
 * a non-default account one. An id this client does not know reads as its kind's default.
 */
export type CardPicks = { foil?: CosmeticId; finish?: CosmeticId; border?: CosmeticId; fullArt?: boolean };

/** One card's picks as stored: a short text is kept for each kind, known or not (as `normalizeCosmetics` keeps one). */
type StoredCardPicks = { foil?: string; finish?: string; border?: string; fullArt?: boolean };

const CARD_PICK_FIELDS = [...CARD_SLOT_KINDS, "fullArt"] as const;
const NO_PICKS: CardPicks = Object.freeze({});

function isPickText(value: unknown): value is string {
  return typeof value === "string" && value !== "" && value.length <= COSMETIC_ID_MAX_LENGTH;
}

function normalizeStoredPicks(raw: unknown): StoredCardPicks {
  const record = raw !== null && typeof raw === "object" && !Array.isArray(raw) ? (raw as Record<string, unknown>) : {};
  const out: StoredCardPicks = {};
  for (const kind of CARD_SLOT_KINDS) {
    const value = record[kind];
    if (isPickText(value)) out[kind] = value;
  }
  if (typeof record["fullArt"] === "boolean") out.fullArt = record["fullArt"];
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

/** Absent, throwing or unparsable storage gives no picks. */
function loadCardPicks(): CardPicksState {
  const state: CardPicksState = { stored: new Map(), resolved: new Map() };
  try {
    const text = defaultStorage()?.getItem(CARD_PICKS_SETTINGS_KEY) ?? null;
    const parsed: unknown = text === null ? null : JSON.parse(text);
    if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)) {
      for (const [defId, raw] of Object.entries(parsed)) {
        const picks = normalizeStoredPicks(raw);
        if (Object.keys(picks).length > 0) {
          state.stored.set(defId, picks);
          state.resolved.set(defId, resolveCardPicks(picks));
        }
      }
    }
  } catch {
    // Unparsable or blocked storage: no card has a pick of its own.
  }
  return state;
}

let cardPicksState: CardPicksState | null = null;
const cardPicksListeners = new Set<() => void>();

function cardPicks(): CardPicksState {
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

export function useCardPicks(defId: string): CardPicks {
  return useSyncExternalStore(subscribeCardPicks, () => readCardPicks(defId), () => readCardPicks(defId));
}

/** Changes `defId`'s stored picks by `change`, persists in try/catch and notifies synchronously, unless nothing changed. */
function updateCardPicks(defId: string, change: (picks: StoredCardPicks) => void): void {
  const state = cardPicks();
  const before = state.stored.get(defId) ?? {};
  const picks: StoredCardPicks = { ...before };
  change(picks);
  if (CARD_PICK_FIELDS.every((field) => before[field] === picks[field])) return;
  if (Object.keys(picks).length === 0) {
    state.stored.delete(defId);
    state.resolved.delete(defId);
  } else {
    state.stored.set(defId, picks);
    state.resolved.set(defId, resolveCardPicks(picks));
  }
  try {
    defaultStorage()?.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify(Object.fromEntries(state.stored)));
  } catch {
    // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  }
  for (const listener of [...cardPicksListeners]) listener();
}

/** Sets `defId`'s `kind` pick to `id`, or clears it with null, which leaves it to the account's pick. */
export function writeCardPick(defId: string, kind: CardSlotKind, id: CosmeticId | null): void {
  updateCardPicks(defId, (picks) => {
    if (id !== null && isPickText(id)) picks[kind] = id;
    else delete picks[kind];
  });
}

/** Sets whether `defId`'s full art prints, or clears the pick with null, which leaves it to the Full-art cards setting. */
export function writeCardFullArt(defId: string, on: boolean | null): void {
  updateCardPicks(defId, (picks) => {
    if (on === null) delete picks.fullArt;
    else picks.fullArt = on;
  });
}

/** The cosmetic ids the viewer owns on each card, past the defaults and starters that every account owns on every card. */
export type OwnedByCard = ReadonlyMap<string, ReadonlySet<string>>;

/** The cosmetics the faces below a `FullArtOwned` may wear: `"every"` (practice, hotseat, R1310004's builds) or the ledger's. */
export type CardOwnership = "every" | OwnedByCard;

/** R1655000: what the faces below it own; `"every"` with none above it. Provided by `FullArtOwned` (fullArt.ts). */
export const OwnershipContext = createContext<CardOwnership>("every");

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
 * the card has a pick of (`card`) that `owns` takes the card's pick; the account's `picks` otherwise.
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
    return own !== undefined && owns(own) ? own : picks[kind];
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
 * the default: the deck's word is final), else the account's, else the default.
 */
export function resolveDeckCosmetic(
  kind: DeckSlotKind,
  deck: DeckCosmetics | null | undefined,
  account: CardCosmetics,
): CosmeticId {
  return resolveCosmetic(kind, deck?.[kind] ?? account[kind]);
}

/** The deck slots a side wears: the defaults when `worn` is false (the opponent's side). */
export function wornDeckCosmetics(
  worn: boolean,
  deck: DeckCosmetics | null | undefined,
  account: CardCosmetics,
): Record<DeckSlotKind, CosmeticId> {
  if (!worn) {
    return { table: DEFAULT_CARD_COSMETICS.table, field: DEFAULT_CARD_COSMETICS.field, cardBack: DEFAULT_CARD_COSMETICS.cardBack };
  }
  return {
    table: resolveDeckCosmetic("table", deck, account),
    field: resolveDeckCosmetic("field", deck, account),
    cardBack: resolveDeckCosmetic("cardBack", deck, account),
  };
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
 * The cosmetic attributes of a face: the device's picks when `worn`, the defaults otherwise, the
 * card's own picks (R1655000) over the account's where the viewer owns them. `animatedFoil` is the
 * card settings' switch, which stills a cosmetic foil as it does the rarity foil. Its hooks run on
 * every face, worn or not.
 */
export function useCosmeticAttributes(face: FaceModel, worn: boolean, animatedFoil: boolean): CosmeticAttributes {
  const picks = useCosmetics();
  const card = useCardPicks(face.defId);
  const owned = useContext(OwnershipContext);
  const wears = wornCosmetics(face, worn ? picks : null, card, (id) => ownsCosmetic(owned, face.defId, id));
  const foiled = wears.foil !== DEFAULT_CARD_COSMETICS.foil;
  const moves = useFoilSlot(foiled && animatedFoil);
  return {
    ...(foiled ? { "data-cosmetic-foil": wears.foil, "data-cosmetic-motion": moves ? "animated" : "static" } : {}),
    ...(wears.finish === DEFAULT_CARD_COSMETICS.finish ? {} : { "data-finish": wears.finish }),
    ...(wears.border === DEFAULT_CARD_COSMETICS.border ? {} : { "data-frame": wears.border }),
  };
}

/** Test seam: drops the in-memory picks, the cards' among them, the listeners and every foil slot. */
export function resetCosmeticsForTests(): void {
  current = null;
  listeners.clear();
  cardPicksState = null;
  cardPicksListeners.clear();
  moving.clear();
  queued.length = 0;
  slotListeners.clear();
}
