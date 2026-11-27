// R4581000–R4581001: the Assault's two device values, their fold and bounded history.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AssaultFile, AssaultRecord } from "@jackioh/run";

import { ASSAULT_ATTEMPT_KEY, ASSAULT_HELD_KEY, ASSAULT_RECORDS_KEY } from "./config.ts";
import { __resetAssaultForTests, useAssault, type Assault } from "./store.ts";
import { CONSTANTS, DAY, RECORD, bindingFixtures, resetBindings, setupOf } from "./test/fixtures.ts";

vi.mock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("./test/fixtures.ts")).bindingFixtures,
  catalogVersion: (): string => "test-catalog",
}));

function mount(): { current: Assault } {
  const { result } = renderHook(() => useAssault());
  return { get current() { return result.current; } };
}

function reload(): { current: Assault } {
  cleanup();
  __resetAssaultForTests();
  return mount();
}

function press<T>(verb: () => T): T {
  let result: T | undefined;
  act(() => { result = verb(); });
  return result as T;
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-23T12:00:00Z"));
  window.localStorage.clear();
  __resetAssaultForTests();
  resetBindings();
});

afterEach(() => {
  cleanup();
  __resetAssaultForTests();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it("R4581000 an attempt of an earlier day plays on its own day", () => {
  const setup = setupOf({ date: "2026-11-22" });
  const file: AssaultFile = { version: "test-catalog", setup, record: { ...RECORD, date: setup.date }, actions: [] };
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));

  const assault = mount();

  expect(assault.current.day.date).toBe("2026-11-22");
  expect(bindingFixtures.assaultDay).toHaveBeenCalledWith("2026-11-22", expect.anything());
  expect(bindingFixtures.assaultFold).toHaveBeenCalledWith(file, expect.objectContaining({ date: "2026-11-22" }));
});

it("R4581001 round trip: start, act and reload fold the kept file", () => {
  const assault = mount();
  expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();

  const kept = JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile };
  expect(kept.file.actions).toEqual([{ kind: "fight" }]);

  const again = reload();
  expect(bindingFixtures.assaultFold).toHaveBeenLastCalledWith(kept.file, expect.objectContaining({ date: DAY.date }));
  expect(again.current.state?.phase).toBe("fight");
});

it("R4581001 a corrupt attempt key and a corrupt records key are ignored", () => {
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, "not json");
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: 1, days: { [DAY.date]: RECORD } }));

  const assault = mount();

  expect(assault.current.state).toBeNull();
  expect(assault.current.records).toEqual({ [DAY.date]: RECORD });
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_RECORDS_KEY)).not.toBeNull();

  cleanup();
  __resetAssaultForTests();
  window.localStorage.clear();
  const setup = setupOf();
  const file: AssaultFile = { version: "test-catalog", setup, record: RECORD, actions: [] };
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file, fight: null }));
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: 999, days: {} }));

  const again = mount();
  expect(again.current.state?.phase).toBe("briefing");
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).not.toBeNull();
  expect(again.current.records).toEqual({ [DAY.date]: RECORD });
});

