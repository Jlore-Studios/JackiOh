// The Created cards of /stats (SPEC §9.11, R2089000), on the Tokens tab (R4279001), in two sections that share no
// row (R3324002): the catalog's tokens, and every other card made during a game (generated and Discovered cards,
// copies, transforms' results, The Coin). A row counts the seats that made a copy, used or not (R2370000). R4082000:
// Tier and Adjusted are the server's, each card read against the other Created cards, and R654's floor ("—" below
// it) is the server's too, nothing counted anew (CLAUDE.md rule 7). R4083000: those two are the only figures, with
// Games behind the tab's Columns toggle (R4155000); the rows sort as the deck table's, by the tab's one sort, which
// every header sets (R3184002), Tier by tier fit within a tier (R3184001); the Table and Grid views and the 95%
// ranges are the deck table's too. R4155000: a row opens the same drill-down a deck row opens. It scrolls, never
// fixed (R1646000). R3887000: with the Plain text button on, a catalog card's name cell, or its grid card, holds
// the card as plain text.

import { useId, type ReactElement, type ReactNode } from "react";
import { t, useLocale } from "../i18n/i18n.ts";
import type { PublicCreatedStat, PublicStatsTiers } from "../net/api.ts";
import { ADJUSTED_RANGE_DIGITS } from "./config.ts";
import { RangeText, TierFitText } from "./RangeText.tsx";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText, tierRank } from "./TierChip.tsx";
import { cardTierFit } from "./tierFit.ts";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

/** R3184002, R4083000: the Created tables' columns, every one sortable, named as the deck table's sort names them. */
export type CreatedColumn = "name" | "games" | "tier" | "adjustedWinRate";

/** R4083000: the columns the Created tables sort by, in the deck table's order: the Tokens tab's sort select. */
export const CREATED_SORT_COLUMNS: readonly CreatedColumn[] = ["name", "games", "tier", "adjustedWinRate"];

/** R3184002: a header's sort: the column and its direction. */
export type CreatedSort = { col: CreatedColumn; dir: "asc" | "desc" };

/** R4082000: the sort a table opens on, its Adjusted best first, as the deck table opens on its own headline rate. */
export const CREATED_DEFAULT_SORT: CreatedSort = { col: "adjustedWinRate", dir: "desc" };

/**
 * R4083000: the page's one sort as the Created tables read it: a column they lack (the deck table's Win rate or
 * Play rate) reads as Adjusted, keeping its direction.
 */
export function createdSortOf(col: string, dir: "asc" | "desc"): CreatedSort {
  return { col: CREATED_SORT_COLUMNS.find((known) => known === col) ?? "adjustedWinRate", dir };
}

/** R4082000: a card's adjusted win rate as the row shows it, null below the floor or from an older server. */
function adjustedOf(card: PublicCreatedStat): number | null {
  return card.hasEnoughGames ? (card.adjustedWinRate ?? null) : null;
}

/**
 * R3184002: the rows in a header's order: a row with no figure (below the floor, or none) after those with one,
 * either way, and rows that tie in the server's order. R4082000: by tier, S first descending, then by Adjusted;
 * R4083000: within a tier by tier fit first (R3184001), a card with none after, as the deck table sorts.
 */
