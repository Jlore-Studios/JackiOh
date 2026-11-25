// /stats' Experimental tab's Running view (SPEC §9.11, R4678000, R4678001): every card the Cards tab's filters list,
// on its live play version (R4675000), the version it plays today, with that version's figures over every patch since
// it began (R4675001), pooled across the patches. Card, Name, Tier and Adjusted as the Created tables show them
// (R4083000), Games behind the tab's Columns toggle, and Since: the version it began at and the patches it reaches.
// It sorts, views and ranges by the page's one state, as the Tokens tab does, and a row opens the card's drill-down.
// Every figure is the server's (CLAUDE.md rule 7); the tier fit only measures its range against its band (R3184001).

import type { ReactElement, ReactNode } from "react";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import type { PublicCardStat, PublicPlayVersion, PublicStatsVersionsResponse } from "../net/api.ts";
import { ADJUSTED_RANGE_DIGITS } from "./config.ts";
import { SortHeader, pressedSort, sortCreated, type CreatedColumn, type CreatedSort, type SortedRow } from "./CreatedPanel.tsx";
import { RangeText, TierFitText } from "./RangeText.tsx";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText } from "./TierChip.tsx";
import { cardTierFit } from "./tierFit.ts";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

/** R4678001: the columns Running sorts by, in the deck table's order: its sort select. It has no Decisiveness. */
export const RUNNING_SORT_COLUMNS: readonly CreatedColumn[] = ["name", "games", "tier", "adjustedWinRate"];

/** R4678001: the page's one sort as Running reads it: a column it lacks reads as Adjusted, keeping its direction. */
export function runningSortOf(col: string, dir: "asc" | "desc"): CreatedSort {
  return { col: RUNNING_SORT_COLUMNS.find((known) => known === col) ?? "adjustedWinRate", dir };
}

/** R4678001: one card's row: its live play version's figures, with the card's cost and rarity from the Cards tab's row. */
export type RunningRow = SortedRow & {
  cost: number;
  rarity: string;
  /** The version its live play version began at. */
  since: string;
  /** The patches that play version reaches; none from a server deployed before it. */
  patches: number | undefined;
};

/**
 * R4678001: Running's rows: each card the Cards tab lists under its filters, in that order, on its live play
 * version; a card with none (no shipped version holds it yet) is left out. The filters are the server's
 * (`GET /api/stats/cards`), so the two tabs list the same cards and no filter is read twice.
 */
export function runningRows(
  cards: readonly Pick<PublicCardStat, "id" | "cost" | "rarity">[],
  versions: readonly PublicPlayVersion[],
): RunningRow[] {
  const live = new Map(versions.filter((version) => version.live).map((version) => [version.card, version]));
  return cards.flatMap((card) => {
    const version = live.get(card.id);
    if (version === undefined) return [];
    return [
      {
        id: card.id,
        name: version.name,
        cost: card.cost,
        rarity: card.rarity,
        games: version.games,
        adjustedWinRate: version.adjustedWinRate,
        adjustedLow: version.adjustedLow,
        adjustedHigh: version.adjustedHigh,
        tier: version.tier,
        hasEnoughGames: version.hasEnoughGames,
        since: version.first,
        patches: version.patches,
      },
    ];
  });
}

/** R4678001: how many patches a play version reaches. */
function patchesText(patches: number): string {
  return patches === 1 ? t("across 1 patch") : t("across {n} patches", { n: formatNumber(patches) });
}

/** R4678001: Since: the version, then the patches it reaches. */
function Since({ row }: { row: RunningRow }): ReactElement {
  return (
    <span data-testid={statsTestid.runningSince}>
      {row.since}
      {row.patches !== undefined && row.patches > 0 && <span className="stats-sample-hint"> {patchesText(row.patches)}</span>}
    </span>
  );
}