it("R4733000 a held won report applies on load, once", () => {
  const assault = mount();
  expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();
  expect(assault.current.state?.phase).toBe("fight");

  press(() => assault.current.hold({ kind: "fightOver", won: true, turns: 7 }));
  expect(JSON.parse(window.localStorage.getItem(ASSAULT_HELD_KEY) ?? "null")).toEqual({
    attempt: 1,
    level: 1,
    action: { kind: "fightOver", won: true, turns: 7 },
  });

  // A reload applies the held report as Continue would: the attempt briefs the next level.
  const again = reload();
  expect(again.current.state?.phase).toBe("briefing");
  expect(again.current.state?.level).toBe(2);
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();
  expect((JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file.actions).toEqual([
    { kind: "fight" },
    { kind: "fightOver", won: true, turns: 7 },
  ]);

  // A second reload applies nothing more.
  const third = reload();
  expect(third.current.state?.phase).toBe("briefing");
  expect((JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file.actions).toHaveLength(2);
});

it("R4733000 a held lost report ends the attempt as after Continue", () => {
  const assault = mount();
  expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();
  press(() => assault.current.hold({ kind: "fightOver", won: false, turns: 3 }));

  const again = reload();
  expect(again.current.state?.phase).toBe("lost");
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();

  // A further reload finds no open attempt, recording nothing more.
  expect(reload().current.state).toBeNull();
});

it("R4733000 a held report the attempt refuses is dropped and the fight is dealt fresh", () => {
  const assault = mount();
  expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();
  press(() => assault.current.hold({ kind: "fightOver", won: true, turns: 7 }));

  // The attempt refuses the held report once: the hold is dropped and the fight still stands.
  bindingFixtures.assaultReduce.mockReturnValueOnce({ error: "there is no fight to report" });
  const again = reload();
  expect(again.current.state?.phase).toBe("fight");
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();
  expect((JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file.actions).toEqual([
    { kind: "fight" },
  ]);
});

it("R4733000 Continue clears the hold, so a reload after it applies nothing more", () => {
  const assault = mount();
  expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();
  press(() => assault.current.hold({ kind: "fightOver", won: true, turns: 7 }));

  expect(press(() => assault.current.act({ kind: "fightOver", won: true, turns: 7 }))).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();

  const again = reload();
  expect(again.current.state?.phase).toBe("briefing");
  expect((JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file.actions).toHaveLength(2);
});

// A kept attempt written before this change holds no report: the R4581001 reload tests above load
// such attempts, and every one still passes with the hold key absent.

it("R4581001 the records keep the last history_days days", () => {
  const days: Record<string, AssaultRecord> = {};
  for (let day = 1; day <= CONSTANTS.historyDays + 3; day += 1) {
    const date = `2026-11-${String(day).padStart(2, "0")}`;
    days[date] = { ...RECORD, date };
  }
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: 1, days }));

  const assault = mount();
  const kept = Object.keys(assault.current.records);

  expect(kept).toHaveLength(CONSTANTS.historyDays);
  expect(kept[0]).toBe("2026-11-04");
  expect(Object.keys((JSON.parse(window.localStorage.getItem(ASSAULT_RECORDS_KEY) ?? "null") as { days: object }).days)).toEqual(kept);
});

describe("reopening the Assault page", () => {
  function begin(assault: { current: Assault }): void {
    expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  }

  function playToLost(assault: { current: Assault }): void {
    begin(assault);
    expect(press(() => assault.current.act({ kind: "fight" }))).toBeNull();
    expect(press(() => assault.current.act({ kind: "fightOver", won: false, turns: 3 }))).toBeNull();
    expect(assault.current.state?.phase).toBe("lost");
    expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  }

  function otherTabRecords(record: AssaultRecord): void {
    window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: 1, days: { [DAY.date]: record } }));
  }

  function storedRecord(): unknown {
    return (JSON.parse(window.localStorage.getItem(ASSAULT_RECORDS_KEY) ?? "null") as { days: Record<string, unknown> }).days[DAY.date];
  }

  function storedFile(): AssaultFile {
    return (JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file;
  }

  it("reads the device again, so another tab's attempts and checkpoints are not written over", () => {
    mount();
    cleanup();
    otherTabRecords({ ...RECORD, best: 3, attempts: 4 });

    const assault = mount();
    expect(assault.current.state).toBeNull();
    expect(assault.current.records[DAY.date]?.attempts).toBe(4);

    begin(assault);
    expect(storedRecord()).toEqual({ ...RECORD, best: 3, attempts: 5 });
    expect(storedFile().setup.attempt).toBe(5);
  });

  it("applies the held report of an attempt another tab left open, and refuses a start over it", () => {
    mount();
    cleanup();
    const file: AssaultFile = { version: "test-catalog", setup: setupOf(), record: RECORD, actions: [{ kind: "fight" }] };
    window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));
    window.localStorage.setItem(
      ASSAULT_HELD_KEY,
      JSON.stringify({ attempt: 1, level: 1, action: { kind: "fightOver", won: true, turns: 7 } }),
    );

    const assault = mount();
    expect(assault.current.state?.phase).toBe("briefing");
    expect(assault.current.state?.level).toBe(2);
    expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();
    expect(storedFile().actions).toHaveLength(2);

    expect(press(() => assault.current.start({ date: DAY.date, deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).not.toBeNull();
    expect(storedFile().actions).toHaveLength(2);
  });

  it("keeps a finished attempt this tab still shows, over another tab's newer records", () => {
    const assault = mount();
    playToLost(assault);
    cleanup();
    otherTabRecords({ ...RECORD, attempts: 4 });

    const again = mount();
    expect(again.current.state?.phase).toBe("lost");
    expect(again.current.record.attempts).toBe(4);

    expect(press(() => again.current.tryAgain())).toBeNull();
    expect(storedFile().setup.attempt).toBe(5);
    expect(storedFile().setup.startLevel).toBe(2);
    expect(storedRecord()).toEqual({ ...RECORD, attempts: 5 });
    expect(again.current.state?.phase).toBe("briefing");
  });

  it("shows an attempt another tab started instead of this tab's finished one", () => {
    const assault = mount();
    playToLost(assault);
    cleanup();
    const file: AssaultFile = { version: "test-catalog", setup: setupOf({ attempt: 5 }), record: RECORD, actions: [] };
    window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));

    const again = mount();
    expect(again.current.state?.phase).toBe("briefing");
    expect(again.current.state?.setup.attempt).toBe(5);
  });

  it("reads the device once on the first mount", () => {
    const file: AssaultFile = { version: "test-catalog", setup: setupOf(), record: RECORD, actions: [] };
    window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));

    mount();

    expect(bindingFixtures.assaultFold).toHaveBeenCalledTimes(1);
  });

  it("keeps a finished attempt when another tab's write fires a storage event", () => {
    const assault = mount();
    playToLost(assault);

    act(() => {
      window.dispatchEvent(new StorageEvent("storage", { key: ASSAULT_RECORDS_KEY }));
    });

    expect(assault.current.state?.phase).toBe("lost");
  });
});

