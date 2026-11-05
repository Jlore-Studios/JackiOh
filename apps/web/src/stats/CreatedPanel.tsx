// The Created cards of /stats (SPEC §9.11, R2089000), below the Cards tab's table: the cards made during a
// game (tokens, generated and Discovered cards, copies, transforms' results, The Coin). A row counts the seats
// that made a copy: Used, split by a copy Radiant at its first use, and Not used, the baseline. The figures and
// R654's floor ("—" below it) are the server's (CLAUDE.md rule 7), and so is the order until a column's header is
// clicked: each sorts the rows, a row with no figure in it last (R3184000). It scrolls, never fixed (R1646000).

import { useId, useState, type KeyboardEvent, type ReactElement } from "react";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import type { PublicCreatedStat } from "../net/api.ts";
import { faceRate, formatPlayedDelta } from "./model.ts";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

/** A rate as the cards table shows one, or "—" when there is none or it is below the floor. */
export function rateText(games: number, rate: number | null, minSample: number): string {
  const shown = faceRate(games, rate, minSample);
  return shown === null ? "—" : `${Math.round(shown * PERCENT)}%`;
}

/** R3184000: the table's columns, each a sort. */
const CREATED_COLUMNS = ["name", "games", "used", "notUsed", "usedRadiant", "usedNormal", "delta"] as const;
type CreatedColumn = (typeof CREATED_COLUMNS)[number];
type CreatedSort = { col: CreatedColumn; dir: "asc" | "desc" };

const CREATED_LABELS: Record<CreatedColumn, string> = {
  name: "Card",
  games: "Games",
  used: "Used",
  notUsed: "Not used",
  usedRadiant: "Used Radiant",
  usedNormal: "Used normal",
  delta: "Used vs not used",
};

/** R3184000: a row's figure in a numeric column as it is shown, null when it shows "—". */
function figureOf(card: PublicCreatedStat, col: Exclude<CreatedColumn, "name">, minSample: number): number | null {
  switch (col) {
    case "games":
      return card.games;
    case "used":
      return faceRate(card.createdPlayedGames, card.createdPlayedWinRate, minSample);
    case "notUsed":
      return faceRate(card.createdNotPlayedGames, card.createdNotPlayedWinRate, minSample);
    case "usedRadiant":
      return faceRate(card.createdPlayedRadiantGames, card.createdPlayedRadiantWinRate, minSample);
    case "usedNormal":
      return faceRate(card.createdPlayedNormalGames, card.createdPlayedNormalWinRate, minSample);
    case "delta":
      return card.createdPlayedDelta;
  }
}

/**
 * R3184000: the rows in `sort`'s order, the server's with none. A name sorts as the page shows it; a row with no
 * figure in the column (below the floor, or none) goes after those with one either way, and ties keep the server's order.
 */
export function sortCreated(
  created: readonly PublicCreatedStat[],
  sort: CreatedSort | null,
  minSample: number,
  nameOf: (id: string, name: string) => string,
  language?: string,
): PublicCreatedStat[] {
  const rows = [...created];
  if (sort === null) return rows;
  const flip = sort.dir === "desc" ? -1 : 1;
  const { col } = sort;
  if (col === "name") {
    return rows.sort((a, b) => nameOf(a.id, a.name).localeCompare(nameOf(b.id, b.name), language) * flip);
  }
  return rows.sort((a, b) => {
    const aFigure = figureOf(a, col, minSample);
    const bFigure = figureOf(b, col, minSample);
    if (aFigure !== null && bFigure === null) return -1;
    if (aFigure === null && bFigure !== null) return 1;
    if (aFigure === null || bFigure === null) return 0;
    return (aFigure - bFigure) * flip;
  });
}

export function CreatedPanel({
  created,
  minSample,
  nameOf,
}: {
  created: readonly PublicCreatedStat[];
  minSample: number;
  /** A card's name as the page shows it. */
  nameOf: (id: string, name: string) => string;
}): ReactElement | null {
  const locale = useLocale();
  const titleId = useId();
  // R3184000: none until a header is clicked, the server's order.
  const [sort, setSort] = useState<CreatedSort | null>(null);
  if (created.length === 0) return null;

  const rows = sortCreated(created, sort, minSample, nameOf, locale === "en" ? undefined : locale);
  // A first click sorts a figure high to low and a name A to Z; a second click turns it round.
  const sortBy = (col: CreatedColumn): void =>
    setSort((now) =>
      now?.col === col ? { col, dir: now.dir === "asc" ? "desc" : "asc" } : { col, dir: col === "name" ? "asc" : "desc" },
    );
  const onHeaderKey = (col: CreatedColumn) => (event: KeyboardEvent<HTMLTableCellElement>): void => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      sortBy(col);
    }
  };

  return (
    <section className="stats-created-panel" data-testid={statsTestid.createdPanel}>
      <h2 id={titleId} className="stats-games-title">{t("Created cards")}</h2>
      <p className="stats-sample-hint stats-games-note">
        {t(
          "Cards made during the game: tokens, generated cards, copies and The Coin. Used is the win rate of the seats that made a Created copy of the card and used one, Not used of those that made copies and used none; each needs {minSample} games.",
          { minSample },
        )}
      </p>
      <TableScroll labeledBy={titleId}>
        <table className="stats-table" data-testid={statsTestid.createdTable}>
          <thead>
            <tr>
              {CREATED_COLUMNS.map((col) => {
                const active = sort?.col === col ? sort.dir : null;
                return (
                  <th
                    key={col}
                    scope="col"
                    className="stats-th stats-th--sortable"
                    aria-sort={active === null ? "none" : active === "asc" ? "ascending" : "descending"}
                    onClick={() => sortBy(col)}
                    onKeyDown={onHeaderKey(col)}
                    tabIndex={0}
                  >
                    {t(CREATED_LABELS[col])}
                    {active !== null && ` ${active === "asc" ? "↑" : "↓"}`}
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {rows.map((card) => (
              <tr key={card.id} className="stats-tr" data-card-id={card.id}>
                <th scope="row" className="stats-td stats-td--name">
                  {nameOf(card.id, card.name)}
                </th>
                <td className="stats-td">{formatNumber(card.games)}</td>
                <td className="stats-td">
                  {rateText(card.createdPlayedGames, card.createdPlayedWinRate, minSample)}
                </td>
                <td className="stats-td">
                  {rateText(card.createdNotPlayedGames, card.createdNotPlayedWinRate, minSample)}
                </td>
                <td className="stats-td">
                  {rateText(card.createdPlayedRadiantGames, card.createdPlayedRadiantWinRate, minSample)}
                </td>
                <td className="stats-td">
                  {rateText(card.createdPlayedNormalGames, card.createdPlayedNormalWinRate, minSample)}
                </td>
                <td className="stats-td">{formatPlayedDelta(card.createdPlayedDelta) ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </TableScroll>
    </section>
  );
}
