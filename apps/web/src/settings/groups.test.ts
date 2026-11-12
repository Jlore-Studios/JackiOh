// The stores the account sync sees (`./groups.ts`, #129, R633, R1295000, R2718000): each group reads its
// store flat, applies back through the store's writer, and watches through its subscription. One `it`
// per rule; every store is put back to its defaults after each test, as `settings/wiring.test.tsx` does.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { DEVICE_CARD_SETTINGS, CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { SETTINGS_GROUPS } from "./groups.ts";
import { __resetSettingsForTests, writeSettings } from "./store.ts";

function group(id: string) {
  const found = SETTINGS_GROUPS.find((candidate) => candidate.id === id);
  if (found === undefined) throw new Error(`no settings group ${id}`);
  return found;
}

beforeEach(() => {
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  resetAudioSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  window.localStorage.clear();
});

afterEach(() => {
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  resetAudioSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  window.localStorage.clear();
});

describe("SETTINGS_GROUPS", () => {
  it("lists one group per store the account keeps", () => {
    expect(SETTINGS_GROUPS.map((candidate) => candidate.id)).toEqual([
      "gameplay",
      "audio",
      "fx",
      "cards",
      "cosmetics",
    ]);
  });

  it("reads the gameplay store flat and applies back through its writer", () => {
    writeSettings({ hoverPreviews: false });
    expect(group("gameplay").read().hoverPreviews).toBe(false);
    group("gameplay").apply({ hoverPreviews: true, notASetting: 1 });
    expect(group("gameplay").read().hoverPreviews).toBe(true);
    expect(group("gameplay").read()).not.toHaveProperty("notASetting");
  });

  it("keeps the device's own card settings out of the account both ways (R2718000)", () => {
    expect(DEVICE_CARD_SETTINGS.length).toBeGreaterThan(0);
    const read = group("cards").read();
    for (const key of DEVICE_CARD_SETTINGS) expect(read).not.toHaveProperty(key);
    expect(read.hoverPreviews).toBe(true);
    const before = writeCardSettings({ setWatermarks: false });
    expect(before.setWatermarks).toBe(false);
    group("cards").apply({ setWatermarks: true, hoverPreviews: false });
    const after = group("cards").read();
    expect(after.hoverPreviews).toBe(false);
    expect(after).not.toHaveProperty("setWatermarks");
  });

  it("does not wake the sync for a device-only card change", () => {
    const listener = vi.fn();
    const stop = group("cards").subscribe(listener);
    try {
      writeCardSettings({ deviceTilt: true });
      expect(listener).not.toHaveBeenCalled();
      writeCardSettings({ hoverPreviews: false });
      expect(listener).toHaveBeenCalledTimes(1);
    } finally {
      stop();
    }
  });

  it("keeps a cosmetic id it does not know, so a newer client's pick comes back intact (R1295000)", () => {
    group("cosmetics").apply({ foil: "a-foil-from-a-newer-client" });
    expect(group("cosmetics").read().foil).toBe("a-foil-from-a-newer-client");
  });
});
