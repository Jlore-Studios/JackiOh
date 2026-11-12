// The gear that opens the settings dialog (`./SettingsButton.tsx`, S8, B23, #128): a top-right
// control per placement, closed until clicked, focused again after the dialog closes. One `it` per
// rule; the dialog is closed and the stores reset after each test.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import SettingsButton from "./SettingsButton.tsx";
import { __resetSettingsForTests } from "./store.ts";

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
  cleanup();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  resetAudioSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  window.localStorage.clear();
});

describe("SettingsButton", () => {
  it("renders one closed gear per placement", () => {
    render(<SettingsButton placement="game" />);
    const gear = screen.getByTestId("settings-open-game");
    expect(gear).toHaveAttribute("aria-haspopup", "dialog");
    expect(gear).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId("settings-panel")).toBeNull();
  });

  it("draws the board's own bar in game and the top bar's round control in nav", () => {
    const game = render(<SettingsButton placement="game" />);
    expect(screen.getByTestId("settings-open-game").className).not.toContain("top-bar-button");
    game.unmount();
    render(<SettingsButton placement="nav" />);
    expect(screen.getByTestId("settings-open-nav").className).toContain("top-bar-button--round");
  });

  it("opens the dialog on click and hands focus back to the gear on close", () => {
    render(<SettingsButton placement="game" />);
    const gear = screen.getByTestId("settings-open-game");
    fireEvent.click(gear);
    expect(gear).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId("settings-panel")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("settings-close"));
    expect(screen.queryByTestId("settings-panel")).toBeNull();
    expect(document.activeElement).toBe(gear);
  });

  it("opens on the asked tab ahead of the remembered one", () => {
    window.localStorage.setItem("jackioh.settings.tab", "audio");
    render(<SettingsButton placement="game" tab="visuals" />);
    fireEvent.click(screen.getByTestId("settings-open-game"));
    expect(screen.getByTestId("settings-tab-visuals")).toHaveAttribute("aria-selected", "true");
  });
});
