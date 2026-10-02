// Issue #37 (patch v0.2.1): a batch of events that crosses into a new turn played the new turn's
// banner and draw over the OLD board — a unit switched to Defense as the player's last move, which
// R345 follows with the turn's end and the next turn's start, stood upright under "Opponent's turn";
// the second mulligan answer played turn 1's banner over the opening hand. Game now splits such a
// batch at its last turn boundary (`turnBoundary`): what comes before it plays over the board it
// happened on, the board swaps to the new view, and the boundary and everything after it play over
// that view. Nothing is dropped, and a view arriving meanwhile queues behind the whole of it.

import type { ActionBody, GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, createAnimationQueue } from "./animations.ts";
import Game, { turnBoundary } from "./Game.tsx";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, card, emptySide, unit, withEvents } from "../test/fixtures.ts";

const END_TURN: ActionBody = { type: "endTurn" };

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

function playing(): string | null {
  return screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;
}

const board = (): HTMLElement => screen.getByTestId("board");

describe("turnBoundary", () => {
  it("is the last turnStarted or turnAutoEnded of a batch, or -1", () => {
    const switched: GameEvent = { type: "positionSwitched", instanceId: "u1", position: "DEF" };
    const auto: GameEvent = { type: "turnAutoEnded", player: "p1", turn: 3 };
    const ended: GameEvent = { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 };
    const started: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
    expect(turnBoundary([switched, auto, ended, started])).toBe(3);
    expect(turnBoundary([switched, auto, ended])).toBe(1);
    expect(turnBoundary([started, switched])).toBe(0);
    expect(turnBoundary([switched])).toBe(-1);
    expect(turnBoundary([])).toBe(-1);
  });
});

describe("the runner's checkpoint", () => {
  it("runs between the entries either side of it, at once when idle, and never after a drain", () => {
    const timers: (() => void)[] = [];
    const queue = createAnimationQueue({ schedule: (fn) => timers.push(fn), reducedMotion: false });
    const view = baseView();
    const order: string[] = [];
    queue.checkpoint(() => order.push("idle"));
    expect(order).toEqual(["idle"]);

    queue.enqueue([{ type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 }], view);
    queue.checkpoint(() => order.push(`swap while ${queue.inFlight()?.type ?? "nothing"} is in flight`));
    queue.enqueue([{ type: "turnStarted", player: "p2", turn: 4 }], view);
    expect(queue.pending(), "a checkpoint is not an entry").toBe(1);
    expect(order).toEqual(["idle"]);
    timers.shift()?.();
    expect(order).toEqual(["idle", "swap while nothing is in flight"]);
    expect(queue.inFlight()?.type).toBe("turnStarted");

    queue.enqueue([{ type: "turnEnded", player: "p2", turn: 4, unspentMana: 0 }], view);
    queue.checkpoint(() => order.push("dropped"));
    queue.drain();
    expect(order).not.toContain("dropped");
  });
});

