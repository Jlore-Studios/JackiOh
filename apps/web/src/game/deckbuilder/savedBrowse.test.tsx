// The deck builder's filter and sort, kept per device (#263, savedBrowse.ts): what is saved, what a
// damaged or stale copy reads back as, and the workshop restoring it on the next visit, with
// "Clear filters" clearing the saved copy too.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import DeckWorkshop from "./DeckWorkshop.tsx";
import { DEFAULT_FILTER, DEFAULT_SORT, type PoolFilter } from "./filters.ts";
import { fixtureCatalog, fixtureCollection } from "./fixtures.ts";
import { SAVED_BROWSE_KEY, isDefaultFilter, loadBrowse, saveBrowse } from "./savedBrowse.ts";
import type { StorageLike } from "./sync.ts";
import { DB_FILTER_CLEAR, DB_SEARCH, DB_SORT, DB_SORT_DIR, filterCostId } from "./testids.ts";
import { decksResponse, fakeDeckServer, memoryStorage, savedDeck } from "./testkit.ts";

const FILTERED: PoolFilter = {
  ...DEFAULT_FILTER,
  sets: new Set(["Core"]),
  costs: new Set(["2", "6+"]),
  types: new Set(["Unit"]),
  tags: new Set(["Human"]),
  rarities: new Set(["Epic"]),
  search: "taunt",
  ownedOnly: false,
};

afterEach(() => {
  cleanup();
});

describe("savedBrowse", () => {
  it("reads back exactly what it saved", () => {
    const storage = memoryStorage();
    saveBrowse(storage, { filter: FILTERED, sort: { key: "name", dir: "desc" } });
    expect(loadBrowse(storage)).toEqual({ filter: FILTERED, sort: { key: "name", dir: "desc" } });
  });

  it("removes the key when filter and sort are both back at their defaults", () => {
    const storage = memoryStorage();
    saveBrowse(storage, { filter: FILTERED, sort: DEFAULT_SORT });
    expect(storage.data.has(SAVED_BROWSE_KEY)).toBe(true);
    saveBrowse(storage, { filter: DEFAULT_FILTER, sort: DEFAULT_SORT });
    expect(storage.data.has(SAVED_BROWSE_KEY)).toBe(false);
  });

  it("keeps a non-default sort when the filter is cleared, with no filter in the saved copy", () => {
    const storage = memoryStorage();
    saveBrowse(storage, { filter: FILTERED, sort: { key: "attack", dir: "asc" } });
    saveBrowse(storage, { filter: DEFAULT_FILTER, sort: { key: "attack", dir: "asc" } });
    const back = loadBrowse(storage);
    expect(isDefaultFilter(back.filter)).toBe(true);
    expect(back.sort).toEqual({ key: "attack", dir: "asc" });
  });

  it("reads nothing, garbage or a stale copy as the defaults, keeping only what still exists", () => {
    const storage = memoryStorage();
    expect(loadBrowse(storage)).toEqual({ filter: DEFAULT_FILTER, sort: DEFAULT_SORT });
    expect(loadBrowse(null)).toEqual({ filter: DEFAULT_FILTER, sort: DEFAULT_SORT });

    storage.setItem(SAVED_BROWSE_KEY, "{not json");
    expect(loadBrowse(storage)).toEqual({ filter: DEFAULT_FILTER, sort: DEFAULT_SORT });
    storage.setItem(SAVED_BROWSE_KEY, "[1,2]");
    expect(loadBrowse(storage)).toEqual({ filter: DEFAULT_FILTER, sort: DEFAULT_SORT });

    storage.setItem(
      SAVED_BROWSE_KEY,
      JSON.stringify({
        costs: ["3", "nine"],
        tags: ["Human", "Retired tag"],
        types: "Unit",
        search: 7,
        ownedOnly: "yes",
        sort: { key: "colour", dir: "asc" },
      }),
    );
    const back = loadBrowse(storage);
    expect([...back.filter.costs]).toEqual(["3"]);
    expect([...back.filter.tags]).toEqual(["Human"]);
    expect(back.filter.types.size).toBe(0);
    expect(back.filter.search).toBe("");
    expect(back.filter.ownedOnly).toBe(true);
    expect(back.sort).toEqual(DEFAULT_SORT);
  });

  it("swallows a storage that throws on every access", () => {
    const broken: StorageLike = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
      removeItem: () => {
        throw new Error("denied");
      },
    };
    expect(() => {
      saveBrowse(broken, { filter: FILTERED, sort: DEFAULT_SORT });
    }).not.toThrow();
    expect(loadBrowse(broken)).toEqual({ filter: DEFAULT_FILTER, sort: DEFAULT_SORT });
  });
});

describe("the workshop remembers its filters and sort", () => {
  const catalog = fixtureCatalog();
  const collection = fixtureCollection();

  function mount(storage: ReturnType<typeof memoryStorage>): () => void {
    const data = decksResponse([savedDeck("deck-a", "Aggro", [], 1)], [], catalog.version);
    const server = fakeDeckServer();
    server.seed(data);
    const { unmount } = render(
      <DeckWorkshop
        catalog={catalog}
        collection={collection}
        data={data}
        profileId="browse-memory"
        api={server.api}
        storage={storage}
        initialOpen={{ kind: "deck", id: "deck-a" }}
      />,
    );
    return unmount;
  }

  it("restores them on the next visit, and Clear filters clears the saved filter", () => {
    const storage = memoryStorage();
    const unmount = mount(storage);
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: "taunt" } });
    fireEvent.click(screen.getByTestId(filterCostId("2")));
    fireEvent.change(screen.getByTestId(DB_SORT), { target: { value: "name" } });
    unmount();

    mount(storage);
    expect(screen.getByTestId(DB_SEARCH)).toHaveValue("taunt");
    expect(screen.getByTestId(filterCostId("2"))).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(DB_SORT)).toHaveValue("name");

    fireEvent.click(screen.getByTestId(DB_FILTER_CLEAR));
    expect(screen.getByTestId(DB_SEARCH)).toHaveValue("");
    expect(isDefaultFilter(loadBrowse(storage).filter)).toBe(true);
    expect(loadBrowse(storage).sort.key).toBe("name");

    // With the sort back at its default too, nothing is left under the key.
    fireEvent.change(screen.getByTestId(DB_SORT), { target: { value: DEFAULT_SORT.key } });
    if (screen.getByTestId(DB_SORT_DIR).getAttribute("data-dir") !== DEFAULT_SORT.dir) {
      fireEvent.click(screen.getByTestId(DB_SORT_DIR));
    }
    expect(storage.data.has(SAVED_BROWSE_KEY)).toBe(false);
  });
});
