// R980: a card's element (Meditative #40 Feng Shui) is printed on its face while a Feng Shui
// acts — a hand card (the tall face), a unit (the minion face) — and with no element there is no
// chip. The views here are fixtures shaped as `viewFor` builds them (src/test/fixtures.ts).

import { cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import type { PlayerView } from "@jackioh/shared";
import { CATALOG } from "@jackioh/cards";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

function withCatalog(node: ReactElement): ReactElement {
  return <CatalogContext.Provider value={lookup}>{node}</CatalogContext.Provider>;
}

function renderBoard(view: PlayerView): void {
  render(withCatalog(<Board view={view} />));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

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