export function sortCreated(
  created: readonly PublicCreatedStat[],
  sort: CreatedSort,
  tiers: Pick<PublicStatsTiers, "bandLow" | "bandHigh"> | undefined,
  nameOf: (id: string, name: string) => string,
  language?: string,
): PublicCreatedStat[] {
  const flip = sort.dir === "desc" ? -1 : 1;
  const { col } = sort;
  return [...created].sort((a, b) => {
    if (col === "name") return nameOf(a.id, a.name).localeCompare(nameOf(b.id, b.name), language) * flip;
    if (col === "games") return (a.games - b.games) * flip;
    if (col === "tier") {
      const x = tierRank(a.hasEnoughGames ? a.tier : null);
      const y = tierRank(b.hasEnoughGames ? b.tier : null);
      if (x !== y) return x < 0 || y < 0 ? (x < 0 ? 1 : -1) : (x - y) * flip;
      const p = cardTierFit(a, tiers);
      const q = cardTierFit(b, tiers);
      if (p !== q) return p === null || q === null ? (p === null ? 1 : -1) : (p - q) * flip;
    }
    const x = adjustedOf(a);
    const y = adjustedOf(b);
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
  sort,
  onSort,
  view,
  showRanges,
  tiers,
  nameOf,
  faceOf,
  plainCardOf,
  visibleThumbnail,
  visibleGames,
  onOpen,
}: {
  kind: CreatedKind;
  created: readonly PublicCreatedStat[];
  /** R4083000: the tab's sort, which a header asks the tab to change, as the deck table's headers do. */
  sort: CreatedSort;
  onSort: (next: CreatedSort) => void;
  /** R4083000: the tab's Table or Grid view and its Show 95% ranges, as the deck cards show them. */
  view: "table" | "grid";
  showRanges: boolean;
  /** R3184001: the band a tier fit reads; absent from a server deployed before it. */
  tiers: Pick<PublicStatsTiers, "bandLow" | "bandHigh"> | undefined;
  /** A card's name as the page shows it. */
  nameOf: (id: string, name: string) => string;
  /** R4083000: a card's face in the Grid, as the deck grid draws it; null for an id with none. */
  faceOf: (id: string) => ReactNode;
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
  if (created.length === 0) return null;

  const pressed = (col: CreatedColumn): void =>
    onSort(
      sort.col === col
        ? { col, dir: sort.dir === "asc" ? "desc" : "asc" }
        : { col, dir: col === "name" ? "asc" : "desc" },
    );
  const rows = sortCreated(created, sort, tiers, nameOf, locale === "en" ? undefined : locale);
  const header = { sort, onSort: pressed };
  const tokens = kind === "tokens";

  return (
    <section className="stats-created-panel" data-testid={tokens ? statsTestid.tokensPanel : statsTestid.createdPanel}>
      <h2 id={titleId} className="stats-games-title">
        {tokens ? t("Tokens") : t("Other created cards")}
      </h2>
      {view === "grid" ? (
        <ul
          className={`stats-grid${plainCardOf ? " stats-grid--plain" : ""}`}
          data-testid={tokens ? statsTestid.tokensGrid : statsTestid.createdGrid}
          aria-labelledby={titleId}
        >
          {rows.map((card) => {
            const plainCard = plainCardOf?.(card.id) ?? null;
            const adjusted = adjustedOf(card);
            return (
              <li key={card.id} className="stats-grid-item" data-card-id={card.id}>
                <button
                  type="button"
                  className={`stats-grid-card${plainCard === null ? "" : " stats-grid-card--plain"}`}
                  onClick={(e) => onOpen(card.id, e.currentTarget)}
                  aria-label={nameOf(card.id, card.name)}
                >
                  {plainCard ?? faceOf(card.id)}
                </button>
                <dl className="stats-grid-figures">
                  <dt>{t("Tier")}</dt>
                  <dd>
                    <TierChip tier={card.hasEnoughGames ? card.tier : null} testid={statsTestid.createdTierCell} />
                    {showRanges && <TierFitText fit={cardTierFit(card, tiers)} />}
                  </dd>
                  <dt>{t("Adjusted")}</dt>
                  <dd data-testid={statsTestid.createdAdjustedCell}>
                    {adjustedText(adjusted)}
                    {showRanges && adjusted !== null && (
                      <RangeText low={card.adjustedLow} high={card.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                    )}
                  </dd>
                  <dt>{t("Games")}</dt>
                  <dd>{card.games}</dd>
                </dl>
              </li>
            );
          })}
        </ul>
      ) : (
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
                <SortHeader col="adjustedWinRate" label={t("Adjusted")} {...header} />
              </tr>
            </thead>
            <tbody>
              {rows.map((card) => {
                const plainCard = plainCardOf?.(card.id) ?? null;
                const adjusted = adjustedOf(card);
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
                      {showRanges && <TierFitText fit={cardTierFit(card, tiers)} />}
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
                          {showRanges && (
                            <RangeText low={card.adjustedLow} high={card.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                          )}
                        </div>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </TableScroll>
      )}
    </section>
  );
}
