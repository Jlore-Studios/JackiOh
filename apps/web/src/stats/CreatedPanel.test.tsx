// Tests for the Created cards panel of /stats (SPEC §9.11, R2089000).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { PublicCreatedStat } from "../net/api.ts";
import { CreatedPanel } from "./CreatedPanel.tsx";

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
    render(<CreatedPanel created={[COIN]} minSample={20} nameOf={(_id, name) => name} />);
    const region = screen.getByRole("region", { name: "Created cards" });
    expect(region.tabIndex).toBe(0);
    expect(region.querySelector("table")).not.toBeNull();
  });
});

/** A Created card: its seats, and each figure as games and a rate (`[games, rate]`). */
function stat(
  id: string,
  name: string,
  used: [number, number | null],
  notUsed: [number, number | null],
  radiant: [number, number | null],
  normal: [number, number | null],
  delta: number | null,
): PublicCreatedStat {
  return {
    ...COIN,
    id,
    name,
    games: used[0] + notUsed[0],
    createdPlayedGames: used[0],
    createdPlayedWinRate: used[1],
    createdNotPlayedGames: notUsed[0],
    createdNotPlayedWinRate: notUsed[1],
    createdPlayedRadiantGames: radiant[0],
    createdPlayedRadiantWinRate: radiant[1],
    createdPlayedNormalGames: normal[0],
    createdPlayedNormalWinRate: normal[1],
    createdPlayedDelta: delta,
  };
}

describe("R3184000 sorting the Created cards", () => {
  // In the server's order. Below the floor of 20: The Coin's Not used, Rush Token's Used Radiant and every Ghoul Token figure.
  const ROWS = [
    stat("coin", "The Coin", [30, 0.5], [10, 0.4], [0, null], [30, 0.5], null),
    stat("rush", "Rush Token", [30, 0.6], [25, 0.52], [8, 0.75], [22, 0.545], 0.08),
    stat("sheep", "Sheep Token", [21, 0.7], [24, 0.45], [21, 0.7], [0, null], 0.25),
    stat("ghoul", "Ghoul Token", [12, 0.9], [0, null], [0, null], [12, 0.9], null),
  ];

  function renderRows(): HTMLElement {
    render(<CreatedPanel created={ROWS} minSample={20} nameOf={(_id, name) => name} />);
    return screen.getByRole("table");
  }

  const order = (table: HTMLElement): (string | null)[] =>
    [...table.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-card-id"));

  it("R3184000 keeps the server's order until a header is clicked", () => {
    const table = renderRows();
    expect(order(table)).toEqual(["coin", "rush", "sheep", "ghoul"]);
    for (const header of within(table).getAllByRole("columnheader")) {
      expect(header).toHaveAttribute("aria-sort", "none");
      expect(header.textContent).not.toMatch(/[↑↓]/);
    }
  });

  it("R3184000 sorts by every column both ways, high first on a first click, a row with no figure last", () => {
    const table = renderRows();
    // Each column's order on a first click, then a second.
    const expected: [string, string[], string[]][] = [
      ["Games", ["rush", "sheep", "coin", "ghoul"], ["ghoul", "coin", "sheep", "rush"]],
      ["Used", ["sheep", "rush", "coin", "ghoul"], ["coin", "rush", "sheep", "ghoul"]],
      ["Not used", ["rush", "sheep", "coin", "ghoul"], ["sheep", "rush", "coin", "ghoul"]],
      ["Used Radiant", ["sheep", "coin", "rush", "ghoul"], ["sheep", "coin", "rush", "ghoul"]],
      ["Used normal", ["rush", "coin", "sheep", "ghoul"], ["coin", "rush", "sheep", "ghoul"]],
      ["Used vs not used", ["sheep", "rush", "coin", "ghoul"], ["rush", "sheep", "coin", "ghoul"]],
    ];
    for (const [label, first, second] of expected) {
      const header = within(table).getByRole("columnheader", { name: new RegExp(`^${label}( [↑↓])?$`) });
      fireEvent.click(header);
      expect(order(table), `${label} descending`).toEqual(first);
      expect(header).toHaveAttribute("aria-sort", "descending");
      expect(header).toHaveTextContent(`${label} ↓`);
      fireEvent.click(header);
      expect(order(table), `${label} ascending`).toEqual(second);
      expect(header).toHaveAttribute("aria-sort", "ascending");
      expect(header).toHaveTextContent(`${label} ↑`);
    }
  });

  it("R3184000 sorts by name A to Z first, as the page shows it, and the keyboard sorts as a click does", () => {
    const table = renderRows();
    const card = within(table).getByRole("columnheader", { name: /^Card/ });
    fireEvent.keyDown(card, { key: "Enter" });
    expect(order(table)).toEqual(["ghoul", "rush", "sheep", "coin"]);
    expect(card).toHaveAttribute("aria-sort", "ascending");
    fireEvent.keyDown(card, { key: " " });
    expect(order(table)).toEqual(["coin", "sheep", "rush", "ghoul"]);
    expect(card).toHaveAttribute("aria-sort", "descending");

    // Another column takes the sort over: only its header holds it.
    const games = within(table).getByRole("columnheader", { name: /^Games/ });
    fireEvent.click(games);
    expect(card).toHaveAttribute("aria-sort", "none");
    expect(games).toHaveAttribute("aria-sort", "descending");
  });
});
