// Tests for the Created cards panels of /stats (SPEC §9.11, R2089000), the Tokens and the Other created cards
// (R3324002), and their sorting (R3184002).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

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

afterEach(cleanup);

describe("CreatedPanel", () => {
  it("scrolls its table in a region the keyboard can reach, as a phone needs (axe scrollable-region-focusable)", () => {
    render(<CreatedPanel kind="other" created={[COIN]} minSample={20} nameOf={(_id, name) => name} />);
    const region = screen.getByRole("region", { name: "Other created cards" });
    expect(region.tabIndex).toBe(0);
    expect(region.querySelector("table")).not.toBeNull();
  });

  it("R3324002 the Tokens panel is the same table under its own title and test ids", () => {
    render(<CreatedPanel kind="tokens" created={[COIN]} minSample={20} nameOf={(_id, name) => name} />);
    const panel = screen.getByTestId(statsTestid.tokensPanel);
    expect(within(panel).getByRole("heading", { name: "Tokens" })).toBeInTheDocument();
    const table = within(panel).getByTestId(statsTestid.tokensTable);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Token",
      "Games",
      "Win rate ↓",
      "Used",
      "Not used",
      "Used Radiant",
      "Used normal",
      "Used vs not used",
    ]);
    expect(screen.getByRole("region", { name: "Tokens" })).toContainElement(table);
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();
  });
});

describe("R3184002 the Created cards table sorts by every column", () => {
  const made = (over: Partial<PublicCreatedStat>): PublicCreatedStat => ({ ...COIN, ...over });
  // The server's order, and each row's figures with a floor of 20 games ("—" below it).
  const ROWS: PublicCreatedStat[] = [
    // Games 55; Used 60%, Not used 52%, Radiant — (8 seats), normal 55%, delta +8.
    made({
      id: "rush",
      name: "Rush Token",
      games: 55,
      createdPlayedGames: 30,
      createdPlayedWinRate: 0.6,
      createdNotPlayedGames: 25,
      createdNotPlayedWinRate: 0.52,
      createdPlayedRadiantGames: 8,
      createdPlayedRadiantWinRate: 0.75,
      createdPlayedNormalGames: 22,
      createdPlayedNormalWinRate: 12 / 22,
      createdPlayedDelta: 0.08,
    }),
    // Games 45; Used 70%, Not used 50%, Radiant 65%, normal — (5 seats), delta +20.
    made({
      id: "sprite",
      name: "Spark Sprite",
      games: 45,
      createdPlayedGames: 25,
      createdPlayedWinRate: 0.7,
      createdNotPlayedGames: 20,
      createdNotPlayedWinRate: 0.5,
      createdPlayedRadiantGames: 20,
      createdPlayedRadiantWinRate: 0.65,
      createdPlayedNormalGames: 5,
      createdPlayedNormalWinRate: 0.6,
      createdPlayedDelta: 0.2,
    }),
    // Games 45, as many as Spark Sprite; Used 50%, Not used — (15 seats), Radiant —, normal 50%, no delta.
    made({ games: 45, createdNotPlayedGames: 15, createdPlayedNormalGames: 30, createdPlayedNormalWinRate: 0.5 }),
    // Below the floor: every figure "—".
    made({
      id: "ember",
      name: "Ember",
      games: 12,
      createdPlayedGames: 7,
      createdPlayedWinRate: 0.9,
      createdNotPlayedGames: 5,
      createdNotPlayedWinRate: 0.2,
      createdPlayedRadiantGames: 2,
      createdPlayedRadiantWinRate: 1,
      createdPlayedNormalGames: 5,
      createdPlayedNormalWinRate: 0.8,
      createdPlayedDelta: null,
      hasEnoughGames: false,
    }),
  ];

  function renderTable(): { headers: () => HTMLElement[]; order: () => (string | null)[] } {
    render(<CreatedPanel kind="other" created={ROWS} minSample={20} nameOf={(_id, name) => name} />);
    const table = screen.getByTestId(statsTestid.createdTable);
    return {
      headers: () => within(table).getAllByRole("columnheader"),
      order: () => [...table.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-card-id")),
    };
  }

  it("R3324002 opens on Win rate, best first, a row below the floor last, every other header unsorted", () => {
    const { headers, order } = renderTable();
    // Win rate, the seats' wins over their games: Spark Sprite 28 of 45, Rush 31 of 55, The Coin 21 of 45; Ember's 12 are below the floor.
    expect(order()).toEqual(["sprite", "rush", "coin", "ember"]);
    expect(headers()[2]).toHaveAttribute("aria-sort", "descending");
    expect(headers()[2]).toHaveTextContent("Win rate ↓");
    for (const header of headers().filter((_, at) => at !== 2)) {
      expect(header).toHaveAttribute("aria-sort", "none");
      expect(header.textContent).not.toMatch(/[↑↓]/);
    }
  });

  it("R3184002 a figure's first press sorts descending and the next ascending, a row with none last either way", () => {
    const { headers, order } = renderTable();
    // Columns: Card, Games, Win rate, Used, Not used, Used Radiant, Used normal, Used vs not used. Win rate, sorted
    // as the table opens, goes last, so its first press here sorts it descending as any other's does.
    const cases: [number, string, (string | null)[], (string | null)[]][] = [
      // Games: Spark Sprite and The Coin tie, and keep the server's order both ways.
      [1, "Games", ["rush", "sprite", "coin", "ember"], ["ember", "sprite", "coin", "rush"]],
      [3, "Used", ["sprite", "rush", "coin", "ember"], ["coin", "rush", "sprite", "ember"]],
      [4, "Not used", ["rush", "sprite", "coin", "ember"], ["sprite", "rush", "coin", "ember"]],
      [5, "Used Radiant", ["sprite", "rush", "coin", "ember"], ["sprite", "rush", "coin", "ember"]],
      [6, "Used normal", ["rush", "coin", "sprite", "ember"], ["coin", "rush", "sprite", "ember"]],
      [7, "Used vs not used", ["sprite", "rush", "coin", "ember"], ["rush", "sprite", "coin", "ember"]],
      [2, "Win rate", ["sprite", "rush", "coin", "ember"], ["coin", "rush", "sprite", "ember"]],
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
      // Only the pressed header is sorted.
      expect(headers().filter((header) => header.getAttribute("aria-sort") !== "none")).toEqual([headers()[at]]);
    }
  });

  it("R3184002 the card's name sorts ascending first, and the keyboard sorts as a press does", () => {
    const { headers, order } = renderTable();
    fireEvent.keyDown(headers()[0]!, { key: "Enter" });
    expect(headers()[0]).toHaveAttribute("aria-sort", "ascending");
    expect(headers()[0]).toHaveTextContent("Card ↑");
    expect(order()).toEqual(["ember", "rush", "sprite", "coin"]);
    fireEvent.keyDown(headers()[0]!, { key: " " });
    expect(headers()[0]).toHaveAttribute("aria-sort", "descending");
    expect(order()).toEqual(["coin", "sprite", "rush", "ember"]);
  });
});
