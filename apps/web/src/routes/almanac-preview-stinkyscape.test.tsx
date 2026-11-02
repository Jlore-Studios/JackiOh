// The Almanac and the deck builder with the Stinkyscape set previewed (#1883, R1883000), as
// almanac-preview.test.tsx previews Meditative: `@jackioh/shared` is mocked so `SHIPPED_SETS` lists
// the set as its release (#1884) does, and every chip, pool and mark that follows what ships (R1380,
// R1381) shows it. The set held is filters.test.ts's R1857000 test; the mock keeps the list free of
// repeats, so it stays valid once the release lists the set itself.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CardDef, SetName, Tag } from "@jackioh/shared";

vi.mock("@jackioh/shared", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/shared")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Stinkyscape"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Stinkyscape",
  };
});
// The Almanac's shelf (`shelf.ts`) reads the wire module itself, without the client's aliases, so the
// static `/almanac` page can import it: the same preview goes there too.
vi.mock("../wire/catalog.ts", async (importOriginal) => {
  const real = await importOriginal<typeof import("../wire/catalog.ts")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Stinkyscape"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Stinkyscape",
  };
});

import { CATALOG } from "@jackioh/cards";

import { closeInspect } from "../cards/index.ts";
import FilterBar from "../game/deckbuilder/FilterBar.tsx";
import { ALMANAC_CHIP_TAGS, DECK_CHIP_TAGS, DEFAULT_FILTER, DEFAULT_SORT, FILTER_SETS, visiblePool } from "../game/deckbuilder/filters.ts";
import { CARD_POOL, DB_FILTERS, filterSetId, filterTagId, poolCardId } from "../game/deckbuilder/testids.ts";
import AlmanacRoute, { ALMANAC_CATALOG } from "./almanac.tsx";

/** The shelf renders every card of the shipped sets, so a render here takes longer than vitest's default. */
const SLOW = 30_000;

const STINKYSCAPE: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards).filter((def) => def.set === "Stinkyscape");

/** The ids the grid shows, in its order. */
function shownIds(): string[] {
  return [...screen.getByTestId(CARD_POOL).querySelectorAll<HTMLElement>(".db-card")].map((card) => card.dataset.card ?? "");
}

function chips(group: string): string[] {
  return within(within(screen.getByTestId(DB_FILTERS)).getByRole("group", { name: group }))
    .getAllByRole("button")
    .map((chip) => chip.textContent ?? "");
}

beforeEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
  // An account read that never answers: a screen that asked the server would sit on it.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  closeInspect();
  cleanup();
  vi.unstubAllGlobals();
});

describe("the Almanac with the Stinkyscape set previewed (R1883000)", () => {
  it("R1883000 with the set shipped the almanac shows its cards and its chip keeps exactly them", () => {
    expect(STINKYSCAPE.length).toBeGreaterThan(0);
    render(<AlmanacRoute />);
    const shown = shownIds();
    for (const def of STINKYSCAPE) expect(shown, def.id).toContain(def.id);
    expect(chips("Set")).toContain("Stinkyscape");
    fireEvent.click(screen.getByTestId(filterSetId("Stinkyscape")));
    expect([...shownIds()].sort()).toEqual(STINKYSCAPE.map((def) => def.id).sort());
  }, SLOW);

  it("R1883000 a Stinkyscape card on the shelf wears the set's planet", () => {
    render(<AlmanacRoute />);
    const mark = screen.getByTestId(poolCardId("stinkyscape-017")).querySelector(".cf-set");
    expect(mark).toHaveAttribute("data-set", "Stinkyscape");
    expect(mark).toHaveAttribute("data-set-mark", "stinkyscape");
    expect(mark).toHaveAttribute("aria-label", "Stinkyscape set");
  }, SLOW);

  it("R1883000 the deck builder's pool and filter bar offer the set, and no token of it", () => {
    const pool = visiblePool(ALMANAC_CATALOG, null, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT);
    expect(pool).toContain("stinkyscape-017");
    // L3: a token is in no deck, so the pool never offers it.
    expect(CATALOG["stinkyscape-137-1"]?.token).toBe(true);
    expect(pool).not.toContain("stinkyscape-137-1");

    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={pool.length} />);
    expect(chips("Set")).toEqual([...FILTER_SETS]);
    // A newer set's chip follows it (#2204), so it is asserted right after Meditative+'s.
    expect(chips("Set")).toContain("Stinkyscape");
    expect(chips("Set").indexOf("Stinkyscape")).toBe(chips("Set").indexOf("Meditative+") + 1);
  });

  it("R1883000 the set's four tags get their chips once it ships: three in the deck builder, War Doctrine only in the almanac", () => {
    const deckable: Tag[] = ["Drone Core", "Weapon", "Swarm"];
    for (const tag of deckable) {
      expect(DECK_CHIP_TAGS, tag).toContain(tag);
      expect(ALMANAC_CHIP_TAGS, tag).toContain(tag);
    }
    expect(DECK_CHIP_TAGS).not.toContain("War Doctrine");
    expect(ALMANAC_CHIP_TAGS).toContain("War Doctrine");
    render(<AlmanacRoute />);
    expect(chips("Tag")).toEqual([...ALMANAC_CHIP_TAGS]);
    for (const tag of STINKYSCAPE.flatMap((def) => def.tags)) expect(chips("Tag"), tag).toContain(tag);
    fireEvent.click(screen.getByTestId(filterTagId("Drone Core")));
    const drones = STINKYSCAPE.filter((def) => def.tags.includes("Drone Core")).map((def) => def.id);
    expect(drones.length).toBeGreaterThan(0);
    for (const id of drones) expect(shownIds(), id).toContain(id);
  }, SLOW);
});
