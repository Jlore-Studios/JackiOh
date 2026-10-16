// R1081000: a Flying Unit standing in the backrow renders as a hovering minion — the unit block
// the view carries — and offers its switch action like a unit-zone minion does.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView, UnitView } from "@jackioh/shared";

import { baseView, emptySide, faceUpBackrow, unit } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type ClickTarget } from "./contract.ts";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

function flyer(position: "ATK" | "DEF"): UnitView {
  return unit("p1", {
    instanceId: "bird-1",
    defId: "core-004",
    attack: 2,
    maxHealth: 3,
    health: 3,
    position,
    keywords: [{ kind: "Flying" }],
  });
}

/** The Flying Unit standing in your backrow lane 1, in `position`. */
function backrowFlyerView(position: "ATK" | "DEF"): PlayerView {
  const bird = flyer(position);
  return baseView({
    you: emptySide("p1", {
      backrow: [
        faceUpBackrow("p1", {
          instanceId: "bird-1",
          defId: "core-004",
          type: "Unit",
          cost: 1,
          unit: bird,
        }),
        null,
        null,
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", { hand: { count: 0 } }),
  });
}

function renderBoard(view: PlayerView, highlight?: { legal: Set<string>; selected: Set<string> }, onClick?: (target: ClickTarget) => void): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} highlight={highlight} onClick={onClick} />
    </CatalogContext.Provider>,
  );
}

describe("R1081000 a Flying Unit in the backrow", () => {
  it("R1081000 a Flying Unit in the backrow renders as a hovering minion", () => {
    renderBoard(backrowFlyerView("ATK"));
    const card = screen.getByTestId(testid.card("bird-1"));
    expect(card.querySelector('[data-flying="true"]')).not.toBeNull();

    cleanup();
    renderBoard(backrowFlyerView("DEF"));
    const landed = screen.getByTestId(testid.card("bird-1"));
    expect(landed.querySelector('[data-flying="true"]')).toBeNull();
  });

  it("R1081000 a backrow minion offers its switch action", () => {
    const view = backrowFlyerView("ATK");
    const onClick = vi.fn<(target: ClickTarget) => void>();
    const legal = new Set([testid.switchPosition("bird-1")]);
    renderBoard(view, { legal, selected: new Set() }, onClick);

    const button = screen.getByTestId(testid.switchPosition("bird-1"));
    expect(button.tagName).toBe("BUTTON");
    // The switch sits beside the card, in its backrow zone, as a unit-zone minion's does.
    expect(button.parentElement?.getAttribute("data-testid")).toBe(testid.zone("you", "backrow", 1));
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledWith({ on: "switch", instanceId: "bird-1" });
  });
});
