// Where the practice worker keeps a free game in progress, so a reload or a closed tab picks it up
// again (SPEC §9.9, R668).
//
// The save holds the action log, and the log holds the AI's actions, which name its hidden cards, so
// it never crosses to the page (rule 7): the worker keeps it itself, in IndexedDB, which a worker
// has and `localStorage` is not. The page keeps only the setup it chose (`resume.ts`), which is how
// it knows to ask for a resume at all.
//
// The core reads and writes synchronously (`handle` is synchronous), so the store holds the save in
// memory and mirrors every write to IndexedDB in order; `ready` is the one read at boot, which the
// hosts wait for before the first request. Storage is untrusted and may be missing (a private
// window, blocked site data, jsdom): a failed open or read is no save, and a failed write loses only
// the resume, never the game.
//
// The store also keeps the finished free games for their replays (R768), under a second key of the
// same object store: each holds its log too, so it never crosses to the page either.

import type { Action, PlayerId } from "@jackioh/shared";

import type { PracticeReplaySummary, PracticeStartConfig } from "./protocol.ts";

/**
 * R668: what a free practice game needs to come back: the start config as the worker had it (its
 * last board included), the action log, the AI stream's cursor, the catalog version and the state's
 * hash. Worker-side only.
 */
export type PracticeSave = {
  catalog: string;
  config: PracticeStartConfig;
  log: Action[];
  aiCursor: number;
  hash: string;
};

/** R768: a finished free game kept for its replay. Worker-side only: the log names the AI's hidden cards. */
export type PracticeReplay = {
  catalog: string;
  config: PracticeStartConfig;
  log: Action[];
  /** `hashState` of the final state. */
  hash: string;
  /** R677: the seat the human played when the game ended. */
  endSeat: PlayerId;
  summary: PracticeReplaySummary;
};

export type PracticeSaveStore = {
  /** Settles once the stored save, if any, is read; it never rejects. */
  readonly ready: Promise<void>;
  read(): PracticeSave | null;
  /** null clears. */
  write(save: PracticeSave | null): void;
  /** R768: the kept finished games, oldest first. */
  readReplays(): readonly PracticeReplay[];
  /** Replaces them all. */
  writeReplays(replays: readonly PracticeReplay[]): void;
};

export const PRACTICE_SAVE_DB = "jackioh.practice";
const STORE = "save";
const KEY = "game";
const REPLAYS_KEY = "replays";

/** A store that forgets on reload: jsdom, a browser without IndexedDB, and the tests. */
export function memorySaveStore(initial: PracticeSave | null = null, replays: readonly PracticeReplay[] = []): PracticeSaveStore {
  let save = initial;
  let kept = [...replays];
  return {
    ready: Promise.resolve(),
    read: () => save,
    write: (next) => {
      save = next;
    },
    readReplays: () => kept,
    writeReplays: (next) => {
      kept = [...next];
    },
  };
}

function isSave(value: unknown): value is PracticeSave {
  if (typeof value !== "object" || value === null) return false;
  const save = value as Record<string, unknown>;
  const config = save.config as Record<string, unknown> | null | undefined;
  return (
    typeof save.catalog === "string" &&
    typeof save.hash === "string" &&
    typeof save.aiCursor === "number" &&
    Array.isArray(save.log) &&
    typeof config === "object" &&
    config !== null &&
    typeof config.seed === "string"
  );
}

function isReplay(value: unknown): value is PracticeReplay {
  if (typeof value !== "object" || value === null) return false;
  const replay = value as Record<string, unknown>;
  const config = replay.config as Record<string, unknown> | null | undefined;
  const summary = replay.summary as Record<string, unknown> | null | undefined;
  return (
    typeof replay.catalog === "string" &&
    typeof replay.hash === "string" &&
    Array.isArray(replay.log) &&
    (replay.endSeat === "p1" || replay.endSeat === "p2") &&
    typeof config === "object" &&
    config !== null &&
    typeof config.seed === "string" &&
    typeof summary === "object" &&
    summary !== null &&
    typeof summary.game === "number" &&
    typeof summary.steps === "number" &&
    typeof summary.portraits === "object"
  );
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    req.onsuccess = () => {
      resolve(req.result);
    };
    req.onerror = () => {
      reject(req.error ?? new Error("IndexedDB request failed"));
    };
  });
}

/** IndexedDB behind an in-memory copy. Writes are issued in order, one transaction each. */
export function indexedDbSaveStore(factory: IDBFactory): PracticeSaveStore {
  let save: PracticeSave | null = null;
  let replays: PracticeReplay[] = [];
  let db: IDBDatabase | null = null;

  async function boot(): Promise<void> {
    const open = factory.open(PRACTICE_SAVE_DB, 1);
    open.onupgradeneeded = () => {
      open.result.createObjectStore(STORE);
    };
    db = await request(open);
    const stored: unknown = await request(db.transaction(STORE, "readonly").objectStore(STORE).get(KEY));
    save = isSave(stored) ? stored : null;
    const kept: unknown = await request(db.transaction(STORE, "readonly").objectStore(STORE).get(REPLAYS_KEY));
    replays = Array.isArray(kept) ? (kept as unknown[]).filter(isReplay) : [];
  }

  const ready = boot().catch(() => {
    db = null;
    save = null;
    replays = [];
  });

  return {
    ready,
    read: () => save,
    write(next) {
      save = next;
      if (db === null) return;
      try {
        const store = db.transaction(STORE, "readwrite").objectStore(STORE);
        if (next === null) store.delete(KEY);
        else store.put(next, KEY);
      } catch {
        // A closed or full database: this answer's save is lost, and the game goes on.
      }
    },
    readReplays: () => replays,
    writeReplays(next) {
      replays = [...next];
      if (db === null) return;
      try {
        db.transaction(STORE, "readwrite").objectStore(STORE).put(replays, REPLAYS_KEY);
      } catch {
        // As for a save: the kept games of this answer are lost, and the game goes on.
      }
    },
  };
}

/** IndexedDB where the scope has it, else memory. */
export function defaultSaveStore(): PracticeSaveStore {
  return typeof indexedDB === "undefined" ? memorySaveStore() : indexedDbSaveStore(indexedDB);
}
