// R3220000: the practice result draws through Dialog.tsx, a layer of its own (amends R2127000), where its
// ✕, Escape, Back and a click outside the panel mean View the board.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { dialogTestid } from "../game/Dialog.tsx";
import { pressBack } from "../test/history.ts";
import { at } from "../test/stubServer.ts";
import { PracticeResult } from "./PracticeResult.tsx";
import { practiceTestid } from "./testids.ts";

afterEach(cleanup);

function renderResult() {
  const handlers = { onPlayAgain: vi.fn(), onChangeSetup: vi.fn(), onMenu: vi.fn(), onViewBoard: vi.fn() };
  render(<PracticeResult result={{ winner: "p2", reason: "concede" }} viewer="p1" difficulty="easy" {...handlers} />);
  return handlers;
}

describe("R3220000 the practice result is a dialog", () => {
  it("R3220000 the practice result draws through Dialog, and its ✕, Escape, a click outside and View the board view the board", () => {
    const { onPlayAgain, onChangeSetup, onViewBoard } = renderResult();
    const dialog = screen.getByTestId(practiceTestid.result);
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("data-outcome", "loss");
    expect(dialog).toHaveClass("dialog", "practice-result");
    expect(dialog).toHaveAccessibleName("Defeat");
    expect(dialog).toHaveAccessibleDescription("You conceded.");
    expect(screen.getByTestId(practiceTestid.playAgain)).toHaveFocus();
    expect(screen.getByTestId(practiceTestid.playAgain)).toHaveClass("button-primary");
    expect(screen.getByTestId(practiceTestid.changeSetup)).toHaveClass("button-secondary");

    fireEvent.click(screen.getByTestId(dialogTestid.close));
    fireEvent.keyDown(window, { key: "Escape" });
    const scrim = dialog.parentElement;
    if (scrim === null) throw new Error("no scrim");
    fireEvent.click(scrim);
    fireEvent.click(dialog);
    fireEvent.click(screen.getByTestId(practiceTestid.viewBoard));
    expect(onViewBoard).toHaveBeenCalledTimes(4);
    expect(onPlayAgain).not.toHaveBeenCalled();
    expect(onChangeSetup).not.toHaveBeenCalled();
  });

  it("R3220000 Back views the board and keeps the address (amends R2127000)", async () => {
    at("/decks");
    window.history.pushState(null, "", "/practice");
    const { onPlayAgain, onViewBoard } = renderResult();
    await pressBack();
    expect(onViewBoard).toHaveBeenCalledTimes(1);
    expect(onPlayAgain).not.toHaveBeenCalled();
    expect(window.location.pathname).toBe("/practice");
  });
});

describe("R4742000 the practice result offers Main menu", () => {
  it("R4742000 the practice result offers Main menu last, after Play again and Change setup", () => {
    const { onPlayAgain, onChangeSetup, onMenu, onViewBoard } = renderResult();
    const row = screen.getByTestId(practiceTestid.result).querySelector(".dialog__actions");
    if (row === null) throw new Error("no answers");
    expect(Array.from(row.querySelectorAll("button"), (button) => button.dataset.testid)).toEqual([
      practiceTestid.playAgain,
      practiceTestid.changeSetup,
      practiceTestid.mainMenu,
    ]);
    const menu = screen.getByTestId(practiceTestid.mainMenu);
    expect(menu).toHaveClass("button-secondary");
    expect(menu).toHaveTextContent("Main menu");
    // Play again still takes the focus and the primary look.
    expect(screen.getByTestId(practiceTestid.playAgain)).toHaveFocus();
    expect(screen.getByTestId(practiceTestid.playAgain)).toHaveClass("button-primary");

    fireEvent.click(menu);
    expect(onMenu).toHaveBeenCalledTimes(1);
    expect(onPlayAgain).not.toHaveBeenCalled();
    expect(onChangeSetup).not.toHaveBeenCalled();
    expect(onViewBoard).not.toHaveBeenCalled();
  });
});