// #5005: the Assault's attempt as docs/implicit-states.md §16 maps it.

it("R4581000 a start sent from a day screen left open past UTC midnight is refused and shows the new day", () => {
  vi.setSystemTime(new Date("2026-11-23T23:59:00Z"));
  const assault = mount();
  expect(assault.current.day.date).toBe("2026-11-23");

  // The screen still offers yesterday's day when the clock passes midnight; its Start names that day.
  vi.setSystemTime(new Date("2026-11-24T00:01:00Z"));
  const refused = press(() => assault.current.start({ date: "2026-11-23", deck: ["core-001"], source: "dayDeck", startLevel: 1 }));

  expect(refused).toBe("A new day's Assault has begun. Look it over and start again.");
  expect(assault.current.day.date).toBe("2026-11-24");
  expect(assault.current.state).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  expect(bindingFixtures.assaultCreate).not.toHaveBeenCalled();

  // The new day's Start is dealt on it.
  expect(press(() => assault.current.start({ date: "2026-11-24", deck: ["core-001"], source: "dayDeck", startLevel: 1 }))).toBeNull();
  expect(assault.current.state?.setup.date).toBe("2026-11-24");
});

it("R4581001 a record no attempt could make, a best past the last level or a checkpoint that is none, reads as absent", () => {
  const recordsWith = (record: AssaultRecord): string => JSON.stringify({ v: 1, days: { [DAY.date]: record } });

  // A best above the last level, a checkpoint past it, or one in range that is no checkpoint, is no record an
  // attempt could have made.
  const notACheckpoint = CONSTANTS.levels - 1;
  expect(CONSTANTS.checkpoints).not.toContain(notACheckpoint);
  for (const record of [
    { ...RECORD, best: CONSTANTS.levels + 1 },
    { ...RECORD, checkpoints: [CONSTANTS.levels + 1] },
    { ...RECORD, checkpoints: [notACheckpoint] },
  ]) {
    window.localStorage.setItem(ASSAULT_RECORDS_KEY, recordsWith(record));
    const assault = reload();
    expect(assault.current.records).toEqual({});
    expect(window.localStorage.getItem(ASSAULT_RECORDS_KEY)).toBeNull();
  }

  // An attempt file whose record is out of range is malformed too, and goes on its own.
  const setup = setupOf();
  const file: AssaultFile = { version: "test-catalog", setup, record: { ...RECORD, best: CONSTANTS.levels + 1 }, actions: [] };
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, recordsWith(RECORD));
  const assault = reload();
  expect(assault.current.state).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  expect(assault.current.records).toEqual({ [DAY.date]: RECORD });

  // The last level and every checkpoint are in range.
  const top = { ...RECORD, best: CONSTANTS.levels, checkpoints: [...CONSTANTS.checkpoints] };
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, recordsWith(top));
  expect(reload().current.records).toEqual({ [DAY.date]: top });
});

