// Conquest series banner renders server-supplied data (SPEC §9.5; R330, R332, R336).
// Poll while the series is active because the server may advance its next pick or game.
// It never enforces rules (CLAUDE.md rule 7).

import { useEffect, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";
import { getSeriesForMatch, type SeriesView } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "./nav.tsx";
import "./lobby.css";

const MS_PER_SECOND = 1000;

/** `e2e/support/testids.ts` mirrors these test IDs. */
export const seriesBannerTestid = {
  banner: "series-banner",
  continue: "series-banner-continue",
  result: "series-banner-result",
  panelContinue: "result-series-continue",
  yourDeck: (slot: number): string => `series-banner-you-deck-${String(slot)}`,
  opponentDeck: (slot: number): string => `series-banner-opponent-deck-${String(slot)}`,
} as const;

function wonWords(decks: readonly { won: boolean }[]): string {
  const won = decks.filter((deck) => deck.won).length;
  return t("{won} of {total} decks have won", { won: String(won), total: String(decks.length) });
}

type SeriesResult = NonNullable<SeriesView["result"]>;

export const SERIES_OUTCOME_HEADLINE: Readonly<Record<SeriesResult["outcome"], string>> = {
  win: "You won the series",
  loss: "You lost the series",
  draw: "The series is a draw",
  abandoned: "The series was called off",
};

/** Defer calls so synchronous test throws become promise rejections. */
function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}

function seriesOf(answer: unknown): SeriesView | null {
  if (typeof answer !== "object" || answer === null) return null;
  const series = (answer as { series?: unknown }).series;
  if (typeof series !== "object" || series === null) return null;
  return typeof (series as { id?: unknown }).id === "string" ? (series as SeriesView) : null;
}

/** Poll on mount and, once a game ends, while its series continues; ignore optional-banner failures. */
export function useMatchSeries(token: string, matchId: string, gameOver: boolean): SeriesView | null {
  const [series, setSeries] = useState<SeriesView | null>(null);
  const [notSeries, setNotSeries] = useState(false);
  const poll = gameOver && series !== null && series.status !== "over";

  useEffect(() => {
    if (notSeries) return;
    let cancelled = false;
    const read = (): void => {
      attempt(() => getSeriesForMatch(token, matchId)).then(
        (answer) => {
          if (cancelled) return;
          const found = seriesOf(answer);
          if (found === null) setNotSeries(true);
          else setSeries(found);
        },
        () => undefined,
      );
    };
    read();
    if (!poll) {
      return () => {
        cancelled = true;
      };
    }
    const handle = setInterval(read, SERIES_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [token, matchId, gameOver, poll, notSeries]);

  return notSeries ? null : series;
}

export function nextStep(series: SeriesView, matchId: string): { href: string; label: string } | null {
  if (series.status === "over") return null;
  const next = t("Continue to game {game}", { game: String(series.gameNo) });
  if (series.status === "picking") return { href: paths.series(series.id), label: next };
  const running = series.currentMatchId;
  if (running !== null && running !== matchId) return { href: paths.match(running), label: next };
  // Before the next game exists, return to the series pick.
  return { href: paths.series(series.id), label: t("Back to the series") };
}

function WayOn({ series, matchId, testid }: { series: SeriesView; matchId: string; testid: string }): ReactElement {
  const step = nextStep(series, matchId);
  if (step === null) {
    return (
      <a href={paths.series(series.id)} data-testid={testid} onClick={followInApp(paths.series(series.id))}>
        {t("See the series")}
      </a>
    );
  }
  return (
    <a
      className="series-banner__continue"
      href={step.href}
      data-testid={testid}
      onClick={followInApp(step.href)}
    >
      {step.label}
    </a>
  );
}

export type SeriesBannerProps = { series: SeriesView | null; matchId: string; gameOver: boolean };

export function SeriesBanner({ series, matchId, gameOver }: SeriesBannerProps): ReactElement | null {
  if (series === null) return null;
  const result = series.result;
  const yours = [...series.you.decks].sort((a, b) => a.slot - b.slot);
  const theirs = [...series.opponent.decks].sort((a, b) => a.slot - b.slot);
  return (
    <div className="series-banner" data-testid={seriesBannerTestid.banner} data-series-id={series.id}>
      <span className="series-banner__score">
        {t("Conquest · You {you} – {them} Opponent", {
          you: String(series.you.wins),
          them: String(series.opponent.wins),
        })}
      </span>
      <span className="series-banner__decks" aria-label={t("Your decks: {standing}", { standing: wonWords(yours) })}>
        {t("You")}
        {yours.map((deck) => (
          <span
            key={deck.slot}
            className="series-banner__pip"
            data-testid={seriesBannerTestid.yourDeck(deck.slot)}
            data-won={deck.won ? "true" : "false"}
            title={`${deck.name}: ${deck.won ? t("won") : t("not won yet")}`}
          />
        ))}
      </span>
      <span className="series-banner__decks" aria-label={t("Their decks: {standing}", { standing: wonWords(theirs) })}>
        {t("Them")}
        {theirs.map((deck) => (
          <span
            key={deck.slot}
            className="series-banner__pip"
            data-testid={seriesBannerTestid.opponentDeck(deck.slot)}
            data-won={deck.won ? "true" : "false"}
            title={t(deck.won ? "Deck {slot}: won" : "Deck {slot}: not won yet", { slot: String(deck.slot + 1) })}
          />
        ))}
      </span>
      {gameOver && result !== null ? (
        <span data-testid={seriesBannerTestid.result} data-outcome={result.outcome}>
          {t(SERIES_OUTCOME_HEADLINE[result.outcome])}
        </span>
      ) : null}
      {gameOver ? <WayOn series={series} matchId={matchId} testid={seriesBannerTestid.continue} /> : null}
    </div>
  );
}

export function SeriesContinue({ series, matchId }: { series: SeriesView | null; matchId: string }): ReactElement | null {
  if (series === null) return null;
  return <WayOn series={series} matchId={matchId} testid={seriesBannerTestid.panelContinue} />;
}
