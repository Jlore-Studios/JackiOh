// The Almanac and the deck builder with the Stinkyscape+ set previewed (#3572, R3572000), as
// almanac-preview-stinkyscape.test.tsx previews Stinkyscape: `@jackioh/shared` is mocked so
// `SHIPPED_SETS` lists the set as its release will, and every chip, pool and mark that follows what
// ships (R1380, R1381) shows it. The mock keeps the list free of repeats, so it stays valid once the
// release lists the set itself.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CardDef, SetName, Tag } from "@jackioh/shared";

vi.mock("@jackioh/shared", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/shared")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Stinkyscape+"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Stinkyscape+",
  };
});
// The Almanac's shelf (`shelf.ts`) reads the wire module itself, without the client's aliases, so the
// static `/almanac` page can import it: the same preview goes there too.
vi.mock("../wire/catalog.ts", async (importOriginal) => {
  const real = await importOriginal<typeof import("../wire/catalog.ts")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Stinkyscape+"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Stinkyscape+",
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

const STINKYSCAPE_PLUS: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards).filter((def) => def.set === "Stinkyscape+");

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

describe("the Almanac with the Stinkyscape+ set previewed (R3572000)", () => {
  it("R3572000 with the set shipped the almanac shows its cards and its chip keeps exactly them", () => {
    expect(STINKYSCAPE_PLUS.length).toBeGreaterThan(0);
    render(<AlmanacRoute />);
    const shown = shownIds();
    for (const def of STINKYSCAPE_PLUS) expect(shown, def.id).toContain(def.id);
    expect(chips("Set")).toContain("Stinkyscape+");
    fireEvent.click(screen.getByTestId(filterSetId("Stinkyscape+")));
    expect([...shownIds()].sort()).toEqual(STINKYSCAPE_PLUS.map((def) => def.id).sort());
  }, SLOW);

  it("R3572000 a Stinkyscape+ card on the shelf wears the set's planet with the plus and draws in the set's own art", () => {
    render(<AlmanacRoute />);
    const card = screen.getByTestId(poolCardId("stinkyscapeplus-001"));
    const mark = card.querySelector(".cf-set");
    expect(mark).toHaveAttribute("data-set", "Stinkyscape+");
    expect(mark).toHaveAttribute("data-set-mark", "stinkyscape-plus");
    expect(mark).toHaveAttribute("aria-label", "Stinkyscape+ set");
    const art = card.querySelector<HTMLElement>(".cf-art");
    expect(art?.dataset.art).toBe("procedural");
    if (art?.dataset.artPending !== "true") expect(art?.dataset.artFamily).toBe("stinkyscapeplus");
  }, SLOW);

  it("R3572000 the deck builder's pool and filter bar offer the set, and no token of it", () => {
    const pool = visiblePool(ALMANAC_CATALOG, null, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT);
    expect(pool).toContain("stinkyscapeplus-001");
    // L3: a token is in no deck, so the pool never offers it.
    expect(CATALOG["stinkyscapeplus-133-1"]?.token).toBe(true);
    expect(pool).not.toContain("stinkyscapeplus-133-1");

    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={pool.length} />);
    expect(chips("Set")).toEqual([...FILTER_SETS]);
    // The set's chip follows Training Grounds' since it shipped (#3573).
    expect(chips("Set")).toContain("Stinkyscape+");
    expect(chips("Set").indexOf("Stinkyscape+")).toBe(chips("Set").indexOf("Training Grounds") + 1);
  });

  it("R3572000 the set's eight deckable tags get chips in the deck builder, and Automa-ζ, on tokens alone, in the almanac only", () => {
    const deckable: Tag[] = ["Mindustrial", "Mineral", "Automa-α", "Automa-β", "Automa-γ", "Automa-δ", "Automa-Ω", "Paperclip"];
    for (const tag of deckable) {
      expect(DECK_CHIP_TAGS, tag).toContain(tag);
      expect(ALMANAC_CHIP_TAGS, tag).toContain(tag);
    }
    expect(DECK_CHIP_TAGS).not.toContain("Automa-ζ");
    expect(ALMANAC_CHIP_TAGS).toContain("Automa-ζ");

    const pool = visiblePool(ALMANAC_CATALOG, null, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT);
    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={pool.length} />);
    for (const tag of deckable) expect(chips("Tag"), tag).toContain(tag);
    expect(chips("Tag")).not.toContain("Automa-ζ");
    cleanup();

    render(<AlmanacRoute />);
    expect(chips("Tag")).toEqual([...ALMANAC_CHIP_TAGS]);
    for (const tag of STINKYSCAPE_PLUS.flatMap((def) => def.tags)) expect(chips("Tag"), tag).toContain(tag);
    expect(chips("Tag")).toContain("Automa-ζ");
    fireEvent.click(screen.getByTestId(filterTagId("Automa-ζ")));
    const zetas = STINKYSCAPE_PLUS.filter((def) => def.tags.includes("Automa-ζ")).map((def) => def.id);
    expect(zetas).toHaveLength(5);
    expect([...shownIds()].sort()).toEqual([...zetas].sort());
  }, SLOW);
});
