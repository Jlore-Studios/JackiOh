// Compact card statistics block for CardDetail view (SPEC §9.11, R654). It names no data source (R661).

import { useEffect, useState, type ReactElement } from "react";
import { INSPECT_STATS } from "../cards/inspect/testids.ts";
import { getCardStats, type PublicCardStat } from "../net/api.ts";
import { readSession } from "../net/session.ts";
import { winPercent } from "./model.ts";
import { usePlayerStats } from "./store.ts";

export type CardStatsBlockProps = {
  cardId: string;
};

export function CardStatsBlock({ cardId }: CardStatsBlockProps): ReactElement {
  const [loading, setLoading] = useState(true);
  const [stat, setStat] = useState<PublicCardStat | null>(null);
  const [minSample, setMinSample] = useState<number>(20);
  const [error, setError] = useState(false);

  const playerStats = usePlayerStats();
  const signedIn = readSession() !== null;

  useEffect(() => {
    let cancelled = false;
    const controller = new AbortController();
    setLoading(true);
    setError(false);

    getCardStats({ card: cardId, signal: controller.signal })
      .then((res) => {
        if (cancelled) return;
        setMinSample(res.minSample);
        const match = res.cards.find((c) => c.id === cardId) ?? null;
        setStat(match);
        setLoading(false);
      })
      .catch(() => {
        if (cancelled) return;
        setError(true);
        setLoading(false);
      });

    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [cardId]);

  if (loading) {
    return (
      <div className="inspect-stats inspect-stats--loading" data-testid={INSPECT_STATS}>
        <span className="inspect-stats-title">Statistics</span>
        <p className="inspect-stats-loading">Loading stats…</p>
      </div>
    );
  }

  if (error || !stat) {
    return (
      <div className="inspect-stats inspect-stats--error" data-testid={INSPECT_STATS}>
        <span className="inspect-stats-title">Statistics</span>
        <p className="inspect-stats-unavailable">Statistics unavailable</p>
      </div>
    );
  }

  const personalPlayed = playerStats.cards[cardId]?.played ?? 0;
  const personalRate = winPercent(playerStats);

  const winRateDisplay =
    stat.hasEnoughGames && stat.winRate !== null
      ? `${Math.round(stat.winRate * 100)}%`
      : "not enough games";

  const drawnWinRateDisplay =
    stat.hasEnoughGames && stat.drawnWinRate !== null
      ? `${Math.round(stat.drawnWinRate * 100)}%`
      : "—";

  const playRateDisplay = `${Math.round(stat.playRate * 100)}%`;

  return (
    <div className="inspect-stats" data-testid={INSPECT_STATS}>
      <div className="inspect-stats-header">
        <span className="inspect-stats-title">Statistics</span>
      </div>

      <div className="inspect-stats-grid">
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">Games</span>
          <span className="inspect-stats-metric-value">{stat.games}</span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">Win rate</span>
          <span className="inspect-stats-metric-value">
            {winRateDisplay}
            {stat.hasEnoughGames ? null : (
              <span className="inspect-stats-sample-hint"> (&lt; {minSample})</span>
            )}
          </span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">When drawn</span>
          <span className="inspect-stats-metric-value">{drawnWinRateDisplay}</span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">Play rate</span>
          <span className="inspect-stats-metric-value">{playRateDisplay}</span>
        </div>
      </div>

      {signedIn && (
        <div className="inspect-stats-personal" data-testid="inspect-stats-personal">
          <span className="inspect-stats-personal-label">Your record:</span>{" "}
          <span>
            {personalPlayed > 0
              ? `played ${personalPlayed}×${personalRate !== null ? `, overall win rate ${personalRate}%` : ""}`
              : "not played yet"}
          </span>
        </div>
      )}

      <div className="inspect-stats-footer">
        <a
          href={`/stats?tab=cards&card=${encodeURIComponent(cardId)}`}
          className="inspect-stats-link"
        >
          View full stats →
        </a>
      </div>
    </div>
  );
}
