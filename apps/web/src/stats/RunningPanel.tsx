// /stats' Experimental tab's Running view (SPEC §9.11, R4678000, R4678001): every card the Cards tab's filters list,
// on its live play version (R4675000), the version it plays today, with that version's figures over every patch since
// it began (R4675001), pooled across the patches. Card, Name, Tier and Adjusted as the Created tables show them
// (R4083000), then R5037000's Win rate, and Play rate behind the tab's Columns toggle as the Cards tab's are (R3185000,
// R3950000), Games behind it too, and Since: the version it began at and the patches it reaches.
// R5033000: its header sorts by Date, when the version it began at shipped.
// It sorts, views and ranges by the page's one state, as the Tokens tab does, and a row opens the card's drill-down.
// Every figure is the server's (CLAUDE.md rule 7); the tier fit only measures its range against its band (R3184001).
// R4680000: the same table and grid draw the Versions view, one row per play version of each card the Cards tab lists,
// its Card as that version printed it and its span in Since; a row near the screen asks for its version's catalog.
// R5035000: both views show each row's Iteration right of Since, the server's label for its play version ("v1", "v1b",
// "v2"), "—" from a server deployed before it, with a tooltip and no sort.
// R5036000: each view shows its play versions' Decisiveness after Adjusted, as the Cards tab does, and sorts by it.

import { useEffect, useState, type ReactElement, type ReactNode } from "react";
import type { CardCost, CardDef } from "@jackioh/shared";
import { canWatchArt, whenNear } from "../cards/art/near.ts";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import type { PublicCardStat, PublicPlayVersion, PublicStatsVersionsResponse } from "../net/api.ts";
import { ADJUSTED_RANGE_DIGITS } from "./config.ts";
import { SortHeader, pressedSort, sortCreated, type CreatedColumn, type CreatedSort, type SortedRow } from "./CreatedPanel.tsx";
import { RangeText, TierFitText } from "./RangeText.tsx";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText, decisivenessText } from "./TierChip.tsx";
import { cardTierFit } from "./tierFit.ts";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

/**
 * R4678001: the columns Running sorts by, in the deck table's order: its sort select. R5036000: Decisiveness among them.
 * R5037000: it has the Cards tab's Win rate and Play rate. R5033000 adds Date, which the Since header also sets.
 */
export const RUNNING_SORT_COLUMNS: readonly CreatedColumn[] = [
  "name",
  "games",
  "tier",
  "adjustedWinRate",
  "decisiveness",
  "winRate",
  "playRate",
  "date",
];

/** R4678001: the page's one sort as Running reads it: a column it lacks (Decisiveness) reads as Adjusted, keeping its direction. */
export function runningSortOf(col: string, dir: "asc" | "desc"): CreatedSort {
  return { col: RUNNING_SORT_COLUMNS.find((known) => known === col) ?? "adjustedWinRate", dir };
}

/**
 * R4678001: one card's row: its live play version's figures, with the card's cost and rarity from the Cards tab's row.
 * R4680000: on Versions, one play version's, its id the play version's own.
 */
export type RunningRow = SortedRow & {
  /** The card the row is: on Running its id too. */
  card: string;
  /** R4680000: as the row's version printed them; none while an old version's catalog loads. */
  cost: number | "X" | undefined;
  rarity: string | undefined;
  /** The version its play version began at. */
  since: string;
  /** R4680000: on Versions, the version it ended at, or null for the card's live version. */
  until?: string | null;
  /** The patches that play version reaches; none from a server deployed before it. */
  patches: number | undefined;
  /** R5037000: its win rate and play rate as the server sends them: null for none, undefined from an older server. */
  winRate: number | null | undefined;
  playRate: number | null | undefined;
  /** R5035000: its play version's iteration; none from a server deployed before it. */
  iteration: string | undefined;
  /** R4680000: the shipped version whose catalog holds an old version's definition, until that catalog has loaded. */
  needs?: string;
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
        card: card.id,
        name: version.name,
        cost: card.cost,
        rarity: card.rarity,
        games: version.games,
        adjustedWinRate: version.adjustedWinRate,
        adjustedLow: version.adjustedLow,
        adjustedHigh: version.adjustedHigh,
        decisiveness: version.decisiveness,
        tier: version.tier,
        hasEnoughGames: version.hasEnoughGames,
        since: version.first,
        patches: version.patches,
        winRate: version.winRate,
        playRate: version.playRate,
        iteration: version.iteration,
        firstShipped: version.firstShipped,
      },
    ];
  });
}

