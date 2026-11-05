// The Created cards of /stats (SPEC §9.11, R2089000), below the Cards tab's table: the cards made during a
// game (tokens, generated and Discovered cards, copies, transforms' results, The Coin). A row counts the seats
// that made a copy: Used, split by a copy Radiant at its first use, and Not used, the baseline. The figures and
// R654's floor ("—" below it) are the server's (CLAUDE.md rule 7); the rows keep the server's order until a
// header sorts them (R3184002). It scrolls, never fixed (R1646000).

import { useId, useState, type ReactElement } from "react";
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

/** R3184002: the Created table's columns, every one sortable. */
export type CreatedColumn = "name" | "games" | "used" | "notUsed" | "usedRadiant" | "usedNormal" | "delta";

/** R3184002: a header's sort: the column and its direction. */
export type CreatedSort = { col: CreatedColumn; dir: "asc" | "desc" };

/** R3184002: a column's figure as the row shows it: its games, or its rate or delta, null for "—". */
function figure(card: PublicCreatedStat, col: Exclude<CreatedColumn, "name">, minSample: number): number | null {
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
 * R3184002: the rows in a header's order: a row with no figure (below the floor, or none) after those with one,
 * either way, and rows that tie in the server's order. No sort keeps the server's order.
 */
export function sortCreated(
  created: readonly PublicCreatedStat[],
  sort: CreatedSort | null,
  minSample: number,
  nameOf: (id: string, name: string) => string,
  language?: string,
): PublicCreatedStat[] {
  if (sort === null) return [...created];
  const flip = sort.dir === "desc" ? -1 : 1;
  const { col } = sort;
  return [...created].sort((a, b) => {
    if (col === "name") return nameOf(a.id, a.name).localeCompare(nameOf(b.id, b.name), language) * flip;
    const x = figure(a, col, minSample);
    const y = figure(b, col, minSample);
    if (x === null || y === null) return x === y ? 0 : x === null ? 1 : -1;
    return (x - y) * flip;
  });
}

/** R3184002: a sortable header: the first press sorts descending (a name ascending), the next flips it. */
function SortHeader({
  col,
  label,
  sort,
  onSort,
}: {
  col: CreatedColumn;
  label: string;
  sort: CreatedSort | null;
  onSort: (col: CreatedColumn) => void;
}): ReactElement {
  const dir = sort?.col === col ? sort.dir : null;
  return (
    <th
      scope="col"
      className="stats-th stats-th--sortable"
      aria-sort={dir === null ? "none" : dir === "asc" ? "ascending" : "descending"}
      onClick={() => onSort(col)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSort(col);
        }
      }}
      tabIndex={0}
    >
      {dir === null ? label : `${label} ${dir === "asc" ? "↑" : "↓"}`}
    </th>
  );
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
  const [sort, setSort] = useState<CreatedSort | null>(null);
  if (created.length === 0) return null;

  const onSort = (col: CreatedColumn): void =>
    setSort((now) =>
      now?.col === col
        ? { col, dir: now.dir === "asc" ? "desc" : "asc" }
        : { col, dir: col === "name" ? "asc" : "desc" },
    );
  const rows = sortCreated(created, sort, minSample, nameOf, locale === "en" ? undefined : locale);
  const header = { sort, onSort };

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
              <SortHeader col="name" label={t("Card")} {...header} />
              <SortHeader col="games" label={t("Games")} {...header} />
              <SortHeader col="used" label={t("Used")} {...header} />
              <SortHeader col="notUsed" label={t("Not used")} {...header} />
              <SortHeader col="usedRadiant" label={t("Used Radiant")} {...header} />
              <SortHeader col="usedNormal" label={t("Used normal")} {...header} />
              <SortHeader col="delta" label={t("Used vs not used")} {...header} />
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
