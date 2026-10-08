// R668: the worker's store for a free game in progress (saveStore.ts), against a minimal IndexedDB
// fake (jsdom has none): the read at boot, writes mirrored in order, and storage that fails.

import { describe, expect, it } from "vitest";

import { PRACTICE_SAVE_DB, indexedDbSaveStore, memorySaveStore, type PracticeReplay, type PracticeSave } from "./saveStore.ts";

const SAVE: PracticeSave = {
  catalog: "v-test",
  config: { seed: "s1", difficulty: "easy", humanSeat: "p1", deck: { kind: "random" } },
  log: [{ type: "endTurn", playerId: "p1", nonce: "h0" }],
  aiCursor: 7,
  hash: "0badc0de",
};

const REPLAY: PracticeReplay = {
  catalog: SAVE.catalog,
  config: SAVE.config,
  log: SAVE.log,
  hash: "0badc0de",
  endSeat: "p1",
  summary: { game: 1, endedAt: 1234, result: "win", turns: 3, steps: 2, portraits: { p1: "vanilla", p2: "gary" } },
};

type FakeRequest = { result: unknown; error: Error | null; onsuccess: (() => void) | null; onerror: (() => void) | null };

/** One database's one object store, as a Map; every request answers on a macrotask. */
function fakeIndexedDb(options: { failOpen?: boolean } = {}): { factory: IDBFactory; rows: Map<string, unknown>; opened: string[] } {
  const rows = new Map<string, unknown>();
  const stores = new Set<string>();
  const opened: string[] = [];

  function answer(result: () => unknown): FakeRequest {
    const req: FakeRequest = { result: undefined, error: null, onsuccess: null, onerror: null };
    setTimeout(() => {
      req.result = result();
      req.onsuccess?.();
    }, 0);
    return req;
  }

  const db = {
    createObjectStore: (name: string) => stores.add(name),
    transaction: (name: string) => {
      if (!stores.has(name)) throw new Error(`no store ${name}`);
      return {
        objectStore: () => ({
          get: (key: string) => answer(() => rows.get(key)),
          put: (value: unknown, key: string) => answer(() => rows.set(key, structuredClone(value))),
          delete: (key: string) => answer(() => rows.delete(key)),
        }),
      };
    },
  };

  const factory = {
    open: (name: string) => {
      opened.push(name);
      const req = { result: db, error: null, onsuccess: null, onerror: null, onupgradeneeded: null } as FakeRequest & {
        onupgradeneeded: (() => void) | null;
      };
      // The upgrade runs before success, as a first open's does.
      setTimeout(() => {
        if (options.failOpen === true) {
          req.error = new Error("blocked");
          req.onerror?.();
          return;
        }
        if (stores.size === 0) req.onupgradeneeded?.();
        req.onsuccess?.();
      }, 0);
      return req;
    },
  };
  return { factory: factory as unknown as IDBFactory, rows, opened };
}

function macrotasks(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 5));
}

describe("R668 the practice worker's save store", () => {
  it("R668 memory keeps what is written until it is cleared", async () => {
    const store = memorySaveStore();
    await store.ready;
    expect(store.read()).toBeNull();
    store.write(SAVE);
    expect(store.read()).toEqual(SAVE);
    store.write(null);
    expect(store.read()).toBeNull();
  });

  it("R668 IndexedDB: a write outlives the store, and the next store reads it at boot", async () => {
    const idb = fakeIndexedDb();
    const first = indexedDbSaveStore(idb.factory);
    await first.ready;
    expect(idb.opened).toEqual([PRACTICE_SAVE_DB]);
    expect(first.read()).toBeNull();
    first.write({ ...SAVE, hash: "first" });
    first.write(SAVE);
    expect(first.read(), "read is synchronous").toEqual(SAVE);
    await macrotasks();

    const second = indexedDbSaveStore(idb.factory);
    expect(second.read(), "nothing before ready").toBeNull();
    await second.ready;
    expect(second.read(), "the last write wins").toEqual(SAVE);

    second.write(null);
    await macrotasks();
    const third = indexedDbSaveStore(idb.factory);
    await third.ready;
    expect(third.read()).toBeNull();
  });

  it("R668 a malformed row is no save, and a database that will not open is memory only", async () => {
    const idb = fakeIndexedDb();
    const boot = indexedDbSaveStore(idb.factory);
    await boot.ready;
    idb.rows.set("game", { catalog: 3, log: "no" });
    const reread = indexedDbSaveStore(idb.factory);
    await reread.ready;
    expect(reread.read()).toBeNull();

    const blocked = indexedDbSaveStore(fakeIndexedDb({ failOpen: true }).factory);
    await expect(blocked.ready).resolves.toBeUndefined();
    expect(blocked.read()).toBeNull();
    blocked.write(SAVE);
    expect(blocked.read()).toEqual(SAVE);
  });
});

describe("R768 the finished games beside the save", () => {
  it("R768 memory keeps the finished games it is given and written, apart from the save", async () => {
    const store = memorySaveStore(SAVE, [REPLAY]);
    await store.ready;
    expect(store.readReplays()).toEqual([REPLAY]);
    store.write(null);
    expect(store.readReplays(), "clearing the save leaves the games").toEqual([REPLAY]);
    store.write(SAVE);
    store.writeReplays([REPLAY, { ...REPLAY, summary: { ...REPLAY.summary, game: 2 } }]);
    expect(store.readReplays().map((replay) => replay.summary.game)).toEqual([1, 2]);
    expect(store.read(), "writing the games leaves the save").toEqual(SAVE);
    store.writeReplays([]);
    expect(store.readReplays()).toEqual([]);
  });

  it("R768 IndexedDB: the finished games outlive the store beside the save, and a malformed row is none", async () => {
    const idb = fakeIndexedDb();
    const first = indexedDbSaveStore(idb.factory);
    await first.ready;
    expect(first.readReplays()).toEqual([]);
    first.write(SAVE);
    first.writeReplays([REPLAY]);
    expect(first.readReplays(), "read is synchronous").toEqual([REPLAY]);
    await macrotasks();

    const second = indexedDbSaveStore(idb.factory);
    await second.ready;
    expect(second.read()).toEqual(SAVE);
    expect(second.readReplays()).toEqual([REPLAY]);

    idb.rows.set("replays", [REPLAY, { catalog: 3 }]);
    const mixed = indexedDbSaveStore(idb.factory);
    await mixed.ready;
    expect(mixed.readReplays(), "a malformed game is dropped, the rest kept").toEqual([REPLAY]);

    idb.rows.set("replays", "no");
    const wrong = indexedDbSaveStore(idb.factory);
    await wrong.ready;
    expect(wrong.readReplays()).toEqual([]);

    const blocked = indexedDbSaveStore(fakeIndexedDb({ failOpen: true }).factory);
    await blocked.ready;
    blocked.writeReplays([REPLAY]);
    expect(blocked.readReplays(), "a database that will not open is memory only").toEqual([REPLAY]);
  });
});
