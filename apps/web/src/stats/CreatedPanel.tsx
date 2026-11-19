// The Created cards of /stats (SPEC §9.11, R2089000), below the Cards tab's table, in two sections that share no
// row (R3324002): the catalog's tokens, and every other card made during a game (generated and Discovered cards,
// copies, transforms' results, The Coin). A row counts the seats that made a copy: Win rate pools them, used or
// not (R2370000). The figures and R654's floor ("—" below it) are the server's, and Win rate is read off them,
// nothing counted anew (CLAUDE.md rule 7). R4082000: Tier and Adjusted are the server's too, each card read against
// the other Created cards, and the rows open on Adjusted, best first, as the deck table does; every header sorts
// them (R3184002). R4155000: both sections show the deck table's columns in its order under its labels, honour
// the tab's Columns toggle as it does, and a row opens the same drill-down a deck row opens. It scrolls, never
// fixed (R1646000). R3887000: with the Plain text button on, a catalog card's name cell holds the card as
// plain text.

import { useId, useState, type ReactElement, type ReactNode } from "react";
import { t, useLocale } from "../i18n/i18n.ts";
import type { PublicCreatedStat } from "../net/api.ts";
import { faceRate } from "./model.ts";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText, tierRank } from "./TierChip.tsx";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

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

/** R3184002, R4155000: the Created tables' columns, every one sortable, as the deck table's. */
export type CreatedColumn = "name" | "games" | "tier" | "adjusted" | "winRate";

/** R3184002: a header's sort: the column and its direction. */
export type CreatedSort = { col: CreatedColumn; dir: "asc" | "desc" };

/** R4082000: the sort a table opens on, its Adjusted best first, as the deck table opens on its own headline rate. */
export const CREATED_DEFAULT_SORT: CreatedSort = { col: "adjusted", dir: "desc" };

/** R4082000: a card's adjusted win rate as the row shows it, null below the floor or from an older server. */
function adjustedOf(card: PublicCreatedStat): number | null {
  return card.hasEnoughGames ? (card.adjustedWinRate ?? null) : null;
}

/** R3184002: a column's figure as the row shows it: its games, or its rate, null for "—". */
function figure(card: PublicCreatedStat, col: Exclude<CreatedColumn, "name" | "tier">, minSample: number): number | null {
  switch (col) {
    case "games":
      return card.games;
    case "adjusted":
      return adjustedOf(card);
    case "winRate":
      return faceRate(card.games, createdWinRate(card), minSample);
  }
}

/**
 * R3184002: the rows in a header's order: a row with no figure (below the floor, or none) after those with one,
 * either way, and rows that tie in the server's order. R4082000: by tier, S first descending, then by Adjusted.
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
    if (col === "tier") {
      const x = tierRank(a.hasEnoughGames ? a.tier : null);
      const y = tierRank(b.hasEnoughGames ? b.tier : null);
      if (x !== y) return x < 0 || y < 0 ? (x < 0 ? 1 : -1) : (x - y) * flip;
    }
    const by = col === "tier" ? "adjusted" : col;
    const x = figure(a, by, minSample);
    const y = figure(b, by, minSample);
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
  plainCardOf,
  visibleThumbnail,
  visibleGames,
  onOpen,
}: {
  kind: CreatedKind;
  created: readonly PublicCreatedStat[];
  minSample: number;
  /** A card's name as the page shows it. */
  nameOf: (id: string, name: string) => string;
  /** R3887000: a card's plain text, or null for an id with none; the name stands in for it. */
  plainCardOf?: (id: string) => ReactNode;
  /** R4155000: the tab's Columns toggle, honoured as the deck table honours it. */
  visibleThumbnail: boolean;
  visibleGames: boolean;
  /** R4155000: a row opens the card's drill-down, as a deck row does. */
  onOpen: (id: string, opener: HTMLElement) => void;
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
      <TableScroll labeledBy={titleId}>
        <table className="stats-table" data-testid={tokens ? statsTestid.tokensTable : statsTestid.createdTable}>
          <thead>
            <tr>
              {visibleThumbnail && (
                <th className="stats-th--thumb" scope="col">
                  {t("Card")}
                </th>
              )}
              <SortHeader col="name" label={t("Name")} {...header} />
              {visibleGames && <SortHeader col="games" label={t("Games")} {...header} />}
              <SortHeader col="tier" label={t("Tier")} {...header} />
              <SortHeader col="adjusted" label={t("Adjusted")} {...header} />
              <SortHeader col="winRate" label={t("Win rate")} {...header} />
            </tr>
          </thead>
          <tbody>
            {rows.map((card) => {
              const plainCard = plainCardOf?.(card.id) ?? null;
              const adjusted = adjustedOf(card);
              const winRate = faceRate(card.games, createdWinRate(card), minSample);
              return (
                <tr
                  key={card.id}
                  className="stats-tr"
                  data-card-id={card.id}
                  onClick={(e) => onOpen(card.id, e.currentTarget)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onOpen(card.id, e.currentTarget);
                    }
                  }}
                >
                  {visibleThumbnail && (
                    <td className="stats-td stats-td--thumb">
                      <span className="stats-card-thumb-badge" data-rarity={card.rarity}>
                        {card.cost}
                      </span>
                    </td>
                  )}
                  <td className="stats-td stats-td--name">
                    {plainCard ?? <span className="stats-card-name-link">{nameOf(card.id, card.name)}</span>}
                  </td>
                  {visibleGames && <td className="stats-td">{card.games}</td>}
                  <td className="stats-td">
                    <TierChip tier={card.hasEnoughGames ? card.tier : null} testid={statsTestid.createdTierCell} />
                  </td>
                  <td className="stats-td" data-testid={statsTestid.createdAdjustedCell}>
                    {adjusted === null ? (
                      "—"
                    ) : (
                      <div className="stats-winrate-cell">
                        <span className="stats-winrate-text">{adjustedText(adjusted)}</span>
                        <div className="stats-confidence-bar">
                          <div className="stats-confidence-fill" style={{ width: `${Math.round(adjusted * PERCENT)}%` }} />
                        </div>
                      </div>
                    )}
                  </td>
                  <td className="stats-td stats-td--winrate">
                    {winRate === null ? "—" : <span className="stats-winrate-text">{`${Math.round(winRate * PERCENT)}%`}</span>}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </TableScroll>
    </section>
  );
}
