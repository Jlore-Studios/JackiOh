// #258: the switch (⟳) out of the card. It is the card's sibling in its zone (Card.tsx), where
// board.css gives it a full 44 px target at the zone's corner; a click on it is the switch's alone,
// and a press on it is never a drag source or a drop spot (drag/targets.ts).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { UnitView } from "@jackioh/shared";

import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type ClickTarget } from "./contract.ts";
import { pickDropSpot, targetFromElement } from "./drag/targets.ts";
import { fullBoardView } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

function mine(lane: number): UnitView {
  const u = fullBoardView().you.units[lane - 1];
  if (u === null || u === undefined) throw new Error(`no unit in lane ${String(lane)}`);
  return u;
}

describe("#258 the switch sits outside its card", () => {
  it("every field unit's switch is a button in its zone, beside the card, never inside it", () => {
    const view = fullBoardView();
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} />
      </CatalogContext.Provider>,
    );
    const units = [...view.you.units, ...view.opponent.units].filter((u): u is UnitView => u !== null);
    expect(units.length).toBe(10);
    for (const u of units) {
      const button = screen.getByTestId(testid.switchPosition(u.instanceId));
      const card = screen.getByTestId(testid.card(u.instanceId));
      expect(card.contains(button), u.instanceId).toBe(false);
      expect(button.parentElement, u.instanceId).toBe(card.parentElement);
      expect(button.parentElement?.classList.contains("zone"), u.instanceId).toBe(true);
      expect(button.getAttribute("aria-label")).toBe("Switch position");
      expect(button.querySelector(".switch-glyph")?.textContent).toBe("⟳");
    }
    // A DEF card is turned sideways by its own inline style; its switch is not.
    const def = mine(3);
    expect(screen.getByTestId(testid.card(def.instanceId)).style.transform).toContain("rotate(90deg)");
    expect(screen.getByTestId(testid.switchPosition(def.instanceId)).getAttribute("style")).toBeNull();
  });

  it("a click on a legal switch reports the switch alone: not the card, not the zone", () => {
    const view = fullBoardView();
    const u = mine(1);
    const onClick = vi.fn<(target: ClickTarget) => void>();
    const legal = new Set([testid.switchPosition(u.instanceId), testid.card(u.instanceId), testid.zone("you", "units", 1)]);
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} highlight={{ legal, selected: new Set() }} onClick={onClick} />
      </CatalogContext.Provider>,
    );
    fireEvent.click(screen.getByTestId(testid.switchPosition(u.instanceId)));
    expect(onClick.mock.calls).toEqual([[{ on: "switch", instanceId: u.instanceId }]]);
  });

  it("a press on the switch is no drag source, and a drop on it falls through to what lies under it", () => {
    const view = fullBoardView();
    const u = mine(1);
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} />
      </CatalogContext.Provider>,
    );
    const button = screen.getByTestId(testid.switchPosition(u.instanceId));
    const glyph = button.querySelector(".switch-glyph");
    if (glyph === null) throw new Error("no glyph");
    expect(targetFromElement(button)).toBeNull();
    expect(targetFromElement(glyph)).toBeNull();
    const card = screen.getByTestId(testid.card(u.instanceId));
    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    expect(pickDropSpot([button, card, zone], new Set([testid.card(u.instanceId)]))).toEqual({
      at: "target",
      target: { on: "unit", instanceId: u.instanceId, side: "you", lane: 1 },
      testid: testid.card(u.instanceId),
    });
    expect(pickDropSpot([button, zone], new Set([testid.zone("you", "units", 1)]))).toMatchObject({ at: "target", testid: zone.getAttribute("data-testid") });
  });
});
