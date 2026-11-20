// Tests for the Created cards panels of /stats (SPEC §9.11, R2089000), the Tokens and the Other created cards
// (R3324002), their sorting (R3184002), their Tier and Adjusted (R4082000), their deck table's columns and
// rows that open the card (R4155000), and R4083000's columns, tier-fit sort, tab-held sort, Grid and ranges.

import { useState, type ComponentProps, type ReactElement } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PublicCreatedStat, PublicStatsTiers } from "../net/api.ts";
import {
  CREATED_DEFAULT_SORT,
  CreatedPanel,
  createdSortOf,
  sortCreated,
  type CreatedSort,
} from "./CreatedPanel.tsx";
import { statsTestid } from "./testids.ts";

const COIN: PublicCreatedStat = {
  id: "coin",
  name: "The Coin",
  cost: 0,
  rarity: "token",
  set: "core",
  games: 40,
  hasEnoughGames: true,
  window: { from: "v0.5.0b", liveGames: 40, source: "live", counted: ["v0.5.0b"] },
};

type PanelProps = ComponentProps<typeof CreatedPanel>;

/** The panel's props with the tab's Columns toggle on, the opening sort, the Table view and a no-op row opener. */
function props(over: Partial<PanelProps> = {}): PanelProps {
  return {
    kind: "other",
    created: [COIN],
    sort: CREATED_DEFAULT_SORT,
    onSort: () => {},
    view: "table",
    showRanges: false,
    tiers: undefined,
    nameOf: (_id: string, name: string) => name,
    faceOf: () => null,
    visibleThumbnail: true,
    visibleGames: true,
    onOpen: () => {},
    ...over,
  };
}

/** R4083000: the panel under a sort it holds itself, as the tab holds it, so a header press sorts it. */
function Sorted(p: PanelProps): ReactElement {
  const [sort, setSort] = useState<CreatedSort>(CREATED_DEFAULT_SORT);
  return <CreatedPanel {...p} sort={sort} onSort={setSort} />;
}

afterEach(cleanup);

