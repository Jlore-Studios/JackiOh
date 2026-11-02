// B15, B18–B21 (DR14 part D, #2705): the Dungeon Run's device store. The run's rules are `crates/run`'s:
// `../wasm/index.ts`'s run bindings are the fixtures' scripted ones (a Go enters a fight, an abandon or a
// lost fight ends the run), so what is proved here is what the store does with their answers: what it
// keeps on the device, folds back, settles and refuses.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";
import type { RunContent, RunFile, RunState } from "@jackioh/run";

import { DUNGEON_HISTORY_KEY, DUNGEON_PROGRESS_KEY, DUNGEON_RUN_KEY } from "./config.ts";
import { __resetDungeonForTests, dailyRunNumber, readRunHistory, useDungeon, type Dungeon } from "./store.ts";
import {
  CONTENT,
  EXTRACTABLE,
  IMP,
  ZERO_STATS,
  resetBindings,
  runState,
  seedFrom,
  setupOf,
  unlocks,
  wasmBindings,
} from "./test/fixtures.ts";

vi.mock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("./test/fixtures.ts")).wasmBindings,
}));

// The account's custom cards, which practice reads and the Sandbox's pool takes (B21).
const account = vi.hoisted(() => ({ cards: [] as CustomCard[] }));
vi.mock("./customCards.ts", () => ({ useSandboxCustomCards: (): CustomCard[] => account.cards }));
vi.mock("../net/gate.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  useAccount: () => ({ kind: "anonymous" }),
}));

const DAY = "2026-11-01";
const NEXT_DAY = "2026-11-02";

/** The content a run with the creator's Ember Imp deals from, which the plain content is not. */
const WITH_IMP: RunContent = {
  ...CONTENT,
  pool: [...CONTENT.pool, { id: IMP.id, name: "Ember Imp", rarity: "Common", set: "Core", cardType: "Unit", tags: [] }],
};

function stored(key: string): unknown {
  const raw = window.localStorage.getItem(key);
  return raw === null ? null : (JSON.parse(raw) as unknown);
}

function mount(): { current: Dungeon } {
  const { result } = renderHook(() => useDungeon());
  return {
    get current() {
      return result.current;
    },
  };
}

/** The store as a page that has just loaded finds it: nothing of the last page in memory. */
function reload(): { current: Dungeon } {
  cleanup();
  __resetDungeonForTests();
  return mount();
}

/** Run a verb as a handler runs it, so the hook renders what it changed. */
function press<T>(verb: () => T): T {
  let answer: T | undefined;
  act(() => {
    answer = verb();
  });
  return answer as T;
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(`${DAY}T12:00:00Z`));
  window.localStorage.clear();
  account.cards = [];
  __resetDungeonForTests();
  resetBindings();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  __resetDungeonForTests();
});

