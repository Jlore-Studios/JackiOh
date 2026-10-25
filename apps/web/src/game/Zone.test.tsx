// B3.1 rule 6, R383, R64: what a held zone's icon says it is held for.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView, Row } from "@jackioh/shared";

import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type Side } from "./contract.ts";
import Zone from "./Zone.tsx";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

const NONE = [false, false, false, false, false];
const HELD_LANE_2 = { units: NONE, backrow: [false, true, false, false, false] };

/** Frostspatula, an animated card standing in a unit zone whose home is backrow lane 2. */
function animated(owner: "p1" | "p2") {
  return unit(owner, {
    instanceId: `spatula-${owner}`,
    defId: "classicplus-012-8",
    attack: 10,
    maxHealth: 3,
    health: 3,
    keywords: [{ kind: "Rush" }],
    animated: { home: 2 },
  });
}

/** The title of the held-zone icon drawn in the zone `side` `row` `lane` of `view`. */
function heldTitle(view: PlayerView, side: Side, row: Row, lane: number): string | null {
  render(
    <CatalogContext.Provider value={lookup}>
      <Zone view={view} side={side} row={row} lane={lane} />
    </CatalogContext.Provider>,
  );
  return screen.getByTestId(testid.zone(side, row, lane)).querySelector(".reserved-icon")?.getAttribute("title") ?? null;
}

describe("B3.1 rule 6, R64: a held zone's icon says what it is held for", () => {
  it("R383 your held backrow zone names the animated card that goes back to it", () => {
    const view = baseView({ you: emptySide("p1", { units: [animated("p1"), null, null, null, null], reserved: HELD_LANE_2 }) });
    expect(heldTitle(view, "you", "backrow", 2)).toBe("Held for Frostspatula to return");
  });

  it("R383 the opponent's held backrow zone names their animated card the same way", () => {
    const view = baseView({
      opponent: emptySide("p2", { hand: { count: 4 }, units: [animated("p2"), null, null, null, null], reserved: HELD_LANE_2 }),
    });
    expect(heldTitle(view, "opponent", "backrow", 2)).toBe("Held for Frostspatula to return");
  });

  it("R64 a held zone no animated card goes back to keeps the Reborn wording", () => {
    // (a) A unit zone is never a home.
    const unitZone = baseView({
      you: emptySide("p1", {
        units: [animated("p1"), null, null, null, null],
        reserved: { units: [false, true, false, false, false], backrow: NONE },
      }),
    });
    expect(heldTitle(unitZone, "you", "units", 2)).toBe("Held for a Reborn unit");
    cleanup();

    // (b) The animated card's home is a different lane.
    const otherLane = baseView({
      you: emptySide("p1", {
        units: [animated("p1"), null, null, null, null],
        reserved: { units: NONE, backrow: [false, false, true, false, false] },
      }),
    });
    expect(heldTitle(otherLane, "you", "backrow", 3)).toBe("Held for a Reborn unit");
    cleanup();

    // (c) An animated card on the other seat does not name this seat's zone.
    const otherSeat = baseView({
      you: emptySide("p1", { reserved: HELD_LANE_2 }),
      opponent: emptySide("p2", { hand: { count: 4 }, units: [animated("p2"), null, null, null, null] }),
    });
    expect(heldTitle(otherSeat, "you", "backrow", 2)).toBe("Held for a Reborn unit");
  });
});
