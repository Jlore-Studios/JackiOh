// The Created cards of /stats (SPEC §9.11, R2089000), below the Cards tab's table, in two sections that share no
// row (R3324002): the catalog's tokens, and every other card made during a game (generated and Discovered cards,
// copies, transforms' results, The Coin). A row counts the seats that made a copy: Win rate pools them, used or
// not (R2370000), Used splits by a copy Radiant at its first use, and Not used is the baseline. The figures and
// R654's floor ("—" below it) are the server's, and Win rate is read off them, nothing counted anew (CLAUDE.md
// rule 7). The rows open on Win rate, best first, and every header sorts them (R3184002). It scrolls, never fixed
// (R1646000).

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

/**
 * R2370000: a Created card's win rate: the wins of the seats that made one, used or not, over their games (each
 * side's rate times its games, rounded, as the Sets tab pools its cards'); null with none.
 */
export function createdWinRate(card: PublicCreatedStat): number | null {
  if (card.games === 0) return null;
  const wins =
    Math.round((card.createdPlayedWinRate ?? 0) * card.createdPlayedGames) +
    Math.round((card.createdNotPlayedWinRate ?? 0) * card.createdNotPlayedGames);
  return wins / card.games;
}

/** R3184002: the Created tables' columns, every one sortable. */
export type CreatedColumn = "name" | "games" | "winRate" | "used" | "notUsed" | "usedRadiant" | "usedNormal" | "delta";

/** R3184002: a header's sort: the column and its direction. */
export type CreatedSort = { col: CreatedColumn; dir: "asc" | "desc" };

/** R3324002: the sort a table opens on, its Win rate best first, as the deck table opens on its own headline rate. */
export const CREATED_DEFAULT_SORT: CreatedSort = { col: "winRate", dir: "desc" };

/** R3184002: a column's figure as the row shows it: its games, or its rate or delta, null for "—". */
function figure(card: PublicCreatedStat, col: Exclude<CreatedColumn, "name">, minSample: number): number | null {
  switch (col) {
    case "games":
      return card.games;
    case "winRate":
      return faceRate(card.games, createdWinRate(card), minSample);
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
 * either way, and rows that tie in the server's order.
 */
export function sortCreated(
  created: readonly PublicCreatedStat[],
  sort: CreatedSort,
  minSample: number,
  nameOf: (id: string, name: string) => string,
  language?: string,
): PublicCreatedStat[] {
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
  sort: CreatedSort;
  onSort: (col: CreatedColumn) => void;
}): ReactElement {
  const dir = sort.col === col ? sort.dir : null;
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

/** R3324002: which of the two sections a panel is: the tokens, or every other Created card. */
export type CreatedKind = "tokens" | "other";

export function CreatedPanel({
  kind,
  created,
  minSample,
  nameOf,
}: {
  kind: CreatedKind;
  created: readonly PublicCreatedStat[];
  minSample: number;
  /** A card's name as the page shows it. */
  nameOf: (id: string, name: string) => string;
}): ReactElement | null {
  const locale = useLocale();
  const titleId = useId();
  const [sort, setSort] = useState<CreatedSort>(CREATED_DEFAULT_SORT);
  if (created.length === 0) return null;

  const onSort = (col: CreatedColumn): void =>
    setSort((now) =>
      now.col === col
        ? { col, dir: now.dir === "asc" ? "desc" : "asc" }
        : { col, dir: col === "name" ? "asc" : "desc" },
    );
  const rows = sortCreated(created, sort, minSample, nameOf, locale === "en" ? undefined : locale);
  const header = { sort, onSort };
  const tokens = kind === "tokens";

  return (
    <section className="stats-created-panel" data-testid={tokens ? statsTestid.tokensPanel : statsTestid.createdPanel}>
      <h2 id={titleId} className="stats-games-title">
        {tokens ? t("Tokens") : t("Other created cards")}
      </h2>
      <p className="stats-sample-hint stats-games-note">
        {tokens
          ? t("Tokens made during the game.")
          : t("Every other card made during the game: generated and Discovered cards, copies and The Coin.")}{" "}
        {t(
          "Win rate is of the seats that made one, used or not; Used of those that used one, Not used of those that made one and used none. Each needs {minSample} games.",
          { minSample },
        )}
      </p>
      <TableScroll labeledBy={titleId}>
        <table className="stats-table" data-testid={tokens ? statsTestid.tokensTable : statsTestid.createdTable}>
          <thead>
            <tr>
              <SortHeader col="name" label={tokens ? t("Token") : t("Card")} {...header} />
              <SortHeader col="games" label={t("Games")} {...header} />
              <SortHeader col="winRate" label={t("Win rate")} {...header} />
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
                <td className="stats-td">{rateText(card.games, createdWinRate(card), minSample)}</td>
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
