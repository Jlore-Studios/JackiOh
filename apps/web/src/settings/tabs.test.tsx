// Issue #128: the settings dialog's sections are tabs. The tablist follows the ARIA tabs pattern
// (roles, aria-selected, aria-controls, a roving tabindex, arrow keys, Home and End); the tab the
// player used last opens next time, on this device; "Reset this tab" puts back only that tab's
// controls; and a section with nothing in it has no tab.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import SettingsPanel from "./SettingsPanel.tsx";
import type { SettingsSectionId, SettingsSlot } from "./slots.ts";
import { DEFAULT_SETTINGS, __resetSettingsForTests, readSettings, writeSettings } from "./store.ts";
import { SETTINGS_TAB_STORAGE_KEY, readRememberedTab, rememberTab } from "./tabs.ts";

const noop = (): void => undefined;

/** One slot per section, so all three tabs exist without the real stores. */
function slotsWith(reset: Partial<Record<SettingsSectionId, () => void>> = {}): SettingsSlot[] {
  return (["gameplay", "visuals", "audio"] as const).map((section) => ({
    section,
    id: `probe-${section}`,
    render: () => <input aria-label={`${section} probe`} data-testid={`probe-${section}`} />,
    ...(reset[section] === undefined ? {} : { reset: reset[section] }),
  }));
}

const tab = (id: SettingsSectionId): HTMLElement => screen.getByTestId(`settings-tab-${id}`);
const panelOf = (id: SettingsSectionId): HTMLElement => screen.getByTestId(`settings-section-${id}`);
const shown = (): string[] =>
  (["gameplay", "visuals", "audio"] as const).filter((id) => {
    const panel = screen.queryByTestId(`settings-section-${id}`);
    return panel !== null && !panel.hidden;
  });

beforeEach(() => {
  localStorage.clear();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
  __resetSettingsForTests();
});

describe("#128 the tablist", () => {
  it("is an ARIA tablist: a tab per section, each controlling its panel, which it labels", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);

    const list = screen.getByRole("tablist", { name: "Settings sections" });
    const tabs = within(list).getAllByRole("tab");
    expect(tabs.map((node) => node.textContent)).toEqual(["Gameplay", "Visuals", "Audio"]);
    for (const [index, id] of (["gameplay", "visuals", "audio"] as const).entries()) {
      const node = tabs[index] as HTMLElement;
      const panel = panelOf(id);
      expect(panel).toHaveAttribute("role", "tabpanel");
      expect(node.getAttribute("aria-controls")).toBe(panel.id);
      expect(panel.getAttribute("aria-labelledby")).toBe(node.id);
    }
  });

  it("opens on Gameplay the first time, showing only that tab, and a roving tabindex", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);

    expect(shown()).toEqual(["gameplay"]);
    expect(tab("gameplay")).toHaveAttribute("aria-selected", "true");
    expect(tab("visuals")).toHaveAttribute("aria-selected", "false");
    expect(tab("gameplay")).toHaveAttribute("tabindex", "0");
    expect(tab("visuals")).toHaveAttribute("tabindex", "-1");
    expect(tab("audio")).toHaveAttribute("tabindex", "-1");
  });

  it("a click shows that tab's panel and no other", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);

    fireEvent.click(tab("audio"));

    expect(shown()).toEqual(["audio"]);
    expect(tab("audio")).toHaveAttribute("aria-selected", "true");
    expect(tab("gameplay")).toHaveAttribute("aria-selected", "false");
    // The hidden panels are still in the document, with their controls.
    expect(screen.getByTestId("setting-dragToPlay")).toBeInTheDocument();
  });

  it("the arrow keys move between the tabs and wrap; Home and End go to the ends; focus follows", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    const list = screen.getByTestId("settings-tablist");

    fireEvent.keyDown(list, { key: "ArrowRight" });
    expect(shown()).toEqual(["visuals"]);
    expect(document.activeElement).toBe(tab("visuals"));

    fireEvent.keyDown(list, { key: "ArrowRight" });
    fireEvent.keyDown(list, { key: "ArrowRight" });
    expect(shown()).toEqual(["gameplay"]);
    expect(document.activeElement).toBe(tab("gameplay"));

    fireEvent.keyDown(list, { key: "ArrowLeft" });
    expect(shown()).toEqual(["audio"]);

    fireEvent.keyDown(list, { key: "Home" });
    expect(shown()).toEqual(["gameplay"]);
    fireEvent.keyDown(list, { key: "End" });
    expect(shown()).toEqual(["audio"]);
    expect(tab("audio")).toHaveAttribute("tabindex", "0");
  });

  it("a key the pattern does not use is left alone", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);

    const notPrevented = fireEvent.keyDown(screen.getByTestId("settings-tablist"), { key: "a" });

    expect(notPrevented).toBe(true);
    expect(shown()).toEqual(["gameplay"]);
  });

  it("a section with no switch and no slot has no tab", () => {
    // Every section draws now: gameplay, visuals and audio each have built-in switches, so no
    // section lacks content. There is no account section since #303.
    render(<SettingsPanel onClose={noop} slots={[]} />);

    expect(screen.getAllByRole("tab").map((node) => node.textContent)).toEqual(["Gameplay", "Visuals", "Audio"]);
    expect(screen.queryByTestId("settings-tab-account")).toBeNull();
  });
});

