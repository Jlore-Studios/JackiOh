// `resume.ts` holds only page setup; worker-side logs name hidden AI cards (SPEC §9.9, R668; rule 7).
// R768 retains finished games for replay. Unavailable storage is no save; failed writes never end play.

import type { PracticeReplay } from "../wire/generated/PracticeReplay.ts";
import type { PracticeReplaySetup } from "../wire/generated/PracticeReplaySetup.ts";
import type { PracticeSave } from "../wire/generated/PracticeSave.ts";

/**
 * R668, R767000, R768: the worker-only state a game in progress resumes from, the setup a kept game was dealt
 * and a kept game itself, as the practice session writes them (`crates/shared`, #3099).
 */
export type { PracticeReplay, PracticeReplaySetup, PracticeSave };

export type PracticeSaveStore = {
  /** Settles once the stored save, if any, is read; it never rejects. */
  readonly ready: Promise<void>;
  read(): PracticeSave | null;
  write(save: PracticeSave | null): void;
  /** R3476000: the run fight in progress, in its own slot; never the free game's. */
  readRunFight(): PracticeSave | null;
  writeRunFight(save: PracticeSave | null): void;
  /** R768: the kept finished games, oldest first. */
  readReplays(): readonly PracticeReplay[];
  writeReplays(replays: readonly PracticeReplay[]): void;
};

export const PRACTICE_SAVE_DB = "jackioh.practice";
const STORE = "save";
const KEY = "game";
const RUN_FIGHT_KEY = "runFight";
const REPLAYS_KEY = "replays";

/** A store that forgets on reload: jsdom, a browser without IndexedDB, and the tests. */
export function memorySaveStore(
  initial: PracticeSave | null = null,
  replays: readonly PracticeReplay[] = [],
  runFight: PracticeSave | null = null,
): PracticeSaveStore {
  let save = initial;
  let fight = runFight;
  let kept = [...replays];
  return {
    ready: Promise.resolve(),
    read: () => save,
    write: (next) => {
      save = next;
    },
    readRunFight: () => fight,
    writeRunFight: (next) => {
      fight = next;
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
  // R3476000: the run fight's own slot, read at boot and written per answer, beside the free game's.
  let runFight: PracticeSave | null = null;
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
    const fight: unknown = await request(db.transaction(STORE, "readonly").objectStore(STORE).get(RUN_FIGHT_KEY));
    runFight = isSave(fight) ? fight : null;
    const kept: unknown = await request(db.transaction(STORE, "readonly").objectStore(STORE).get(REPLAYS_KEY));
    replays = Array.isArray(kept) ? (kept as unknown[]).filter(isReplay) : [];
  }

  const ready = boot().catch(() => {
    db = null;
    save = null;
    runFight = null;
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
    readRunFight: () => runFight,
    writeRunFight(next) {
      runFight = next;
      if (db === null) return;
      try {
        const store = db.transaction(STORE, "readwrite").objectStore(STORE);
        if (next === null) store.delete(RUN_FIGHT_KEY);
        else store.put(next, RUN_FIGHT_KEY);
      } catch {
        // As for a free game's save: this answer's fight is lost, and the fight goes on.
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

/**
 * R3476000: forget the kept run fight, and only it (a finished fight, an abandoned run, a new
 * run). Fire-and-forget for the page: it deletes the run fight's key without ever reading the
 * log (rule 7), and a missing database, store or key is no kept fight to forget.
 */
export function clearStoredRunFight(): void {
  try {
    if (typeof indexedDB === "undefined") return;
    const open = indexedDB.open(PRACTICE_SAVE_DB, 1);
    open.onsuccess = () => {
      let db: IDBDatabase | null = null;
      try {
        db = open.result;
        const tx = db.transaction(STORE, "readwrite");
        tx.oncomplete = () => db?.close();
        tx.onerror = () => db?.close();
        tx.objectStore(STORE).delete(RUN_FIGHT_KEY);
      } catch {
        // No store, or a transaction that would not start: nothing kept to forget.
        try {
          db?.close();
        } catch {
          // A close that fails leaves nothing behind worth reporting.
        }
      }
    };
  } catch {
    // A database that will not open holds no fight to forget.
  }
}
