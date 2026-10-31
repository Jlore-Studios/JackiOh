// Compact card statistics block for CardDetail view (SPEC §9.11, R654). It names no data source (R661).

import { useEffect, useState, type ReactElement } from "react";
import { INSPECT_STATS } from "../cards/inspect/testids.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { getCardStats, type PublicCardStat } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { CARD_STATS_MIN_SAMPLE } from "./config.ts";
import { formatPlayedDelta, winPercent } from "./model.ts";
import { usePlayerStats } from "./store.ts";

export type CardStatsBlockProps = {
  cardId: string;
};

export function CardStatsBlock({ cardId }: CardStatsBlockProps): ReactElement {
  const [loading, setLoading] = useState(true);
  const [stat, setStat] = useState<PublicCardStat | null>(null);
  const [minSample, setMinSample] = useState<number>(CARD_STATS_MIN_SAMPLE);
  const [error, setError] = useState(false);
  /** Bumped by Try again, which loads the card's statistics again. */
  const [attempt, setAttempt] = useState(0);

  useLocale();
  const playerStats = usePlayerStats();
  const signedIn = readSession() !== null;

  useEffect(() => {
    let canceled = false;
    const controller = new AbortController();
    setLoading(true);
    setError(false);

    getCardStats({ card: cardId, signal: controller.signal })
      .then((res) => {
        if (canceled) return;
        setMinSample(res.minSample);
        const match = res.cards.find((c) => c.id === cardId) ?? null;
        setStat(match);
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
  }, [cardId, attempt]);

  if (loading) {
    return (
      <div className="inspect-stats inspect-stats--loading" data-testid={INSPECT_STATS}>
        <span className="inspect-stats-title">{t("Statistics")}</span>
        <Skeleton variant="panel" label={t("Loading stats…")} />
      </div>
    );
  }

  if (error || !stat) {
    return (
      <div className="inspect-stats inspect-stats--error" data-testid={INSPECT_STATS}>
        <span className="inspect-stats-title">{t("Statistics")}</span>
        <p className="inspect-stats-unavailable">{t("Statistics unavailable")}</p>
        {error ? (
          <button type="button" className="inspect-stats-retry" onClick={() => setAttempt((before) => before + 1)}>
            {t("Try again")}
          </button>
        ) : null}
      </div>
    );
  }

  const personalPlayed = playerStats.cards[cardId]?.played ?? 0;
  const personalRate = winPercent(playerStats);

  const winRateDisplay =
    stat.hasEnoughGames && stat.winRate !== null
      ? `${Math.round(stat.winRate * 100)}%`
      : t("not enough games");

  const drawnWinRateDisplay =
    stat.hasEnoughGames && stat.drawnWinRate !== null
      ? `${Math.round(stat.drawnWinRate * 100)}%`
      : "—";

  const drawnDeltaDisplay =
    stat.hasEnoughGames && typeof stat.drawnDelta === "number" ? (
      <>
        {formatPlayedDelta(stat.drawnDelta)}{" "}
        <span className="inspect-stats-sample-hint">
          ({stat.drawnGames} / {stat.notDrawnGames ?? 0})
        </span>
      </>
    ) : (
      t("not enough data")
    );

  const playRateDisplay = `${Math.round(stat.playRate * 100)}%`;

  return (
    <div className="inspect-stats" data-testid={INSPECT_STATS}>
      <div className="inspect-stats-header">
        <span className="inspect-stats-title">{t("Statistics")}</span>
      </div>

      <div className="inspect-stats-grid">
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Games")}</span>
          <span className="inspect-stats-metric-value">{stat.games}</span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Win rate")}</span>
          <span className="inspect-stats-metric-value">
            {winRateDisplay}
            {stat.hasEnoughGames ? null : (
              <span className="inspect-stats-sample-hint"> (&lt; {minSample})</span>
            )}
          </span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("When drawn")}</span>
          <span className="inspect-stats-metric-value">{drawnWinRateDisplay}</span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Play rate")}</span>
          <span className="inspect-stats-metric-value">{playRateDisplay}</span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Drawn vs not drawn")}</span>
          <span className="inspect-stats-metric-value">{drawnDeltaDisplay}</span>
        </div>
      </div>

      {signedIn && (
        <div className="inspect-stats-personal" data-testid="inspect-stats-personal">
          <span className="inspect-stats-personal-label">{t("Your record:")}</span>{" "}
          <span>
            {personalPlayed > 0
              ? personalRate !== null
                ? t("played {n}×, overall win rate {rate}%", { n: personalPlayed, rate: personalRate })
                : t("played {n}×", { n: personalPlayed })
              : t("not played yet")}
          </span>
        </div>
      )}

      <div className="inspect-stats-footer">
        <a
          href={paths.statsCard(cardId)}
          className="inspect-stats-link"
          onClick={followInApp(paths.statsCard(cardId))}
        >
          {t("View full stats →")}
        </a>
      </div>
    </div>
  );
}
