// Compact card statistics block for CardDetail view (SPEC §9.11, R654). It names no data source (R661). It shows
// the four figures that read a card, Tier, Adjusted win rate, Win rate and Play rate, with Games as their sample,
// as /stats prints them, and nothing else of the server's (R3185000) but the patch they were collected on, the
// patch before while a newer one gathers its games (R3447000).

import { useEffect, useState, type ReactElement } from "react";
import { INSPECT_STATS } from "../cards/inspect/testids.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { getCardStats, type PublicCardStat } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { followInApp } from "../routes/nav.tsx";
import { ArrowIcon } from "../routes/ArrowIcon.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { CARD_STATS_MIN_SAMPLE } from "./config.ts";
import { winPercent } from "./model.ts";
import { collectedOn } from "./patches.ts";
import { usePlayerStats } from "./store.ts";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText } from "./TierChip.tsx";

export type CardStatsBlockProps = {
  cardId: string;
};

export function CardStatsBlock({ cardId }: CardStatsBlockProps): ReactElement {
  const [loading, setLoading] = useState(true);
  const [stat, setStat] = useState<PublicCardStat | null>(null);
  /** R3447000: the patch the card's figures were collected on, or null when the server names no window. */
  const [collected, setCollected] = useState<string | null>(null);
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
        setCollected(collectedOn(res.patches ?? [], match?.window?.counted));
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

  const playRateDisplay = `${Math.round(stat.playRate * 100)}%`;

  return (
    <div className="inspect-stats" data-testid={INSPECT_STATS}>
      <div className="inspect-stats-header">
        <span className="inspect-stats-title">{t("Statistics")}</span>
        {/* R3447000: the patch the figures were collected on, the patch before while a newer one gathers its games */}
        {collected !== null && (
          <span className="inspect-stats-sample-hint" data-testid={statsTestid.cardStatsPatch}>
            {t("Collected on patch {patch}", { patch: collected })}
          </span>
        )}
      </div>

      <div className="inspect-stats-grid">
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Games")}</span>
          <span className="inspect-stats-metric-value">{stat.games}</span>
        </div>
        {/* R3185000: below the floor, or from a server without them, the tier and the adjusted rate read "—", as on /stats */}
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Tier")}</span>
          <span className="inspect-stats-metric-value">
            <TierChip tier={stat.hasEnoughGames ? stat.tier : null} testid={statsTestid.cardStatsTier} />
          </span>
        </div>
        <div className="inspect-stats-metric">
          <span className="inspect-stats-metric-label">{t("Adjusted win rate")}</span>
          <span className="inspect-stats-metric-value" data-testid={statsTestid.cardStatsAdjusted}>
            {stat.hasEnoughGames ? adjustedText(stat.adjustedWinRate) : "—"}
          </span>
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
          <span className="inspect-stats-metric-label">{t("Play rate")}</span>
          <span className="inspect-stats-metric-value">{playRateDisplay}</span>
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
          {t("View full stats")}
          <ArrowIcon direction="forward" />
        </a>
      </div>
    </div>
  );
}
