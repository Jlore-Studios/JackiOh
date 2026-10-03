// #37, Global Cosmetic: the mulligan (and a turn passing) bled into the start-turn animation. Game
// holds the previous view while a burst's entries play, which left two stale things on screen over
// the `turnStarted` entry: the banner still read the old turn ("Mulligan", or the turn just ended),
// and a prompt that had faded out came back at full opacity and stayed until the burst was done.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, type AnimationEntry } from "./animations.ts";
import Game from "./Game.tsx";
import { promptOver } from "./promptOver.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide, pendingFor, withEvents } from "../test/fixtures.ts";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  localStorage.clear();
  __resetSettingsForTests();
});

const advance = (ms: number): void => {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
};

const banner = (): HTMLElement => screen.getByTestId("turn-banner");

describe("#37 the turn banner names the turn it announces", () => {
  const started: GameEvent = { type: "turnStarted", player: "p1", turn: 3 };
  const ended: GameEvent = { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 };
  const theirs: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
  const mana: GameEvent = { type: "manaChanged", player: "p2", current: 4, max: 4 };

  it("reads Opponent's turn while their turnStarted plays, though the board still shows your turn", () => {
    const before = withEvents(baseView({ active: "p1", turn: 3 }), [started]);
    const after = withEvents(baseView({ active: "p2", turn: 4 }), [started, ended, theirs, mana]);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);

    // First the end of your turn: the board is still the one it ended on.
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "turnEnded");
    expect(banner()).toHaveTextContent("Your turn");

    advance(ANIMATIONS.turnEnded.durationMs);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "turnStarted");
    expect(banner()).toHaveAttribute("data-animating", "turnStarted");
    expect(banner()).toHaveTextContent("Opponent's turn");

    // The banner's entry is over and the board has not caught up: it does not fall back to your turn.
    advance(ANIMATIONS.turnStarted.durationMs);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "manaChanged");
    expect(banner()).not.toHaveAttribute("data-animating");
    expect(banner()).toHaveTextContent("Opponent's turn");

    advance(ANIMATIONS.manaChanged.durationMs);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(banner()).toHaveTextContent("Opponent's turn");
  });

  it("reads Your turn while your turnStarted plays after the opponent's", () => {
    const before = withEvents(baseView({ active: "p2", turn: 4 }), [theirs]);
    const mine: GameEvent = { type: "turnStarted", player: "p1", turn: 5 };
    const after = withEvents(baseView({ active: "p1", turn: 5 }), [theirs, mine]);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    expect(banner()).toHaveTextContent("Opponent's turn");
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);

    expect(banner()).toHaveAttribute("data-animating", "turnStarted");
    expect(banner()).toHaveTextContent("Your turn");
  });
});

describe("#37 an answered mulligan does not come back over the first turn", () => {
  const HAND = [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" })];

  function mulliganOpen(): PlayerView {
    return baseView({
      turn: 0,
      phase: "mulligan",
      active: "p1",
      you: emptySide("p1", { hand: HAND }),
      opponent: emptySide("p2", { hand: { count: 4 } }),
      pending: pendingFor(
        "mulligan",
        HAND.map((c) => ({ key: c.instanceId, label: c.defId, instanceId: c.instanceId })),
        { min: 0, max: HAND.length, prompt: "Choose the cards to keep" },
      ),
      mulligan: { youReady: false, opponentReady: true },
    });
  }

  function firstTurn(): PlayerView {
    return withEvents(baseView({ turn: 1, phase: "main", active: "p1", you: emptySide("p1", { hand: HAND }) }), [
      { type: "promptAnswered", player: "p1", choiceId: "mulligan" },
      { type: "turnStarted", player: "p1", turn: 1 },
      { type: "manaChanged", player: "p1", current: 1, max: 1 },
    ]);
  }

  it("fades the modal out, then takes it down for the turn's banner, which names the turn", () => {
    const { rerender } = render(<Game view={mulliganOpen()} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("prompt-modal")).toBeInTheDocument();
    expect(banner()).toHaveTextContent("Mulligan");

    rerender(<Game view={firstTurn()} legal={[]} onAction={vi.fn()} />);
    // Its own fade-out: still on the board, animating.
    expect(screen.getByTestId("prompt-modal")).toHaveAttribute("data-animating", "promptAnswered");

    advance(ANIMATIONS.promptAnswered.durationMs);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "turnStarted");
    expect(screen.queryByTestId("prompt-modal")).toBeNull();
    expect(banner()).toHaveTextContent("Your turn");

    // And it stays the first turn's banner, not the mulligan's, until the board catches up.
    advance(ANIMATIONS.turnStarted.durationMs);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "manaChanged");
    expect(banner()).toHaveTextContent("Your turn");

    advance(ANIMATIONS.manaChanged.durationMs);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.queryByTestId("prompt-modal")).toBeNull();
    expect(screen.getByTestId("board")).toHaveAttribute("data-phase", "main");
  });
});

describe("#37 promptOver", () => {
  const entry = (...events: GameEvent[]): AnimationEntry =>
    ({ events, type: events[0]?.type, durationMs: 0, frames: new Map(), view: baseView() }) as unknown as AnimationEntry;
  const answered = entry({ type: "promptAnswered", player: "p1", choiceId: "c" });
  const theirAnswer = entry({ type: "promptAnswered", player: "p2", choiceId: "c" });
  const turn = entry({ type: "turnStarted", player: "p1", turn: 1 });
  const mulligan = baseView({ phase: "mulligan" });
  const main = baseView({ phase: "main" });

  it("is false for a quiet burst, and while the answer's own entry is still in flight", () => {
    expect(promptOver(main, [], null)).toBe(false);
    expect(promptOver(main, [answered], answered)).toBe(false);
  });

  it("is true once the viewer's answered entry is behind the one in flight", () => {
    expect(promptOver(main, [answered, turn], turn)).toBe(true);
  });

  it("ignores the other seat's answer", () => {
    expect(promptOver(main, [theirAnswer, turn], turn)).toBe(false);
  });

  it("ends a mulligan view (the seat that answered first, still waiting) when the first turn starts", () => {
    expect(promptOver(mulligan, [turn], turn)).toBe(true);
    expect(promptOver(mulligan, [theirAnswer], theirAnswer)).toBe(false);
    expect(promptOver(main, [turn], turn)).toBe(false);
  });
});
