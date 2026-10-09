// The Almanac and the deck builder with the Meditative set previewed (issue #550): `@jackioh/shared`
// is mocked so that `SHIPPED_SETS` lists the set the day before its patch does, and every chip, pool
// and mark that follows what ships (R1380, R1381) shows it. The same pieces with the set not shipped
// are almanac.test.tsx's, filters.test.ts's and browse.test.tsx's, so a test here holds nothing the
// release has to rewrite: the mock keeps the list free of repeats, so it stays valid once the
// release lists the set itself.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CardDef, SetName } from "@jackioh/shared";

vi.mock("@jackioh/shared", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/shared")>();
  const shipped: readonly SetName[] = [...new Set<SetName>([...real.SHIPPED_SETS, "Meditative"])];
  return {
    ...real,
    SHIPPED_SETS: shipped,
    setShips: (set: SetName): boolean => shipped.includes(set),
    newestShippedSet: (): SetName => "Meditative",
  };
});

import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import { CATALOG } from "@jackioh/cards";

import { closeInspect } from "../cards/index.ts";
import FilterBar from "../game/deckbuilder/FilterBar.tsx";
import {
  ALMANAC_CHIP_TAGS,
  DEFAULT_FILTER,
  DEFAULT_SORT,
  FILTER_SETS,
  FILTER_TAGS,
  shippedTags,
  visiblePool,
} from "../game/deckbuilder/filters.ts";
import { CARD_POOL, DB_FILTERS, filterSetId, poolCardId } from "../game/deckbuilder/testids.ts";
import AlmanacRoute, { ALMANAC_CATALOG } from "./almanac.tsx";

const MEDITATIVE: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards).filter((def) => def.set === "Meditative" && def.id !== GLITCH_DEF_ID);

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

describe("the Almanac with the Meditative set previewed", () => {
  it("R1380 with the set shipped the almanac shows its cards and its chip keeps exactly them", () => {
    expect(MEDITATIVE.length).toBeGreaterThan(0);
    render(<AlmanacRoute />);
    const shown = shownIds();
    for (const def of MEDITATIVE) expect(shown, def.id).toContain(def.id);
    expect(chips("Set")).toContain("Meditative");
    fireEvent.click(screen.getByTestId(filterSetId("Meditative")));
    expect([...shownIds()].sort()).toEqual(MEDITATIVE.map((def) => def.id).sort());
  });

  it("R503 a Meditative card on the shelf wears the ensō", () => {
    render(<AlmanacRoute />);
    const mark = screen.getByTestId(poolCardId("meditative-027")).querySelector(".cf-set");
    expect(mark).toHaveAttribute("data-set", "Meditative");
    expect(mark).toHaveAttribute("data-set-mark", "meditative");
    expect(mark).toHaveAttribute("aria-label", "Meditative set");
  });

  it("R1380 the deck builder's pool and filter bar offer the set", () => {
    const pool = visiblePool(ALMANAC_CATALOG, null, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT);
    expect(pool).toContain("meditative-027");
    // L3: a token is in no deck, so the pool never offers it.
    expect(CATALOG["meditative-028-1"]?.token).toBe(true);
    expect(pool).not.toContain("meditative-028-1");

    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={pool.length} />);
    expect(chips("Set")).toEqual([...FILTER_SETS]);
    expect(chips("Set")).toContain("Meditative");
  });

  it("R1381 a Meditative card's tag gets its chip once the set ships", () => {
    const fixture: CardDef = { ...(CATALOG["meditative-027"] as CardDef), id: "meditative-fixture", tags: ["Wincon"] };
    expect([...shippedTags(FILTER_TAGS, [fixture])]).toEqual(["Wincon"]);
    // The bar's chips are the tags a shipped card of the bundled catalog carries, Meditative's included.
    render(<FilterBar filter={DEFAULT_FILTER} onFilter={() => {}} sort={DEFAULT_SORT} onSort={() => {}} count={0} tags={ALMANAC_CHIP_TAGS} />);
    expect(chips("Tag")).toEqual([...ALMANAC_CHIP_TAGS]);
    for (const tag of MEDITATIVE.flatMap((def) => def.tags)) expect(chips("Tag"), tag).toContain(tag);
  });
});
