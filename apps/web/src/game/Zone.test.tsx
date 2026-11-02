// B3.1 rule 6, R383, R64: what a held zone's icon says it is held for. #2015: a zone the player
// cannot pick draws no tap highlight.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView, Row } from "@jackioh/shared";

import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type Highlight, type Side } from "./contract.ts";
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

function renderZone(highlight: Highlight): { zone: HTMLElement; onClick: ReturnType<typeof vi.fn> } {
  const onClick = vi.fn();
  render(
    <CatalogContext.Provider value={lookup}>
      <Zone view={baseView()} side="you" row="units" lane={1} highlight={highlight} onClick={onClick} />
    </CatalogContext.Provider>,
  );
  return { zone: screen.getByTestId(testid.zone("you", "units", 1)), onClick };
}

describe("#2015 a zone the player cannot pick draws no tap feedback", () => {
  it("#2015 an empty zone the player cannot pick is a disabled group that fires nothing", () => {
    const { zone, onClick } = renderZone({ legal: new Set(), selected: new Set() });
    expect(zone.getAttribute("data-legal")).toBe("false");
    expect(zone.getAttribute("aria-disabled")).toBe("true");
    expect(zone.getAttribute("role")).toBe("group");
    expect(zone.hasAttribute("tabindex")).toBe(false);

    fireEvent.click(zone);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("#2015 an empty zone the player may pick keeps its button role and its click", () => {
    const { zone, onClick } = renderZone({ legal: new Set([testid.zone("you", "units", 1)]), selected: new Set() });
    expect(zone.getAttribute("data-legal")).toBe("true");
    expect(zone.getAttribute("role")).toBe("button");
    expect(zone.getAttribute("tabindex")).toBe("0");
    expect(zone.hasAttribute("aria-disabled")).toBe(false);

    fireEvent.click(zone);
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onClick).toHaveBeenCalledWith({ on: "zone", side: "you", row: "units", lane: 1 });
  });

  it("#2015 board.css drops the tap highlight on a zone that is not legal, and only there", () => {
    // The zone keeps its onClick (#757), so React gives its div an onclick and a phone greys it on
    // a tap; the rule keys on the `data-legal="false"` the zone always writes.
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "board.css"), "utf8");
    expect(css).toMatch(/\.zone:not\(\[data-legal="true"\]\)\s*\{[^}]*-webkit-tap-highlight-color:\s*transparent/);
    expect(css).toMatch(/\.zone:not\(\[data-legal="true"\]\) > \*\s*\{[^}]*-webkit-tap-highlight-color:\s*initial/);
    expect(css).not.toMatch(/\.zone\[data-legal="true"\][^{]*\{[^}]*tap-highlight/);
  });
});
