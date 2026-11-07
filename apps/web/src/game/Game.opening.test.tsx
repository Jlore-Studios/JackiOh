// #3347: the opening deal. The first view of a game in the mulligan phase is dealt a card at a time,
// after the decks riffle, with a sound for each; a tap skips it; reduced motion and effects off drop its motion.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { getAudioEngine } from "../audio/engine.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { DEAL_STAGGER_MS, RIFFLE_MS } from "./config.ts";
import Game from "./Game.tsx";

const HAND = 3;

/** A game's first view, in the mulligan: three cards in your hand, three backs in theirs. */
const mulligan = () =>
  baseView({
    phase: "mulligan",
    you: emptySide("p1", { hand: Array.from({ length: HAND }, () => card()) }),
    opponent: emptySide("p2", { hand: { count: HAND } }),
  });

let playSfx: ReturnType<typeof vi.spyOn>;

const sounds = (id: string): number => playSfx.mock.calls.filter((call: readonly unknown[]) => call[0] === id).length;
const advance = (ms: number): void => {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
};

beforeEach(() => {
  vi.useFakeTimers();
  playSfx = vi.spyOn(getAudioEngine(), "playSfx").mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  playSfx.mockRestore();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

describe("#3347 the opening deal", () => {
  it("#3347 a mulligan first view is dealt: the board is marked, and both decks riffle", () => {
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    const game = screen.getByTestId("game");
    expect(game).toHaveAttribute("data-dealing", "true");
    expect(game).toHaveAttribute("data-card-motion", "full");
    expect(game).toHaveAttribute("data-riffle", "you opponent");
  });

  it("#3347 the riffle sounds once at the start, then one deal sound a card, DEAL_STAGGER_MS apart", () => {
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    expect(sounds("riffle")).toBe(1);
    expect(sounds("deal")).toBe(0);
    advance(RIFFLE_MS - 1);
    expect(sounds("deal")).toBe(0);
    advance(1);
    expect(sounds("deal")).toBe(1);
    advance(DEAL_STAGGER_MS - 1);
    expect(sounds("deal")).toBe(1);
    advance(1);
    expect(sounds("deal")).toBe(2);
    advance(10_000);
    expect(sounds("deal")).toBe(2 * HAND);
    expect(sounds("riffle")).toBe(1);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-riffle");
  });

  it("#3347 a tap on the board skips the deal: the marks go and no later card sounds", () => {
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    advance(RIFFLE_MS + DEAL_STAGGER_MS);
    expect(sounds("deal")).toBe(2);
    fireEvent.click(screen.getByTestId("hand-you"));
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-riffle");
    advance(10_000);
    expect(sounds("deal")).toBe(2);
  });

  it("#3347 the tap that skips goes on to what it was on, so a press on a control is never eaten", () => {
    const onAction = vi.fn();
    render(<Game view={mulligan()} legal={[{ type: "endTurn" }]} onAction={onAction} />);
    fireEvent.click(screen.getByTestId("end-turn"));
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(onAction).toHaveBeenCalledWith({ type: "endTurn" });
  });

  it("#3347 reduced motion has no deal: no mark, no deal sound", () => {
    writeSettings({ reduceMotion: true });
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(screen.getByTestId("game")).toHaveAttribute("data-card-motion", "still");
    advance(10_000);
    expect(sounds("deal")).toBe(0);
    expect(sounds("riffle")).toBe(0);
  });

  it("#3347 effects off keeps the sounds and drops the motion", () => {
    setFxSettings({ intensity: "off" });
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(screen.getByTestId("game")).toHaveAttribute("data-card-motion", "calm");
    advance(10_000);
    expect(sounds("riffle")).toBe(1);
    expect(sounds("deal")).toBe(2 * HAND);
  });

  it("#3347 the opponent's cards stay backs: counted, never named", () => {
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    const theirs = screen.getByTestId("hand-opponent");
    expect(theirs.querySelectorAll(".hand-slot")).toHaveLength(HAND);
    expect(theirs.querySelector('[data-testid^="hand-card-"]')).toBeNull();
    expect(theirs.innerHTML).not.toContain("core-001");
  });

  it("#3347 only a game's first view is dealt: not a later view, not another phase, not a replay", () => {
    const { rerender } = render(<Game view={baseView()} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    cleanup();
    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} replay />);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    expect(sounds("deal")).toBe(0);
  });

  it("#3347 under StrictMode's double-run effects the deal still ends, by its time and by a tap", () => {
    render(
      <StrictMode>
        <Game view={mulligan()} legal={[]} onAction={vi.fn()} />
      </StrictMode>,
    );
    expect(screen.getByTestId("game")).toHaveAttribute("data-dealing", "true");
    advance(10_000);
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
    cleanup();
    render(
      <StrictMode>
        <Game view={mulligan()} legal={[]} onAction={vi.fn()} />
      </StrictMode>,
    );
    fireEvent.click(screen.getByTestId("hand-you"));
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-dealing");
  });

  it("#3347 a board that unmounts mid-deal sounds nothing more", () => {
    const { unmount } = render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    advance(RIFFLE_MS);
    unmount();
    advance(10_000);
    expect(sounds("deal")).toBe(1);
  });
});