export function RunningPanel({
  rows,
  sort,
  onSort,
  view,
  showRanges,
  band,
  labeledBy,
  nameOf,
  faceOf,
  plainCardOf,
  visibleThumbnail,
  visibleGames,
  onOpen,
}: {
  rows: readonly RunningRow[];
  /** The page's one sort, which a header asks the page to change, as the deck table's headers do. */
  sort: CreatedSort;
  onSort: (next: CreatedSort) => void;
  view: "table" | "grid";
  showRanges: boolean;
  /** R3184001: the band a tier fit reads: the play versions' own (R4675001). */
  band: Pick<PublicStatsVersionsResponse, "bandLow" | "bandHigh">;
  /** The id of what names the table: the tab. */
  labeledBy: string;
  nameOf: (id: string, name: string) => string;
  /** A card's face in the Grid, as the deck grid draws it. */
  faceOf: (id: string) => ReactNode;
  /** R3887000: a card's plain text, or null for an id with none; the name stands in for it. */
  plainCardOf?: (id: string) => ReactNode;
  visibleThumbnail: boolean;
  visibleGames: boolean;
  /** A row opens the card's drill-down, as a deck row does. */
  onOpen: (id: string, opener: HTMLElement) => void;
}): ReactElement {
  const locale = useLocale();
  const sorted = sortCreated(rows, sort, band, nameOf, locale === "en" ? undefined : locale);
  const header = { sort, onSort: (col: CreatedColumn) => onSort(pressedSort(sort, col)) };
  const adjustedOf = (row: RunningRow): number | null => (row.hasEnoughGames ? (row.adjustedWinRate ?? null) : null);

  if (view === "grid") {
    return (
      <ul className={`stats-grid${plainCardOf ? " stats-grid--plain" : ""}`} data-testid={statsTestid.runningGrid} aria-labelledby={labeledBy}>
        {sorted.map((row) => {
          const plainCard = plainCardOf?.(row.id) ?? null;
          const adjusted = adjustedOf(row);
          return (
            <li key={row.id} className="stats-grid-item" data-card-id={row.id}>
              <button
                type="button"
                className={`stats-grid-card${plainCard === null ? "" : " stats-grid-card--plain"}`}
                onClick={(e) => onOpen(row.id, e.currentTarget)}
                aria-label={nameOf(row.id, row.name)}
              >
                {plainCard ?? faceOf(row.id)}
              </button>
              <dl className="stats-grid-figures">
                <dt>{t("Tier")}</dt>
                <dd>
                  <TierChip tier={row.hasEnoughGames ? row.tier : null} testid={statsTestid.tierCell} />
                  {showRanges && <TierFitText fit={cardTierFit(row, band)} />}
                </dd>
                <dt>{t("Adjusted")}</dt>
                <dd data-testid={statsTestid.adjustedCell}>
                  {adjustedText(adjusted)}
                  {showRanges && adjusted !== null && (
                    <RangeText low={row.adjustedLow} high={row.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                  )}
                </dd>
                <dt>{t("Games")}</dt>
                <dd>{row.games}</dd>
                <dt>{t("Since")}</dt>
                <dd>
                  <Since row={row} />
                </dd>
              </dl>
            </li>
          );
        })}
      </ul>
    );
  }

  return (
    <TableScroll labeledBy={labeledBy}>
      <table className="stats-table" data-testid={statsTestid.runningTable}>
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
            <SortHeader col="adjustedWinRate" label={t("Adjusted")} {...header} />
            <th scope="col" className="stats-th">
              {t("Since")}
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => {
            const plainCard = plainCardOf?.(row.id) ?? null;
            const adjusted = adjustedOf(row);
            return (
              <tr
                key={row.id}
                className="stats-tr"
                data-card-id={row.id}
                onClick={(e) => onOpen(row.id, e.currentTarget)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onOpen(row.id, e.currentTarget);
                  }
                }}
              >
                {visibleThumbnail && (
                  <td className="stats-td stats-td--thumb">
                    <span className="stats-card-thumb-badge" data-rarity={row.rarity}>
                      {row.cost}
                    </span>
                  </td>
                )}
                <td className="stats-td stats-td--name">
                  {plainCard ?? <span className="stats-card-name-link">{nameOf(row.id, row.name)}</span>}
                </td>
                {visibleGames && <td className="stats-td">{row.games}</td>}
                <td className="stats-td">
                  <TierChip tier={row.hasEnoughGames ? row.tier : null} testid={statsTestid.tierCell} />
                  {showRanges && <TierFitText fit={cardTierFit(row, band)} />}
                </td>
                <td className="stats-td" data-testid={statsTestid.adjustedCell}>
                  {adjusted === null ? (
                    "—"
                  ) : (
                    <div className="stats-winrate-cell">
                      <span className="stats-winrate-text">{adjustedText(adjusted)}</span>
                      <div className="stats-confidence-bar">
                        <div className="stats-confidence-fill" style={{ width: `${Math.round(adjusted * PERCENT)}%` }} />
                      </div>
                      {showRanges && (
                        <RangeText low={row.adjustedLow} high={row.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                      )}
                    </div>
                  )}
                </td>
                <td className="stats-td">
                  <Since row={row} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </TableScroll>
  );
}