describe("the store's start", () => {
  it("starts a run on the content and the unlocks, and keeps its file on the device", () => {
    const dungeon = mount();
    expect(dungeon.current.run).toBeNull();
    expect(dungeon.current.startedAt).toBeNull();
    expect(dungeon.current.progress).toEqual({ unlocks: unlocks(), embers: 0 });
    expect(dungeon.current.content).toBe(CONTENT);

    expect(press(() => dungeon.current.start(setupOf()))).toBeNull();

    expect(wasmBindings.runCheckSetup).toHaveBeenCalledWith(setupOf(), unlocks(), CONTENT);
    expect(wasmBindings.runCreate).toHaveBeenCalledWith(setupOf(), unlocks(), CONTENT);
    expect(dungeon.current.run?.seed).toBe("ABCDEF");
    expect(dungeon.current.startedAt).toBe(Date.now());
    expect(stored(DUNGEON_RUN_KEY)).toEqual({
      v: 1,
      file: { setup: setupOf(), unlocks: unlocks(), log: [] },
      customCards: [],
      startedAt: Date.now(),
      daily: false,
    });
  });

  it("relays the run's refusal of a setup, and of its creation, word for word", () => {
    const dungeon = mount();

    wasmBindings.runCheckSetup.mockReturnValue("traininggrounds-005 is not unlocked: extract it at a mini-boss first");
    expect(press(() => dungeon.current.start(setupOf()))).toBe(
      "traininggrounds-005 is not unlocked: extract it at a mini-boss first",
    );
    expect(wasmBindings.runCreate).not.toHaveBeenCalled();

    wasmBindings.runCheckSetup.mockReturnValue(null);
    wasmBindings.runCreate.mockReturnValue({ error: "the content has no Boss enemy" });
    expect(press(() => dungeon.current.start(setupOf()))).toBe("the content has no Boss enemy");

    expect(dungeon.current.run).toBeNull();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
  });

  it("starts no second run while one is in progress", () => {
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf()));
    wasmBindings.runCreate.mockClear();

    expect(press(() => dungeon.current.start(setupOf({ seed: "QWERTY" })))).toBe("Finish or abandon the run in progress first.");

    expect(wasmBindings.runCreate).not.toHaveBeenCalled();
    expect(dungeon.current.run?.seed).toBe("ABCDEF");
  });
});

describe("B19 a run in progress survives a reload", () => {
  it("B19 a run in progress survives reload by folding its RunFile", () => {
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf({ curses: ["heavy-hands"] })));
    expect(press(() => dungeon.current.act({ type: "go", col: 1 }))).toBeNull();
    expect(dungeon.current.run?.phase).toEqual({ phase: "fight" });
    const startedAt = dungeon.current.startedAt;

    // The device holds the setup, the unlocks it began with and the actions taken.
    const file: RunFile = {
      setup: setupOf({ curses: ["heavy-hands"] }),
      unlocks: unlocks(),
      log: [{ type: "go", col: 1 }],
    };
    expect(stored(DUNGEON_RUN_KEY)).toEqual({ v: 1, file, customCards: [], startedAt, daily: false });

    // A reload folds it back, as it was.
    vi.setSystemTime(new Date(`${DAY}T18:00:00Z`));
    wasmBindings.runFold.mockClear();
    const again = reload();
    expect(wasmBindings.runFold).toHaveBeenCalledTimes(1);
    expect(wasmBindings.runFold).toHaveBeenCalledWith(file, CONTENT);
    expect(again.current.run).toEqual(dungeon.current.run);
    expect(again.current.run?.phase).toEqual({ phase: "fight" });
    expect(again.current.run?.row).toBe(0);
    // The timer's start is the run's own, not the reload's.
    expect(again.current.startedAt).toBe(startedAt);

    // And it goes on from there, kept again.
    expect(press(() => again.current.act({ type: "fightOver", won: true, hp: 20, damageDealt: 5, healingDone: 0, cardsDrawn: 3, turns: 4 }))).toBeNull();
    expect(again.current.run?.phase).toEqual({ phase: "map" });
    expect((stored(DUNGEON_RUN_KEY) as { file: RunFile }).file.log).toHaveLength(2);
  });

  it("B19 a reload folds the run on the custom cards it began with, not the ones the account has now", () => {
    wasmBindings.runContent.mockImplementation((cards) => (cards.length === 0 ? CONTENT : WITH_IMP));
    account.cards = [IMP];
    const dungeon = mount();
    expect(dungeon.current.content).toBe(WITH_IMP);
    press(() => dungeon.current.start(setupOf()));
    expect((stored(DUNGEON_RUN_KEY) as { customCards: CustomCard[] }).customCards).toEqual([IMP]);

    // The creator's card is gone by the next visit: the run under way still has it.
    account.cards = [];
    wasmBindings.runFold.mockClear();
    const again = reload();
    expect(wasmBindings.runFold).toHaveBeenCalledWith(expect.anything(), WITH_IMP);
    expect(again.current.content).toBe(WITH_IMP);

    // Once it is over, the pool is the account's as it stands.
    press(() => again.current.abandon());
    expect(again.current.run).toBeNull();
    expect(again.current.content).toBe(CONTENT);
  });

  it("B19 a kept run that no longer folds is dropped alone: the progress and the history stay", () => {
    window.localStorage.setItem(DUNGEON_PROGRESS_KEY, JSON.stringify({ v: 1, unlocks: unlocks({ level: 3 }), embers: 7, daily: null }));
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf({ seed: "QWERTY" })));
    cleanup();
    __resetDungeonForTests();

    wasmBindings.runFold.mockReturnValue({ error: "step 0: row 0 has no node 1" });
    const again = mount();

    expect(again.current.run).toBeNull();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
    expect(again.current.progress).toEqual({ unlocks: unlocks({ level: 3 }), embers: 7 });
  });

  it("B19 a kept run the binding throws on, or of a shape nothing wrote, is dropped too", () => {
    window.localStorage.setItem(DUNGEON_RUN_KEY, JSON.stringify({ v: 1, file: { setup: 3 } }));
    expect(mount().current.run).toBeNull();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
    expect(wasmBindings.runFold).not.toHaveBeenCalled();
    cleanup();

    const dungeon = mount();
    press(() => dungeon.current.start(setupOf()));
    wasmBindings.runFold.mockImplementation(() => {
      throw new Error("the engine is not loaded");
    });
    expect(reload().current.run).toBeNull();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
  });
});

