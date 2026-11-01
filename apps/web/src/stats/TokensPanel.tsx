// The Tokens table of /stats (SPEC §9.11, R2370000), behind the Cards tab's "Show tokens" toggle: the
// catalog's tokens out of R2089000's Created cards, from the Created tallies the server already sends,
// pooled as the Sets tab pools its cards' (R1966000), so nothing is counted anew (CLAUDE.md rule 7). A
// token below R654's floor shows "—" and comes last. It sits in the scrolling panel (R1646000).

import { useId, type ReactElement } from "react";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import type { PublicCreatedStat } from "../net/api.ts";
import { rateText } from "./CreatedPanel.tsx";
import { formatPlayedDelta } from "./model.ts";
import { TableScroll } from "./TableScroll.tsx";
import { statsTestid } from "./testids.ts";

/** A token's win rate: the wins of the seats that made one, used or not, over their games; null with none. */
export function tokenWinRate(token: PublicCreatedStat): number | null {
  if (token.games === 0) return null;
  const wins =
    Math.round((token.createdPlayedWinRate ?? 0) * token.createdPlayedGames) +
    Math.round((token.createdNotPlayedWinRate ?? 0) * token.createdNotPlayedGames);
  return wins / token.games;
}

/** The tokens best first, each with its win rate; those below the floor last, in the order given. */
export function rankTokens(
  tokens: readonly PublicCreatedStat[],
  minSample: number,
): { token: PublicCreatedStat; winRate: number | null }[] {
  return tokens
    .map((token) => ({ token, winRate: token.games >= minSample ? tokenWinRate(token) : null }))
    .sort((a, b) => {
      if (a.winRate === null || b.winRate === null) return Number(a.winRate === null) - Number(b.winRate === null);
      return b.winRate - a.winRate;
    });
}

export function TokensPanel({
  tokens,
  minSample,
  nameOf,
}: {
  tokens: readonly PublicCreatedStat[];
  minSample: number;
  /** A card's name as the page shows it. */
  nameOf: (id: string, name: string) => string;
}): ReactElement {
  useLocale();
  const titleId = useId();
  const rows = rankTokens(tokens, minSample);

  return (
    <section className="stats-created-panel" data-testid={statsTestid.tokensPanel}>
      <h2 id={titleId} className="stats-games-title">{t("Tokens")}</h2>
      {rows.length === 0 ? (
        <p className="stats-sample-hint stats-games-note">{t("No tokens match the selected filters.")}</p>
      ) : (
        <>
          <p className="stats-sample-hint stats-games-note">
            {t(
              "Win rate is of the seats that made the token, used or not; Used of those that used one, Not used of those that made it and used none. Each needs {minSample} games.",
              { minSample },
            )}
          </p>
          <TableScroll labeledBy={titleId}>
            <table className="stats-table" data-testid={statsTestid.tokensTable}>
              <thead>
                <tr>
                  <th scope="col" className="stats-th">{t("Token")}</th>
                  <th scope="col" className="stats-th">{t("Games")}</th>
                  <th scope="col" className="stats-th">{t("Win rate")}</th>
                  <th scope="col" className="stats-th">{t("Used")}</th>
                  <th scope="col" className="stats-th">{t("Not used")}</th>
                  <th scope="col" className="stats-th">{t("Used vs not used")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map(({ token, winRate }) => (
                  <tr key={token.id} className="stats-tr" data-card-id={token.id}>
                    <th scope="row" className="stats-td stats-td--name">
                      {nameOf(token.id, token.name)}
                    </th>
                    <td className="stats-td">{formatNumber(token.games)}</td>
                    <td className="stats-td">{rateText(token.games, winRate, minSample)}</td>
                    <td className="stats-td">
                      {rateText(token.createdPlayedGames, token.createdPlayedWinRate, minSample)}
                    </td>
                    <td className="stats-td">
                      {rateText(token.createdNotPlayedGames, token.createdNotPlayedWinRate, minSample)}
                    </td>
                    <td className="stats-td">{formatPlayedDelta(token.createdPlayedDelta) ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </TableScroll>
        </>
      )}
    </section>
  );
}
