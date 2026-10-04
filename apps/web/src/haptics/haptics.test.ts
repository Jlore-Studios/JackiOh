// R658 (#259): the haptics player and its switch.

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import { baseView } from "../test/fixtures.ts";
import { HAPTIC_MIN_GAP_MS, HAPTIC_PATTERNS, createHaptics, hapticFor } from "./haptics.ts";
import {
  DEFAULT_HAPTICS_SETTINGS,
  HAPTICS_SETTINGS_KEY,
  readHapticsSettings,
  resetHapticsSettingsForTests,
  writeHapticsSettings,
} from "./settings.ts";

const view = baseView(); // the viewer is p1
const played = (player: "p1" | "p2"): GameEvent => ({ type: "cardPlayed", player, instanceId: "c1", defId: "core-004", costPaid: 2 });
const hit = (amount: number): GameEvent => ({ type: "damage", sourceId: "u1", targetId: "hero-p2", amount, combat: true });
const turn = (player: "p1" | "p2"): GameEvent => ({ type: "turnStarted", player, turn: 4 });

beforeEach(() => {
  localStorage.clear();
  resetHapticsSettingsForTests();
});

describe("R658 haptics", () => {
  it("R658 a drop is the viewer's own cardPlayed, a hit any damage over 0, a turn the viewer's own turnStarted", () => {
    expect(hapticFor(played("p1"), view)).toBe("drop");
    expect(hapticFor(played("p2"), view)).toBeNull();
    expect(hapticFor(hit(3), view)).toBe("hit");
    expect(hapticFor(hit(0), view)).toBeNull();
    expect(hapticFor(turn("p1"), view)).toBe("turn");
    expect(hapticFor(turn("p2"), view)).toBeNull();
    expect(hapticFor({ type: "healthLost", player: "p1", amount: 2 }, view)).toBeNull();
  });

  it("R658 each moment buzzes its pattern, at most once every HAPTIC_MIN_GAP_MS", () => {
    let t = 1000;
    const vibrate = vi.fn((_pattern: number[]) => true);
    const haptics = createHaptics({ vibrate, now: () => t, reducedMotion: () => false });
    haptics.onEvent(played("p1"), view);
    haptics.onEvent(hit(5), view);
    expect(vibrate.mock.calls).toEqual([[[...HAPTIC_PATTERNS.drop]]]);
    t += HAPTIC_MIN_GAP_MS;
    haptics.onEvent(hit(5), view);
    t += HAPTIC_MIN_GAP_MS;
    haptics.onEvent(turn("p1"), view);
    expect(vibrate.mock.calls.slice(1)).toEqual([[[...HAPTIC_PATTERNS.hit]], [[...HAPTIC_PATTERNS.turn]]]);
  });

  it("R658 nothing buzzes with the switch off, under reduced motion, or on a device with no vibrate", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    let reduced = true;
    const haptics = createHaptics({ vibrate, now: () => 0, reducedMotion: () => reduced });
    haptics.onEvent(played("p1"), view);
    reduced = false;
    writeHapticsSettings({ vibration: false });
    haptics.onEvent(played("p1"), view);
    expect(vibrate).not.toHaveBeenCalled();
    writeHapticsSettings({ vibration: true });
    haptics.onEvent(played("p1"), view);
    expect(vibrate).toHaveBeenCalledTimes(1);
    expect(() => {
      createHaptics({ vibrate: null }).onEvent(played("p1"), view);
    }).not.toThrow();
  });

  it("R658 a vibrate that throws is swallowed", () => {
    const haptics = createHaptics({
      vibrate: () => {
        throw new Error("blocked");
      },
      reducedMotion: () => false,
    });
    expect(() => {
      haptics.onEvent(turn("p1"), view);
    }).not.toThrow();
  });
});

describe("R658 the vibration switch", () => {
  it("R658 defaults on, persists to localStorage, and reads back a stored value", () => {
    expect(readHapticsSettings()).toEqual(DEFAULT_HAPTICS_SETTINGS);
    expect(DEFAULT_HAPTICS_SETTINGS.vibration).toBe(true);
    writeHapticsSettings({ vibration: false });
    expect(JSON.parse(localStorage.getItem(HAPTICS_SETTINGS_KEY) ?? "null")).toEqual({ vibration: false });
    resetHapticsSettingsForTests();
    expect(readHapticsSettings().vibration).toBe(false);
  });

  it("R658 unparsable or mistyped storage falls back to the default, and a refused write still holds in memory", () => {
    localStorage.setItem(HAPTICS_SETTINGS_KEY, "{not json");
    expect(readHapticsSettings()).toEqual(DEFAULT_HAPTICS_SETTINGS);
    resetHapticsSettingsForTests();
    localStorage.setItem(HAPTICS_SETTINGS_KEY, JSON.stringify({ vibration: "yes" }));
    expect(readHapticsSettings()).toEqual(DEFAULT_HAPTICS_SETTINGS);
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("quota");
    });
    expect(writeHapticsSettings({ vibration: false }).vibration).toBe(false);
    expect(readHapticsSettings().vibration).toBe(false);
    setItem.mockRestore();
  });
});
