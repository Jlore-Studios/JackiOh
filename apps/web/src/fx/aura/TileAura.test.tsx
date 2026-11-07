// R3342000: tests for the TileAura component (issue #3342).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import { setAudioEngineForTests } from "../../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../../audio/test/fakeAudio.ts";
import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import { testid } from "../../game/contract.ts";
import Zone from "../../game/Zone.tsx";
import { touchHaptic } from "../../haptics/haptics.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import { baseView, emptySide, unit } from "../../test/fixtures.ts";
import { setReducedMotion } from "../../test/setup.ts";
import { CATALOG } from "../../wire/cards.ts";
import { FX_PARTICLE_CAP_MOBILE } from "../constants.ts";
import { resetFxSettingsForTests, setFxSettings } from "../settings.ts";
import { TILE_PARTICLE_SHARE, TILE_TOUCH_GAP_MS } from "./config.ts";
import { tileMoteBudget } from "./tile.ts";

vi.mock("../../haptics/haptics.ts", async () => {
  const actual = await vi.importActual("../../haptics/haptics.ts");
  return {
    ...actual,
    touchHaptic: vi.fn(),
  };
});

const lookup = lookupFromDefs(CATALOG);

let engine: FakeAudioEngine;

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance", "Date"] });
  localStorage.clear();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  engine = fakeAudioEngine();
  setAudioEngineForTests(engine);
  vi.clearAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
  setReducedMotion(false);
  resetFxSettingsForTests();
  __resetSettingsForTests();
  setAudioEngineForTests(null);
  localStorage.clear();
  vi.clearAllMocks();
});

function renderZone(view: PlayerView, highlight?: Parameters<typeof Zone>[0]["highlight"], animating?: Parameters<typeof Zone>[0]["animating"]) {
  return render(
    <CatalogContext.Provider value={lookup}>
      <Zone view={view} side="you" row="units" lane={1} highlight={highlight} animating={animating} />
    </CatalogContext.Provider>,
  );
}