/** R4680000: a cost as the badge prints it: an Embiggen card's base cost. */
function badgeCost(cost: CardCost): number | "X" {
  return typeof cost === "object" ? cost.base : cost;
}

/**
 * R4680000: Versions' rows: every play version of each card the Cards tab lists under its filters, in that order and
 * each card's oldest first, or only the closed ones under Past versions only. A live version's cost and rarity are the
 * Cards tab's row's; an old one's are its definition's (`defOf`: undefined until its catalog loads, null where none
 * could be read), and until it has one the row names the version whose catalog it needs.
 */
export function versionRows(
  cards: readonly Pick<PublicCardStat, "id" | "cost" | "rarity">[],
  versions: readonly PublicPlayVersion[],
  pastOnly: boolean,
  defOf: (version: PublicPlayVersion) => CardDef | null | undefined,
): RunningRow[] {
  const byCard = new Map<string, PublicPlayVersion[]>();
  for (const version of versions) {
    if (pastOnly && version.live) continue;
    byCard.set(version.card, [...(byCard.get(version.card) ?? []), version]);
  }
  return cards.flatMap((card) =>
    (byCard.get(card.id) ?? []).map((version): RunningRow => {
      const def = version.live ? null : defOf(version);
      return {
        id: version.id,
        card: card.id,
        name: version.name,
        cost: version.live ? card.cost : def ? badgeCost(def.cost) : undefined,
        rarity: version.live ? card.rarity : def?.rarity,
        games: version.games,
        adjustedWinRate: version.adjustedWinRate,
        adjustedLow: version.adjustedLow,
        adjustedHigh: version.adjustedHigh,
        decisiveness: version.decisiveness,
        tier: version.tier,
        hasEnoughGames: version.hasEnoughGames,
        since: version.first,
        until: version.live ? null : version.last,
        patches: version.patches,
        winRate: version.winRate,
        playRate: version.playRate,
        iteration: version.iteration,
        firstShipped: version.firstShipped,
        needs: version.live || def ? undefined : version.last,
      };
    }),
  );
}

/** R4680000: a play version's span, "v0.5.8 – v0.6.9", one version alone, or "v0.6.10 – live" for its live one. */
export function spanText(since: string, until: string | null): string {
  if (until === null) return `${since} – ${t("live")}`;
  return until === since ? since : `${since} – ${until}`;
}

/** R5037000: a rate as the Cards tab's cells print it, a whole percent. */
function wholePercent(rate: number): string {
  return `${Math.round(rate * PERCENT)}%`;
}

/**
 * R5037000: a row's Win rate as its cell and its grid line show it, as the Cards tab does: "not enough games" below
 * R654's floor and for a play version with no games, "—" from a server deployed before the figure.
 */
function WinRateText({ row }: { row: RunningRow }): ReactElement | string {
  if (row.winRate === undefined) return "—";
  if (!row.hasEnoughGames || row.winRate === null) return <span className="stats-not-enough">{t("not enough games")}</span>;
  return <span className="stats-winrate-text">{wholePercent(row.winRate)}</span>;
}

/** R4678001: how many patches a play version reaches. */
export function patchesText(patches: number): string {
  return patches === 1 ? t("across 1 patch") : t("across {n} patches", { n: formatNumber(patches) });
}

/** R4678001: Since: the version, then the patches it reaches. R4680000: on Versions, its span, its live one marked. */
function Since({ row, testid }: { row: RunningRow; testid: string }): ReactElement {
  return (
    <span data-testid={testid}>
      {row.until === undefined ? (
        row.since
      ) : row.until === null ? (
        <>
          {row.since} – <span className="stats-version-live">{t("live")}</span>
        </>
      ) : (
        spanText(row.since, row.until)
      )}
      {row.patches !== undefined && row.patches > 0 && <span className="stats-sample-hint"> {patchesText(row.patches)}</span>}
    </span>
  );
}

