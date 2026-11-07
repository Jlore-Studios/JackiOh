// R669 (#259): the haptics player and its switch.

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "../audio/constants.ts";
import { baseView } from "../test/fixtures.ts";
import { HAPTIC_MIN_GAP_MS, HAPTIC_PATTERNS, createHaptics, hapticFor, resetTapHapticsForTests, tapHaptic } from "./haptics.ts";
import {
  DEFAULT_HAPTICS_SETTINGS,
  HAPTICS_SETTINGS_KEY,
  readHapticsSettings,
  resetHapticsSettingsForTests,
  writeHapticsSettings,
} from "./settings.ts";

const view = baseView(); // the viewer is p1
const played = (player: "p1" | "p2"): GameEvent => ({ type: "cardPlayed", player, instanceId: "c1", defId: "core-004", costPaid: 2 });
const playedFor = (player: "p1" | "p2", costPaid: number, over: Partial<Extract<GameEvent, { type: "cardPlayed" }>> = {}): GameEvent => ({
  type: "cardPlayed",
  player,
  instanceId: "c1",
  defId: "core-004",
  costPaid,
  ...over,
});
const hit = (amount: number): GameEvent => ({ type: "damage", sourceId: "u1", targetId: "hero-p2", amount, combat: true });
const turn = (player: "p1" | "p2"): GameEvent => ({ type: "turnStarted", player, turn: 4 });

beforeEach(() => {
  localStorage.clear();
  resetHapticsSettingsForTests();
  resetTapHapticsForTests();
});

