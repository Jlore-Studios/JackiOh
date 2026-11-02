// The Created cards of /stats (SPEC §9.11, R2089000), below the Cards tab's table: the cards made during a
// game (tokens, generated and Discovered cards, copies, transforms' results, The Coin). A row counts the seats
// that made a copy: Used, split by a copy Radiant at its first use, and Not used, the baseline. The figures,
// order and R654's floor ("—" below it) are the server's (CLAUDE.md rule 7). It scrolls, never fixed (R1646000).

import { useId, type ReactElement } from "react";
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
  useLocale();
  const titleId = useId();
  if (created.length === 0) return null;

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
              <th scope="col" className="stats-th">{t("Card")}</th>
              <th scope="col" className="stats-th">{t("Games")}</th>
              <th scope="col" className="stats-th">{t("Used")}</th>
              <th scope="col" className="stats-th">{t("Not used")}</th>
              <th scope="col" className="stats-th">{t("Used Radiant")}</th>
              <th scope="col" className="stats-th">{t("Used normal")}</th>
              <th scope="col" className="stats-th">{t("Used vs not used")}</th>
            </tr>
          </thead>
          <tbody>
            {created.map((card) => (
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
