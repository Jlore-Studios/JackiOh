// The deck workshop's store: saved decks and trios as this device holds them, and the autosave
// that keeps the server in step (SPEC §9.4, R250, R252, R256).
//
// The client mints each id, so every save is `PUT /api/decks/:id`, an idempotent upsert: a save
// that timed out may be sent again and never makes a second deck. Every edit goes to a
// `localStorage` mirror first; the next visit merges it over the server's copy (R256).
// `localStorage` is untrusted and may throw or be missing, so every access sits in try/catch.
//
// The server is law (rule 7): the caps, D1–D4 and T1–T3 are its to enforce. This store checks the
// caps only so the UI can say so early. Order of a save: decks, then trios, then deletions.
//
// R2119000: a deletion is held behind the workshop's undo toast; when the hold ends, the workshop
// calls `deleteDeck` or `deleteTrio` and then `flush`, with `keepalive` when the page is going away.

import { checkDeckDraft, checkImportRoom, checkTrioDraft, normalizeName } from "@jackioh/validator";

import {
  DECK_AUTOSAVE_DEBOUNCE_MS,
  DECK_AUTOSAVE_RETRY_SECONDS,
  DECK_NAME_MAX_LENGTH,
  MAX_SAVED_DECKS,
  MAX_SAVED_TRIOS,
} from "@jackioh/server-config";
import {
  ApiRequestError,
  ApiUnreachableError,
  type DeckInput,
  type DecksResponse,
  type DeckVersion,
  type SendOptions,
  type TrioImportInput,
  type TrioInput,
  type TrioSlots,
} from "../../net/api.ts";
import { isRecord } from "../../wire/json.ts";
import { pickedDeckCosmetics, type DeckCosmeticFields } from "../../cards/cosmetics.ts";
import { deckVersionsFrom } from "./deckHistory.ts";
import { messageOf } from "../../net/errors.ts";

// The shapes

/** R1297000: also its playmat, backdrop and card back picks, absent or null for none. R2097000: the catalog version of its latest save (R253's stamp, or this build's after an edit here); absent on a mirror entry from before #2097. */
export type DeckItem = { id: string; name: string; cards: readonly string[]; portrait: string | null; catalogVersion?: string; createdAt: number; updatedAt: number } & DeckCosmeticFields;
export type TrioItem = { id: string; name: string; deckIds: TrioSlots; createdAt: number; updatedAt: number };

export type SyncState = "saved" | "saving" | "offline" | "error";
export type SyncStatus = { state: SyncState; message: string | null };
export type WorkshopLimits = { decks: number; trios: number; nameLength: number };

export type WorkshopSnapshot = {
  /** Oldest first (created, then id), as `GET /api/decks` lists them. */
  decks: readonly DeckItem[];
  trios: readonly TrioItem[];
  status: SyncStatus;
  /** Deck and trio ids whose latest edit the server has not confirmed (refused ones included). */
  unsynced: ReadonlySet<string>;
  /** Ids the server refused, with its message verbatim; the item's next edit clears it. */
  refused: ReadonlyMap<string, string>;
  limits: WorkshopLimits;
  /** R2119000: the deck or trio whose deletion the undo toast holds (`hide`), listed nowhere above. */
  held: HeldItem | null;
};

/** R2119000: a deck or trio a held deletion has taken off the screen. */
export type HeldItem = { kind: "deck" | "trio"; id: string };

/** The writes, bound to the session's token by the route (`routes/decks.tsx`). */
export type DeckSyncApi = {
  putDeck(id: string, input: DeckInput): Promise<unknown>;
  deleteDeck(id: string, options?: SendOptions): Promise<unknown>;
  putTrio(id: string, input: TrioInput): Promise<unknown>;
  deleteTrio(id: string, options?: SendOptions): Promise<unknown>;
  importTrio(input: TrioImportInput): Promise<unknown>;
  getDeckVersions(id: string): Promise<unknown>;
  /** R2100000: a kept version saved as the deck's current one; answers the deck and the versions. */
  restoreDeckVersion(id: string, versionId: string, catalogVersion: string): Promise<unknown>;
};

/** What a trio import makes (R339). */
export type TrioImport = {
  name: string;
  slots: readonly ({ name: string; cards: readonly string[] } | null)[];
};

/** How an import ended: the new trio's id, or the sentence saying why nothing was made (R340). */
export type TrioImportResult = { ok: true; trioId: string } | { ok: false; message: string };

/** R2100000: a deck's kept versions, newest first, or the sentence saying why there are none to show. */
export type DeckHistoryResult = { ok: true; versions: readonly DeckVersion[] } | { ok: false; message: string };

export type StorageLike = Pick<Storage, "getItem" | "setItem" | "removeItem">;

