// R2800000: Twisting Nether's set piece on the board. Its wipe holds the board for the set piece's beat
// (`data-set-piece`, `--set-piece-hold`), names its shape and the sides it takes from for board.css's
// twist (`data-set-piece-shape`, `data-set-piece-sides`), and a click skips it, as every set piece
// (R2785000). Big moments' "short" and "off", the effects speed and Reduce motion set it.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SET_PIECE_HOLD_MS, SET_PIECE_SHORT_HOLD_MS, WIPE_MS, scaleForSpeed } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { readRootVar } from "./rootVars.ts";

const lookup = lookupFromDefs(CATALOG);
const NETHER = "core-088";
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
const SKIP_HINT = "set-piece-skip";
/** The longest the play's own entry can take before the wipe starts (`ANIMATIONS.cardPlayed`, squeezed or not). */
const PLAY_MAX_MS = 1_000;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  document.body.innerHTML = "";
});

const unitsOf = (view: PlayerView, side: "you" | "opponent"): UnitView[] => view[side].units.flatMap((u) => (u === null ? [] : [u]));

/** p1 casts Twisting Nether over `sides`, as the engine streams it. */
function nether(view: PlayerView, sides: readonly ("you" | "opponent")[]): GameEvent[] {
  const doomed = sides.flatMap((side) => unitsOf(view, side));
  return [
    { type: "cardPlayed", player: "p1", instanceId: "nx", defId: NETHER, costPaid: 4 },
    { type: "hookResolving", sourceId: "nx", defId: NETHER, controller: "p1", hook: "cry" },
    ...doomed.map((u): GameEvent => ({
      type: "destroyed",
      instanceId: u.instanceId,
      defId: u.defId,
      owner: u.owner,
      controller: u.controller,
      attack: u.attack,
      maxHealth: u.maxHealth,
      killerId: null,
    })),
    { type: "cardResolved", player: "p1", instanceId: "nx", defId: NETHER, permanent: false, costPaid: 4 },
  ];
}

const animating = (): Element | null => screen.getByTestId("game").querySelector("[data-animating]");
const inFlight = (): string | null => screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;

/** The board before the Nether, then after it, run on until the wipe's entry is in flight. */
function castNether(sides: readonly ("you" | "opponent")[] = ["you", "opponent"]): HTMLElement {
  const view = fullBoardView();
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(view, START)} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(view, [...START, ...nether(view, sides)])} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  for (let waited = 0; waited < PLAY_MAX_MS && inFlight() === "cardPlayed"; waited += 10) {
    act(() => {
      vi.advanceTimersByTime(10);
    });
  }
  return screen.getByTestId("game");
}

describe("R2800000 Twisting Nether's set piece on the board", () => {
  it("R2800000 Nether holds as nether, shape vortex, sides \"you opponent\"", () => {
    const game = castNether();
    expect(inFlight()).toBe("destroyed");
    expect(game).toHaveAttribute("data-set-piece", "nether");
    expect(game).toHaveAttribute("data-set-piece-shape", "vortex");
    expect(game).toHaveAttribute("data-set-piece-sides", "you opponent");
    expect(readRootVar(game, "--set-piece-hold")).toBe(`${String(SET_PIECE_HOLD_MS)}ms`);
    expect(screen.getByTestId(SKIP_HINT)).toBeInTheDocument();
    expect(animating()).not.toBeNull();
    // The hold, then the wipe's own motion, and the board is the board again.
    act(() => {
      vi.advanceTimersByTime(SET_PIECE_HOLD_MS + WIPE_MS);
    });
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(game).not.toHaveAttribute("data-set-piece-shape");
    expect(game).not.toHaveAttribute("data-set-piece-sides");
  });

  it("R2800000 enemy-only sides \"opponent\", and board.css twists that side alone", () => {
    const game = castNether(["opponent"]);
    expect(game).toHaveAttribute("data-set-piece", "nether");
    expect(game).toHaveAttribute("data-set-piece-sides", "opponent");
    // Each side's zones twist only when the wipe takes from that side.
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "board.css"), "utf8");
    for (const side of ["you", "opponent"]) {
      expect(css).toContain(`[data-set-piece-shape="vortex"][data-set-piece-sides~="${side}"] .field .zone[data-side="${side}"]`);
    }
    expect(css).toMatch(/\[data-set-piece-sides~="opponent"\] \.field \.zone\[data-side="opponent"\] \{\s*animation: set-piece-twist var\(--set-piece-hold, 0ms\)/u);
    expect(css).toMatch(/@keyframes set-piece-twist \{/u);
  });

  it("R2800000 a click skips it", () => {
    const game = castNether();
    expect(game).toHaveAttribute("data-set-piece", "nether");
    fireEvent.click(game);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(game).not.toHaveAttribute("data-set-piece-shape");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(inFlight()).not.toBe("destroyed");
  });

  it("R2800000 short holds SET_PIECE_SHORT_HOLD_MS; off holds nothing and plays WIPE_MS", () => {
    setFxSettings({ bigMoments: "short" });
    const short = castNether();
    expect(readRootVar(short, "--set-piece-hold")).toBe(`${String(SET_PIECE_SHORT_HOLD_MS)}ms`);
    cleanup();

    setFxSettings({ bigMoments: "off" });
    const off = castNether();
    expect(inFlight()).toBe("destroyed");
    expect(off).not.toHaveAttribute("data-set-piece");
    expect(off).not.toHaveAttribute("data-set-piece-shape");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(WIPE_MS - 1);
    });
    expect(inFlight()).toBe("destroyed");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(inFlight()).not.toBe("destroyed");
  });

  it("R2800000 speed 2 halves the hold", () => {
    setFxSettings({ speed: 2 });
    const game = castNether();
    expect(game).toHaveAttribute("data-set-piece", "nether");
    expect(readRootVar(game, "--set-piece-hold")).toBe(`${String(scaleForSpeed(SET_PIECE_HOLD_MS, 2))}ms`);
    expect(scaleForSpeed(SET_PIECE_HOLD_MS, 2)).toBe(SET_PIECE_HOLD_MS / 2);
  });

  it("R2800000 Reduce motion: no hold, no hint, fx off", () => {
    writeSettings({ reduceMotion: true });
    const game = castNether();
    expect(game).toHaveAttribute("data-reduced-motion", "true");
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(game).not.toHaveAttribute("data-set-piece-shape");
    expect(readRootVar(game, "--set-piece-hold")).toBe("");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(animating()).toBeNull();
    expect(screen.getByTestId("fx-layer")).toHaveAttribute("data-fx", "off");
    expect(document.querySelectorAll('[data-fx]:not([data-testid="fx-layer"])')).toHaveLength(0);
  });
});
