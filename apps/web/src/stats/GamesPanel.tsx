// The Games panel of /stats (SPEC §9.11, R1994000): how the patch's games went, for the match type
// shown: the first player's win share of the decided games with its margin, and no draw figure
// (R2567000); the median game's length and the share that reach fatigue, and nothing of the set traps
// (R2446000). It names no data source and no gate (R661), and sits in the scrolling panel, never the
// fixed top (R1646000).

import { useEffect, useState, type ReactElement } from "react";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import { getGameStats, type PublicStatsGamesResponse } from "../net/api.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { statsTestid } from "./testids.ts";

/** Unit conversion, not configuration: a share of 0..1 as a percentage. */
const PERCENT = 100;

export function GamesPanel({ patch, mode }: { patch: string; mode: string }): ReactElement {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<PublicStatsGamesResponse | null>(null);
  const [error, setError] = useState(false);
  /** Bumped by Try again, to load the figures again. */
  const [attempt, setAttempt] = useState(0);

  useLocale();

  useEffect(() => {
    let canceled = false;
    const controller = new AbortController();
    setLoading(true);
    setError(false);

    getGameStats({ patch: patch || undefined, mode: mode || undefined, signal: controller.signal })
      .then((res) => {
        if (canceled) return;
        setData(res);
        setLoading(false);
      })
      .catch(() => {
        if (canceled) return;
        setError(true);
        setLoading(false);
      });

    return () => {
      canceled = true;
      controller.abort();
    };
  }, [patch, mode, attempt]);

  const figures = data?.figures ?? null;
  // The figures to show, none below the sample floor; and the games they count against the patch's.
  const shown = data !== null && figures !== null && figures.games >= data.minSample ? figures : null;
  const counted =
    data !== null && figures !== null && figures.games < data.totalGames
      ? { games: figures.games, total: data.totalGames }
      : null;

  return (
    <section className="stats-games-panel" data-testid={statsTestid.gamesPanel}>
      <h2 className="stats-games-title">{t("Game figures")}</h2>
      {loading ? (
        <Skeleton variant="rows" label={t("Loading game figures…")} />
      ) : error ? (
        <div className="stats-games-status">
          <p>{t("Game figures unavailable")}</p>
          <button
            type="button"
            className="stats-retry-btn"
            data-testid={statsTestid.gamesRetry}
            onClick={() => setAttempt((before) => before + 1)}
          >
            {t("Try again")}
          </button>
        </div>
      ) : (
        <>
          {shown === null ? (
            <p className="stats-games-status" data-testid={statsTestid.gamesFew}>
              {t("Not enough games on this patch yet.")}
            </p>
          ) : (
            <ul className="stats-games-lines">
              <li>
                {shown.firstShare === null
                  ? t("First player wins: unknown, no game was won")
                  : t("First player wins {share}% of decided games (±{margin})", {
                      share: Math.round(shown.firstShare * PERCENT),
                      margin: Math.round(((shown.firstHigh - shown.firstLow) / 2) * PERCENT),
                    })}
              </li>
              <li>{t("Median {turns} turns", { turns: formatNumber(shown.turnQuartiles[1]) })}</li>
              {shown.fatigueShare !== null && (
                <li>{t("{share}% of games reach fatigue", { share: Math.round(shown.fatigueShare * PERCENT) })}</li>
              )}
            </ul>
          )}
          {counted !== null && (
            <p className="stats-sample-hint stats-games-note">
              {t("Counted over {games} of the patch's {total} games: games recorded before these figures were kept are not in them.", {
                games: formatNumber(counted.games),
                total: formatNumber(counted.total),
              })}
            </p>
          )}
        </>
      )}
    </section>
  );
}
