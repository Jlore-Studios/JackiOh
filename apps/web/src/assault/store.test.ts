// R4581000–R4581001: the Assault's two device values, their fold and bounded history.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

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
