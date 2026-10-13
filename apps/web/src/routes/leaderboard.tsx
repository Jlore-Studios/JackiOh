// `/leaderboard` — the global ranked ladder (SPEC §9.12, R608, R612).
//
// Jlorious #1–#100 first, then every other placed player grouped by Grape tier, highest tier
// first, then how many are still playing placements. The caller's own standing heads the page.
// The page is read-only and decides nothing (CLAUDE.md rule 7): it renders `GET /api/leaderboard`
// through `rankWords`, and the hidden rating never reaches it. A row is keyed by its profile id and
// shows the player's username, never the id (R1436).

import { useEffect, useState, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t } from "../i18n/i18n.ts";
import { getLeaderboard, type LeaderboardResponse } from "../net/api.ts";
import { divisionNumeral, rankWords, tierName } from "../rank/rank.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./leaderboard.css";

export const leaderboardTestid = {
  screen: "leaderboard-screen",
  loading: "leaderboard-loading",
  error: "leaderboard-error",
  season: "leaderboard-season",
  you: "leaderboard-you",
  jlorious: "leaderboard-jlorious",
  tier: "leaderboard-tier",
  raisins: "leaderboard-raisins",
} as const;

export type LeaderboardRouteProps = {
  token: string;
};

export default function LeaderboardRoute({ token }: LeaderboardRouteProps): ReactElement {
  const [board, setBoard] = useState<LeaderboardResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getLeaderboard(token)
      .then((next) => {
        if (!cancelled) setBoard(next);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  return (
    <div className="app-shell tavern leaderboard-screen" data-testid={leaderboardTestid.screen}>
      <BackLink />
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Leaderboard")}</h2>

        {error !== null ? (
          <p className="notice" data-testid={leaderboardTestid.error} role="alert">
            {t(error)}
          </p>
        ) : null}

        {board === null && error === null ? (
          <p className="notice shell-panel__loading" data-testid={leaderboardTestid.loading} role="status">
            {t("Loading the ladder…")}
          </p>
        ) : null}

        {board !== null ? (
          <>
            <p data-testid={leaderboardTestid.season}>{t("Season {season}", { season: board.season })}</p>
            <p data-testid={leaderboardTestid.you}>
              {board.jlorious.some((row) => row.you) || board.tiers.some((tier) => tier.players.some((row) => row.you))
                ? t("You are {rank}", { rank: rankWords(board.you) })
                : t("You are {rank} — play rated games to climb", { rank: rankWords(board.you) })}
            </p>

            <h3 className="leaderboard-heading">{t("Jlorious")}</h3>
            {board.jlorious.length === 0 ? (
              <p>{t("No Jlorious players yet this season.")}</p>
            ) : (
              <ol data-testid={leaderboardTestid.jlorious}>
                {board.jlorious.map((row) => (
                  <li key={row.profileId} data-you={row.you ? "you" : undefined}>
                    {t("#{position}", { position: row.position })} <Username name={row.username} />
                    {row.you ? ` ${t("(you)")}` : ""}
                  </li>
                ))}
              </ol>
            )}

            {board.tiers.map((tier) => (
              <section key={tier.tier} data-testid={leaderboardTestid.tier} data-tier={tier.tier}>
                <h3 className="leaderboard-heading">
                  {tierName(tier.tier)} ({tier.count})
                </h3>
                {tier.players.length === 0 ? (
                  <p>{t("Nobody here yet.")}</p>
                ) : (
                  <ul>
                    {tier.players.map((row) => (
                      <li key={row.profileId} data-you={row.you ? "you" : undefined}>
                        <Username name={row.username} /> · {t("Division {division}", { division: divisionNumeral(row.division) })} ·{" "}
                        {row.pips === 1 ? t("{n} pip", { n: row.pips }) : t("{n} pips", { n: row.pips })}
                        {row.you ? ` ${t("(you)")}` : ""}
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            ))}

            <p data-testid={leaderboardTestid.raisins}>
              {board.raisins === 0
                ? t("Nobody is playing placements right now.")
                : board.raisins === 1
                  ? t("{n} player is still playing placements.", { n: board.raisins })
                  : t("{n} players are still playing placements.", { n: board.raisins })}
            </p>
          </>
        ) : null}
      </section>
    </div>
  );
}