/**
 * R5036000: a row's Decisiveness: "—" from a server deployed before it, "not enough games" below the floor or with no
 * turns (every version played before games recorded them), else ±X%.
 */
function decisivenessCell(row: RunningRow): ReactNode {
  if (row.decisiveness === undefined) return "—";
  if (!row.hasEnoughGames || row.decisiveness === null) return <span className="stats-not-enough">{t("not enough games")}</span>;
  return decisivenessText(row.decisiveness);
}

/** R5035000: Iteration's tooltip: what a number and a letter mean. */
function iterationTitle(): string {
  return t(
    "v1 is the card's first version. A new number (v2, v3) means what the card shows changed: its name, cost, text, keywords or stats. A new letter (v1b, v1c) means only how it plays changed.",
  );
}

/** A row's card, and on Versions its play version, whether it is live, and the catalog it still needs (R4680000). */
function rowData(row: RunningRow): Record<string, string | undefined> {
  if (row.until === undefined) return { "data-card-id": row.card };
  return {
    "data-card-id": row.card,
    "data-version-id": row.id,
    "data-live": row.until === null ? "true" : undefined,
    "data-needs": row.needs,
  };
}

/** R4678001, R4680000: what each view's table, grid, Since and Iteration (R5035000) are tested by. */
const PANEL_TESTIDS = {
  running: {
    table: statsTestid.runningTable,
    grid: statsTestid.runningGrid,
    since: statsTestid.runningSince,
    iteration: statsTestid.runningIteration,
  },
  versions: {
    table: statsTestid.versionsTable,
    grid: statsTestid.versionsGrid,
    since: statsTestid.versionsSpan,
    iteration: statsTestid.versionsIteration,
  },
} as const;

