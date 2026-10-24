// R169 at the table: every live modifier, in full, one tap away (game/ModifierList.tsx).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { baseView, fullBoardView, opponentModifiers, yourModifiers } from "../test/fixtures.ts";
import { ModifierList, modifierListTestid } from "./ModifierList.tsx";

afterEach(() => {
  cleanup();
});

describe("R169 the modifier list", () => {
  it("shows nothing while no modifier is live on either side", () => {
    const { container } = render(<ModifierList view={baseView()} opponentLabel="AI" />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId(modifierListTestid.chip)).toBeNull();
  });

  it("counts both sides' modifiers and opens every label in full, grouped You and AI", () => {
    render(<ModifierList view={fullBoardView()} opponentLabel="AI" />);
    const chip = screen.getByTestId(modifierListTestid.chip);
    const count = yourModifiers.length + opponentModifiers.length;
    expect(chip).toHaveAttribute("data-count", String(count));
    expect(chip).toHaveAccessibleName(`${String(count)} active modifiers`);
    expect(chip).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId(modifierListTestid.panel)).toBeNull();

    fireEvent.click(chip);
    const panel = screen.getByTestId(modifierListTestid.panel);
    expect(chip).toHaveAttribute("aria-expanded", "true");
    for (const modifier of [...yourModifiers, ...opponentModifiers]) {
      expect(panel).toHaveTextContent(modifier.label);
    }
    expect(panel.querySelector('[data-side="you"]')?.textContent).toContain(yourModifiers[0]?.label ?? "");
    expect(panel.querySelector('[data-side="opponent"]')?.textContent).toContain(opponentModifiers[0]?.label ?? "");
    expect(Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent)).toEqual(["You", "AI"]);
  });

  it("R169 heads the opponent's list with the label it is given: Opponent against a person", () => {
    render(<ModifierList view={fullBoardView()} opponentLabel="Opponent" />);
    fireEvent.click(screen.getByTestId(modifierListTestid.chip));
    const panel = screen.getByTestId(modifierListTestid.panel);
    expect(Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent)).toEqual([
      "You",
      "Opponent",
    ]);
    expect(panel).not.toHaveTextContent("AI");
  });

  it("closes on Escape, on a tap outside, and on the chip again", () => {
    render(<ModifierList view={fullBoardView()} opponentLabel="AI" />);
    const chip = screen.getByTestId(modifierListTestid.chip);

    fireEvent.click(chip);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(modifierListTestid.panel)).toBeNull();

    fireEvent.click(chip);
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId(modifierListTestid.panel)).toBeNull();

    fireEvent.click(chip);
    fireEvent.pointerDown(screen.getByTestId(modifierListTestid.panel));
    expect(screen.getByTestId(modifierListTestid.panel), "a tap inside the list keeps it open").toBeInTheDocument();
    fireEvent.click(chip);
    expect(screen.queryByTestId(modifierListTestid.panel)).toBeNull();
  });
});