describe("the store's act", () => {
  it("answers the run's refusal and changes nothing", () => {
    const dungeon = mount();
    expect(press(() => dungeon.current.act({ type: "go", col: 0 }))).toBe("No run is in progress.");

    press(() => dungeon.current.start(setupOf()));
    const before = stored(DUNGEON_RUN_KEY);
    expect(press(() => dungeon.current.act({ type: "leave" }))).toBe("there is nothing to leave");

    expect(stored(DUNGEON_RUN_KEY)).toEqual(before);
    expect(dungeon.current.run?.phase).toEqual({ phase: "map" });
    expect(dungeon.current.run?.rngCursor).toBe(0);
  });

  it("answers a binding that throws, as a refusal", () => {
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf()));
    wasmBindings.runReduce.mockImplementation(() => {
      throw new Error("the run's JSON would not parse");
    });

    expect(press(() => dungeon.current.act({ type: "go", col: 0 }))).toBe("the run's JSON would not parse");
    expect(dungeon.current.run?.phase).toEqual({ phase: "map" });
  });
});

describe("B20 a finished run", () => {
  it("B20 a won run settles extractions Embers and history", () => {
    window.localStorage.setItem(
      DUNGEON_PROGRESS_KEY,
      JSON.stringify({ v: 1, unlocks: unlocks({ level: 2, cards: ["traininggrounds-009"], artifacts: ["lucky-coin"] }), embers: 5, daily: null }),
    );
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf({ seed: "QWERTY", curses: ["heavy-hands", "frail"] })));

    // The final boss falls: the run is won on the 12th row, and it extracted a card and a relic.
    const won: RunState = runState({
      seed: "QWERTY",
      row: 11,
      curses: ["heavy-hands", "frail"],
      extractedCards: [EXTRACTABLE, "traininggrounds-009"],
      extractedArtifacts: ["locked-sigil"],
      phase: { phase: "won" },
      stats: { ...ZERO_STATS, fightsWon: 9, bossesWon: 1, turns: 40 },
    });
    wasmBindings.runReduce.mockReturnValue({ state: won });
    wasmBindings.runScore.mockReturnValue(250);
    vi.setSystemTime(new Date(`${DAY}T12:05:30Z`));

    expect(press(() => dungeon.current.act({ type: "fightOver", won: true, hp: 12, damageDealt: 90, healingDone: 4, cardsDrawn: 30, turns: 6 }))).toBeNull();

    // What it extracted joins the unlocks, once; its Embers join the balance; the level is kept.
    expect(dungeon.current.progress).toEqual({
      unlocks: unlocks({ level: 2, cards: ["traininggrounds-009", EXTRACTABLE], artifacts: ["lucky-coin", "locked-sigil"] }),
      embers: 30,
    });
    expect(stored(DUNGEON_PROGRESS_KEY)).toEqual({
      v: 1,
      unlocks: unlocks({ level: 2, cards: ["traininggrounds-009", EXTRACTABLE], artifacts: ["lucky-coin", "locked-sigil"] }),
      embers: 30,
      daily: null,
    });
    // Its record joins the history.
    expect(wasmBindings.runScore).toHaveBeenCalledWith(won, CONTENT);
    expect(wasmBindings.runEmbers).toHaveBeenCalledWith(250);
    const record = {
      seed: "QWERTY",
      daily: false,
      mode: "sandbox",
      curses: ["heavy-hands", "frail"],
      cursePoints: 5,
      rowReached: 12,
      won: true,
      score: 250,
      embers: 25,
      timeMs: 330_000,
      endedAt: `${DAY}T12:05:30.000Z`,
      stats: { ...ZERO_STATS, fightsWon: 9, bossesWon: 1, turns: 40 },
    };
    expect(readRunHistory()).toEqual([record]);
    expect(stored(DUNGEON_HISTORY_KEY)).toEqual({ v: 1, runs: [record] });
    // The run is no longer kept, but stays on the screen until it is left.
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
    expect(dungeon.current.run?.phase).toEqual({ phase: "won" });

    // It pays once: a finished run takes no more actions, and leaving it records nothing more.
    wasmBindings.runReduce.mockClear();
    expect(press(() => dungeon.current.act({ type: "leave" }))).toBe("The run is over.");
    expect(wasmBindings.runReduce).not.toHaveBeenCalled();
    press(() => dungeon.current.abandon());
    expect(dungeon.current.run).toBeNull();
    expect(dungeon.current.progress.embers).toBe(30);
    expect(readRunHistory()).toHaveLength(1);

    // A reload finds the start screen, with everything kept.
    const again = reload();
    expect(again.current.run).toBeNull();
    expect(again.current.progress.embers).toBe(30);
    expect(readRunHistory()).toEqual([record]);
  });

  it("B20 a lost fight is a run lost, a row reached and no extraction", () => {
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf({ seed: "QWERTY" })));
    press(() => dungeon.current.act({ type: "go", col: 0 }));
    wasmBindings.runScore.mockReturnValue(37);

    press(() => dungeon.current.act({ type: "fightOver", won: false, hp: 0, damageDealt: 3, healingDone: 0, cardsDrawn: 5, turns: 2 }));

    expect(dungeon.current.run?.phase).toEqual({ phase: "dead" });
    expect(readRunHistory()).toEqual([expect.objectContaining({ seed: "QWERTY", won: false, rowReached: 1, score: 37, embers: 3 })]);
    expect(dungeon.current.progress.embers).toBe(3);
    expect(dungeon.current.progress.unlocks).toEqual(unlocks());
  });

  it("B20 abandon records a loss and clears the run", () => {
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf({ seed: "QWERTY" })));

    press(() => dungeon.current.abandon());

    // The run is sent the abandon, and is left behind: no run, no timer, nothing kept.
    expect(wasmBindings.runReduce).toHaveBeenLastCalledWith(expect.anything(), CONTENT, { type: "abandon" });
    expect(dungeon.current.run).toBeNull();
    expect(dungeon.current.startedAt).toBeNull();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
    // Before its first step the run stood on no row.
    expect(readRunHistory()).toEqual([expect.objectContaining({ seed: "QWERTY", won: false, rowReached: 0 })]);

    // With no run on, there is nothing to give up and nothing is recorded.
    press(() => dungeon.current.abandon());
    expect(readRunHistory()).toHaveLength(1);
    expect(reload().current.run).toBeNull();
  });

  it("B20 the history is a copy, oldest first", () => {
    const dungeon = mount();
    for (const seed of ["AAAAAA", "BBBBBB"]) {
      press(() => dungeon.current.start(setupOf({ seed })));
      press(() => dungeon.current.abandon());
    }

    const history = readRunHistory();
    expect(history.map((record) => record.seed)).toEqual(["AAAAAA", "BBBBBB"]);
    history.pop();
    expect(readRunHistory()).toHaveLength(2);
  });
});

