// Client-side preferences (`./store.ts`, S8, R2000000, R82/R345): invalid values take their defaults,
// unknown keys are dropped, writes notify, and the panel's switch reaches the document. One `it` per
// rule; the store is reset before and after each test, as `settings/wiring.test.tsx` does.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DEFAULT_SETTINGS,
  SETTINGS_STORAGE_KEY,
  __resetSettingsForTests,
  parseSettings,
  readSettings,
  resetSettings,
  subscribeSettings,
  useSetting,
  useSettings,
  writeSettings,
  type Settings,
} from "./store.ts";

/** S8's defaults, written out: what a new player gets. */
const NEW_PLAYER: Settings = {
  dragToPlay: true,
  confirmEndTurn: false,
  autoEndTurn: true,
  hoverPreviews: true,
  lethalHint: true,
  attackOutcomes: true,
  reduceMotion: false,
  muteOpponentEmotes: false,
  muteSpectatorEmotes: false,
};

/** Every kept key the other way round from its default. */
const ALL_FLIPPED = Object.fromEntries(Object.entries(NEW_PLAYER).map(([key, on]) => [key, !on])) as Settings;

const noop = (): void => undefined;

/** Another tab's write (or, keyless, a clear), as the browser tells this one. */
function storageEvent(key: string | null, newValue: string | null): void {
  window.dispatchEvent(new StorageEvent("storage", { key, newValue }));
}

beforeEach(() => {
  window.localStorage.clear();
  __resetSettingsForTests();
});

afterEach(() => {
  window.localStorage.clear();
  __resetSettingsForTests();
});

describe("readSettings", () => {
  it("reads the defaults with nothing stored", () => {
    expect(readSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("keeps the same object while nothing changes", () => {
    expect(readSettings()).toBe(readSettings());
  });
});

describe("writeSettings", () => {
  it("writes the booleans it knows and keeps the rest", () => {
    const next = writeSettings({ hoverPreviews: false, reduceMotion: true });
    expect(next.hoverPreviews).toBe(false);
    expect(next.reduceMotion).toBe(true);
    expect(next.dragToPlay).toBe(DEFAULT_SETTINGS.dragToPlay);
    expect(readSettings()).toBe(next);
  });

  it("drops what is not a boolean and still notifies", () => {
    const listener = vi.fn();
    const stop = subscribeSettings(listener);
    try {
      const next = writeSettings({ hoverPreviews: "yes" as unknown as boolean });
      expect(next.hoverPreviews).toBe(DEFAULT_SETTINGS.hoverPreviews);
      expect(listener).toHaveBeenCalledTimes(1);
    } finally {
      stop();
    }
  });

  it("tells every subscriber and stops telling the unsubscribed", () => {
    const first = vi.fn();
    const second = vi.fn();
    const stopFirst = subscribeSettings(first);
    const stopSecond = subscribeSettings(second);
    writeSettings({ autoEndTurn: false });
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    stopFirst();
    writeSettings({ autoEndTurn: true });
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(2);
    stopSecond();
  });

  it("maps the reduce-motion switch onto the document", () => {
    writeSettings({ reduceMotion: true });
    expect(document.documentElement.getAttribute("data-reduce-motion")).toBe("true");
    writeSettings({ reduceMotion: false });
    expect(document.documentElement.hasAttribute("data-reduce-motion")).toBe(false);
  });
});

describe("resetSettings", () => {
  it("puts every key back to its default", () => {
    writeSettings({ hoverPreviews: false, muteOpponentEmotes: true });
    expect(resetSettings()).toEqual(DEFAULT_SETTINGS);
    expect(readSettings()).toEqual(DEFAULT_SETTINGS);
  });
});

describe("stored settings", () => {
  it("reads stored JSON and drops an older device's unknown keys (R2000000)", () => {
    window.localStorage.setItem(
      SETTINGS_STORAGE_KEY,
      JSON.stringify({ hoverPreviews: false, publicStats: true }),
    );
    __resetSettingsForTests();
    const read = readSettings();
    expect(read.hoverPreviews).toBe(false);
    expect(read).not.toHaveProperty("publicStats");
  });

  it("takes the defaults for stored text that is not a record", () => {
    for (const raw of ["[1,2]", "42", "{oops", "null"]) {
      window.localStorage.setItem(SETTINGS_STORAGE_KEY, raw);
      __resetSettingsForTests();
      expect(readSettings()).toEqual(DEFAULT_SETTINGS);
    }
  });

  it("parseSettings ignores proto keys (#4015)", () => {
    window.localStorage.setItem(
      SETTINGS_STORAGE_KEY,
      `{"dragToPlay":false,"__proto__":{"polluted":true},"constructor":{"prototype":{"polluted":true}}}`,
    );
    __resetSettingsForTests();
    const read = readSettings();
    expect(read.dragToPlay).toBe(false);
    expect(read).not.toHaveProperty("polluted");
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });

  it("follows another tab's storage event and ignores other keys", () => {
    const listener = vi.fn();
    const stop = subscribeSettings(listener);
    try {
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "some.other.key",
          newValue: JSON.stringify({ hoverPreviews: false }),
        }),
      );
      expect(readSettings().hoverPreviews).toBe(DEFAULT_SETTINGS.hoverPreviews);
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: SETTINGS_STORAGE_KEY,
          newValue: JSON.stringify({ hoverPreviews: false }),
        }),
      );
      expect(readSettings().hoverPreviews).toBe(false);
      expect(listener).toHaveBeenCalled();
    } finally {
      stop();
    }
  });
});