describe("a batch that crosses into a new turn", () => {
  // p1 switches its last unit to Defense; R345 ends the turn it has no move left in, and p2's turn
  // starts with its refresh and its draw. All of it arrives as one view.
  const mine = unit("p1", { instanceId: "u-mine", position: "ATK" });
  const START: GameEvent = { type: "turnStarted", player: "p1", turn: 3 };
  const switched: GameEvent = { type: "positionSwitched", instanceId: "u-mine", position: "DEF" };
  const autoEnded: GameEvent = { type: "turnAutoEnded", player: "p1", turn: 3 };
  const ended: GameEvent = { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 };
  const started: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
  const refreshed: GameEvent = { type: "manaChanged", player: "p2", current: 5, max: 5 };
  const drew: GameEvent = { type: "drawn", player: "p2", instanceId: "hidden", defId: "hidden" };

  const before = withEvents(
    baseView({ active: "p1", turn: 3, you: emptySide("p1", { units: [mine, null, null, null, null] }) }),
    [START],
  );
  const after = withEvents(
    baseView({
      active: "p2",
      turn: 4,
      you: emptySide("p1", { units: [{ ...mine, position: "DEF" }, null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 5 }, mana: { current: 5, max: 5 }, libraryCount: 11 }),
    }),
    [START, switched, autoEnded, ended, started, refreshed, drew],
  );

  it("plays the switch and the turn's end over the old board, then the new turn over the new one, dropping nothing", () => {
    const { rerender } = render(<Game view={before} legal={[END_TURN]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);

    // 1. The switch, over the board it happened on: the unit turns from Attack.
    expect(playing()).toBe("positionSwitched");
    const turning = screen.getByTestId("card-u-mine");
    expect(turning).toHaveAttribute("data-animating", "positionSwitched");
    expect(turning).toHaveAttribute("data-position", "ATK");
    expect(board()).toHaveAttribute("data-active", "you");

    // 2. The auto-ended turn's banner, still on the old board — and the unit stays in Defense,
    //    where its rotation ended, rather than snapping back upright until the swap.
    advance(ANIMATIONS.positionSwitched.durationMs);
    expect(playing()).toBe("turnAutoEnded");
    const settled = screen.getByTestId("card-u-mine");
    expect(settled).not.toHaveAttribute("data-animating");
    expect(settled).toHaveAttribute("data-position", "DEF");
    expect(settled.getAttribute("style")).toContain("rotate(90deg)");
    expect(screen.getByTestId("turn-banner")).toHaveTextContent("No moves left. Turn ended.");
    expect(board()).toHaveAttribute("data-active", "you");

    advance(ANIMATIONS.turnAutoEnded.durationMs);
    expect(playing()).toBe("turnEnded");
    expect(screen.getByTestId("end-turn")).toHaveAttribute("data-animating", "turnEnded");
    expect(board()).toHaveAttribute("data-active", "you");

    // 3. The boundary: the board swaps to the new view, and the new turn's banner plays over it.
    advance(ANIMATIONS.turnEnded.durationMs);
    expect(playing()).toBe("turnStarted");
    expect(board()).toHaveAttribute("data-active", "opponent");
    expect(board()).toHaveAttribute("data-turn", "4");
    const banner = screen.getByTestId("turn-banner");
    expect(banner).toHaveAttribute("data-animating", "turnStarted");
    expect(banner).toHaveTextContent("Opponent's turn");
    expect(screen.getByTestId("card-u-mine")).toHaveAttribute("data-position", "DEF");

    // 4. What follows the boundary still plays, over the new view.
    advance(ANIMATIONS.turnStarted.durationMs);
    expect(playing()).toBe("manaChanged");
    expect(screen.getByTestId("mana-opponent")).toHaveAttribute("data-animating", "manaChanged");
    advance(ANIMATIONS.manaChanged.durationMs);
    expect(playing()).toBe("drawn");
    expect(screen.getByTestId("library-opponent")).toHaveAttribute("data-animating", "drawn");
    advance(ANIMATIONS.drawn.durationMs);
    expect(playing()).toBeNull();
    expect(board()).toHaveAttribute("data-active", "opponent");
  });

  it("queues a view that arrives during the new turn's animations behind them, in order", () => {
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
    advance(
      ANIMATIONS.positionSwitched.durationMs + ANIMATIONS.turnAutoEnded.durationMs + ANIMATIONS.turnEnded.durationMs,
    );
    expect(playing()).toBe("turnStarted");

    // p2 plays a card while its banner is still up.
    const played: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "c9", defId: "core-004", costPaid: 3 };
    const landed: GameEvent = { type: "summoned", player: "p2", instanceId: "c9", defId: "core-004", row: "units", lane: 2 };
    const later = withEvents(
      {
        ...after,
        opponent: {
          ...after.opponent,
          hand: { count: 4 },
          units: [null, unit("p2", { instanceId: "c9" }), null, null, null],
        },
      },
      [...after.events, played, landed],
    );
    rerender(<Game view={later} legal={[]} onAction={vi.fn()} />);

    // The tail goes on as it was, over the view it was planned against, and the play waits for it.
    expect(playing()).toBe("turnStarted");
    advance(ANIMATIONS.turnStarted.durationMs);
    expect(playing()).toBe("manaChanged");
    expect(screen.queryByTestId("card-c9"), "the later view is not on the board yet").toBeNull();
    advance(ANIMATIONS.manaChanged.durationMs);
    expect(playing()).toBe("drawn");
    advance(ANIMATIONS.drawn.durationMs);
    expect(playing()).toBe("cardPlayed");
    advance(ANIMATIONS.cardPlayed.durationMs);
    expect(playing()).toBeNull();
    expect(screen.getByTestId("card-c9")).toBeInTheDocument();
  });

  it("holds the new turn's moves until its banner and draw have played", () => {
    // p2 ends its turn and p1's begins: the batch starts with the boundary's turnEnded.
    const theirs = withEvents(baseView({ active: "p2", turn: 4 }), [started]);
    const p2Ended: GameEvent = { type: "turnEnded", player: "p2", turn: 4, unspentMana: 1 };
    const mineStarted: GameEvent = { type: "turnStarted", player: "p1", turn: 5 };
    const myRefresh: GameEvent = { type: "manaChanged", player: "p1", current: 5, max: 5 };
    const myDraw: GameEvent = { type: "drawn", player: "p1", instanceId: "c40", defId: "core-004" };
    const ours = withEvents(
      baseView({ active: "p1", turn: 5, you: emptySide("p1", { hand: [card({ instanceId: "c40" })] }) }),
      [started, p2Ended, mineStarted, myRefresh, myDraw],
    );
    const onAction = vi.fn();
    const { rerender } = render(<Game view={theirs} legal={[]} onAction={onAction} />);
    rerender(<Game view={ours} legal={[END_TURN]} onAction={onAction} />);

    expect(playing()).toBe("turnEnded");
    expect(board()).toHaveAttribute("data-active", "opponent");
    expect(screen.getByTestId("end-turn")).toBeDisabled();

    advance(ANIMATIONS.turnEnded.durationMs);
    expect(playing()).toBe("turnStarted");
    expect(board()).toHaveAttribute("data-active", "you");
    expect(screen.getByTestId("turn-banner")).toHaveTextContent("Your turn");
    expect(screen.getByTestId("end-turn"), "not while the new turn is still animating").toBeDisabled();

    advance(ANIMATIONS.turnStarted.durationMs + ANIMATIONS.manaChanged.durationMs);
    expect(playing()).toBe("drawn");
    expect(screen.getByTestId("end-turn")).toBeDisabled();
    advance(ANIMATIONS.drawn.durationMs);
    expect(playing()).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });

  it("turns a unit a start-of-turn switch knocks upright from Defense, over a view that already has it upright", () => {
    const guard = unit("p1", { instanceId: "u-guard", position: "DEF" });
    const theirs = withEvents(baseView({ active: "p2", you: emptySide("p1", { units: [guard, null, null, null, null] }) }), [
      { type: "turnStarted", player: "p2", turn: 4 },
    ]);
    const knocked: GameEvent = { type: "positionSwitched", instanceId: "u-guard", position: "ATK" };
    const ours = withEvents(
      baseView({ active: "p1", turn: 5, you: emptySide("p1", { units: [{ ...guard, position: "ATK" }, null, null, null, null] }) }),
      [...theirs.events, { type: "turnStarted", player: "p1", turn: 5 }, knocked],
    );
    const { rerender } = render(<Game view={theirs} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={ours} legal={[]} onAction={vi.fn()} />);

    expect(playing()).toBe("turnStarted");
    advance(ANIMATIONS.turnStarted.durationMs);
    expect(playing()).toBe("positionSwitched");
    // Drawn in the pose it turns from, so animations.css turns it the right way, to Attack.
    expect(screen.getByTestId("card-u-guard")).toHaveAttribute("data-position", "DEF");
    advance(ANIMATIONS.positionSwitched.durationMs);
    expect(playing()).toBeNull();
    expect(screen.getByTestId("card-u-guard")).toHaveAttribute("data-position", "ATK");
  });

  it("under Reduce motion shows the newest view and offers its moves at once", () => {
    writeSettings({ reduceMotion: true });
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[END_TURN]} onAction={vi.fn()} />);
    expect(playing()).toBeNull();
    expect(board()).toHaveAttribute("data-turn", "4");
    expect(screen.getByTestId("card-u-mine")).toHaveAttribute("data-position", "DEF");
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });
});

