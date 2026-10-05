// Where the practice worker keeps a free game in progress, so a reload or a closed tab picks it up
// again (SPEC §9.9, R662).
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

import type { Action } from "@jackioh/shared";

import type { PracticeStartConfig } from "./protocol.ts";

/**
 * R662: what a free practice game needs to come back: the start config as the worker had it (its
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

export type PracticeSaveStore = {
  /** Settles once the stored save, if any, is read; it never rejects. */
  readonly ready: Promise<void>;
  read(): PracticeSave | null;
  /** null clears. */
  write(save: PracticeSave | null): void;
};

export const PRACTICE_SAVE_DB = "jackioh.practice";
const STORE = "save";
const KEY = "game";

/** A store that forgets on reload: jsdom, a browser without IndexedDB, and the tests. */
export function memorySaveStore(initial: PracticeSave | null = null): PracticeSaveStore {
  let save = initial;
  return {
    ready: Promise.resolve(),
    read: () => save,
    write: (next) => {
      save = next;
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
  let db: IDBDatabase | null = null;

  async function boot(): Promise<void> {
    const open = factory.open(PRACTICE_SAVE_DB, 1);
    open.onupgradeneeded = () => {
      open.result.createObjectStore(STORE);
    };
    db = await request(open);
    const stored: unknown = await request(db.transaction(STORE, "readonly").objectStore(STORE).get(KEY));
    save = isSave(stored) ? stored : null;
  }

  const ready = boot().catch(() => {
    db = null;
    save = null;
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
  };
}

/** IndexedDB where the scope has it, else memory. */
export function defaultSaveStore(): PracticeSaveStore {
  return typeof indexedDB === "undefined" ? memorySaveStore() : indexedDbSaveStore(indexedDB);
}