describe("CreatedPanel", () => {
  it("scrolls its table in a region the keyboard can reach, as a phone needs (axe scrollable-region-focusable)", () => {
    render(<CreatedPanel {...props()} />);
    const region = screen.getByRole("region", { name: "Other created cards" });
    expect(region.tabIndex).toBe(0);
    expect(region.querySelector("table")).not.toBeNull();
  });

  it("R3324002 the Tokens panel is the same table under its own title and test ids", () => {
    render(<CreatedPanel {...props({ kind: "tokens" })} />);
    const panel = screen.getByTestId(statsTestid.tokensPanel);
    expect(within(panel).getByRole("heading", { name: "Tokens" })).toBeInTheDocument();
    const table = within(panel).getByTestId(statsTestid.tokensTable);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Card",
      "Name",
      "Games",
      "Tier",
      "Adjusted ↓",
    ]);
    expect(screen.getByRole("region", { name: "Tokens" })).toContainElement(table);
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();
  });

  it("R3887000 a plainCardOf result stands in the name cell, and null falls back to the name", () => {
    const COPY: PublicCreatedStat = { ...COIN, id: "core-001", name: "Spark Pup" };
    render(
      <CreatedPanel
        {...props({
          created: [COIN, COPY],
          plainCardOf: (id: string) => (id === "coin" ? null : <span data-testid="x" />),
        })}
      />,
    );
    const cell = (id: string): HTMLElement => {
      const row = document.querySelector(`tr[data-card-id="${id}"]`);
      if (row === null) throw new Error(`no row for ${id}`);
      const name = row.querySelector("td.stats-td--name");
      if (name === null) throw new Error(`no name cell for ${id}`);
      return name as HTMLElement;
    };
    expect(within(cell("core-001")).getByTestId("x")).toBeInTheDocument();
    expect(cell("core-001")).not.toHaveTextContent("Spark Pup");
    expect(within(cell("coin")).queryByTestId("x")).toBeNull();
    expect(cell("coin")).toHaveTextContent("The Coin");
  });

  it("R3887000 without plainCardOf every name cell holds the name as a link, as the deck table's", () => {
    render(<CreatedPanel {...props()} />);
    const name = document.querySelector("td.stats-td--name") as HTMLElement;
    expect(name).toHaveTextContent("The Coin");
    expect(name.querySelector(".stats-card-name-link")).toHaveTextContent("The Coin");
  });

  it("R4082000 a row shows the server's tier and Adjusted from the floor, with the deck table's cells", () => {
    const TOKEN: PublicCreatedStat = { ...COIN, id: "rush", adjustedWinRate: 0.5234, tier: "B" };
    const FEW: PublicCreatedStat = { ...TOKEN, id: "ember", games: 12, adjustedWinRate: 0.61, tier: null, hasEnoughGames: false };
    render(<CreatedPanel {...props({ kind: "tokens", created: [TOKEN, FEW] })} />);
    const row = (id: string): HTMLElement => document.querySelector(`tr[data-card-id="${id}"]`) as HTMLElement;
    expect(within(row("rush")).getByTestId(statsTestid.createdTierCell)).toHaveTextContent("B");
    const adjusted = within(row("rush")).getByTestId(statsTestid.createdAdjustedCell);
    expect(adjusted).toHaveTextContent("52.3%");
    // The Adjusted cell carries the deck table's bar, sized by the adjusted rate.
    const fill = adjusted.querySelector(".stats-confidence-fill") as HTMLElement;
    expect(fill.style.width).toBe("52%");
    // Below the floor, neither shows, as in the deck table, and R4083000 there is no Win rate cell to read.
    expect(within(row("ember")).getByTestId(statsTestid.createdTierCell)).toHaveTextContent("—");
    expect(within(row("ember")).getByTestId(statsTestid.createdAdjustedCell)).toHaveTextContent("—");
    expect(row("ember").querySelector(".stats-confidence-bar")).toBeNull();
    expect(row("ember").querySelector(".stats-td--winrate")).toBeNull();
    expect(row("ember")).not.toHaveTextContent("not enough games");
    // The thumbnail badge reads the cost, coloured by rarity.
    const badge = row("rush").querySelector(".stats-card-thumb-badge") as HTMLElement;
    expect(badge).toHaveTextContent("0");
    expect(badge).toHaveAttribute("data-rarity", "token");
    // No hint paragraph and no tiers line (R4155000).
    expect(document.querySelector(".stats-created-panel p")).toBeNull();
  });
});

describe("R4155000 the Created tables show the deck table's columns", () => {
  function renderTable(over: Partial<PanelProps> = {}): HTMLElement {
    render(<CreatedPanel {...props({ kind: "tokens", ...over })} />);
    return screen.getByTestId(statsTestid.tokensTable);
  }

  it("R4155000 R4083000 the headers are exactly Card, Name, Games, Tier and Adjusted", () => {
    const table = renderTable();
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Card",
      "Name",
      "Games",
      "Tier",
      "Adjusted ↓",
    ]);
    // The Card header is the deck table's thumbnail header: no sort.
    const thumb = within(table).getAllByRole("columnheader")[0]!;
    expect(thumb).toHaveClass("stats-th--thumb");
    expect(thumb).not.toHaveAttribute("aria-sort");
  });

  it("R4155000 the Columns toggle hides Card and Games in them, as in the deck table", () => {
    cleanup();
    const table = renderTable({ visibleThumbnail: false, visibleGames: false });
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Name",
      "Tier",
      "Adjusted ↓",
    ]);
    const row = table.querySelector("tbody tr") as HTMLElement;
    expect(row.querySelector(".stats-td--thumb")).toBeNull();
    expect([...row.querySelectorAll("td")].map((td) => td.className)).toEqual([
      "stats-td stats-td--name",
      "stats-td",
      "stats-td",
    ]);
  });

  it("R4155000 a Created row opens the card on click, Enter and Space", () => {
    const onOpen = vi.fn();
    const A: PublicCreatedStat = { ...COIN, id: "rush", name: "Rush Token" };
    const B: PublicCreatedStat = { ...COIN, id: "sprite", name: "Spark Sprite" };
    render(<CreatedPanel {...props({ created: [A, B], onOpen })} />);
    const row = (id: string): HTMLElement => document.querySelector(`tr[data-card-id="${id}"]`) as HTMLElement;
    for (const candidate of [row("rush"), row("sprite")]) {
      expect(candidate).toHaveAttribute("role", "button");
      expect(candidate.tabIndex).toBe(0);
    }
    fireEvent.click(row("rush"));
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenLastCalledWith("rush", row("rush"));
    fireEvent.keyDown(row("sprite"), { key: "Enter" });
    expect(onOpen).toHaveBeenLastCalledWith("sprite", row("sprite"));
    fireEvent.keyDown(row("sprite"), { key: " " });
    expect(onOpen).toHaveBeenCalledTimes(3);
    expect(onOpen).toHaveBeenLastCalledWith("sprite", row("sprite"));
  });
});