describe("R3342000 TileAura component", () => {
  it("R3342000 the aura is the zone's first child with tier, look, pattern; no defId leaks into HTML", () => {
    // core-100 is a Mythic unit
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: "u-mythic-1" })] }),
    });
    renderZone(view);

    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    const aura = zone.querySelector(".tile-aura");
    expect(aura).not.toBeNull();
    expect(zone.firstElementChild).toBe(aura);

    expect(aura?.getAttribute("data-tile-tier")).toBe("Mythic");
    expect(aura?.getAttribute("data-tile-look")).toBeDefined();
    expect(aura?.getAttribute("data-tile-pattern")).toBe("circuitry");
    expect(aura?.outerHTML).not.toContain("core-100");
  });

  it("R3342000 mouse hover and press triggers one ripple, auraTouch sound, no haptic, throttling rapid touches", async () => {
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: "u-mythic-1" })] }),
    });
    renderZone(view);

    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    const aura = zone.querySelector(".tile-aura")!;
    expect(aura).not.toBeNull();

    // Hover (pointerenter with mouse)
    fireEvent.pointerEnter(zone);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledWith("auraTouch", expect.objectContaining({ hue: expect.any(Number) }));
    expect(touchHaptic).not.toHaveBeenCalled();
    expect(aura.querySelector(".tile-aura-ripple")).not.toBeNull();

    // Second touch inside gap is throttled
    fireEvent.pointerDown(zone);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);

    // After gap, pointerDown fires again
    vi.advanceTimersByTime(TILE_TOUCH_GAP_MS + 20);
    fireEvent.pointerDown(zone);
    expect(engine.playSfx).toHaveBeenCalledTimes(2);
    expect(touchHaptic).not.toHaveBeenCalled(); // mouse pointerdown does not call touchHaptic
  });

  it("R3342000 touch pointerdown calls touchHaptic", async () => {
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: "u-mythic-1" })] }),
    });
    renderZone(view);

    const zone = screen.getByTestId(testid.zone("you", "units", 1));

    vi.advanceTimersByTime(TILE_TOUCH_GAP_MS + 10);

    const touchEvent = new Event("pointerdown", { bubbles: true });
    Object.defineProperty(touchEvent, "pointerType", { value: "touch" });
    fireEvent(zone, touchEvent);

    expect(touchHaptic).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
  });

  it("R3342000 animating damage sets hit, destroyed sets out, selected sets lit, slumber sets dim", () => {
    const unitId = "u-state-1";
    const cardTestid = testid.card(unitId);

    // Damage -> hit
    const damageView = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: unitId })] }),
    });
    const { unmount: unmount1 } = renderZone(damageView, undefined, new Map([[cardTestid, "damage"]]));
    expect(screen.getByTestId(testid.zone("you", "units", 1)).querySelector(".tile-aura")?.getAttribute("data-tile-hit")).toBe("true");
    unmount1();

    // Destroyed -> state="out"
    const { unmount: unmount2 } = renderZone(damageView, undefined, new Map([[cardTestid, "destroyed"]]));
    expect(screen.getByTestId(testid.zone("you", "units", 1)).querySelector(".tile-aura")?.getAttribute("data-tile-state")).toBe("out");
    unmount2();

    // Selected -> lit="true"
    const { unmount: unmount3 } = renderZone(damageView, {
      legal: new Set(),
      selected: new Set([cardTestid]),
    });
    expect(screen.getByTestId(testid.zone("you", "units", 1)).querySelector(".tile-aura")?.getAttribute("data-tile-lit")).toBe("true");
    unmount3();

    // Slumber -> state="dim"
    const slumberView = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: unitId, slumber: {} })] }),
    });
    const { unmount: unmount4 } = renderZone(slumberView);
    expect(screen.getByTestId(testid.zone("you", "units", 1)).querySelector(".tile-aura")?.getAttribute("data-tile-state")).toBe("dim");
    unmount4();
  });

  it("R3342000 under setReducedMotion and reduceMotion setting: motion is still, no motes, no ripple, sound plays", async () => {
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: "u-still-1" })] }),
    });

    // 1. via setReducedMotion(true)
    setReducedMotion(true);
    const { unmount: unmount1 } = renderZone(view);
    const zone1 = screen.getByTestId(testid.zone("you", "units", 1));
    const aura1 = zone1.querySelector(".tile-aura")!;
    expect(aura1.getAttribute("data-tile-motion")).toBe("still");
    expect(aura1.querySelectorAll(".tile-aura-mote")).toHaveLength(0);

    fireEvent.pointerEnter(zone1);
    expect(aura1.querySelectorAll(".tile-aura-ripple")).toHaveLength(0);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    unmount1();

    // 2. via setting panel reduceMotion
    setReducedMotion(false);
    writeSettings({ reduceMotion: true });
    vi.advanceTimersByTime(10);

    const { unmount: unmount2 } = renderZone(view);
    const zone2 = screen.getByTestId(testid.zone("you", "units", 1));
    const aura2 = zone2.querySelector(".tile-aura")!;
    expect(aura2.getAttribute("data-tile-motion")).toBe("still");
    expect(aura2.querySelectorAll(".tile-aura-mote")).toHaveLength(0);
    unmount2();
  });

  it("R3342000 intensity off renders no .tile-aura", () => {
    setFxSettings({ intensity: "off" });
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-100", instanceId: "u-off-1" })] }),
    });
    renderZone(view);

    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    expect(zone.querySelector(".tile-aura")).toBeNull();
  });

  it("R3342000 at width 390 motes in a 20-tile board stay within budget", () => {
    const singleBudget = tileMoteBudget("Mythic", 20, 1, 390);
    expect(20 * singleBudget).toBeLessThanOrEqual(Math.floor(TILE_PARTICLE_SHARE * FX_PARTICLE_CAP_MOBILE));
  });
});