describe("#128 the tab the player used last opens next", () => {
  it("is remembered on this device and read back by the next dialog", () => {
    const first = render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    fireEvent.click(tab("visuals"));
    expect(localStorage.getItem(SETTINGS_TAB_STORAGE_KEY)).toBe("visuals");
    first.unmount();

    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);

    expect(shown()).toEqual(["visuals"]);
  });

  it("a caller's initialTab comes before the remembered one; one the dialog lacks is ignored", () => {
    rememberTab("visuals");
    const first = render(<SettingsPanel onClose={noop} slots={slotsWith()} initialTab="audio" />);
    expect(shown()).toEqual(["audio"]);
    first.unmount();

    // With no explicit tab the remembered one wins.
    render(<SettingsPanel onClose={noop} slots={[]} />);
    expect(shown()).toEqual(["visuals"]);
  });

  it("a stored value that is not a tab, or a storage that throws, opens Gameplay", () => {
    localStorage.setItem(SETTINGS_TAB_STORAGE_KEY, "<script>");
    const first = render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    expect(shown()).toEqual(["gameplay"]);
    first.unmount();

    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    expect(shown()).toEqual(["gameplay"]);
    expect(() => fireEvent.click(tab("audio"))).not.toThrow();
    expect(shown()).toEqual(["audio"]);
  });

  it("readRememberedTab only returns a tab the dialog has", () => {
    rememberTab("audio");
    expect(readRememberedTab(["gameplay", "visuals", "audio"])).toBe("audio");
    expect(readRememberedTab(["gameplay", "visuals"])).toBeNull();
  });
});

describe("#128 Reset this tab", () => {
  it("puts back this tab's switches and slots, and leaves every other tab as the player set it", () => {
    const resets = { gameplay: vi.fn(), visuals: vi.fn(), audio: vi.fn() };
    writeSettings({ dragToPlay: false, confirmEndTurn: true, reduceMotion: true });
    render(<SettingsPanel onClose={noop} slots={slotsWith(resets)} />);

    fireEvent.click(screen.getByTestId("settings-reset-tab"));

    expect(readSettings()).toEqual({ ...DEFAULT_SETTINGS, reduceMotion: true });
    expect(resets.gameplay).toHaveBeenCalledTimes(1);
    expect(resets.visuals).not.toHaveBeenCalled();
    expect(resets.audio).not.toHaveBeenCalled();

    fireEvent.click(tab("visuals"));
    fireEvent.click(screen.getByTestId("settings-reset-tab"));

    expect(readSettings()).toEqual(DEFAULT_SETTINGS);
    expect(resets.visuals).toHaveBeenCalledTimes(1);
    expect(resets.audio).not.toHaveBeenCalled();
  });

  it("Reset all still puts back every tab", () => {
    const resets = { gameplay: vi.fn(), visuals: vi.fn(), audio: vi.fn() };
    writeSettings({ dragToPlay: false, reduceMotion: true });
    render(<SettingsPanel onClose={noop} slots={slotsWith(resets)} />);

    fireEvent.click(screen.getByTestId("settings-reset"));

    expect(readSettings()).toEqual(DEFAULT_SETTINGS);
    for (const reset of Object.values(resets)) expect(reset).toHaveBeenCalledTimes(1);
  });
});

describe("#128 the dialog around the tabs", () => {
  it("opens with the first switch of the open tab focused", () => {
    // Every tab has a switch now (no account section since #303), so focus always lands on one.
    // The panel keeps its fallback to the tab itself, now unreachable, for a tab with none.
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    expect(document.activeElement).toBe(screen.getByTestId("setting-dragToPlay"));
  });

  it("Tab and Shift+Tab cycle inside the dialog, past the open tab's controls only", () => {
    render(<SettingsPanel onClose={noop} slots={slotsWith()} />);
    const dialog = screen.getByTestId("settings-panel");
    const reset = screen.getByTestId("settings-reset");

    reset.focus();
    expect(fireEvent.keyDown(dialog, { key: "Tab" })).toBe(false);
    expect(document.activeElement).toBe(screen.getByTestId("settings-close"));

    expect(fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true })).toBe(false);
    expect(document.activeElement).toBe(reset);
  });
});