describe("R3184002 the Created cards table sorts by every column", () => {
  const made = (over: Partial<PublicCreatedStat>): PublicCreatedStat => ({ ...COIN, ...over });
  // The server's order, and each row's figures with the server's floor ("—" below it).
  const ROWS: PublicCreatedStat[] = [
    // Games 55; tier A, Adjusted 53%.
    made({ id: "rush", name: "Rush Token", games: 55, tier: "A", adjustedWinRate: 0.53 }),
    // Games 45; tier B, Adjusted 50.5%.
    made({ id: "sprite", name: "Spark Sprite", games: 45, tier: "B", adjustedWinRate: 0.505 }),
    // Games 45, as many as Spark Sprite; tier B as Spark Sprite, Adjusted 51.2%.
    made({ games: 45, tier: "B", adjustedWinRate: 0.512 }),
    // Below the floor: its tier and Adjusted "—".
    made({ id: "ember", name: "Ember", games: 12, tier: null, adjustedWinRate: 0.6, hasEnoughGames: false }),
  ];

  function renderTable(): { headers: () => HTMLElement[]; order: () => (string | null)[] } {
    render(<Sorted {...props({ created: ROWS })} />);
    const table = screen.getByTestId(statsTestid.createdTable);
    return {
      headers: () => within(table).getAllByRole("columnheader"),
      order: () => [...table.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-card-id")),
    };
  }

  it("R4082000 opens on Adjusted, best first, a row below the floor last, every other header unsorted", () => {
    const { headers, order } = renderTable();
    // Adjusted: Rush 53%, The Coin 51.2%, Spark Sprite 50.5%; Ember's 12 games are below the floor.
    expect(order()).toEqual(["rush", "coin", "sprite", "ember"]);
    expect(headers()[4]).toHaveAttribute("aria-sort", "descending");
    expect(headers()[4]).toHaveTextContent("Adjusted ↓");
    headers().forEach((header, at) => {
      if (at === 0) {
        // The Card thumbnail header sorts nothing.
        expect(header).not.toHaveAttribute("aria-sort");
      } else if (at !== 4) {
        expect(header).toHaveAttribute("aria-sort", "none");
        expect(header.textContent).not.toMatch(/[↑↓]/);
      }
    });
  });

  it("R3184002 a figure's first press sorts descending and the next ascending, a row with none last either way", () => {
    const { headers, order } = renderTable();
    // Columns: Card, Name, Games, Tier, Adjusted.
    // Adjusted, sorted as the table opens, goes last, so its first press here sorts it descending as any other's does.
    const cases: [number, string, (string | null)[], (string | null)[]][] = [
      // Games: Spark Sprite and The Coin tie, and keep the server's order both ways.
      [2, "Games", ["rush", "sprite", "coin", "ember"], ["ember", "sprite", "coin", "rush"]],
      // R4082000: Tier, S first, then by Adjusted within a tier: The Coin and Spark Sprite share B.
      [3, "Tier", ["rush", "coin", "sprite", "ember"], ["sprite", "coin", "rush", "ember"]],
      [4, "Adjusted", ["rush", "coin", "sprite", "ember"], ["sprite", "coin", "rush", "ember"]],
    ];
    for (const [at, label, descending, ascending] of cases) {
      fireEvent.click(headers()[at]!);
      expect(headers()[at]).toHaveAttribute("aria-sort", "descending");
      expect(headers()[at]).toHaveTextContent(`${label} ↓`);
      expect(order()).toEqual(descending);
      fireEvent.click(headers()[at]!);
      expect(headers()[at]).toHaveAttribute("aria-sort", "ascending");
      expect(headers()[at]).toHaveTextContent(`${label} ↑`);
      expect(order()).toEqual(ascending);
      // Only the pressed header is sorted; the Card thumbnail header sorts nothing and carries no aria-sort.
      expect(headers().filter((header) => !["none", null].includes(header.getAttribute("aria-sort")))).toEqual([
        headers()[at],
      ]);
    }
  });

  it("R3184002 the card's name sorts ascending first, and the keyboard sorts as a press does", () => {
    const { headers, order } = renderTable();
    fireEvent.keyDown(headers()[1]!, { key: "Enter" });
    expect(headers()[1]).toHaveAttribute("aria-sort", "ascending");
    expect(headers()[1]).toHaveTextContent("Name ↑");
    expect(order()).toEqual(["ember", "rush", "sprite", "coin"]);
    fireEvent.keyDown(headers()[1]!, { key: " " });
    expect(headers()[1]).toHaveAttribute("aria-sort", "descending");
    expect(order()).toEqual(["coin", "sprite", "rush", "ember"]);
  });
});

describe("R4083000 the Created tables show Tier and Adjusted alone, sorted, viewed and ranged as the deck cards are", () => {
  /** R3022000's band, 45% to 55%: B is 50% to 52.5%. */
  const TIERS: PublicStatsTiers = { center: 0, spread: 0.03, cards: 40, bandLow: 0.45, bandHigh: 0.55 };
  const made = (over: Partial<PublicCreatedStat>): PublicCreatedStat => ({ ...COIN, ...over });
  // A: its range 52%–56%. B, 48%–54%: 2.5 of its 6 points in B, 42%. B, 49.5%–51.5%: 1.5 of its 2 points, 75%. B with no range.
  const W = made({ id: "w", name: "W", tier: "A", adjustedWinRate: 0.53, adjustedLow: 0.52, adjustedHigh: 0.56 });
  const X = made({ id: "x", name: "X", tier: "B", adjustedWinRate: 0.51, adjustedLow: 0.48, adjustedHigh: 0.54 });
  const Y = made({ id: "y", name: "Y", tier: "B", adjustedWinRate: 0.505, adjustedLow: 0.495, adjustedHigh: 0.515 });
  const Z = made({ id: "z", name: "Z", tier: "B", adjustedWinRate: 0.52, adjustedLow: null, adjustedHigh: 0.55 });
  // Below the floor, its range and tier sent all the same.
  const FEW = made({
    id: "few",
    name: "Few",
    games: 12,
    tier: "B",
    adjustedWinRate: 0.51,
    adjustedLow: 0.4,
    adjustedHigh: 0.6,
    hasEnoughGames: false,
  });
  const nameOf = (_id: string, name: string): string => name;
  const ids = (rows: readonly PublicCreatedStat[]): string[] => rows.map((row) => row.id);
  const row = (id: string): HTMLElement => document.querySelector(`[data-card-id="${id}"]`) as HTMLElement;

  it("R4083000 no row carries a Win rate or a Used cell", () => {
    render(<CreatedPanel {...props({ created: [W, X, FEW] })} />);
    const table = screen.getByTestId(statsTestid.createdTable);
    for (const tr of table.querySelectorAll("tbody tr")) {
      // Card, Name, Games, Tier and Adjusted.
      expect(tr.querySelectorAll("td")).toHaveLength(5);
    }
    expect(table.querySelector(".stats-td--winrate")).toBeNull();
    expect(table.textContent).not.toMatch(/Win rate|Used/);
  });

  it("R4083000 by Tier a tier's cards order by tier fit, a card with none after, then by Adjusted", () => {
    const rows = [X, Z, W, Y];
    expect(ids(sortCreated(rows, { col: "tier", dir: "desc" }, TIERS, nameOf))).toEqual(["w", "y", "x", "z"]);
    expect(ids(sortCreated(rows, { col: "tier", dir: "asc" }, TIERS, nameOf))).toEqual(["x", "y", "z", "w"]);
    // With no band there is no fit, so a tier's cards order by Adjusted alone.
    expect(ids(sortCreated(rows, { col: "tier", dir: "desc" }, undefined, nameOf))).toEqual(["w", "z", "x", "y"]);
    // A card below the floor has no tier, and goes last either way.
    expect(ids(sortCreated([FEW, ...rows], { col: "tier", dir: "asc" }, TIERS, nameOf)).at(-1)).toBe("few");
    expect(ids(sortCreated([FEW, ...rows], { col: "tier", dir: "desc" }, TIERS, nameOf)).at(-1)).toBe("few");
  });

  it("R4083000 a header asks the tab for its sort, and the panel shows the sort it is given", () => {
    const onSort = vi.fn();
    render(<CreatedPanel {...props({ created: [W, X], sort: { col: "tier", dir: "asc" }, onSort })} />);
    const header = (label: string): HTMLElement =>
      within(screen.getByTestId(statsTestid.createdTable))
        .getAllByRole("columnheader")
        .find((th) => th.textContent?.startsWith(label)) as HTMLElement;
    expect(header("Tier")).toHaveTextContent("Tier ↑");
    expect(header("Tier")).toHaveAttribute("aria-sort", "ascending");
    expect(header("Adjusted")).toHaveAttribute("aria-sort", "none");
    // The panel holds no sort of its own: a press asks for the next one, and the shown one stays until the tab sends it.
    fireEvent.click(header("Tier"));
    expect(onSort).toHaveBeenLastCalledWith({ col: "tier", dir: "desc" });
    expect(header("Tier")).toHaveTextContent("Tier ↑");
    fireEvent.click(header("Name"));
    expect(onSort).toHaveBeenLastCalledWith({ col: "name", dir: "asc" });
    fireEvent.click(header("Games"));
    expect(onSort).toHaveBeenLastCalledWith({ col: "games", dir: "desc" });
    fireEvent.click(header("Adjusted"));
    expect(onSort).toHaveBeenLastCalledWith({ col: "adjustedWinRate", dir: "desc" });
    expect(onSort).toHaveBeenCalledTimes(4);
  });

  it("R4083000 createdSortOf keeps the Created columns and reads Win rate or Play rate as Adjusted in its direction", () => {
    for (const col of ["name", "games", "tier", "adjustedWinRate"] as const) {
      expect(createdSortOf(col, "asc")).toEqual({ col, dir: "asc" });
      expect(createdSortOf(col, "desc")).toEqual({ col, dir: "desc" });
    }
    expect(createdSortOf("winRate", "asc")).toEqual({ col: "adjustedWinRate", dir: "asc" });
    expect(createdSortOf("playRate", "desc")).toEqual({ col: "adjustedWinRate", dir: "desc" });
  });

  it("R4083000 Show 95% ranges puts the Adjusted range under it and the tier fit under the tier, from the floor", () => {
    const { rerender } = render(<CreatedPanel {...props({ created: [X, FEW], tiers: TIERS, showRanges: true })} />);
    expect(within(row("x")).getByTestId(statsTestid.adjustedRange)).toHaveTextContent("48.0–54.0%");
    expect(within(row("x")).getByTestId(statsTestid.createdAdjustedCell)).toContainElement(
      within(row("x")).getByTestId(statsTestid.adjustedRange),
    );
    expect(within(row("x")).getByTestId(statsTestid.tierFit)).toHaveTextContent("42%");
    // Below the floor neither shows, as its tier and Adjusted read "—".
    expect(within(row("few")).queryByTestId(statsTestid.adjustedRange)).toBeNull();
    expect(within(row("few")).queryByTestId(statsTestid.tierFit)).toBeNull();
    // With the toggle off, nothing shows.
    rerender(<CreatedPanel {...props({ created: [X, FEW], tiers: TIERS, showRanges: false })} />);
    expect(screen.queryByTestId(statsTestid.adjustedRange)).toBeNull();
    expect(screen.queryByTestId(statsTestid.tierFit)).toBeNull();
  });

  it("R4083000 in Grid a Created card shows its face, tier chip, Adjusted and games, and opens the card", () => {
    const onOpen = vi.fn();
    const grid = (over: Partial<PanelProps>): PanelProps =>
      props({
        kind: "tokens",
        view: "grid",
        created: [X, FEW, W],
        tiers: TIERS,
        faceOf: (id: string) => <span data-testid={`face-${id}`} />,
        onOpen,
        ...over,
      });
    const { rerender } = render(<CreatedPanel {...grid({})} />);
    const list = screen.getByTestId(statsTestid.tokensGrid);
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.queryByTestId(statsTestid.tokensTable)).toBeNull();
    // In the tab's sort: Adjusted, best first, a card below the floor last.
    expect([...list.querySelectorAll("li")].map((li) => li.getAttribute("data-card-id"))).toEqual(["w", "x", "few"]);
    const x = row("x");
    expect(within(x).getByTestId("face-x")).toBeInTheDocument();
    expect(within(x).getByTestId(statsTestid.createdTierCell)).toHaveTextContent("B");
    expect(within(x).getByTestId(statsTestid.createdAdjustedCell)).toHaveTextContent("51.0%");
    expect([...x.querySelectorAll("dt")].map((dt) => dt.textContent)).toEqual(["Tier", "Adjusted", "Games"]);
    expect(x.querySelector("dl")).toHaveTextContent("40");
    expect(x).not.toHaveTextContent("Win rate");
    expect(within(row("few")).getByTestId(statsTestid.createdTierCell)).toHaveTextContent("—");
    expect(within(row("few")).getByTestId(statsTestid.createdAdjustedCell)).toHaveTextContent("—");
    // Its card opens the drill-down, as a deck card's does.
    const button = within(x).getByRole("button", { name: "X" });
    fireEvent.click(button);
    expect(onOpen).toHaveBeenCalledWith("x", button);
    expect(screen.queryByTestId(statsTestid.adjustedRange)).toBeNull();
    expect(screen.queryByTestId(statsTestid.tierFit)).toBeNull();
    // Show 95% ranges adds the range and the fit, from the floor.
    rerender(<CreatedPanel {...grid({ showRanges: true })} />);
    expect(within(row("x")).getByTestId(statsTestid.adjustedRange)).toHaveTextContent("48.0–54.0%");
    expect(within(row("x")).getByTestId(statsTestid.tierFit)).toHaveTextContent("42%");
    expect(within(row("few")).queryByTestId(statsTestid.adjustedRange)).toBeNull();
    // R3887000: with Plain text on, the plain card stands in for the face.
    expect(screen.getByTestId(statsTestid.tokensGrid)).not.toHaveClass("stats-grid--plain");
    rerender(<CreatedPanel {...grid({ plainCardOf: (id: string) => <span data-testid={`plain-${id}`} /> })} />);
    expect(screen.getByTestId(statsTestid.tokensGrid)).toHaveClass("stats-grid--plain");
    expect(within(row("x")).getByTestId("plain-x")).toBeInTheDocument();
    expect(within(row("x")).queryByTestId("face-x")).toBeNull();
    expect(within(row("x")).getByRole("button", { name: "X" })).toHaveClass("stats-grid-card--plain");
  });

  it("R4083000 the Other created cards' Grid has its own test id", () => {
    render(<CreatedPanel {...props({ view: "grid" })} />);
    expect(screen.getByTestId(statsTestid.createdGrid)).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.createdTable)).toBeNull();
  });
});