describe("B18 the starting deck's level", () => {
  it("B18 buyLevel spends runLevelCost", () => {
    window.localStorage.setItem(DUNGEON_PROGRESS_KEY, JSON.stringify({ v: 1, unlocks: unlocks(), embers: 30, daily: null }));
    const dungeon = mount();

    expect(press(() => dungeon.current.buyLevel())).toBeNull();
    expect(wasmBindings.runLevelCost).toHaveBeenCalledWith(1);
    expect(dungeon.current.progress).toEqual({ unlocks: unlocks({ level: 2 }), embers: 20 });
    expect(stored(DUNGEON_PROGRESS_KEY)).toEqual({ v: 1, unlocks: unlocks({ level: 2 }), embers: 20, daily: null });

    // The next level costs more than the balance: refused, nothing spent.
    expect(press(() => dungeon.current.buyLevel())).toBe("You need 25 Embers to raise your starting deck.");
    expect(dungeon.current.progress).toEqual({ unlocks: unlocks({ level: 2 }), embers: 20 });

    // The top of the ladder has no next level, however many Embers.
    window.localStorage.setItem(DUNGEON_PROGRESS_KEY, JSON.stringify({ v: 1, unlocks: unlocks({ level: 6 }), embers: 999, daily: null }));
    const top = reload();
    expect(press(() => top.current.buyLevel())).toBe("Your starting deck is at its top level.");
    expect(top.current.progress.embers).toBe(999);
  });

  it("B18 a level bought during a run is not lost when the run is over", () => {
    window.localStorage.setItem(DUNGEON_PROGRESS_KEY, JSON.stringify({ v: 1, unlocks: unlocks(), embers: 10, daily: null }));
    const dungeon = mount();
    press(() => dungeon.current.start(setupOf()));
    press(() => dungeon.current.buyLevel());

    press(() => dungeon.current.abandon());

    expect(dungeon.current.progress.unlocks.level).toBe(2);
    expect(dungeon.current.progress.embers).toBe(12);
  });
});

