// Test fakes for the deck workshop: an in-memory server with the refusals that matter to the store
// (R256), an in-memory storage and a manual clock. Only tests import this file.

import {
  DECK_NAME_MAX_LENGTH,
  MAX_SAVED_DECKS,
  MAX_SAVED_TRIOS,
} from "@jackioh/server-config";
import { pickedDeckCosmetics } from "../../cards/cosmetics.ts";
import {
  ApiRequestError,
  ApiUnreachableError,
  type DeckInput,
  type DecksResponse,
  type DeckVersion,
  type SavedDeck,
  type SavedTrio,
  type SendOptions,
  type TrioImportInput,
  type TrioInput,
  type TrioSlots,
} from "../../net/api.ts";
import type { DeckSyncApi, StorageLike, SyncClock } from "./sync.ts";

export const TEST_PROFILE = "profile-test";

export function savedDeck(id: string, name: string, cards: readonly string[], createdAt: number, catalogVersion = "test"): SavedDeck {
  return { id, name, cards: [...cards], catalogVersion, portrait: null, createdAt, updatedAt: createdAt };
}

export function savedTrio(id: string, name: string, deckIds: TrioSlots, createdAt: number): SavedTrio {
  return { id, name, deckIds: [...deckIds] as TrioSlots, createdAt, updatedAt: createdAt };
}

export function decksResponse(decks: SavedDeck[] = [], trios: SavedTrio[] = [], catalogVersion = "test"): DecksResponse {
  return {
    catalogVersion,
    decks,
    trios,
    limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
  };
}

/** Microtask turns `settle` gives a save: a pass awaits a few promises per request. */
const SETTLE_TURNS = 50;

/** Lets every pending promise callback run: the fake server answers in microtasks. */
export async function settle(): Promise<void> {
  for (let turn = 0; turn < SETTLE_TURNS; turn += 1) {
    await Promise.resolve();
  }
}

export type ManualClock = SyncClock & { advance(ms: number): Promise<void>; pending(): number };

/** A clock whose timers fire only when `advance` passes their moment. */
export function manualClock(start = 1_000): ManualClock {
  let now = start;
  let nextId = 1;
  const timers = new Map<number, { at: number; run: () => void }>();
  return {
    now: () => now,
    setTimeout(run, ms) {
      const id = nextId;
      nextId += 1;
      timers.set(id, { at: now + ms, run });
      return id;
    },
    clearTimeout(handle) {
      timers.delete(handle as number);
    },
    pending: () => timers.size,
    async advance(ms) {
      const target = now + ms;
      for (;;) {
        let due: [number, { at: number; run: () => void }] | null = null;
        for (const entry of timers) {
          if (entry[1].at <= target && (due === null || entry[1].at < due[1].at)) due = entry;
        }
        if (due === null) break;
        timers.delete(due[0]);
        now = due[1].at;
        due[1].run();
        await settle();
      }
      now = target;
      await settle();
    },
  };
}

export function memoryStorage(): StorageLike & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return {
    data,
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => {
      data.set(key, value);
    },
    removeItem: (key) => {
      data.delete(key);
    },
  };
}

export type ServerCall = {
  op: "putDeck" | "deleteDeck" | "putTrio" | "deleteTrio" | "importTrio" | "getDeckVersions" | "restoreDeckVersion";
  id: string;
  body?: DeckInput | TrioInput | TrioImportInput;
  versionId?: string;
  /** R2119000: the request was sent with `keepalive`, as the page went away. */
  keepalive?: true;
};

/** The server's 404 for a deck or version that is not the caller's. */
function notFound(what: "deck" | "version"): ApiRequestError {
  return new ApiRequestError(404, { code: "not_found", message: `There is no such ${what}.` });
}

function sentWith(options: SendOptions | undefined): { keepalive?: true } {
  return options?.keepalive === true ? { keepalive: true } : {};
}

