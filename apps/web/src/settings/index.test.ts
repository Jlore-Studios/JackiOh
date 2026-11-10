// The settings barrel (`./index.ts`, docs/polish/7-mobile-ux.md S8): every slice imports
// from here. The integration that proves the wiring is `wiring.test.tsx`; here the barrel's
// own re-exports are pinned, so a dropped export fails colocated with the source.

import { describe, expect, it } from "vitest";

import {
  DEFAULT_SETTINGS,
  readSettings,
  resetSettings,
  SETTINGS_SLOTS,
  SettingsButton,
  SettingsPanel,
  writeSettings,
} from "./index.ts";

describe("settings barrel", () => {
  it("re-exports the store's defaults and its reader and writer", () => {
    expect(DEFAULT_SETTINGS.dragToPlay).toBe(true);
    expect(typeof readSettings).toBe("function");
    expect(typeof writeSettings).toBe("function");
    expect(typeof resetSettings).toBe("function");
  });

  it("re-exports the slots and the panel and button", () => {
    expect(SETTINGS_SLOTS.length).toBeGreaterThan(0);
    expect(typeof SettingsPanel).toBe("function");
    expect(typeof SettingsButton).toBe("function");
  });
});