describe("B21 the Sandbox's custom cards", () => {
  it("B21 new content receives Practice's custom cards", () => {
    wasmBindings.runContent.mockImplementation((cards) => (cards.length === 0 ? CONTENT : WITH_IMP));
    account.cards = [IMP];
    const dungeon = mount();

    expect(wasmBindings.runContent).toHaveBeenCalledWith([IMP]);
    expect(dungeon.current.content).toBe(WITH_IMP);

    press(() => dungeon.current.start(setupOf()));
    // The run is dealt on the pool with the card, and keeps the card with it.
    expect(wasmBindings.runCreate).toHaveBeenCalledWith(setupOf(), unlocks(), WITH_IMP);
    expect(dungeon.current.content).toBe(WITH_IMP);
  });

  it("B21 the content is built once for the same cards", () => {
    const dungeon = mount();
    const calls = wasmBindings.runContent.mock.calls.length;

    press(() => dungeon.current.start(setupOf()));
    press(() => dungeon.current.abandon());

    expect(wasmBindings.runContent.mock.calls.length).toBe(calls);
  });
});

describe("B15 the daily run's number", () => {
  it("B15 daily starts advance only that day's number", () => {
    const dungeon = mount();
    const daily = (n: number, date = DAY): string => seedFrom(`daily:${date}:${String(n)}`);
    expect(dailyRunNumber(DAY)).toBe(1);

    // The day's first run is run 1; once started, the next daily run is the second.
    expect(press(() => dungeon.current.start(setupOf({ seed: daily(1) })))).toBeNull();
    expect(dailyRunNumber(DAY)).toBe(2);
    expect(stored(DUNGEON_PROGRESS_KEY)).toEqual(expect.objectContaining({ daily: { date: DAY, started: 1 } }));
    press(() => dungeon.current.abandon());
    expect(readRunHistory().map((record) => record.daily)).toEqual([true]);

    // A random or a typed seed, and a start the run refuses, take no number.
    press(() => dungeon.current.start(setupOf({ seed: "QWERTY" })));
    press(() => dungeon.current.abandon());
    wasmBindings.runCheckSetup.mockReturnValueOnce("a starting deck holds 10 cards, not 9");
    press(() => dungeon.current.start(setupOf({ seed: daily(2) })));
    expect(dailyRunNumber(DAY)).toBe(2);
    expect(readRunHistory().map((record) => record.daily)).toEqual([true, false]);

    expect(press(() => dungeon.current.start(setupOf({ seed: daily(2) })))).toBeNull();
    press(() => dungeon.current.abandon());
    expect(dailyRunNumber(DAY)).toBe(3);

    // It is kept on the device, and the next day starts again from the first.
    reload();
    expect(dailyRunNumber(DAY)).toBe(3);
    expect(dailyRunNumber(NEXT_DAY)).toBe(1);
    vi.setSystemTime(new Date(`${NEXT_DAY}T00:30:00Z`));
    const tomorrow = mount();
    press(() => tomorrow.current.start(setupOf({ seed: daily(1, NEXT_DAY) })));
    expect(dailyRunNumber(NEXT_DAY)).toBe(2);
    expect(stored(DUNGEON_PROGRESS_KEY)).toEqual(expect.objectContaining({ daily: { date: NEXT_DAY, started: 1 } }));
  });
});