/** An in-memory `/api/decks` and `/api/trios`, with the refusals the store has to handle. */
export function fakeDeckServer() {
  const decks = new Map<string, DeckInput>();
  const trios = new Map<string, TrioInput>();
  /** R2100000: each deck's kept versions, newest first, as a test seeds them (a save keeps none here). */
  const versions = new Map<string, DeckVersion[]>();
  const calls: ServerCall[] = [];
  const state = {
    offline: false,
    refusals: new Map<string, ApiRequestError>(),
  };

  function gate(call: ServerCall): void {
    calls.push(call);
    if (state.offline) throw new ApiUnreachableError(new TypeError("Failed to fetch"));
    const refusal = state.refusals.get(call.id);
    if (refusal !== undefined) throw refusal;
  }

  const api: DeckSyncApi = {
    async putDeck(id, input) {
      gate({ op: "putDeck", id, body: input });
      decks.set(id, input);
      return { deck: { id } };
    },
    async deleteDeck(id, options) {
      gate({ op: "deleteDeck", id, ...sentWith(options) });
      return { deleted: decks.delete(id) };
    },
    async putTrio(id, input) {
      gate({ op: "putTrio", id, body: input });
      if (input.deckIds.some((deckId) => deckId !== null && !decks.has(deckId))) {
        throw new ApiRequestError(409, {
          code: "conflict",
          message: "That trio names a deck that is not saved.",
          details: { unknownDeck: true },
        });
      }
      trios.set(id, input);
      return { trio: { id } };
    },
    async deleteTrio(id, options) {
      gate({ op: "deleteTrio", id, ...sentWith(options) });
      return { deleted: trios.delete(id) };
    },
    async importTrio(input) {
      gate({ op: "importTrio", id: input.trio.id, body: input });
      const catalogVersion = input.catalogVersion;
      for (const deck of input.slots) {
        if (deck !== null) decks.set(deck.id, { name: deck.name, cards: deck.cards, catalogVersion });
      }
      const deckIds = input.slots.map((deck) => deck?.id ?? null) as TrioSlots;
      trios.set(input.trio.id, { name: input.trio.name, deckIds });
      return {};
    },
    async getDeckVersions(id) {
      gate({ op: "getDeckVersions", id });
      if (!decks.has(id)) throw notFound("deck");
      return { versions: versions.get(id) ?? [] };
    },
    async restoreDeckVersion(id, versionId, catalogVersion) {
      gate({ op: "restoreDeckVersion", id, versionId });
      const deck = decks.get(id);
      if (deck === undefined) throw notFound("deck");
      const kept = versions.get(id) ?? [];
      const version = kept.find((entry) => entry.id === versionId);
      if (version === undefined) throw notFound("version");
      // As the server: the version's name and cards, the deck's portrait and picks, kept as a new version.
      const savedAt = (kept[0]?.savedAt ?? 0) + 1;
      decks.set(id, { ...deck, name: version.name, cards: [...version.cards], catalogVersion });
      const listed = [{ ...version, id: `${versionId}-restored`, savedAt }, ...kept];
      versions.set(id, listed);
      // The fake keeps no creation time, so the answer's is the restore's.
      const saved = savedDeck(id, version.name, version.cards, savedAt, catalogVersion);
      return { deck: { ...saved, portrait: deck.portrait ?? null, ...pickedDeckCosmetics(deck) }, versions: listed };
    },
  };

  function seed(response: DecksResponse): void {
    for (const deck of response.decks) {
      decks.set(deck.id, { name: deck.name, cards: deck.cards, catalogVersion: deck.catalogVersion });
    }
    for (const trio of response.trios) trios.set(trio.id, { name: trio.name, deckIds: trio.deckIds });
  }

  return { api, calls, decks, trios, versions, state, seed };
}

/** An api that accepts everything and remembers nothing. */
export function quietDeckApi(): DeckSyncApi {
  return {
    putDeck: async () => ({}),
    deleteDeck: async () => ({}),
    putTrio: async () => ({}),
    deleteTrio: async () => ({}),
    importTrio: async () => ({}),
    getDeckVersions: async () => ({ versions: [] }),
    restoreDeckVersion: async () => ({}),
  };
}
