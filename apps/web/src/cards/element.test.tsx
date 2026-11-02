// R980: a card's element (Meditative #40 Feng Shui) is printed on its face, a hand card's and a
// unit's, while a Feng Shui acts; with no element there is no chip.

import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { testid } from "../game/contract.ts";
import { renderBoard } from "../test/board.tsx";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { must } from "../test/stubServer.ts";

afterEach(() => {
  cleanup();
});

describe("R980 a card's element is printed on its face", () => {
  it("R980 a hand card's element is printed on its face", () => {
    renderBoard(
      baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-003", element: "木" })] }) }),
    );
    const chip = must(
      screen.getByTestId(testid.handCard("h1")).querySelector<HTMLElement>('[data-testid="card-element"]'),
      "the element chip",
    );
    expect(chip.textContent).toBe("木");
    expect(chip.getAttribute("title")).toBe("Element 木");
  });

  it("R980 a unit's element is printed on its minion face", () => {
    renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [unit("p1", { instanceId: "u1", defId: "core-003", element: "木" }), null, null, null, null],
        }),
      }),
    );
    const chip = must(
      screen.getByTestId(testid.card("u1")).querySelector<HTMLElement>('[data-testid="card-element"]'),
      "the element chip",
    );
    expect(chip.textContent).toBe("木");
  });

  it("R980 no element, no chip", () => {
    renderBoard(
      baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-003" })] }) }),
    );
    expect(
      screen.getByTestId(testid.handCard("h1")).querySelector('[data-testid="card-element"]'),
    ).toBeNull();
  });
});
