// R2102000: the board's side of the practice AI's lethal hint (Game.tsx `lethalHint`). Told
// the human has lethal, the board marks itself (`data-lethal-hint`, which highlights.css turns into
// rings on the enemy hero and End turn) and says "You have lethal" in its notices, never how.

import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { testid } from "./contract.ts";
import Game from "./Game.tsx";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
/** A hit on the viewer's hero: an event the runner animates, so it holds the view back. */
const HIT: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false };

function hinted(): boolean {
  return screen.getByTestId("game").getAttribute("data-lethal-hint") === "true";
}

describe("R2102000 the board's lethal hint", () => {
  it("R2102000 a board told it has lethal marks the board and says so", () => {
    render(<Game view={withEvents(baseView(), START)} legal={[]} onAction={vi.fn()} lethalHint />);
    expect(hinted()).toBe(true);
    const line = screen.getByTestId(testid.lethalHint);
    expect(line).toHaveAttribute("role", "status");
    expect(line).toHaveTextContent("You have lethal");
  });

  it("R2102000 without the prop, as online, there is no hint", () => {
    render(<Game view={withEvents(baseView(), START)} legal={[]} onAction={vi.fn()} />);
    expect(hinted()).toBe(false);
    expect(screen.queryByTestId(testid.lethalHint)).toBeNull();
  });

  it("R2102000 a replay's board shows no hint even when told", () => {
    render(<Game view={withEvents(baseView(), START)} legal={[]} onAction={vi.fn()} replay lethalHint />);
    expect(hinted()).toBe(false);
    expect(screen.queryByTestId(testid.lethalHint)).toBeNull();
  });

  it("R2102000 a finished game's board shows no hint even when told", () => {
    const over = baseView({ result: { winner: "p1", reason: "hero-death" } });
    render(<Game view={over} legal={[]} onAction={vi.fn()} lethalHint />);
    expect(hinted()).toBe(false);
    expect(screen.queryByTestId(testid.lethalHint)).toBeNull();
  });

  it("R2102000 the hint waits for the board to show the newest view", () => {
    const before = withEvents(baseView({ active: "p2" }), START);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);

    // The opponent's last hit arrives with the viewer's turn and its lethal: the board still shows the hit.
    const after = withEvents(baseView({ active: "p1" }), [...START, HIT]);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} lethalHint />);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(hinted()).toBe(false);
    expect(screen.queryByTestId(testid.lethalHint)).toBeNull();

    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(hinted()).toBe(true);
    expect(screen.getByTestId(testid.lethalHint)).toHaveTextContent("You have lethal");
  });
});