it("an attempt whose catalog version the build no longer carries is dropped and its day's record kept", () => {
  const setup = setupOf();
  const file: AssaultFile = { version: "a-catalog-gone", setup, record: RECORD, actions: [{ kind: "fight" }] };
  window.localStorage.setItem(ASSAULT_ATTEMPT_KEY, JSON.stringify({ v: 1, file }));
  window.localStorage.setItem(ASSAULT_RECORDS_KEY, JSON.stringify({ v: 1, days: { [DAY.date]: RECORD } }));
  window.localStorage.setItem(ASSAULT_HELD_KEY, JSON.stringify({ attempt: 1, level: 1, action: { kind: "fightOver", won: true, turns: 7 } }));
  bindingFixtures.assaultContentAt.mockReturnValue(null);

  const assault = mount();

  expect(bindingFixtures.assaultContentAt).toHaveBeenCalledWith("a-catalog-gone");
  expect(bindingFixtures.assaultFold).not.toHaveBeenCalled();
  expect(assault.current.state).toBeNull();
  expect(assault.current.day.date).toBe(DAY.date);
  expect(assault.current.records).toEqual({ [DAY.date]: RECORD });
  expect(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY)).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();
  expect(window.localStorage.getItem(ASSAULT_RECORDS_KEY)).not.toBeNull();
});

it("another tab's start, action or hold reloads this tab, and a held report is applied once", () => {
  const assault = mount();
  expect(assault.current.state).toBeNull();
  const fromAnotherTab = (key: string, value: unknown): void => {
    window.localStorage.setItem(key, JSON.stringify(value));
    act(() => { window.dispatchEvent(new StorageEvent("storage", { key })); });
  };
  const setup = setupOf();
  const keptActions = (): unknown =>
    (JSON.parse(window.localStorage.getItem(ASSAULT_ATTEMPT_KEY) ?? "null") as { file: AssaultFile }).file.actions;

  // Another tab starts an attempt, then plays its fight.
  fromAnotherTab(ASSAULT_ATTEMPT_KEY, { v: 1, file: { version: "test-catalog", setup, record: RECORD, actions: [] } });
  expect(assault.current.state?.phase).toBe("briefing");
  fromAnotherTab(ASSAULT_ATTEMPT_KEY, { v: 1, file: { version: "test-catalog", setup, record: RECORD, actions: [{ kind: "fight" }] } });
  expect(assault.current.state?.phase).toBe("fight");

  // It holds the fight's report and closes: this tab applies it as Continue would, and clears the hold.
  fromAnotherTab(ASSAULT_HELD_KEY, { attempt: setup.attempt, level: 1, action: { kind: "fightOver", won: true, turns: 7 } });
  expect(assault.current.state?.phase).toBe("briefing");
  expect(assault.current.state?.level).toBe(2);
  expect(window.localStorage.getItem(ASSAULT_HELD_KEY)).toBeNull();
  expect(keptActions()).toEqual([{ kind: "fight" }, { kind: "fightOver", won: true, turns: 7 }]);

  // A later reload, by another tab's write or this one's, applies nothing more.
  act(() => { window.dispatchEvent(new StorageEvent("storage", { key: ASSAULT_ATTEMPT_KEY })); });
  expect(assault.current.state?.level).toBe(2);
  expect(keptActions()).toHaveLength(2);

  // A key that is not the Assault's reloads nothing.
  const folds = bindingFixtures.assaultFold.mock.calls.length;
  act(() => { window.dispatchEvent(new StorageEvent("storage", { key: "jackioh.something-else" })); });
  expect(bindingFixtures.assaultFold.mock.calls.length).toBe(folds);
});