describe("R669 haptics", () => {
  it("R669 a drop is the viewer's own cardPlayed, a hit any damage over 0, a turn the viewer's own turnStarted", () => {
    expect(hapticFor(played("p1"), view)).toBe("drop");
    expect(hapticFor(played("p2"), view)).toBeNull();
    expect(hapticFor(hit(3), view)).toBe("hit");
    expect(hapticFor(hit(0), view)).toBeNull();
    expect(hapticFor(turn("p1"), view)).toBe("turn");
    expect(hapticFor(turn("p2"), view)).toBeNull();
    expect(hapticFor({ type: "healthLost", player: "p1", amount: 2 }, view)).toBeNull();
  });

  it("R669 each moment buzzes its pattern, at most once every HAPTIC_MIN_GAP_MS", () => {
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

  it("R669 nothing buzzes with the switch off, under reduced motion, or on a device with no vibrate", () => {
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

  it("R669 a vibrate that throws is swallowed", () => {
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

describe("R669 the vibration switch", () => {
  it("R669 defaults on, persists to localStorage, and reads back a stored value", () => {
    expect(readHapticsSettings()).toEqual(DEFAULT_HAPTICS_SETTINGS);
    expect(DEFAULT_HAPTICS_SETTINGS.vibration).toBe(true);
    writeHapticsSettings({ vibration: false });
    expect(JSON.parse(localStorage.getItem(HAPTICS_SETTINGS_KEY) ?? "null")).toEqual({ vibration: false });
    resetHapticsSettingsForTests();
    expect(readHapticsSettings().vibration).toBe(false);
  });

  it("R669 unparsable or mistyped storage falls back to the default, and a refused write still holds in memory", () => {
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

describe("R3348000 a press's buzz", () => {
  it("R3348000 each moment buzzes its pattern, a legal tile harder than another, and a rare outcome longest", () => {
    let t = 0;
    const vibrate = vi.fn((_pattern: number[]) => true);
    const haptics = createHaptics({ vibrate, now: () => t, reducedMotion: () => false });
    for (const moment of ["tile", "tileLegal", "corner", "cornerBig", "cornerRare"] as const) {
      haptics.buzz(moment);
      t += HAPTIC_MIN_GAP_MS;
    }
    expect(vibrate.mock.calls).toEqual([
      [[...HAPTIC_PATTERNS.tile]],
      [[...HAPTIC_PATTERNS.tileLegal]],
      [[...HAPTIC_PATTERNS.corner]],
      [[...HAPTIC_PATTERNS.cornerBig]],
      [[...HAPTIC_PATTERNS.cornerRare]],
    ]);
    expect(HAPTIC_PATTERNS.tileLegal[0]).toBeGreaterThan(HAPTIC_PATTERNS.tile[0] ?? 0);
    const total = (pattern: readonly number[]): number => pattern.reduce((sum, part) => sum + part, 0);
    expect(total(HAPTIC_PATTERNS.cornerRare)).toBeGreaterThan(total(HAPTIC_PATTERNS.cornerBig));
  });

  it("R3348000 a buzz follows the switch, reduced motion and a device with no vibrate, and keeps its own gap", () => {
    let t = 0;
    let reduced = true;
    const vibrate = vi.fn((_pattern: number[]) => true);
    const haptics = createHaptics({ vibrate, now: () => t, reducedMotion: () => reduced });
    haptics.buzz("tile");
    reduced = false;
    writeHapticsSettings({ vibration: false });
    haptics.buzz("tile");
    expect(vibrate).not.toHaveBeenCalled();
    writeHapticsSettings({ vibration: true });
    haptics.buzz("tile");
    haptics.buzz("tile");
    expect(vibrate).toHaveBeenCalledTimes(1);
    t += HAPTIC_MIN_GAP_MS;
    haptics.buzz("tile");
    expect(vibrate).toHaveBeenCalledTimes(2);
    expect(() => {
      createHaptics({ vibrate: null }).buzz("corner");
    }).not.toThrow();
  });

  it("R3348000 a buzz never holds back the game's own drop, nor a drop a buzz", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    const haptics = createHaptics({ vibrate, now: () => 1000, reducedMotion: () => false });
    haptics.buzz("tileLegal");
    haptics.onEvent(played("p1"), view);
    expect(vibrate.mock.calls).toEqual([[[...HAPTIC_PATTERNS.tileLegal]], [[...HAPTIC_PATTERNS.drop]]]);
  });

  it("R3348000 tapHaptic buzzes through the browser's vibrate, and stops with the switch", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    Object.defineProperty(navigator, "vibrate", { configurable: true, value: vibrate });
    try {
      tapHaptic("tile");
      expect(vibrate).toHaveBeenCalledWith([...HAPTIC_PATTERNS.tile]);
      resetTapHapticsForTests();
      writeHapticsSettings({ vibration: false });
      tapHaptic("tile");
      expect(vibrate).toHaveBeenCalledTimes(1);
    } finally {
      Reflect.deleteProperty(navigator, "vibrate");
    }
  });
});

describe("R3346000 a heavy play's buzz", () => {
  it("R3346000 a play that cost 6 to 8 buzzes heavy and one that cost 9 or more massive, for either seat's play", () => {
    for (const player of ["p1", "p2"] as const) {
      expect([6, 8].map((cost) => hapticFor(playedFor(player, cost), view)), player).toEqual(["playHeavy", "playHeavy"]);
      expect([9, 12].map((cost) => hapticFor(playedFor(player, cost), view)), player).toEqual(["playMassive", "playMassive"]);
    }
  });

  it("R3346000 a lighter play keeps the viewer's own drop and the other seat's none", () => {
    expect([0, 2, 5].map((cost) => hapticFor(playedFor("p1", cost), view))).toEqual(["drop", "drop", "drop"]);
    expect([0, 2, 5].map((cost) => hapticFor(playedFor("p2", cost), view))).toEqual([null, null, null]);
  });

  it("R3346000 a hidden card or a set face-down gets no weight, at any cost, whichever seat plays it", () => {
    const sentinel = (player: "p1" | "p2", cost: number): GameEvent => playedFor(player, cost, { defId: HIDDEN_DEF_ID });
    expect(hapticFor(sentinel("p2", 9), view)).toBeNull();
    expect(hapticFor(sentinel("p2", 6), view)).toBeNull();
    // The viewer's own Trap set (R227's `formerId`) is no landing: it keeps the plain drop.
    expect(hapticFor(playedFor("p1", 9, { formerId: "h1" }), view)).toBe("drop");
    expect(hapticFor(playedFor("p2", 9, { formerId: "h1" }), view)).toBeNull();
  });

  it("R3346000 a massive pattern is longer and rolls on past a heavy one, and each buzzes once per HAPTIC_MIN_GAP_MS", () => {
    const total = (pattern: readonly number[]): number => pattern.reduce((sum, part) => sum + part, 0);
    expect(total(HAPTIC_PATTERNS.playMassive)).toBeGreaterThan(total(HAPTIC_PATTERNS.playHeavy));
    expect(total(HAPTIC_PATTERNS.playHeavy)).toBeGreaterThan(total(HAPTIC_PATTERNS.drop));
    let t = 1000;
    const vibrate = vi.fn((_pattern: number[]) => true);
    const haptics = createHaptics({ vibrate, now: () => t, reducedMotion: () => false });
    haptics.onEvent(playedFor("p2", 6), view);
    haptics.onEvent(playedFor("p2", 9), view);
    expect(vibrate.mock.calls).toEqual([[[...HAPTIC_PATTERNS.playHeavy]]]);
    t += HAPTIC_MIN_GAP_MS;
    haptics.onEvent(playedFor("p1", 9), view);
    expect(vibrate.mock.calls.slice(1)).toEqual([[[...HAPTIC_PATTERNS.playMassive]]]);
  });

  it("R3346000 nothing buzzes for a heavy play with the Vibration switch off, under reduced motion or with no vibrate", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    let reduced = true;
    const haptics = createHaptics({ vibrate, now: () => 0, reducedMotion: () => reduced });
    haptics.onEvent(playedFor("p2", 9), view);
    reduced = false;
    writeHapticsSettings({ vibration: false });
    haptics.onEvent(playedFor("p2", 9), view);
    haptics.onEvent(playedFor("p1", 6), view);
    expect(vibrate).not.toHaveBeenCalled();
    writeHapticsSettings({ vibration: true });
    haptics.onEvent(playedFor("p2", 9), view);
    expect(vibrate.mock.calls).toEqual([[[...HAPTIC_PATTERNS.playMassive]]]);
    expect(() => {
      createHaptics({ vibrate: null }).onEvent(playedFor("p2", 9), view);
    }).not.toThrow();
  });
});
