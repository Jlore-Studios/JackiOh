// R2785000: a set piece on the board. While one holds, the board says so (`data-set-piece`, its
// `--set-piece-hold`) and keeps `data-animating` on, a "Tap to skip" hint fades in, and a tap or a
// click on the board, or Space, ends the hold at once and does nothing else. Replays, watching and
// both hotseat seats play it; the Duos teammate's silent board and Reduce motion take none. The
// fixture is #97 Zephrys, a Mythic Spell, whose takeover (R2722001) is the first set piece.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, SET_PIECE_SKIP_HINT_MS, TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game, { type GameProps } from "./Game.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";
import { readRootVar } from "./rootVars.ts";

const lookup = lookupFromDefs(CATALOG);
const ZEPHRYS = "core-097";
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
const cast = (defId: string, player: PlayerId = "p1"): GameEvent => ({ type: "cardPlayed", player, instanceId: `i-${defId}`, defId, costPaid: 3 });
const SKIP_HINT = "set-piece-skip";

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

/** The board before the play, then after it: the play's entry is in flight once this returns. */
function playOut(defId: string, props: Partial<GameProps> = {}, player: PlayerId = "p1"): HTMLElement {
  const before = withEvents(baseView(), START);
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={before} legal={[]} onAction={vi.fn()} {...props} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(baseView(), [...START, cast(defId, player)])} legal={[]} onAction={vi.fn()} {...props} />
    </CatalogContext.Provider>,
  );
  return screen.getByTestId("game");
}

/** An element of the board mid-animation, which `cy.settled()` waits on. */
const animating = (): Element | null => screen.getByTestId("game").querySelector("[data-animating]");

describe("R2785000 a set piece on the board", () => {
  it("R2785000 a click on the board skips the hold and the board catches up at once; the click reaches no control", () => {
    const game = playOut(ZEPHRYS);
    expect(game).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    expect(screen.getByTestId(SKIP_HINT)).toHaveTextContent("Tap to skip");
    expect(animating()).not.toBeNull();

    const zone = screen.getByTestId("zone-you-units-1");
    const pressed = vi.fn();
    zone.addEventListener("click", pressed);
    fireEvent.click(zone);
    expect(pressed).not.toHaveBeenCalled();
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(animating()).toBeNull();
    // With the hold over, a click is a click again.
    fireEvent.click(zone);
    expect(pressed).toHaveBeenCalledTimes(1);
  });

  it("R2785000 a tap on the board's own surface skips it too", () => {
    const game = playOut(ZEPHRYS);
    fireEvent.click(game);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(animating()).toBeNull();
  });

  it("R2785000 Space skips it, and the keyup of that Space presses nothing; Space in a text field does not", () => {
    const field = document.createElement("input");
    document.body.appendChild(field);
    const game = playOut(ZEPHRYS);
    expect(fireEvent.keyDown(field, { key: " " })).toBe(true);
    expect(game).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    // Another key, or Space with a modifier, is not the skip.
    fireEvent.keyDown(document, { key: "Enter" });
    fireEvent.keyDown(document, { key: " ", ctrlKey: true });
    expect(game).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);

    expect(fireEvent.keyDown(document, { key: " " })).toBe(false);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(animating()).toBeNull();
    // The keyup that would press a focused button is swallowed, once; Space after it is Space again.
    expect(fireEvent.keyUp(document, { key: " " })).toBe(false);
    expect(fireEvent.keyDown(document, { key: " " })).toBe(true);
    expect(fireEvent.keyUp(document, { key: " " })).toBe(true);
  });

  it("R2785000 data-animating stays on through the hold", () => {
    playOut(ZEPHRYS);
    const total = TAKEOVER_HOLD_MS + ANIMATIONS.cardPlayed.durationMs;
    act(() => {
      vi.advanceTimersByTime(total - 1);
    });
    expect(animating()).not.toBeNull();
    expect(screen.getByTestId("game")).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(animating()).toBeNull();
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-set-piece");
  });

  it("R2785000 a replay's and a watched board play it, the silent Duos teammate board does not", () => {
    // A replay and watching (together/WatchRoute.tsx) both draw a `replay` board.
    const replay = playOut(ZEPHRYS, { replay: true });
    expect(replay).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    expect(screen.getByTestId(SKIP_HINT)).toBeInTheDocument();
    fireEvent.click(replay);
    expect(replay).not.toHaveAttribute("data-set-piece");
    cleanup();

    const teammate = playOut(ZEPHRYS, { replay: true, silent: true });
    expect(teammate).not.toHaveAttribute("data-set-piece");
    expect(readRootVar(teammate, "--set-piece-hold")).toBe("");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    // Its entry plays the table's time alone.
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.cardPlayed.durationMs);
    });
    expect(animating()).toBeNull();
  });

  it("R2785000 the opponent's Mythic holds the viewer's board too (hotseat's both seats)", () => {
    const game = playOut(ZEPHRYS, {}, "p2");
    expect(game).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    expect(readRootVar(game, "--set-piece-hold")).toBe(`${String(TAKEOVER_HOLD_MS)}ms`);
  });

  it("R2785000 under reduced motion no hold, no hint, no effects", () => {
    writeSettings({ reduceMotion: true });
    const game = playOut(ZEPHRYS);
    expect(game).toHaveAttribute("data-reduced-motion", "true");
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(game).not.toHaveAttribute("data-takeover");
    expect(readRootVar(game, "--set-piece-hold")).toBe("");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    expect(animating()).toBeNull();
    // The effects layer is off under Reduce motion (R200), and draws nothing: no shake, flash or sweep.
    expect(screen.getByTestId("fx-layer")).toHaveAttribute("data-fx", "off");
    expect(document.querySelectorAll('[data-fx]:not([data-testid="fx-layer"])')).toHaveLength(0);
  });

  it("R2785000 Big moments off plays the Mythic with no hold and no hint", () => {
    setFxSettings({ bigMoments: "off" });
    const game = playOut(ZEPHRYS);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.cardPlayed.durationMs);
    });
    expect(animating()).toBeNull();
  });

  it("R2785000 the hint fades in after SET_PIECE_SKIP_HINT_MS", () => {
    const game = playOut(ZEPHRYS);
    expect(readRootVar(game, "--set-piece-skip-delay")).toBe(`${String(SET_PIECE_SKIP_HINT_MS)}ms`);
    const hint = screen.getByTestId(SKIP_HINT);
    expect(hint).toHaveClass("set-piece-skip");
    expect(hint).toHaveAttribute("aria-hidden", "true");
    // board.css fades it in after that delay, from nothing.
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "board.css"), "utf8");
    const rule = /\.set-piece-skip \{([^}]*)\}/u.exec(css)?.[1] ?? "";
    expect(rule).toMatch(/animation: set-piece-skip-in [^;]*var\(--set-piece-skip-delay, 0ms\) both;/u);
    expect(rule).toContain("pointer-events: none;");
    expect(css).toMatch(/@keyframes set-piece-skip-in \{\s*from \{ opacity: 0; \}/u);
  });
});
