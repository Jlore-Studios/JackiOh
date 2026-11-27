// R5128000: the panel's Low graphics. It holds the effects layer at intensity off and Big moments off without writing
// over the player's own choices, stills the moving card art and depth planes through them, draws no backdrop
// particles and leaves the CSS a root attribute that stills the pads and portraits and drops every backdrop blur.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { useArtMotion } from "../cards/cosmetics.ts";
import { useDepthMotion } from "../cards/depth.ts";
import {
  getChosenFxSettings,
  getFxSettings,
  LOW_GRAPHICS_FX,
  resetFxSettingsForTests,
  setFxSettings,
  useFxSettings,
} from "../fx/settings.ts";
import Backdrop from "../game/Backdrop.tsx";
import { currentCardMotion } from "../game/cardMotion.ts";
import { BACKDROP_PARTICLE_MAX } from "../game/config.ts";
import { SETTINGS_GROUPS } from "./groups.ts";
import SettingsPanel from "./SettingsPanel.tsx";
import { DEFAULT_SETTINGS, LOW_GRAPHICS_ATTRIBUTE, __resetSettingsForTests, readSettings, writeSettings } from "./store.ts";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");
const sheet = (path: string): string => readFileSync(join(SRC, path), "utf8");

const noop = (): void => undefined;

function reset(): void {
  localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
}

beforeEach(reset);

afterEach(() => {
  cleanup();
  reset();
});

function lowGraphics(on: boolean): void {
  act(() => {
    writeSettings({ lowGraphics: on });
  });
}

