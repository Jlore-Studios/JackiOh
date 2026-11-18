// Tests for the Created cards panels of /stats (SPEC §9.11, R2089000), the Tokens and the Other created cards
// (R3324002), their sorting (R3184002), their Tier and Adjusted (R4082000), and their deck table's columns and
// rows that open the card (R4155000).

import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PublicCreatedStat } from "../net/api.ts";
import { CreatedPanel } from "./CreatedPanel.tsx";
import { statsTestid } from "./testids.ts";

const COIN: PublicCreatedStat = {
  id: "coin",
  name: "The Coin",
  cost: 0,
  rarity: "token",
  set: "core",
  games: 40,
  createdPlayedGames: 30,
  createdPlayedWinRate: 0.5,
  createdNotPlayedGames: 10,
  createdNotPlayedWinRate: 0.4,
  createdPlayedRadiantGames: 0,
  createdPlayedRadiantWinRate: null,
  createdPlayedNormalGames: 30,
  createdPlayedNormalWinRate: 0.5,
  createdPlayedDelta: null,
  hasEnoughGames: true,
  window: { from: "v0.5.0b", liveGames: 40, source: "live", counted: ["v0.5.0b"] },
};

type PanelProps = {
  kind: "tokens" | "other";
  created: readonly PublicCreatedStat[];
  minSample: number;
  nameOf: (id: string, name: string) => string;
  plainCardOf?: (id: string) => ReactNode;
  visibleThumbnail: boolean;
  visibleGames: boolean;
  onOpen: (id: string, opener: HTMLElement) => void;
};

/** The panel's props with the tab's Columns toggle on and a no-op row opener. */
function props(over: Partial<PanelProps> = {}): PanelProps {
  return {
    kind: "other",
    created: [COIN],
    minSample: 20,
    nameOf: (_id: string, name: string) => name,
    visibleThumbnail: true,
    visibleGames: true,
    onOpen: () => {},
    ...over,
  };
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
      "Win rate",
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
    // Below the floor, neither shows, as in the deck table; the Win rate reads "—", not "not enough games".
    expect(within(row("ember")).getByTestId(statsTestid.createdTierCell)).toHaveTextContent("—");
    expect(within(row("ember")).getByTestId(statsTestid.createdAdjustedCell)).toHaveTextContent("—");
    expect(row("ember").querySelector(".stats-confidence-bar")).toBeNull();
    expect(row("ember").querySelector(".stats-td--winrate")).toHaveTextContent("—");
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

  it("R4155000 the headers are exactly Card, Name, Games, Tier, Adjusted, Win rate", () => {
    const table = renderTable();
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Card",
      "Name",
      "Games",
      "Tier",
      "Adjusted ↓",
      "Win rate",
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
      "Win rate",
    ]);
    const row = table.querySelector("tbody tr") as HTMLElement;
    expect(row.querySelector(".stats-td--thumb")).toBeNull();
    expect([...row.querySelectorAll("td")].map((td) => td.className)).toEqual([
      "stats-td stats-td--name",
      "stats-td",
      "stats-td",
      "stats-td stats-td--winrate",
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
  // The server's order, and each row's figures with a floor of 20 games ("—" below it).
  const ROWS: PublicCreatedStat[] = [
    // Games 55; tier A, Adjusted 53%; Win rate 56% (31 of 55).
    made({
      id: "rush",
      name: "Rush Token",
      games: 55,
      tier: "A",
      adjustedWinRate: 0.53,
      createdPlayedGames: 30,
      createdPlayedWinRate: 0.6,
      createdNotPlayedGames: 25,
      createdNotPlayedWinRate: 0.52,
    }),
    // Games 45; tier B, Adjusted 50.5%; Win rate 62% (28 of 45).
    made({
      id: "sprite",
      name: "Spark Sprite",
      games: 45,
      tier: "B",
      adjustedWinRate: 0.505,
      createdPlayedGames: 25,
      createdPlayedWinRate: 0.7,
      createdNotPlayedGames: 20,
      createdNotPlayedWinRate: 0.5,
    }),
    // Games 45, as many as Spark Sprite; tier B as Spark Sprite, Adjusted 51.2%; Win rate 47% (21 of 45).
    made({
      games: 45,
      tier: "B",
      adjustedWinRate: 0.512,
      createdNotPlayedGames: 15,
    }),
    // Below the floor: every figure "—", its tier and Adjusted too.
    made({
      id: "ember",
      name: "Ember",
      games: 12,
      tier: null,
      adjustedWinRate: 0.6,
      createdPlayedGames: 7,
      createdPlayedWinRate: 0.9,
      createdNotPlayedGames: 5,
      createdNotPlayedWinRate: 0.2,
      hasEnoughGames: false,
    }),
  ];

  function renderTable(): { headers: () => HTMLElement[]; order: () => (string | null)[] } {
    render(<CreatedPanel {...props({ created: ROWS })} />);
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
    // Columns: Card, Name, Games, Tier, Adjusted, Win rate.
    // Adjusted, sorted as the table opens, goes last, so its first press here sorts it descending as any other's does.
    const cases: [number, string, (string | null)[], (string | null)[]][] = [
      // Games: Spark Sprite and The Coin tie, and keep the server's order both ways.
      [2, "Games", ["rush", "sprite", "coin", "ember"], ["ember", "sprite", "coin", "rush"]],
      // R4082000: Tier, S first, then by Adjusted within a tier: The Coin and Spark Sprite share B.
      [3, "Tier", ["rush", "coin", "sprite", "ember"], ["sprite", "coin", "rush", "ember"]],
      [5, "Win rate", ["sprite", "rush", "coin", "ember"], ["coin", "rush", "sprite", "ember"]],
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
