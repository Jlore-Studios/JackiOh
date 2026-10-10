// `resume.ts` holds only page setup; worker-side logs name hidden AI cards (SPEC §9.9, R668; rule 7).
// R768 retains finished games for replay. Unavailable storage is no save; failed writes never end play.

import type { Action, PlayerId } from "@jackioh/shared";

import type { PracticeReplaySummary, PracticeStartConfig } from "./protocol.ts";

/** R668: worker-only start state, action log, AI cursor, catalog version and state hash for resuming. */
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
  hash: string;
  /** R677: the seat the human played when the game ended. */
  endSeat: PlayerId;
  summary: PracticeReplaySummary;
};

export type PracticeSaveStore = {
  /** Settles once the stored save, if any, is read; it never rejects. */
  readonly ready: Promise<void>;
  read(): PracticeSave | null;
  write(save: PracticeSave | null): void;
  /** R768: the kept finished games, oldest first. */
  readReplays(): readonly PracticeReplay[];
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

export function defaultSaveStore(): PracticeSaveStore {
  return typeof indexedDB === "undefined" ? memorySaveStore() : indexedDbSaveStore(indexedDB);
}
