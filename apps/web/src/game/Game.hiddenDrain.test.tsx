// R3497000: a hidden tab shows the live board at once instead of replaying the backlog while the
// turn clock runs. The visibility source is injected; no test touches `document.visibilityState`.
import type { ActionBody, GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FxVisibility } from "../fx/types.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";
import Game from "./Game.tsx";

const END_TURN: ActionBody = { type: "endTurn" };

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
/** A batch long enough to hold the board back: several animated entries behind the newest view. */
const BATCH: GameEvent[] = [
  ...START,
  { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false },
  { type: "healed", targetId: "hero-p1", amount: 2 },
  { type: "drawn", player: "p1", instanceId: "c1", defId: "core-004" },
  { type: "buffed", instanceId: "u1", attack: 1, health: 1 },
  { type: "damage", sourceId: null, targetId: "hero-p1", amount: 5, combat: false },
];

function rig(hidden: boolean): { source: FxVisibility; notify: () => void } {
  let value = hidden;
  const listeners = new Set<() => void>();
  return {
    source: {
      hidden: () => value,
      subscribe: (listener) => {
        listeners.add(listener);
        return () => {
          listeners.delete(listener);
        };
      },
    },
    notify: () => {
      value = false;
      for (const listener of [...listeners]) listener();
    },
  };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

describe("R3497000 a hidden online board drains its animation queue", () => {
  it("R3497000 a hidden board shows a new view's long batch at once with End turn enabled", () => {
    const { source } = rig(true);
    const before = withEvents(baseView({ active: "p2" }), START);
    const onAction = vi.fn();
    const { rerender } = render(<Game view={before} legal={[]} onAction={onAction} drainWhenHidden visibility={source} />);
    expect(screen.queryByTestId("animation-queue")).toBeNull();

    // The opponent's burst arrives with the viewer's turn while the tab is hidden.
    const after = withEvents(baseView({ active: "p1" }), BATCH);
    rerender(<Game view={after} legal={[END_TURN]} onAction={onAction} drainWhenHidden visibility={source} />);

    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });

  it("R3497000 a tab that turns visible with steps queued shows the newest view at once", () => {
    const { source, notify } = rig(false);
    const before = withEvents(baseView({ active: "p2" }), START);
    const onAction = vi.fn();
    const { rerender } = render(<Game view={before} legal={[]} onAction={onAction} drainWhenHidden visibility={source} />);
    const after = withEvents(baseView({ active: "p1" }), BATCH);
    rerender(<Game view={after} legal={[END_TURN]} onAction={onAction} drainWhenHidden visibility={source} />);

    // The visible tab replays the burst first, holding the moves back.
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.getByTestId("end-turn")).toBeDisabled();

    // The tab turns visible: the queued steps drain at once, with no timer advancing.
    act(() => {
      notify();
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });

  it("R3497000 a board without drainWhenHidden, a replay, or a visible tab behaves as before", () => {
    const { source } = rig(true);
    const before = withEvents(baseView({ active: "p2" }), START);
    const after = withEvents(baseView({ active: "p1" }), BATCH);

    const first = render(<Game view={before} legal={[]} onAction={vi.fn()} visibility={source} />);
    first.rerender(<Game view={after} legal={[END_TURN]} onAction={vi.fn()} visibility={source} />);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.getByTestId("end-turn")).toBeDisabled();
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
    first.unmount();

    // A replay board with a hidden tab still replays: the viewer sets the pace.
    const replayed = rig(true);
    const second = render(<Game view={before} legal={[]} onAction={vi.fn()} replay visibility={replayed.source} />);
    second.rerender(<Game view={after} legal={[END_TURN]} onAction={vi.fn()} replay visibility={replayed.source} />);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.getByTestId("end-turn")).toBeDisabled();
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    second.unmount();
  });
});