describe("R5128000 the Low graphics setting", () => {
  it("R5128000 is off for a new player and puts data-low-graphics on <html> while it is on", () => {
    expect(DEFAULT_SETTINGS.lowGraphics).toBe(false);
    expect(readSettings().lowGraphics).toBe(false);
    expect(document.documentElement.hasAttribute(LOW_GRAPHICS_ATTRIBUTE)).toBe(false);
    lowGraphics(true);
    expect(document.documentElement.getAttribute(LOW_GRAPHICS_ATTRIBUTE)).toBe("true");
    lowGraphics(false);
    expect(document.documentElement.hasAttribute(LOW_GRAPHICS_ATTRIBUTE)).toBe(false);
  });

  it("R5128000 holds intensity and Big moments off and keeps the speed and motion the player chose", () => {
    setFxSettings({ speed: 2, intensity: "high", bigMoments: "short", motion: "system" });
    lowGraphics(true);
    expect(getFxSettings()).toEqual({ speed: 2, intensity: "off", bigMoments: "off", motion: "system" });
    expect(LOW_GRAPHICS_FX).toEqual({ intensity: "off", bigMoments: "off" });
    // One snapshot while nothing changes, as useSyncExternalStore needs.
    expect(getFxSettings()).toBe(getFxSettings());
  });

  it("R5128000 never writes over the player's choices: the store keeps them and turning it off gives them back", () => {
    setFxSettings({ intensity: "high", bigMoments: "short" });
    lowGraphics(true);
    expect(getChosenFxSettings()).toMatchObject({ intensity: "high", bigMoments: "short" });
    // A change made while it holds is the player's own choice, kept for later.
    setFxSettings({ intensity: "low" });
    expect(getFxSettings().intensity).toBe("off");
    expect(getChosenFxSettings().intensity).toBe("low");
    lowGraphics(false);
    expect(getFxSettings()).toMatchObject({ intensity: "low", bigMoments: "short" });
    expect(getFxSettings()).toBe(getChosenFxSettings());
  });

  it("R5128000 the account keeps the chosen effects settings, never the held ones", () => {
    setFxSettings({ intensity: "high" });
    lowGraphics(true);
    const fx = SETTINGS_GROUPS.find((group) => group.id === "fx");
    expect(fx?.read()).toMatchObject({ intensity: "high", bigMoments: "full" });
    expect(SETTINGS_GROUPS.find((group) => group.id === "gameplay")?.read()).toMatchObject({ lowGraphics: true });
  });

  it("R5128000 a mounted reader follows it on and off at once", () => {
    const { result } = renderHook(() => useFxSettings()[0].intensity);
    expect(result.current).toBe("normal");
    lowGraphics(true);
    expect(result.current).toBe("off");
    lowGraphics(false);
    expect(result.current).toBe("normal");
  });

  it("R5128000 card motion goes calm, not still, and moving art, foil and depth planes hold still", () => {
    expect(currentCardMotion()).toBe("full");
    const art = renderHook(() => useArtMotion(true));
    const depth = renderHook(() => useDepthMotion());
    expect(art.result.current.enabled).toBe(true);
    expect(depth.result.current.flat).toBe(false);
    lowGraphics(true);
    expect(currentCardMotion()).toBe("calm");
    expect(art.result.current.enabled).toBe(false);
    expect(depth.result.current).toEqual({ flat: true, fxOff: true });
  });

  it("R5128000 the board backdrop keeps its look and draws no particles", () => {
    const { container } = render(<Backdrop field="field-embers" />);
    const backdrop = container.querySelector('.board-backdrop[data-backdrop="field-embers"]');
    expect(backdrop?.querySelectorAll(".board-backdrop-particle")).toHaveLength(BACKDROP_PARTICLE_MAX);
    lowGraphics(true);
    expect(backdrop?.querySelectorAll(".board-backdrop-particle")).toHaveLength(0);
    expect(container.querySelector('.board-backdrop[data-backdrop="field-embers"]')).not.toBeNull();
  });

  it("R5128000 the sheets drop every backdrop blur and still the pads' and portraits' loops under the attribute", () => {
    const index = sheet("index.css");
    expect(index).toMatch(/:root\[data-low-graphics="true"\] \*,[\s\S]*?backdrop-filter: none !important;/);
    // The pads' override comes after their motion gate, so at the pad rules' own weight it wins.
    const table = sheet("game/table.css");
    const still = table.indexOf(':root[data-low-graphics="true"] .field[data-table] .zone[data-side][data-row="units"]');
    expect(still).toBeGreaterThan(table.lastIndexOf('animation: table-twinkle'));
    expect(table.slice(still, table.indexOf("}", still))).toMatch(/\{\s*will-change: auto;\s*animation-name: none;\s*$/);
    const portrait = sheet("emotes/portrait.css");
    expect(portrait).toMatch(
      /:root\[data-low-graphics="true"\] \.portrait-art-breath,\s*:root\[data-low-graphics="true"\] \.portrait-art-mote \{\s*animation: none;/,
    );
  });

  it("R5128000 Settings shows the switch on the Visuals tab, and holds the effects' intensity and Big moments while it is on", () => {
    setFxSettings({ intensity: "high", bigMoments: "short" });
    render(<SettingsPanel onClose={noop} />);
    fireEvent.click(screen.getByTestId("settings-tab-visuals"));
    const toggle = screen.getByRole("switch", { name: "Low graphics" });
    expect(toggle).toBe(screen.getByTestId("setting-lowGraphics"));
    expect(screen.getByTestId("settings-section-visuals").contains(toggle)).toBe(true);
    const intensity = screen.getByTestId<HTMLSelectElement>("setting-fxIntensity");
    const big = screen.getByTestId<HTMLSelectElement>("setting-fxBigMoments");
    expect(intensity.disabled).toBe(false);

    fireEvent.click(toggle);
    expect(readSettings().lowGraphics).toBe(true);
    // The player's own choices stay on show, held.
    expect(intensity.disabled).toBe(true);
    expect(big.disabled).toBe(true);
    expect(intensity.value).toBe("high");
    expect(big.value).toBe("short");
    expect(screen.getAllByText("Off while Low graphics is on.")).toHaveLength(2);

    fireEvent.click(screen.getByTestId("settings-reset-tab"));
    expect(readSettings().lowGraphics).toBe(false);
    expect(intensity.disabled).toBe(false);
  });
});
