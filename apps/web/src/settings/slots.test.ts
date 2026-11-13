// The feature-owned settings slots (`./slots.ts`, R669): one entry per section control,
// each with a unique id, a render and a reset. The panel integration that proves they mount
// is `wiring.test.tsx`; here the table's own shape, each slot's control and each reset are pinned.

import { cleanup, render, screen } from "@testing-library/react";
import { Fragment, createElement } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { CARD_SETTINGS_DEFAULTS, readCardSettings, writeCardSettings, type CardSettings } from "../cards/settings.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { readHapticsSettings, resetHapticsSettingsForTests, writeHapticsSettings } from "../haptics/settings.ts";
import { SETTINGS_SLOTS, type SettingsSlot } from "./slots.ts";

function slotById(id: string): SettingsSlot {
  const found = SETTINGS_SLOTS.find((entry) => entry.id === id);
  if (found === undefined) throw new Error(`no settings slot ${id}`);
  return found;
}

function resetStores(): void {
  localStorage.clear();
  resetAudioSettingsForTests();
  resetFxSettingsForTests();
  resetHapticsSettingsForTests();
}

beforeEach(resetStores);

afterEach(() => {
  cleanup();
  resetStores();
});

describe("SETTINGS_SLOTS", () => {
  it("lists one entry per control with a unique id", () => {
    const ids = SETTINGS_SLOTS.map((slot) => slot.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain("haptics");
    expect(ids).toContain("fx");
    expect(ids).toContain("audio");
    expect(ids).toContain("language");
  });

  it("files every slot under a known section", () => {
    for (const slot of SETTINGS_SLOTS) {
      expect(["gameplay", "visuals", "audio"]).toContain(slot.section);
      expect(typeof slot.render).toBe("function");
    }
  });

  it("gives every slot a reset", () => {
    for (const slot of SETTINGS_SLOTS) expect(typeof slot.reset).toBe("function");
  });

  it("R669 renders each slot's own control", () => {
    const controls: readonly (readonly [string, string])[] = [
      ["haptics", "setting-vibration"],
      ["fx", "setting-fxSpeed"],
      ["card-rarity-foil", "setting-rarityFoil"],
      ["card-foil", "setting-animatedFoil"],
      ["card-full-art", "setting-fullArt"],
      ["card-set-watermarks", "setting-setWatermarks"],
      ["card-opponent-cosmetics", "setting-opponentCosmetics"],
      ["card-board-names", "setting-boardNames"],
      ["card-device-tilt", "setting-deviceTilt"],
      ["audio", "audio-controls"],
      ["language", "setting-language"],
    ];
    expect(controls.map(([id]) => id)).toEqual(SETTINGS_SLOTS.map((entry) => entry.id));
    for (const [id, testid] of controls) {
      render(createElement(Fragment, null, slotById(id).render()));
      expect(screen.getByTestId(testid), id).toBeInTheDocument();
      cleanup();
    }
  });

  it("R669 mounts the audio controls with the panel's own class", () => {
    render(createElement(Fragment, null, slotById("audio").render()));
    expect(screen.getByTestId("audio-controls")).toHaveClass("settings-audio");
  });

  it("R669 the vibration slot's reset turns vibration back on", () => {
    writeHapticsSettings({ vibration: false });
    slotById("haptics").reset?.();
    expect(readHapticsSettings().vibration).toBe(true);
  });

  it("R669 each card slot's reset puts its own setting back to its default", () => {
    const changed: readonly (readonly [string, Partial<CardSettings>])[] = [
      ["card-rarity-foil", { rarityFoil: !CARD_SETTINGS_DEFAULTS.rarityFoil }],
      ["card-full-art", { fullArt: !CARD_SETTINGS_DEFAULTS.fullArt }],
      ["card-set-watermarks", { setWatermarks: !CARD_SETTINGS_DEFAULTS.setWatermarks }],
      ["card-opponent-cosmetics", { opponentCosmetics: !CARD_SETTINGS_DEFAULTS.opponentCosmetics }],
      ["card-board-names", { boardNames: "never" }],
      ["card-device-tilt", { deviceTilt: !CARD_SETTINGS_DEFAULTS.deviceTilt }],
    ];
    for (const [id, patch] of changed) {
      writeCardSettings(patch);
      expect(readCardSettings(), id).not.toEqual(CARD_SETTINGS_DEFAULTS);
      slotById(id).reset?.();
      expect(readCardSettings(), id).toEqual(CARD_SETTINGS_DEFAULTS);
    }
  });
});