export type SyncClock = {
  now(): number;
  setTimeout(run: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
};

export type DeckStoreOptions = {
  profileId: string;
  /** Stamped on every deck PUT: the catalog the builder is showing (R253's `update_required`). */
  catalogVersion: string;
  /** `GET /api/decks`. */
  server: DecksResponse;
  api: DeckSyncApi;
  /** Default `browserStorage()`. `null`: no mirror at all. */
  storage?: StorageLike | null;
  clock?: SyncClock;
  newId?: () => string;
};

export type DeckStore = {
  /** The same object until something changes, as `useSyncExternalStore` wants. */
  getSnapshot(): WorkshopSnapshot;
  subscribe(listener: () => void): () => void;
  /** Null at `limits.decks`. The UI supplies the name. */
  createDeck(init: { name: string; cards?: readonly string[]; portrait?: string | null } & DeckCosmeticFields): string | null;
  updateDeck(id: string, patch: { name?: string; cards?: readonly string[]; portrait?: string | null } & DeckCosmeticFields): void;
  /** Also empties every trio slot that named it (R252), as the server does. Ends a `hide` of it. */
  deleteDeck(id: string): void;
  /** Null at `limits.trios`. */
  createTrio(init: { name: string; deckIds?: TrioSlots }): string | null;
  updateTrio(id: string, patch: { name?: string; deckIds?: TrioSlots }): void;
  deleteTrio(id: string): void;
  /**
   * R340, R341: imports a trio code's decks and the trio, all or nothing, in one request, never
   * through the autosave (which would leave half an import behind a refusal). The ids are minted
   * once per import and reused on a retry, so a lost answer never makes it twice.
   */
  importTrio(init: TrioImport): Promise<TrioImportResult>;
  /**
   * R2100000: the deck's kept versions, as the server lists them. What is unsaved goes first, so the
   * newest is the deck the editor shows.
   */
  deckVersions(deckId: string): Promise<DeckHistoryResult>;
  /**
   * R2100000: makes a kept version the deck's current one, in one request and never through the
   * autosave (whose window would fold it into the newest version). What is unsaved goes first; a
   * deck still unsaved after that restores nothing, since the restore would save over its edit.
   */
  restoreDeckVersion(deckId: string, versionId: string): Promise<DeckHistoryResult>;
  /**
   * R2119000: takes a deck or trio off the snapshot and out of every save while its deletion is held.
   * One at a time: hiding another shows the first again.
   */
  hide(kind: HeldItem["kind"], id: string): void;
  /** R2119000: Undo; the hidden item is listed and saved again. */
  unhide(): void;
  /**
   * Saves now (the debounce is skipped); settles once this flush and any re-run it caused have.
   * `keepalive` (R2119000): the page is going away, so its deletions go as `keepalive` requests.
   */
  flush(options?: SendOptions): Promise<void>;
  /** Saves on `pagehide`, on the page going hidden and on coming back online. Idempotent. */
  start(): void;
  /** Detaches, clears the timers and sends what is unsaved one last time. Idempotent. */
  stop(): void;
};

// Names

export const UNTITLED_DECK = "Untitled deck";
export const UNTITLED_TRIO = "Untitled trio";

/** Truncates to `max` code points, as D1 counts them (an emoji is one character to a player). */
function truncateCodePoints(value: string, max: number): string {
  const points = [...value];
  return points.length <= max ? value : points.slice(0, max).join("");
}

/**
 * The name a deck PUT sends: "Untitled deck" whenever the validator's D1 would refuse the cleaned name.
 * The field keeps what the player typed; only the save is cleaned, so D1 is met gently rather than refused.
 */
export function deckNameForSave(raw: string, nameLength: number): string {
  const name = truncateCodePoints(normalizeName(raw), nameLength);
  const refused = checkDeckDraft({ name, cards: [], isDeckable: () => true, nameMaxLength: nameLength }).some(
    (issue) => issue.rule === "D1",
  );
  return refused ? UNTITLED_DECK : name;
}

/** As `deckNameForSave`, for a trio (T1). */
export function trioNameForSave(raw: string, nameLength: number): string {
  const name = truncateCodePoints(normalizeName(raw), nameLength);
  const refused = checkTrioDraft({ name, deckIds: [null, null, null], nameMaxLength: nameLength }).some(
    (issue) => issue.rule === "T1",
  );
  return refused ? UNTITLED_TRIO : name;
}

// Storage

export const DECK_MIRROR_PREFIX = "jackioh.decks.v1.";

export function mirrorKey(profileId: string): string {
  return `${DECK_MIRROR_PREFIX}${profileId}`;
}

export function browserStorage(): StorageLike | null {
  try {
    if (typeof window === "undefined") return null;
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

/** The mirror's format version, so a later shape can tell an old value apart and drop it. */
const MIRROR_VERSION = 1;

type Tracked<T> = {
  item: T;
  /** The server has not confirmed this item's latest edit. */
  dirty: boolean;
  /** Bumped by every edit, so a confirmation of an older revision does not clear `dirty`. */
  rev: number;
  /** The server's refusal of the latest edit, verbatim. Not retried until the next edit. */
  refused: string | null;
  /** A trio refused for naming a deck not saved yet: it goes again once a deck lands. */
  blocked: boolean;
};

type MirrorEntry<T> = { item: T; dirty: boolean };

type Mirror = {
  decks: MirrorEntry<DeckItem>[];
  trios: MirrorEntry<TrioItem>[];
  deletedDecks: string[];
  deletedTrios: string[];
};

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function deckFrom(value: unknown): DeckItem | null {
  if (!isRecord(value)) return null;
  const { id, name, cards, createdAt, updatedAt } = value;
  if (typeof id !== "string" || id.length === 0 || typeof name !== "string" || !isStringArray(cards)) return null;
  if (!isFiniteNumber(createdAt) || !isFiniteNumber(updatedAt)) return null;
  // R641: the portrait rides the same row; an old mirror without it reads as `vanilla` (null).
  const portrait = "portrait" in value && typeof value.portrait === "string" ? value.portrait : null;
  // R1297000: the deck slots ride the same row; an old mirror without them reads as no pick.
  const stamp = typeof value.catalogVersion === "string" ? { catalogVersion: value.catalogVersion } : {};
  return { id, name, cards: [...cards], portrait, ...pickedDeckCosmetics(value), ...stamp, createdAt, updatedAt };
}

function slotsFrom(value: unknown): TrioSlots | null {
  if (!Array.isArray(value) || value.length !== TRIO_SLOT_COUNT) return null;
  const slots = value.map((slot) => (typeof slot === "string" && slot.length > 0 ? slot : null));
  if (value.some((slot) => slot !== null && typeof slot !== "string")) return null;
  return [slots[0] ?? null, slots[1] ?? null, slots[2] ?? null];
}

function trioFrom(value: unknown): TrioItem | null {
  if (!isRecord(value)) return null;
  const { id, name, deckIds, createdAt, updatedAt } = value;
  if (typeof id !== "string" || id.length === 0 || typeof name !== "string") return null;
  const slots = slotsFrom(deckIds);
  if (slots === null || !isFiniteNumber(createdAt) || !isFiniteNumber(updatedAt)) return null;
  return { id, name, deckIds: slots, createdAt, updatedAt };
}

const TRIO_SLOT_COUNT = 3;

function entriesFrom<T>(value: unknown, read: (raw: unknown) => T | null): MirrorEntry<T>[] {
  if (!Array.isArray(value)) return [];
  const entries: MirrorEntry<T>[] = [];
  for (const raw of value) {
    if (!isRecord(raw)) continue;
    const item = read(raw.item);
    if (item === null) continue;
    entries.push({ item, dirty: raw.dirty === true });
  }
  return entries;
}

/** Tolerant: a malformed value, or one from another format version, reads as no mirror at all. */
export function parseMirror(raw: string | null): Mirror | null {
  if (raw === null) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!isRecord(value) || value.v !== MIRROR_VERSION) return null;
    return {
      decks: entriesFrom(value.decks, deckFrom),
      trios: entriesFrom(value.trios, trioFrom),
      deletedDecks: isStringArray(value.deletedDecks) ? value.deletedDecks : [],
      deletedTrios: isStringArray(value.deletedTrios) ? value.deletedTrios : [],
    };
  } catch {
    return null;
  }
}

function readMirror(storage: StorageLike | null, key: string): Mirror | null {
  if (storage === null) return null;
  try {
    return parseMirror(storage.getItem(key));
  } catch {
    return null;
  }
}

// The clock

const MS_PER_SECOND = 1000;
const RETRY_MS = DECK_AUTOSAVE_RETRY_SECONDS * MS_PER_SECOND;

function browserClock(): SyncClock {
  return {
    now: () => Date.now(),
    setTimeout: (run, ms) => globalThis.setTimeout(run, ms),
    clearTimeout: (handle) => {
      globalThis.clearTimeout(handle as ReturnType<typeof globalThis.setTimeout>);
    },
  };
}

function randomId(): string {
  return globalThis.crypto.randomUUID();
}

// Errors

/** What the status line says while a failed save waits to go again. */
export const OFFLINE_MESSAGE = "Offline. Your changes are kept on this device.";

export const IMPORT_OFFLINE_MESSAGE = "You're offline, so nothing was imported. Try again once you're back online.";
/** R341: why an import made nothing, when earlier edits (a deletion made to make room) have not landed. */
export const IMPORT_UNSAVED_MESSAGE =
  "Your latest changes haven't reached the server yet, so nothing was imported. Try again once they're saved.";

export const HISTORY_OFFLINE_MESSAGE = "You're offline, so this deck's history can't be shown. Try again once you're back online.";
export const RESTORE_OFFLINE_MESSAGE = "You're offline, so nothing was restored. Try again once you're back online.";
/** R2100000: why a restore changed nothing, when the deck's latest edit has not landed (a restore would save over it). */
export const RESTORE_UNSAVED_MESSAGE =
  "This deck's latest changes haven't reached the server yet, so nothing was restored. Try again once they're saved.";
/** The server's own sentence for a deck it does not hold for this account (`crates/server/src/api/decks.rs`). */
const NO_SUCH_DECK_MESSAGE = "There is no such deck.";

const HTTP_UNAUTHORIZED = 401;
const HTTP_TIMEOUT = 408;
const HTTP_CONFLICT = 409;
const HTTP_TOO_MANY = 429;
const HTTP_SERVER_ERROR = 500;

type Failure =
  | { kind: "offline" }
  /** Worth sending again later: the server is down, busy, or the token is being renewed. */
  | { kind: "transient"; message: string }
  /** The server refused this body. Sending it again would be refused again. */
  | { kind: "refused"; message: string; unknownDeck: boolean };

function failureOf(cause: unknown): Failure {
  if (cause instanceof ApiUnreachableError) return { kind: "offline" };
  if (cause instanceof ApiRequestError) {
    const status = cause.status;
    if (status === HTTP_UNAUTHORIZED || status === HTTP_TIMEOUT || status === HTTP_TOO_MANY || status >= HTTP_SERVER_ERROR) {
      return { kind: "transient", message: cause.message };
    }
    const details = cause.details;
    const unknownDeck = status === HTTP_CONFLICT && isRecord(details) && details.unknownDeck === true;
    return { kind: "refused", message: cause.message, unknownDeck };
  }
  return { kind: "transient", message: messageOf(cause) };
}

// The store

function byAge<T extends { createdAt: number; id: string }>(left: Tracked<T>, right: Tracked<T>): number {
  if (left.item.createdAt !== right.item.createdAt) return left.item.createdAt - right.item.createdAt;
  return left.item.id < right.item.id ? -1 : left.item.id > right.item.id ? 1 : 0;
}

function tracked<T>(item: T, dirty: boolean): Tracked<T> {
  return { item, dirty, rev: 0, refused: null, blocked: false };
}

function sameList(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function sameSlots(left: TrioSlots, right: TrioSlots): boolean {
  return left[0] === right[0] && left[1] === right[1] && left[2] === right[2];
}

/**
 * R256's merge. Unsynced local edits win over the server's copy; local items the server lacks
 * survive only while unsynced, since a synced one missing from the server was deleted elsewhere.
 * A pending deletion stays pending while the server still has the id.
 */
function merge<T extends { id: string; createdAt: number }>(
  serverItems: readonly T[],
  local: readonly MirrorEntry<T>[],
  deleted: readonly string[],
): { items: Tracked<T>[]; deleting: Set<string> } {
  const localById = new Map(local.map((entry) => [entry.item.id, entry]));
  const serverIds = new Set(serverItems.map((item) => item.id));
  const deleting = new Set(deleted.filter((id) => serverIds.has(id)));
  const items: Tracked<T>[] = [];
  for (const item of serverItems) {
    if (deleting.has(item.id)) continue;
    const mine = localById.get(item.id);
    items.push(mine?.dirty === true ? tracked(mine.item, true) : tracked(item, false));
  }
  // The mirror is untrusted text: one id listed twice is restored once, or the workshop would list
  // it twice and send it twice.
  const restored = new Set<string>();
  for (const entry of local) {
    const { id } = entry.item;
    if (serverIds.has(id) || !entry.dirty || deleting.has(id) || restored.has(id)) continue;
    restored.add(id);
    items.push(tracked(entry.item, true));
  }
  items.sort(byAge);
  return { items, deleting };
}

export function createDeckStore(options: DeckStoreOptions): DeckStore {
  const { api, catalogVersion } = options;
  const storage = options.storage === undefined ? browserStorage() : options.storage;
  const clock = options.clock ?? browserClock();
  const newId = options.newId ?? randomId;
  const key = mirrorKey(options.profileId);
  // A server from before R250's limits would not send them; the shared config says the same.
  const serverLimits: Partial<WorkshopLimits> = (options.server.limits as Partial<WorkshopLimits> | undefined) ?? {};
  const limits: WorkshopLimits = {
    decks: serverLimits.decks ?? MAX_SAVED_DECKS,
    trios: serverLimits.trios ?? MAX_SAVED_TRIOS,
    nameLength: serverLimits.nameLength ?? DECK_NAME_MAX_LENGTH,
  };

  const mirror = readMirror(storage, key);
  const mergedDecks = merge(
    options.server.decks.map((deck) => ({ ...deck, cards: [...deck.cards] })),
    mirror?.decks ?? [],
    mirror?.deletedDecks ?? [],
  );
  const mergedTrios = merge(
    options.server.trios.map((trio) => ({ ...trio, deckIds: [...trio.deckIds] as TrioSlots })),
    mirror?.trios ?? [],
    mirror?.deletedTrios ?? [],
  );

  let decks: Tracked<DeckItem>[] = mergedDecks.items;
  let trios: Tracked<TrioItem>[] = mergedTrios.items;
  const deletingDecks = mergedDecks.deleting;
  const deletingTrios = mergedTrios.deleting;

  // A slot naming a deck that is not here empties, as the server's foreign key empties it, and the
  // trio is sent again so both sides agree.
  {
    const known = new Set(decks.map((deck) => deck.item.id));
    for (const trio of trios) {
      const slots = trio.item.deckIds.map((id) => (id !== null && known.has(id) ? id : null)) as TrioSlots;
      if (!sameSlots(slots, trio.item.deckIds)) {
        trio.item = { ...trio.item, deckIds: slots };
        trio.dirty = true;
      }
    }
  }

  const listeners = new Set<() => void>();
  let snapshot: WorkshopSnapshot | null = null;

  let debounce: unknown = null;
  let retry: unknown = null;
  let running: Promise<void> | null = null;
  let rerun = false;
  /** The last pass stopped on a transport failure; a retry is scheduled. */
  let offline = false;
  /** The last pass stopped on a failure worth retrying; its message, until a pass gets through. */
  let transient: string | null = null;
  let started = false;
  /** After `stop()`: nothing is scheduled any more, so a left screen leaves no timer behind. */
  let stopped = false;
  /** R2119000: the item `hide` took off the snapshot, if any. */
  let hidden: HeldItem | null = null;
  /** R2119000: the flush under way was asked for as the page goes away. */
  let keepalive = false;
  /**
   * R2119000: what this visit deleted, kept until its DELETE has an answer, so a refusal can put it
   * back. A deletion the mirror brought from an earlier visit is not here.
   */
  const removed = new Map<string, { kind: "deck"; entry: Tracked<DeckItem> } | { kind: "trio"; entry: Tracked<TrioItem> }>();

  function isHidden(id: string): boolean {
    return hidden !== null && hidden.id === id;
  }

  function persist(): void {
    if (storage === null) return;
    try {
      const value = {
        v: MIRROR_VERSION,
        decks: decks.map((entry) => ({ item: entry.item, dirty: entry.dirty })),
        trios: trios.map((entry) => ({ item: entry.item, dirty: entry.dirty })),
        deletedDecks: [...deletingDecks],
        deletedTrios: [...deletingTrios],
      };
      storage.setItem(key, JSON.stringify(value));
    } catch {
      // Quota, private mode or blocked storage: the in-memory state stays in force.
    }
  }

  function sendable<T extends { id: string }>(entry: Tracked<T>): boolean {
    return entry.dirty && entry.refused === null && !entry.blocked && !isHidden(entry.item.id);
  }

  function hasWork(): boolean {
    return decks.some(sendable) || trios.some(sendable) || deletingDecks.size > 0 || deletingTrios.size > 0;
  }

  function statusNow(): SyncStatus {
    if (offline) return { state: "offline", message: OFFLINE_MESSAGE };
    if (transient !== null) return { state: "error", message: transient };
    if (debounce !== null || running !== null || hasWork()) return { state: "saving", message: null };
    const refusal = [...decks, ...trios].find((entry) => entry.refused !== null && !isHidden(entry.item.id))?.refused ?? null;
    if (refusal !== null) return { state: "error", message: refusal };
    return { state: "saved", message: null };
  }

  function emit(): void {
    snapshot = null;
    for (const listener of [...listeners]) listener();
  }

  function changed(): void {
    persist();
    emit();
  }

  function schedule(): void {
    if (stopped) return;
    if (debounce !== null) clock.clearTimeout(debounce);
    debounce = clock.setTimeout(() => {
      debounce = null;
      void flush();
    }, DECK_AUTOSAVE_DEBOUNCE_MS);
  }

  function scheduleRetry(): void {
    if (stopped) return;
    if (retry !== null) clock.clearTimeout(retry);
    retry = clock.setTimeout(() => {
      retry = null;
      void flush();
    }, RETRY_MS);
  }

  function edited<T>(entry: Tracked<T>, item: T): void {
    entry.item = item;
    entry.dirty = true;
    entry.rev += 1;
    entry.refused = null;
    entry.blocked = false;
    changed();
    schedule();
  }

  function deckInput(item: DeckItem): DeckInput {
    // R1297000: a pick the deck does not have is left out, which the server stores as none.
    return { name: deckNameForSave(item.name, limits.nameLength), cards: [...item.cards], catalogVersion, portrait: item.portrait, ...pickedDeckCosmetics(item) };
  }

  function trioInput(item: TrioItem): TrioInput {
    return { name: trioNameForSave(item.name, limits.nameLength), deckIds: [...item.deckIds] as TrioSlots };
  }

  /** R2119000: a deletion made on this visit that the server refused comes back, in its words. */
  function putBack(id: string, message: string): void {
    const gone = removed.get(id);
    if (gone === undefined) return;
    gone.entry.refused = message;
    if (gone.kind === "deck") {
      if (!decks.some((entry) => entry.item.id === id)) decks = [...decks, gone.entry].sort(byAge);
    } else if (!trios.some((entry) => entry.item.id === id)) {
      trios = [...trios, gone.entry].sort(byAge);
    }
  }

  /** How a pass ended: everything it could send went, or it stopped on a failure worth retrying. */
  type PassEnd = "done" | "stopped";

  function halt(failure: Failure): PassEnd {
    if (failure.kind === "offline") {
      offline = true;
      transient = null;
    } else if (failure.kind === "transient") {
      offline = false;
      transient = failure.message;
    }
    scheduleRetry();
    return "stopped";
  }

  /** One pass, and whether any deck landed (for blocked trios). */
  async function pass(): Promise<{ end: PassEnd; deckLanded: boolean }> {
    let deckLanded = false;

    for (const entry of decks.filter(sendable)) {
      const { id } = entry.item;
      const rev = entry.rev;
      try {
        await api.putDeck(id, deckInput(entry.item));
      } catch (cause: unknown) {
        const failure = failureOf(cause);
        if (failure.kind !== "refused") return { end: halt(failure), deckLanded };
        const current = decks.find((candidate) => candidate.item.id === id);
        if (current !== undefined && current.rev === rev) current.refused = failure.message;
        changed();
        continue;
      }
      offline = false;
      transient = null;
      deckLanded = true;
      const current = decks.find((candidate) => candidate.item.id === id);
      if (current !== undefined && current.rev === rev) current.dirty = false;
      changed();
    }

    for (const entry of trios.filter(sendable)) {
      const { id } = entry.item;
      const rev = entry.rev;
      try {
        await api.putTrio(id, trioInput(entry.item));
      } catch (cause: unknown) {
        const failure = failureOf(cause);
        if (failure.kind !== "refused") return { end: halt(failure), deckLanded };
        const current = trios.find((candidate) => candidate.item.id === id);
        if (current !== undefined && current.rev === rev) {
          const waiting =
            failure.unknownDeck &&
            current.item.deckIds.some((deckId) => deckId !== null && decks.some((deck) => deck.item.id === deckId && deck.dirty));
          if (waiting) current.blocked = true;
          else current.refused = failure.message;
        }
        changed();
        continue;
      }
      offline = false;
      transient = null;
      const current = trios.find((candidate) => candidate.item.id === id);
      if (current !== undefined && current.rev === rev) current.dirty = false;
      changed();
    }

    for (const [pending, remove] of [
      [deletingTrios, api.deleteTrio],
      [deletingDecks, api.deleteDeck],
    ] as const) {
      for (const id of [...pending]) {
        try {
          if (keepalive) await remove.call(api, id, { keepalive: true });
          else await remove.call(api, id);
        } catch (cause: unknown) {
          const failure = failureOf(cause);
          if (failure.kind !== "refused") return { end: halt(failure), deckLanded };
          // A refused deletion has nothing to try again. One made on this visit comes back in the
          // server's words (R2119000); one only the mirror knew about is forgotten.
          putBack(id, failure.message);
        }
        removed.delete(id);
        offline = false;
        transient = null;
        pending.delete(id);
        changed();
      }
    }

    return { end: "done", deckLanded };
  }

  async function run(): Promise<void> {
    for (;;) {
      rerun = false;
      const { end, deckLanded } = await pass();
      if (end === "stopped") return;
      // Nothing failed on the way: whatever the last failed pass said no longer holds.
      offline = false;
      transient = null;
      // A deck that landed may be the one a blocked trio names: let those go again now.
      let unblocked = false;
      if (deckLanded) {
        for (const trio of trios) {
          if (trio.blocked) {
            trio.blocked = false;
            unblocked = true;
          }
        }
      }
      if (retry !== null) {
        clock.clearTimeout(retry);
        retry = null;
      }
      if (!rerun && !unblocked) return;
    }
  }

  function flush(options: SendOptions = {}): Promise<void> {
    if (options.keepalive === true) keepalive = true;
    if (debounce !== null) {
      clock.clearTimeout(debounce);
      debounce = null;
    }
    if (running !== null) {
      rerun = true;
      return running;
    }
    running = run()
      .catch(() => {
        // `pass` turns every failure into state; nothing is expected here, and nothing is lost.
      })
      .finally(() => {
        running = null;
        keepalive = false;
        emit();
      });
    emit();
    return running;
  }

  let lastImport: { key: string; trioId: string; deckIds: readonly string[] } | null = null;

  function importedDeck(value: unknown, fallback: DeckItem): DeckItem {
    const read = deckFrom(value);
    return read !== null && read.id === fallback.id ? read : fallback;
  }

  function importedTrio(value: unknown, fallback: TrioItem): TrioItem {
    const read = trioFrom(value);
    return read !== null && read.id === fallback.id ? read : fallback;
  }

  async function importTrio(init: TrioImport): Promise<TrioImportResult> {
    await flush();
    // What the caps are checked against must be what the server holds: a deletion made to make
    // room that has not landed would pass here and be refused there, after the player had deleted.
    if (hasWork()) {
      return { ok: false, message: offline ? IMPORT_OFFLINE_MESSAGE : IMPORT_UNSAVED_MESSAGE };
    }
    const filled = init.slots.filter((slot) => slot !== null).length;
    const room = checkImportRoom({
      saved: { decks: decks.length, trios: trios.length },
      limits: { decks: limits.decks, trios: limits.trios },
      adding: { decks: filled, trios: 1 },
    });
    if (!room.ok) return { ok: false, message: room.message };

    const key = JSON.stringify(init);
    const ids =
      lastImport !== null && lastImport.key === key
        ? lastImport
        : { key, trioId: newId(), deckIds: init.slots.map(() => newId()) };
    lastImport = ids;

    // One millisecond apart, in slot order, as the server stamps them (R341), so the list shows
    // them in their slots' order before and after the server's answer.
    const now = clock.now();
    let order = 0;
    const deckItems = init.slots.map((slot, at): DeckItem | null => {
      if (slot === null) return null;
      const stamped = now + order;
      order += 1;
      return {
        id: ids.deckIds[at] ?? newId(),
        name: deckNameForSave(slot.name, limits.nameLength),
        cards: [...slot.cards],
        // D5: an import carries no portrait; the deck reads `vanilla` until the player picks one.
        // R1297000: nor any playmat, backdrop or card back.
        portrait: null,
        catalogVersion,
        createdAt: stamped,
        updatedAt: stamped,
      };
    });
    const slotIds = [0, 1, 2].map((at) => deckItems[at]?.id ?? null) as TrioSlots;
    const trioItem: TrioItem = {
      id: ids.trioId,
      name: trioNameForSave(init.name, limits.nameLength),
      deckIds: slotIds,
      createdAt: now,
      updatedAt: now,
    };
    const input: TrioImportInput = {
      catalogVersion,
      trio: { id: trioItem.id, name: trioItem.name },
      slots: [0, 1, 2].map((at) => {
        const deck = deckItems[at] ?? null;
        return deck === null ? null : { id: deck.id, name: deck.name, cards: [...deck.cards] };
      }) as TrioImportInput["slots"],
    };

    let answer: unknown;
    try {
      answer = await api.importTrio(input);
    } catch (cause: unknown) {
      const failure = failureOf(cause);
      if (failure.kind === "offline") return { ok: false, message: IMPORT_OFFLINE_MESSAGE };
      if (failure.kind === "transient") return { ok: false, message: `Nothing was imported (${failure.message}). Try again.` };
      return { ok: false, message: failure.message };
    }
    lastImport = null;

    const body = isRecord(answer) ? answer : {};
    const answered = Array.isArray(body.decks) ? body.decks : [];
    const known = new Set(decks.map((entry) => entry.item.id));
    for (const deck of deckItems) {
      if (deck === null || known.has(deck.id)) continue;
      const saved = importedDeck(answered.find((entry) => isRecord(entry) && entry.id === deck.id), deck);
      decks = [...decks, tracked(saved, false)];
    }
    decks.sort(byAge);
    if (!trios.some((entry) => entry.item.id === trioItem.id)) {
      trios = [...trios, tracked(importedTrio(body.trio, trioItem), false)];
      trios.sort(byAge);
    }
    changed();
    return { ok: true, trioId: trioItem.id };
  }

  async function deckVersions(deckId: string): Promise<DeckHistoryResult> {
    await flush();
    try {
      return { ok: true, versions: deckVersionsFrom(await api.getDeckVersions(deckId)) };
    } catch (cause: unknown) {
      const failure = failureOf(cause);
      return { ok: false, message: failure.kind === "offline" ? HISTORY_OFFLINE_MESSAGE : failure.message };
    }
  }

  async function restoreDeckVersion(deckId: string, versionId: string): Promise<DeckHistoryResult> {
    await flush();
    const entry = decks.find((candidate) => candidate.item.id === deckId);
    if (entry === undefined) return { ok: false, message: NO_SUCH_DECK_MESSAGE };
    if (entry.dirty) return { ok: false, message: offline ? RESTORE_OFFLINE_MESSAGE : RESTORE_UNSAVED_MESSAGE };
    const rev = entry.rev;

    let answer: unknown;
    try {
      answer = await api.restoreDeckVersion(deckId, versionId, catalogVersion);
    } catch (cause: unknown) {
      const failure = failureOf(cause);
      return { ok: false, message: failure.kind === "offline" ? RESTORE_OFFLINE_MESSAGE : failure.message };
    }

    const body = isRecord(answer) ? answer : {};
    const saved = deckFrom(body.deck);
    // An edit made while the restore was on its way is newer than it, and saves over it as any edit does.
    const current = decks.find((candidate) => candidate.item.id === deckId);
    if (saved !== null && saved.id === deckId && current === entry && !current.dirty && current.rev === rev) {
      decks = decks.map((candidate) => (candidate === current ? tracked(saved, false) : candidate));
      changed();
    }
    return { ok: true, versions: deckVersionsFrom(body) };
  }

  const onPageHide = (): void => {
    void flush();
  };
  const onVisibility = (): void => {
    if (typeof document !== "undefined" && document.visibilityState === "hidden") void flush();
  };
  const onOnline = (): void => {
    void flush();
  };

  const store: DeckStore = {
    getSnapshot() {
      if (snapshot === null) {
        const unsynced = new Set<string>();
        const refused = new Map<string, string>();
        for (const entry of [...decks, ...trios]) {
          if (entry.dirty) unsynced.add((entry.item as { id: string }).id);
          if (entry.refused !== null) refused.set((entry.item as { id: string }).id, entry.refused);
        }
        // R2119000: a hidden deck is in no list and no trio's slot; a hidden trio is in no list.
        const hiddenDeck = hidden?.kind === "deck" ? hidden.id : null;
        snapshot = {
          decks: decks.filter((entry) => !isHidden(entry.item.id)).map((entry) => entry.item),
          trios: trios
            .filter((entry) => !isHidden(entry.item.id))
            .map((entry) =>
              hiddenDeck !== null && entry.item.deckIds.includes(hiddenDeck)
                ? { ...entry.item, deckIds: entry.item.deckIds.map((slot) => (slot === hiddenDeck ? null : slot)) as TrioSlots }
                : entry.item,
            ),
          status: statusNow(),
          unsynced,
          refused,
          limits,
          held: hidden,
        };
      }
      return snapshot;
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    createDeck(init) {
      if (decks.length >= limits.decks) return null;
      const now = clock.now();
      const item: DeckItem = {
        id: newId(),
        name: init.name,
        cards: [...(init.cards ?? [])],
        portrait: init.portrait ?? null,
        ...pickedDeckCosmetics(init),
        catalogVersion,
        createdAt: now,
        updatedAt: now,
      };
      const entry = tracked(item, true);
      decks = [...decks, entry];
      changed();
      schedule();
      return item.id;
    },

    updateDeck(id, patch) {
      const entry = decks.find((candidate) => candidate.item.id === id);
      if (entry === undefined) return;
      const name = patch.name ?? entry.item.name;
      const cards = patch.cards ?? entry.item.cards;
      const portrait = patch.portrait === undefined ? entry.item.portrait : patch.portrait;
      // R1297000: each deck slot merges as the portrait does; null clears a pick, and an absent
      // key and a null are the same pick, none.
      const picks = {
        tableCosmetic: patch.tableCosmetic === undefined ? (entry.item.tableCosmetic ?? null) : patch.tableCosmetic,
        fieldCosmetic: patch.fieldCosmetic === undefined ? (entry.item.fieldCosmetic ?? null) : patch.fieldCosmetic,
        cardBackCosmetic: patch.cardBackCosmetic === undefined ? (entry.item.cardBackCosmetic ?? null) : patch.cardBackCosmetic,
      };
      const samePicks =
        picks.tableCosmetic === (entry.item.tableCosmetic ?? null) &&
        picks.fieldCosmetic === (entry.item.fieldCosmetic ?? null) &&
        picks.cardBackCosmetic === (entry.item.cardBackCosmetic ?? null);
      if (name === entry.item.name && sameList(cards, entry.item.cards) && portrait === entry.item.portrait && samePicks) return;
      const { tableCosmetic: _table, fieldCosmetic: _field, cardBackCosmetic: _cardBack, ...rest } = entry.item;
      edited(entry, { ...rest, name, cards: [...cards], portrait, ...pickedDeckCosmetics(picks), catalogVersion, updatedAt: clock.now() });
    },

    deleteDeck(id) {
      const gone = decks.find((entry) => entry.item.id === id);
      if (gone === undefined) return;
      removed.set(id, { kind: "deck", entry: gone });
      if (isHidden(id)) hidden = null;
      decks = decks.filter((entry) => entry.item.id !== id);
      deletingDecks.add(id);
      for (const trio of trios) {
        if (!trio.item.deckIds.includes(id)) continue;
        const slots = trio.item.deckIds.map((slot) => (slot === id ? null : slot)) as TrioSlots;
        trio.item = { ...trio.item, deckIds: slots, updatedAt: clock.now() };
        trio.dirty = true;
        trio.rev += 1;
        trio.refused = null;
        trio.blocked = false;
      }
      changed();
      schedule();
    },

    createTrio(init) {
      if (trios.length >= limits.trios) return null;
      const now = clock.now();
      const item: TrioItem = {
        id: newId(),
        name: init.name,
        deckIds: [...(init.deckIds ?? [null, null, null])] as TrioSlots,
        createdAt: now,
        updatedAt: now,
      };
      trios = [...trios, tracked(item, true)];
      changed();
      schedule();
      return item.id;
    },

    updateTrio(id, patch) {
      const entry = trios.find((candidate) => candidate.item.id === id);
      if (entry === undefined) return;
      const name = patch.name ?? entry.item.name;
      // R2119000: the editor shows a hidden deck's slot empty, so an empty slot there keeps it.
      const kept = entry.item.deckIds;
      const deckIds =
        patch.deckIds === undefined
          ? kept
          : (patch.deckIds.map((slot, at) => {
              const before = kept[at] ?? null;
              return slot === null && hidden?.kind === "deck" && before === hidden.id ? before : slot;
            }) as TrioSlots);
      if (name === entry.item.name && sameSlots(deckIds, entry.item.deckIds)) return;
      edited(entry, { ...entry.item, name, deckIds: [...deckIds] as TrioSlots, updatedAt: clock.now() });
    },

    deleteTrio(id) {
      const gone = trios.find((entry) => entry.item.id === id);
      if (gone === undefined) return;
      removed.set(id, { kind: "trio", entry: gone });
      if (isHidden(id)) hidden = null;
      trios = trios.filter((entry) => entry.item.id !== id);
      deletingTrios.add(id);
      changed();
      schedule();
    },

    importTrio,

    deckVersions,

    restoreDeckVersion,

    hide(kind, id) {
      const list: readonly Tracked<{ id: string }>[] = kind === "deck" ? decks : trios;
      if (!list.some((entry) => entry.item.id === id)) return;
      hidden = { kind, id };
      emit();
    },

    unhide() {
      if (hidden === null) return;
      hidden = null;
      emit();
      // An edit the hold kept back goes now.
      if (hasWork()) schedule();
    },

    flush,

    start() {
      stopped = false;
      if (started || typeof window === "undefined") return;
      started = true;
      window.addEventListener("pagehide", onPageHide);
      window.addEventListener("online", onOnline);
      if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibility);
    },

    stop() {
      if (started && typeof window !== "undefined") {
        window.removeEventListener("pagehide", onPageHide);
        window.removeEventListener("online", onOnline);
        if (typeof document !== "undefined") document.removeEventListener("visibilitychange", onVisibility);
      }
      started = false;
      if (retry !== null) {
        clock.clearTimeout(retry);
        retry = null;
      }
      // One last send of what is unsaved (the mirror already holds it either way), then quiet.
      const unsaved = hasWork();
      stopped = true;
      if (unsaved) void flush();
      else if (debounce !== null) {
        clock.clearTimeout(debounce);
        debounce = null;
      }
    },
  };

  // Work the merge found unsaved (an offline edit from the last visit) goes as soon as it can.
  if (hasWork()) schedule();
  persist();

  return store;
}
