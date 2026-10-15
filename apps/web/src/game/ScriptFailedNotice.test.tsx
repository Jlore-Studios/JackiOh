// R915004: a custom card's script that failed did nothing, and the board says so beside the board,
// naming the card from the view's own definitions, until it is dismissed or another replaces it.

import type { CardDef, GameEvent } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import Game from "./Game.tsx";
import { newestScriptFailure, scriptFailedTestid } from "./ScriptFailedNotice.tsx";
import { baseView, withEvents } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
});

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];

/** The custom card's definition, as a game with it hands the page (`PlayerView.defs`). */
const BURNER: CardDef = {
  id: "custom-burner",
  index: "custom-burner",
  name: "Burner",
  set: "Custom",
  type: "Spell",
  tags: [],
  rarity: "Common",
  token: false,
  cost: 0,
  base: { keywords: [], text: "Deal 3 damage to the enemy hero." },
  radiant: { keywords: [], text: "Deal 6 damage to the enemy hero." },
};

function failed(message: string): GameEvent {
  return { type: "scriptFailed", player: "p1", instanceId: null, defId: BURNER.id, message };
}

function viewWith(events: GameEvent[]) {
  return { ...withEvents(baseView(), events), defs: { [BURNER.id]: BURNER } };
}

describe("R915004 the board shows a custom card's script failure", () => {
  it("R915004 a failure a view brings shows as a notice naming the card, and Close puts it away", () => {
    const { rerender } = render(<Game view={viewWith(START)} legal={[]} onAction={vi.fn()} />);
    expect(screen.queryByTestId(scriptFailedTestid.notice)).toBeNull();

    rerender(<Game view={viewWith([...START, failed("cry failed on line 3: boom")])} legal={[]} onAction={vi.fn()} />);
    const notice = screen.getByTestId(scriptFailedTestid.notice);
    expect(notice).toHaveTextContent("Burner's script failed: cry failed on line 3: boom");
    expect(notice).toHaveAttribute("role", "status");

    fireEvent.click(screen.getByTestId(scriptFailedTestid.dismiss));
    expect(screen.queryByTestId(scriptFailedTestid.notice)).toBeNull();

    // The next failure brings it back, with its own reason.
    rerender(
      <Game
        view={viewWith([...START, failed("cry failed on line 3: boom"), failed("death ran out of its budget")])}
        legal={[]}
        onAction={vi.fn()}
      />,
    );
    expect(screen.getByTestId(scriptFailedTestid.notice)).toHaveTextContent("Burner's script failed: death ran out of its budget");
  });

  it("R915004 a failure already in the first view (a resumed game) shows nothing, and a replay never does", () => {
    render(<Game view={viewWith([...START, failed("boom")])} legal={[]} onAction={vi.fn()} />);
    expect(screen.queryByTestId(scriptFailedTestid.notice)).toBeNull();
    cleanup();

    const { rerender } = render(<Game view={viewWith(START)} legal={[]} onAction={vi.fn()} replay />);
    act(() => {
      rerender(<Game view={viewWith([...START, failed("boom")])} legal={[]} onAction={vi.fn()} replay />);
    });
    expect(screen.queryByTestId(scriptFailedTestid.notice)).toBeNull();
  });

  it("R915004 only the events a view adds count, and only the same seat's", () => {
    const before = viewWith(START);
    const after = viewWith([...START, failed("boom")]);
    expect(newestScriptFailure(null, after)).toBeNull();
    expect(newestScriptFailure(before, after)).toMatchObject({ type: "scriptFailed", message: "boom" });
    expect(newestScriptFailure(after, after)).toBeNull();
    expect(newestScriptFailure({ ...before, viewer: "p2" }, after)).toBeNull();
  });
});