export function RunningPanel({
  kind = "running",
  rows,
  sort,
  onSort,
  view,
  showRanges,
  band,
  typical,
  labeledBy,
  nameOf,
  faceOf,
  plainCardOf,
  visibleThumbnail,
  visibleGames,
  visiblePlayRate,
  onOpen,
  onNeed,
}: {
  /** R4680000: Running, or Versions, which draws a row per play version. */
  kind?: keyof typeof PANEL_TESTIDS;
  rows: readonly RunningRow[];
  /** The page's one sort, which a header asks the page to change, as the deck table's headers do. */
  sort: CreatedSort;
  onSort: (next: CreatedSort) => void;
  view: "table" | "grid";
  showRanges: boolean;
  /** R3184001: the band a tier fit reads: the play versions' own (R4675001). */
  band: Pick<PublicStatsVersionsResponse, "bandLow" | "bandHigh">;
  /** R5036000: the typical play version's Decisiveness, which its header names; none from an older server. */
  typical?: number;
  /** The id of what names the table: the tab. */
  labeledBy: string;
  nameOf: (id: string, name: string) => string;
  /** A row's face in the Grid, as the deck grid draws it, by the row's id. */
  faceOf: (id: string) => ReactNode;
  /** R3887000: a row's plain text by its id, or null for one with none; the name stands in for it. */
  plainCardOf?: (id: string) => ReactNode;
  visibleThumbnail: boolean;
  visibleGames: boolean;
  /** R5037000: the Columns toggle's Play rate, which the Grid never shows, as the Cards tab's grid has none. */
  visiblePlayRate: boolean;
  /** A row opens its drill-down by its id, as a deck row does. */
  onOpen: (id: string, opener: HTMLElement) => void;
  /** R4680000: a row near the screen whose version's catalog has not loaded asks for it. */
  onNeed?: (version: string) => void;
}): ReactElement {
  const locale = useLocale();
  const testids = PANEL_TESTIDS[kind];
  const cardOf = new Map(rows.map((row) => [row.id, row.card]));
  const sorted = sortCreated(rows, sort, band, (id, name) => nameOf(cardOf.get(id) ?? id, name), locale === "en" ? undefined : locale);
  const header = { sort, onSort: (col: CreatedColumn) => onSort(pressedSort(sort, col)) };
  const adjustedOf = (row: RunningRow): number | null => (row.hasEnoughGames ? (row.adjustedWinRate ?? null) : null);
  const labelOf = (row: RunningRow): string =>
    row.until === undefined ? nameOf(row.card, row.name) : `${nameOf(row.card, row.name)}, ${spanText(row.since, row.until)}`;

  // R4680000: each row whose version's catalog has not loaded asks for it once it nears the screen (at once where
  // nothing can watch the screen), so only the rows on screen load theirs. Watched again when the rows or view change.
  const [root, setRoot] = useState<HTMLElement | null>(null);
  const order = sorted.map((row) => row.id).join("\n");
  useEffect(() => {
    if (onNeed === undefined || root === null) return;
    const stops = [...root.querySelectorAll<HTMLElement>("[data-needs]")].map((element) => {
      const version = element.dataset.needs ?? "";
      if (canWatchArt()) return whenNear(element, () => onNeed(version));
      onNeed(version);
      return () => undefined;
    });
    return () => {
      for (const stop of stops) stop();
    };
  }, [root, order, onNeed]);

  if (view === "grid") {
    return (
      <ul ref={setRoot} className={`stats-grid${plainCardOf ? " stats-grid--plain" : ""}`} data-testid={testids.grid} aria-labelledby={labeledBy}>
        {sorted.map((row) => {
          const plainCard = plainCardOf?.(row.id) ?? null;
          const adjusted = adjustedOf(row);
          return (
            <li key={row.id} className="stats-grid-item" {...rowData(row)}>
              <button
                type="button"
                className={`stats-grid-card${plainCard === null ? "" : " stats-grid-card--plain"}`}
                onClick={(e) => onOpen(row.id, e.currentTarget)}
                aria-label={labelOf(row)}
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
                <dt>{t("Decisiveness")}</dt>
                <dd data-testid={statsTestid.decisivenessCell}>{decisivenessCell(row)}</dd>
                <dt>{t("Win rate")}</dt>
                <dd>
                  <WinRateText row={row} />
                </dd>
                <dt>{t("Games")}</dt>
                <dd>{row.games}</dd>
                <dt>{t("Since")}</dt>
                <dd>
                  <Since row={row} testid={testids.since} />
                </dd>
                <dt title={iterationTitle()}>{t("Iteration")}</dt>
                <dd data-testid={testids.iteration}>{row.iteration ?? "—"}</dd>
              </dl>
            </li>
          );
        })}
      </ul>
    );
  }

  return (
    <TableScroll labeledBy={labeledBy}>
      <table ref={setRoot} className="stats-table" data-testid={testids.table}>
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
            <SortHeader
              col="decisiveness"
              label={t("Decisiveness")}
              title={t(
                "Decisiveness: how far playing a card typically moves your chance of winning, one way or the other; ± is the swing itself, not a margin of error. A typical card: {typical}. Versions played before games recorded their turns have none.",
                { typical: decisivenessText(typical) },
              )}
              {...header}
            />
            <SortHeader col="winRate" label={t("Win rate")} {...header} />
            {visiblePlayRate && <SortHeader col="playRate" label={t("Play rate")} {...header} />}
            <SortHeader col="date" label={t("Since")} {...header} />
            <th scope="col" className="stats-th" title={iterationTitle()}>
              {t("Iteration")}
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
                {...rowData(row)}
                aria-label={row.until === undefined ? undefined : labelOf(row)}
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
                  {plainCard ?? <span className="stats-card-name-link">{nameOf(row.card, row.name)}</span>}
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
                <td className="stats-td" data-testid={statsTestid.decisivenessCell}>
                  {decisivenessCell(row)}
                </td>
                <td className="stats-td stats-td--winrate" data-testid={statsTestid.winRateCell}>
                  <WinRateText row={row} />
                </td>
                {visiblePlayRate && (
                  <td className="stats-td" data-testid={statsTestid.playRateCell}>
                    {row.playRate == null ? "—" : wholePercent(row.playRate)}
                  </td>
                )}
                <td className="stats-td">
                  <Since row={row} testid={testids.since} />
                </td>
                <td className="stats-td" data-testid={testids.iteration}>
                  {row.iteration ?? "—"}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </TableScroll>
  );
}
