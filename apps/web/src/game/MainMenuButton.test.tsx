// #4742, R4742000: the one Main menu button every end-of-game result shares.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { MainMenuButton, mainMenuAction } from "./MainMenuButton.tsx";

afterEach(cleanup);

describe("R4742000 the Main menu button", () => {
  it("R4742000 MainMenuButton draws a button named Main menu with its testid, and a click calls onClick once", () => {
    const onClick = vi.fn();
    render(<MainMenuButton testid="some-menu" onClick={onClick} />);
    const button = screen.getByTestId("some-menu");
    expect(button.tagName).toBe("BUTTON");
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveTextContent("Main menu");
    expect(button).toHaveAccessibleName("Main menu");
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("R4742000 mainMenuAction is the same words as a secondary answer for a Dialog's row", () => {
    const onClick = vi.fn();
    const action = mainMenuAction(onClick, "row-menu");
    expect(action).toMatchObject({ label: "Main menu", kind: "secondary", testid: "row-menu" });
    action.onClick();
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
