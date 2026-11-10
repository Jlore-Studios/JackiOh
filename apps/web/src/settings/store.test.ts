// Client-side preferences (`./store.ts`, S8, R2000000, R82/R345): invalid values take their defaults,
// unknown keys are dropped, writes notify, and the panel's switch reaches the document. One `it` per
// rule; the store is reset before and after each test, as `settings/wiring.test.tsx` does.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DEFAULT_SETTINGS,
  SETTINGS_STORAGE_KEY,
  __resetSettingsForTests,
  readSettings,
  resetSettings,
  subscribeSettings,
  writeSettings,
} from "./store.ts";

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
