// R2801000: a Starship's launch on the board. After its awakening breaks the seal (#2435), its launch
// holds the board for the Starship's set piece (`data-set-piece="starship"`, `--set-piece-hold`) behind
// the "Tap to skip" hint, and a click skips it, as every set piece (R2785000). Big moments' "short" and
// "off", the effects speed and Reduce motion set it; under Reduce motion nothing holds and the launch is
// one still frame.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, SET_PIECE_SHORT_HOLD_MS, STARSHIP_LAUNCH_HOLD_MS, STARSHIP_SET_PIECE, scaleForSpeed } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, dormantStarship, emptySide, unit, withEvents } from "../test/fixtures.ts";

const lookup = lookupFromDefs(CATALOG);
const HULL = "stinkyscape-t-dormant-starship";
const PIECES = ["stinkyscape-140", "stinkyscape-141"];
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 5 }];
const SKIP_HINT = "set-piece-skip";
/** The longest the awakening's entry can take before the launch's starts. */
const AWAKEN_MAX_MS = 1_000;
const LAUNCH_MS = ANIMATIONS.starshipLaunched.durationMs;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  document.body.innerHTML = "";
});

/** p1's board, its Starship in lane 3: Dormant before the launch, launched after it. */
function boardOf(launched: boolean): PlayerView {
  const ship = launched
    ? unit("p1", { instanceId: "ship", defId: HULL, attack: 0, starship: { pieces: PIECES.map((defId) => ({ defId })), name: "Ember of Departure", launched: 1 } })
    : dormantStarship("p1", PIECES, { instanceId: "ship" });
  return baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, ship, null, null] }),
    opponent: emptySide("p2", { units: [unit("p2", { instanceId: "foe" }), null, null, null, null] }),
  });
}

const LAUNCH: GameEvent[] = [
  { type: "awakened", player: "p1", instanceId: "ship", defId: HULL },
  { type: "starshipLaunched", player: "p1", instanceId: "ship", defId: HULL, name: "Ember of Departure", pieces: PIECES },
];

const inFlight = (): string | null => screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;

/** The board before the launch, then after it, run on until the launch's entry is in flight. */
function launch(): HTMLElement {
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(boardOf(false), START)} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(boardOf(true), [...START, ...LAUNCH])} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  for (let waited = 0; waited < AWAKEN_MAX_MS && inFlight() === "awakened"; waited += 10) {
    act(() => {
      vi.advanceTimersByTime(10);
    });
  }
  return screen.getByTestId("game");
}

describe("R2801000 a Starship's launch on the board", () => {
  it("R2801000 holds as starship with the hint", () => {
    const game = launch();
    expect(inFlight()).toBe("starshipLaunched");
    expect(game).toHaveAttribute("data-set-piece", STARSHIP_SET_PIECE);
    // No board clear's twist: the launch draws over the ship alone.
    expect(game).not.toHaveAttribute("data-set-piece-shape");
    expect(game.style.getPropertyValue("--set-piece-hold")).toBe(`${String(STARSHIP_LAUNCH_HOLD_MS)}ms`);
    expect(screen.getByTestId(SKIP_HINT)).toBeInTheDocument();
    expect(screen.getByTestId("card-ship")).toHaveAttribute("data-animating", "starshipLaunched");
    // The hold, then the lift-off and the landing, and the board shows the launched Starship.
    act(() => {
      vi.advanceTimersByTime(STARSHIP_LAUNCH_HOLD_MS + LAUNCH_MS);
    });
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.getByTestId("card-ship")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("card-ship").querySelector(".cf-starship-hull")).toBeNull();
  });

  it("R2801000 a click skips it", () => {
    const game = launch();
    expect(game).toHaveAttribute("data-set-piece", STARSHIP_SET_PIECE);
    fireEvent.click(game);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(inFlight()).not.toBe("starshipLaunched");
  });

  it("R2801000 short and off", () => {
    setFxSettings({ bigMoments: "short" });
    const short = launch();
    expect(short).toHaveAttribute("data-set-piece", STARSHIP_SET_PIECE);
    expect(short.style.getPropertyValue("--set-piece-hold")).toBe(`${String(SET_PIECE_SHORT_HOLD_MS)}ms`);
    cleanup();

    setFxSettings({ bigMoments: "off" });
    const off = launch();
    expect(inFlight()).toBe("starshipLaunched");
    expect(off).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(LAUNCH_MS - 1);
    });
    expect(inFlight()).toBe("starshipLaunched");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(inFlight()).not.toBe("starshipLaunched");
  });

  it("R2801000 speed 2", () => {
    setFxSettings({ speed: 2 });
    const game = launch();
    expect(game).toHaveAttribute("data-set-piece", STARSHIP_SET_PIECE);
    expect(game.style.getPropertyValue("--set-piece-hold")).toBe(`${String(scaleForSpeed(STARSHIP_LAUNCH_HOLD_MS, 2))}ms`);
    expect(scaleForSpeed(STARSHIP_LAUNCH_HOLD_MS, 2)).toBe(STARSHIP_LAUNCH_HOLD_MS / 2);
  });

  it("R2801000 R3334001 reduce motion: no hold, the still title card [data-fx=cinematic][data-still]", () => {
    // jsdom lays nothing out: give every element a box, so the still frame has somewhere to stand.
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue({
      x: 300, y: 200, left: 300, top: 200, width: 90, height: 126, right: 390, bottom: 326, toJSON: () => ({}),
    } as DOMRect);
    writeSettings({ reduceMotion: true });
    const game = launch();
    expect(game).toHaveAttribute("data-reduced-motion", "true");
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(game.style.getPropertyValue("--set-piece-hold")).toBe("");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(inFlight()).toBeNull();
    act(() => {
      vi.advanceTimersByTime(50);
    });
    const still = document.querySelector<HTMLElement>('[data-fx="cinematic"]');
    expect(still).not.toBeNull();
    expect(still).toHaveAttribute("data-still", "true");
    // R3334001: the ship's picture and its public name, nothing moving.
    expect(still?.querySelector<HTMLImageElement>(".fx-cinematic-camera > .fx-cinematic-art")?.getAttribute("src")).toMatch(/^data:image\/svg\+xml/);
    expect(still?.querySelector(".fx-cinematic-name")?.getAttribute("data-text")).toBe("Ember of Departure");
    // Nothing else is drawn: no canvas, and the title card is the one effect.
    expect(document.querySelectorAll('[data-fx]:not([data-testid="fx-layer"])')).toHaveLength(1);
  });
});