describe("the mulligan's resolution", () => {
  it("plays the answers over the opening hand, then turn 1's banner over the hand that was kept", () => {
    const kept = card({ instanceId: "c1" });
    const returned = card({ instanceId: "c2" });
    const replacement = card({ instanceId: "c7" });
    const opening = withEvents(
      baseView({ phase: "mulligan", turn: 0, you: emptySide("p1", { hand: [kept, returned] }) }),
      [{ type: "promptOpened", player: "p1", choiceId: "m1", kind: "mulligan" }],
    );
    const firstTurn = withEvents(
      baseView({ phase: "main", turn: 1, you: emptySide("p1", { hand: [kept, replacement] }) }),
      [
        ...opening.events,
        { type: "promptAnswered", player: "p1", choiceId: "m1" },
        { type: "drawn", player: "p1", instanceId: "c7", defId: "core-004" },
        { type: "shuffledIn", player: "p1", instanceId: "c2", defId: "core-004", position: 3 },
        { type: "turnStarted", player: "p1", turn: 1 },
        { type: "manaChanged", player: "p1", current: 1, max: 1 },
      ],
    );
    const { rerender } = render(<Game view={opening} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={firstTurn} legal={[END_TURN]} onAction={vi.fn()} />);

    expect(playing()).toBe("promptAnswered");
    expect(screen.getByTestId("turn-banner")).toHaveTextContent("Mulligan");
    expect(screen.getByTestId("hand-card-c2")).toBeInTheDocument();
    advance(ANIMATIONS.promptAnswered.durationMs);
    expect(playing()).toBe("drawn");
    advance(ANIMATIONS.drawn.durationMs);
    expect(playing()).toBe("shuffledIn");
    advance(ANIMATIONS.shuffledIn.durationMs);

    expect(playing()).toBe("turnStarted");
    expect(screen.getByTestId("turn-banner")).toHaveTextContent("Your turn");
    expect(screen.getByTestId("turn-banner")).toHaveAttribute("data-animating", "turnStarted");
    expect(screen.queryByTestId("hand-card-c2")).toBeNull();
    expect(screen.getByTestId("hand-card-c7")).toBeInTheDocument();

    advance(ANIMATIONS.turnStarted.durationMs);
    expect(playing()).toBe("manaChanged");
    advance(ANIMATIONS.manaChanged.durationMs);
    expect(playing()).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });
});