// The store's module as a page first loads it: evaluated afresh, so its constants are the ones a
// player's first visit, and the settings their browser already holds, meet.
describe("S8 a fresh page", () => {
  it("gives a new player S8's defaults and keeps them under jackioh.settings, the key browsers hold", async () => {
    vi.resetModules();
    const fresh = await import("./store.ts");
    try {
      expect(fresh.DEFAULT_SETTINGS).toEqual(NEW_PLAYER);
      expect(fresh.readSettings()).toEqual(NEW_PLAYER);
      fresh.writeSettings({ confirmEndTurn: true });
      expect(JSON.parse(window.localStorage.getItem("jackioh.settings") ?? "null")).toEqual({
        ...NEW_PLAYER,
        confirmEndTurn: true,
      });
    } finally {
      fresh.__resetSettingsForTests();
    }
  });

  it("R2000000 reads every kept key a browser holds, and applies reduce motion on that first read", async () => {
    window.localStorage.setItem("jackioh.settings", JSON.stringify(ALL_FLIPPED));
    vi.resetModules();
    const fresh = await import("./store.ts");
    try {
      expect(fresh.readSettings()).toEqual(ALL_FLIPPED);
      expect(document.documentElement.getAttribute("data-reduce-motion")).toBe("true");
    } finally {
      fresh.__resetSettingsForTests();
    }
  });
});

describe("parseSettings", () => {
  it("takes an object as it is, keeping only its booleans", () => {
    expect(parseSettings({ hoverPreviews: false, reduceMotion: "yes", dragToPlay: 1 })).toEqual({
      ...NEW_PLAYER,
      hoverPreviews: false,
    });
  });

  it("holds an array to be no settings, whatever keys it carries", () => {
    expect(parseSettings(Object.assign([], { hoverPreviews: false }))).toEqual(NEW_PLAYER);
  });

  it("never throws: a record whose field throws on read takes every default", () => {
    const hostile = {
      dragToPlay: false,
      get confirmEndTurn(): boolean {
        throw new Error("unreadable");
      },
    };
    expect(parseSettings(hostile)).toEqual(NEW_PLAYER);
  });
});

describe("one settings object while nothing changes (S8: useSyncExternalStore needs that)", () => {
  it("keeps the object through a write that changes nothing, and replaces it on one that does", () => {
    const before = readSettings();
    expect(writeSettings({ hoverPreviews: before.hoverPreviews })).toBe(before);
    expect(writeSettings({ hoverPreviews: !before.hoverPreviews })).not.toBe(before);
  });

  it("keeps the object through another tab's write of what this one holds", () => {
    const stop = subscribeSettings(noop);
    try {
      const before = readSettings();
      storageEvent(SETTINGS_STORAGE_KEY, JSON.stringify(before));
      expect(readSettings()).toBe(before);
    } finally {
      stop();
    }
  });
});

describe("S8 another tab's storage events", () => {
  it("follows one that comes before this page has read its settings, reduce motion included", () => {
    const stop = subscribeSettings(noop);
    try {
      storageEvent(SETTINGS_STORAGE_KEY, JSON.stringify({ hoverPreviews: false, reduceMotion: true }));
      expect(readSettings()).toEqual({ ...NEW_PLAYER, hoverPreviews: false, reduceMotion: true });
      expect(document.documentElement.getAttribute("data-reduce-motion")).toBe("true");
    } finally {
      stop();
    }
  });

  it("re-reads storage on a keyless event (a clear somewhere), keeping what is still stored", () => {
    const stop = subscribeSettings(noop);
    try {
      writeSettings({ hoverPreviews: false });
      storageEvent(null, null);
      expect(readSettings().hoverPreviews).toBe(false);
      window.localStorage.clear();
      storageEvent(null, null);
      expect(readSettings()).toEqual(NEW_PLAYER);
    } finally {
      stop();
    }
  });

  it("listens while any subscriber is left and lets go after the last", () => {
    const remove = vi.spyOn(window, "removeEventListener");
    try {
      const stopFirst = subscribeSettings(noop);
      const stopSecond = subscribeSettings(noop);
      stopFirst();
      storageEvent(SETTINGS_STORAGE_KEY, JSON.stringify({ hoverPreviews: false }));
      expect(readSettings().hoverPreviews).toBe(false);
      expect(remove).not.toHaveBeenCalledWith("storage", expect.any(Function));
      stopSecond();
      expect(remove).toHaveBeenCalledWith("storage", expect.any(Function));
    } finally {
      remove.mockRestore();
    }
  });
});

describe("useSettings and useSetting", () => {
  it("re-render a component on a write", () => {
    const all = renderHook(() => useSettings());
    const one = renderHook(() => useSetting("hoverPreviews"));
    expect(all.result.current).toEqual(NEW_PLAYER);
    expect(one.result.current).toBe(true);
    act(() => {
      writeSettings({ hoverPreviews: false });
    });
    expect(all.result.current.hoverPreviews).toBe(false);
    expect(one.result.current).toBe(false);
    all.unmount();
    one.unmount();
  });
});

describe("__resetSettingsForTests", () => {
  it("leaves the document as a fresh page finds it", () => {
    writeSettings({ reduceMotion: true });
    __resetSettingsForTests();
    expect(document.documentElement.hasAttribute("data-reduce-motion")).toBe(false);
  });
});
