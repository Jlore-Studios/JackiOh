// The Almanac and the deck builder with the Training Grounds set previewed (#2202, R2202001):
// `@jackioh/shared` is mocked so `SHIPPED_SETS` lists the set as its release (#2204) does, and every
// pool and mark that follows what ships (R1380, R1381) shows it, in the set's own art (R2202000). The
// mock keeps the list free of repeats, so it stays valid once the release lists the set itself.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CardDef, SetName } from "@jackioh/shared";

vi.mock("@jackioh/shared", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/shared")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Training Grounds"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Training Grounds",
  };
});
// The shelf (`shelf.ts`) reads the wire module without the client's aliases, for the static `/almanac`
// page, so the same preview goes there too.
vi.mock("../wire/catalog.ts", async (importOriginal) => {
  const real = await importOriginal<typeof import("../wire/catalog.ts")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Training Grounds"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Training Grounds",
  };
});

import { CATALOG } from "@jackioh/cards";

import { closeInspect } from "../cards/index.ts";
import FilterBar from "../game/deckbuilder/FilterBar.tsx";
import { DEFAULT_FILTER, DEFAULT_SORT, FILTER_SETS, visiblePool } from "../game/deckbuilder/filters.ts";
import { CARD_POOL, DB_FILTERS, filterSetId, poolCardId } from "../game/deckbuilder/testids.ts";
import AlmanacRoute, { ALMANAC_CATALOG } from "./almanac.tsx";

/** The shelf renders every card of seven sets, so a render here takes longer than vitest's default. */
const SLOW = 30_000;

const TRAINING_GROUNDS: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards).filter((def) => def.set === "Training Grounds");

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

describe("the Almanac with the Training Grounds set previewed (R2202001)", () => {
  it("R2202001 with the set shipped the almanac shows its cards and its chip keeps exactly them", () => {
    expect(TRAINING_GROUNDS.length).toBeGreaterThan(0);
    render(<AlmanacRoute />);
    const shown = shownIds();
    for (const def of TRAINING_GROUNDS) expect(shown, def.id).toContain(def.id);
    expect(chips("Set")).toContain("Training Grounds");
    fireEvent.click(screen.getByTestId(filterSetId("Training Grounds")));
    expect([...shownIds()].sort()).toEqual(TRAINING_GROUNDS.map((def) => def.id).sort());
  }, SLOW);

  it("R2202001 a Training Grounds card on the shelf wears the set's target and draws in the set's own art", () => {
    render(<AlmanacRoute />);
    const card = screen.getByTestId(poolCardId("traininggrounds-036"));
    const mark = card.querySelector(".cf-set");
    expect(mark).toHaveAttribute("data-set", "Training Grounds");
    expect(mark).toHaveAttribute("data-set-mark", "training-grounds");
    expect(mark).toHaveAttribute("aria-label", "Training Grounds set");
    const art = card.querySelector<HTMLElement>(".cf-art");
    expect(art?.dataset.art).toBe("procedural");
    if (art?.dataset.artPending !== "true") expect(art?.dataset.artFamily).toBe("dummy");
  }, SLOW);

  it("R2202001 the deck builder's pool and filter bar offer the set, and not its token", () => {
    const pool = visiblePool(ALMANAC_CATALOG, null, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT);
    expect(pool).toContain("traininggrounds-036");
    // L3: a token is in no deck, so the pool never offers it.
    expect(CATALOG["traininggrounds-039-1"]?.token).toBe(true);
    expect(pool).not.toContain("traininggrounds-039-1");

    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={pool.length} />);
    expect(chips("Set")).toEqual([...FILTER_SETS]);
    expect(chips("Set").at(-1)).toBe("Training Grounds");
  });
});