describe("the device's storage", () => {
  it("reads nothing from data no version of the store wrote", () => {
    for (const bad of ["not json", "[]", "null", JSON.stringify({ v: 2, unlocks: unlocks(), embers: 9 }), JSON.stringify({ v: 1, unlocks: { level: 0 }, embers: 9 }), JSON.stringify({ v: 1, unlocks: unlocks(), embers: -3 })]) {
      window.localStorage.setItem(DUNGEON_PROGRESS_KEY, bad);
      window.localStorage.setItem(DUNGEON_HISTORY_KEY, bad);
      const dungeon = reload();
      expect(dungeon.current.progress, bad).toEqual({ unlocks: unlocks(), embers: 0 });
      expect(readRunHistory(), bad).toEqual([]);
    }
  });

  it("keeps the history's good records when one is damaged", () => {
    const record = { seed: "QWERTY", daily: false, mode: "sandbox", curses: [], cursePoints: 0, rowReached: 3, won: false, score: 10, embers: 1, timeMs: 5, endedAt: "2026-10-31T10:00:00.000Z", stats: ZERO_STATS };
    window.localStorage.setItem(DUNGEON_HISTORY_KEY, JSON.stringify({ v: 1, runs: [record, { seed: 4 }, null, record] }));

    expect(readRunHistory()).toEqual([record, record]);
  });

  it("plays on where the storage is blocked, and where it is full", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("full", "QuotaExceededError");
    });
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const dungeon = mount();
    expect(dungeon.current.progress).toEqual({ unlocks: unlocks(), embers: 0 });

    expect(press(() => dungeon.current.start(setupOf()))).toBeNull();
    expect(press(() => dungeon.current.act({ type: "go", col: 0 }))).toBeNull();
    expect(press(() => dungeon.current.abandon())).toBeUndefined();

    // The run was played and settled in memory, if not kept.
    expect(dungeon.current.run).toBeNull();
    expect(readRunHistory()).toHaveLength(1);
  });
});
