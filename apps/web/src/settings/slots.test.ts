// The feature-owned settings slots (`./slots.ts`, R669): one entry per section control,
// each with a unique id, a render and a reset. The panel integration that proves they mount
// is `wiring.test.tsx`; here the table's own shape, each slot's control and each reset are pinned.

import { cleanup, render, screen } from "@testing-library/react";
import { Fragment, createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { CARD_SETTINGS_DEFAULTS, readCardSettings, writeCardSettings, type CardSettings } from "../cards/settings.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { readHapticsSettings, resetHapticsSettingsForTests, writeHapticsSettings } from "../haptics/settings.ts";
import { SETTINGS_SLOTS, type SettingsSlot } from "./slots.ts";

type SlotsModule = typeof import("./slots.ts");

/** The slots as a page load builds them: a fresh module imported inside the test, so the table is made for it. */
async function freshSlots(): Promise<SlotsModule> {
  vi.resetModules();
  return import("./slots.ts");
}

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

  it("R669 as a page load builds them: every section filled, every id its own, every slot mounting a control no other slot mounts", async () => {
    const { SETTINGS_SLOTS: slots } = await freshSlots();
    const sections = ["gameplay", "visuals", "audio"];
    for (const section of sections) {
      expect(slots.some((entry) => entry.section === section), section).toBe(true);
    }
    const controls: string[] = [];
    for (const entry of slots) {
      expect(sections, entry.id).toContain(entry.section);
      expect(entry.id.length, entry.section).toBeGreaterThan(0);
      const { container } = render(createElement(Fragment, null, entry.render()));
      const control = container.querySelector("[data-testid]")?.getAttribute("data-testid");
      expect(control, entry.id).toBeTruthy();
      controls.push(control ?? "");
      cleanup();
    }
    expect(new Set(slots.map((entry) => entry.id)).size).toBe(slots.length);
    expect(new Set(controls).size).toBe(slots.length);
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
