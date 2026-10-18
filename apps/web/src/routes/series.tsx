// `/series/<id>` renders only this player's server `SeriesView` (SPEC §9.5; CLAUDE.md rule 7).
// Won decks lock (R330); hidden picks stay hidden (R331, R336), and the server decides valid picks (R338).
// Rebase `pickDeadline` from server `now` at view arrival (R333); open each newly running game once.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "@jackioh/server-config";
import { useSecondsUntil } from "../auth/cooldown.ts";
import { t } from "../i18n/i18n.ts";
import { pageWake } from "../game/net.ts";
import { ApiRequestError, forfeitSeries, getSeries, pickSeriesDeck, type SeriesView } from "../net/api.ts";
import { navigate, paths } from "../net/navigate.ts";
import { BackLink, followInApp } from "./nav.tsx";
import ConquestExplainer, { conquestExplainerTestid } from "./ConquestExplainer.tsx";
import { conquestText, isExplainerDismissed, setExplainerDismissed } from "./conquestCopy.ts";
import { SERIES_OUTCOME_HEADLINE } from "./SeriesBanner.tsx";
import SeriesPicker, { deckStanding, seriesPickerTestid } from "./SeriesPicker.tsx";
import "../auth/tavern.css";
import "./lobby.css";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

/** Screen IDs; `e2e/support/testids.ts` mirrors their strings (R330, R331, R333, R336, R338). */
export const seriesTestid = {
  screen: "series-screen",
  loading: "series-loading",
  error: "series-error",
  score: "series-score",
  deck: (slot: number): string => `series-deck-${String(slot)}`,
  picker: seriesPickerTestid.picker,
  pick: seriesPickerTestid.choice,
  lockIn: seriesPickerTestid.lockIn,
  opponentDeck: (slot: number): string => `series-opponent-deck-${String(slot)}`,
  clock: seriesPickerTestid.clock,
  opponentStatus: seriesPickerTestid.opponentStatus,
  openMatch: "series-open-match",
  forfeit: "series-forfeit",
  forfeitConfirm: "series-forfeit-confirm",
  forfeitCancel: "series-forfeit-cancel",
  result: "series-result",
  game: (gameNo: number): string => `series-game-${String(gameNo)}`,
  backToPlay: "series-back-to-play",
  explainer: conquestExplainerTestid.explainer,
  explainerDismiss: conquestExplainerTestid.dismiss,
  explainerToggle: "conquest-explainer-toggle",
  endReminder: "series-end-reminder",
} as const;

type SeriesResult = NonNullable<SeriesView["result"]>;

const GAME_RESULT_WORD: Readonly<Record<"win" | "loss" | "draw", string>> = {
  win: "Win",
  loss: "Loss",
  draw: "Draw",
};

/** R259, R330, R333, R334, R337: preserve a pre-Conquest result below `winsNeeded`. */
export function endReasonWords(
  result: SeriesResult,
  winsNeeded: number,
  maxGames: number,
  wins?: { you: number; opponent: number },
): string {
  switch (result.endReason) {
    case "decided": {
      const winner = wins === undefined ? winsNeeded : result.outcome === "win" ? wins.you : wins.opponent;
      if (winner < winsNeeded) {
        return result.outcome === "win"
          ? t("You reached {wins} game wins, which took the series under the Best-of-3 rules it began with.", {
              wins: String(winner),
            })
          : t("Your opponent reached {wins} game wins, which took the series under the Best-of-3 rules it began with.", {
              wins: String(winner),
            });
      }
      return result.outcome === "win"
        ? t("You won a game with each of your {count} decks.", { count: String(winsNeeded) })
        : t("Your opponent won a game with each of their {count} decks.", { count: String(winsNeeded) });
    }
    case "exhausted":
      if (result.outcome === "draw") {
        return t("The series reached its {max}-game limit with the wins level.", { max: String(maxGames) });
      }
      return result.outcome === "win"
        ? t("The series reached its {max}-game limit, and you had won more games.", { max: String(maxGames) })
        : t("The series reached its {max}-game limit, and your opponent had won more games.", { max: String(maxGames) });
    case "forfeit":
      return result.outcome === "win" ? t("Your opponent forfeited the series.") : t("You forfeited the series.");
    case "abandoned":
      return t("The series ended without a winner: nobody picked a deck in time, or a game could not be started.");
  }
}

/** R262, R604, R612: the server decides the ranked result; no rating value reaches this view. */
export function ratingWords(result: SeriesResult): string {
  if (result.outcome === "abandoned" || !result.ranked) {
    return t("Unrated: no rating changed.");
  }
  return t("Ranked series: counted as one rated game.");
}

function messageOf(cause: unknown): string {
  if (cause instanceof ApiRequestError) return cause.message;
  return cause instanceof Error ? cause.message : String(cause);
}

/** Normalize synchronous test stubs as promises. */
function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}

type Received = { view: SeriesView; receivedAt: number };

export function localDeadline(received: Received): number | null {
  const { view, receivedAt } = received;
  if (view.status !== "picking" || view.pickDeadline === null) return null;
  return receivedAt + (view.pickDeadline - view.now);
}

function SeriesHero(): ReactElement {
  return (
    <header className="play-hero">
      <div className="brand">
        <h1>JackiOh</h1>
      </div>
      <p className="play-hero__title">{t("Conquest")}</p>
      <p className="play-hero__lead">{t("Win a game with each of your three decks. A deck that wins is locked.")}</p>
    </header>
  );
}

export type SeriesRouteProps = { seriesId: string; token: string };

export default function SeriesRoute({ seriesId, token }: SeriesRouteProps): ReactElement {
  const [received, setReceived] = useState<Received | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [showExplainer, setShowExplainer] = useState(() => !isExplainerDismissed());

  function onDismissExplainer(): void {
    setExplainerDismissed(true);
    setShowExplainer(false);
  }

  // Request order keeps a stale poll from undoing a pick.
  const sent = useRef(0);
  const shown = useRef(0);
  const inFlight = useRef(false);
  const accept = useCallback((view: SeriesView, request: number) => {
    if (request < shown.current) return;
    shown.current = request;
    setReceived({ view, receivedAt: Date.now() });
  }, []);

  const over = received?.view.status === "over";
  useEffect(() => {
    if (over) return;
    let cancelled = false;
    // A wake while a read is out would only repeat it; each effect run counts its own.
    let reading = false;
    const read = (): void => {
      // Let an action's fresher response win over a poll sent before its write.
      if (inFlight.current || reading) return;
      reading = true;
      sent.current += 1;
      const request = sent.current;
      attempt(() => getSeries(token, seriesId))
        .then(
          (view) => {
            if (cancelled) return;
            accept(view, request);
            setLoadError(null);
          },
          (cause: unknown) => {
            if (!cancelled) setLoadError(messageOf(cause));
          },
        )
        .finally(() => {
          reading = false;
        });
    };
    read();
    const handle = setInterval(read, SERIES_POLL_SECONDS * MS_PER_SECOND);
    // A throttled tab's interval stalls; waking it reads the series again at once.
    const stopWake = pageWake(read);
    return () => {
      cancelled = true;
      stopWake();
      clearInterval(handle);
    };
  }, [token, seriesId, over, accept]);

  // The first view may be an intentional return from the board; only open new games.
  const lastRunning = useRef<string | null | undefined>(undefined);
  const opened = useRef(new Set<string>());
  const view = received?.view ?? null;
  useEffect(() => {
    if (view === null) return;
    const running = view.status === "playing" ? view.currentMatchId : null;
    const before = lastRunning.current;
    lastRunning.current = running;
    if (before === undefined || running === null || running === before || opened.current.has(running)) return;
    opened.current.add(running);
    navigate(paths.match(running));
  }, [view]);

  const deadline = received === null ? null : localDeadline(received);
  const secondsLeft = useSecondsUntil(deadline);

  function act(work: () => Promise<SeriesView>): void {
    if (busy) return;
    inFlight.current = true;
    setBusy(true);
    setActionError(null);
    sent.current += 1;
    const request = sent.current;
    attempt(work)
      .then(
        (next) => {
          accept(next, request);
        },
        (cause: unknown) => {
          setActionError(messageOf(cause));
        },
      )
      .finally(() => {
        inFlight.current = false;
        setBusy(false);
      });
  }

  function onPick(slot: number, gameNo: number): void {
    act(() => pickSeriesDeck(token, seriesId, slot, gameNo));
  }

  function onForfeit(): void {
    setConfirming(false);
    act(() => forfeitSeries(token, seriesId));
  }

  if (view === null) {
    return (
      <div className="app-shell tavern lobby play-screen series" data-testid={seriesTestid.screen}>
        <BackLink to={paths.play} />
        <SeriesHero />
        {loadError === null ? (
          <p className="notice" data-testid={seriesTestid.loading} role="status">
            {t("Loading the series…")}
          </p>
        ) : (
          <p className="notice" data-testid={seriesTestid.error} role="alert">
            {t(loadError)}
          </p>
        )}
      </div>
    );
  }

  const { you, opponent } = view;
  const picking = view.status === "picking";
  const yourDecks = [...you.decks].sort((a, b) => a.slot - b.slot);
  const theirDecks = [...opponent.decks].sort((a, b) => a.slot - b.slot);
  const deckName = (slot: number): string =>
    you.decks.find((deck) => deck.slot === slot)?.name ?? t("Deck {number}", { number: String(slot + 1) });

  return (
    <div className="app-shell tavern lobby play-screen series" data-testid={seriesTestid.screen} data-status={view.status}>
      <BackLink to={paths.play} />
      <SeriesHero />

      {showExplainer ? (
        <ConquestExplainer onDismiss={onDismissExplainer} />
      ) : (
        <button
          type="button"
          className="link-button series-explainer-toggle"
          data-testid={seriesTestid.explainerToggle}
          onClick={() => {
            setShowExplainer(true);
          }}
        >
          {conquestText("toggleOpen")}
        </button>
      )}

      {actionError === null ? null : (
        <p className="notice" data-testid={seriesTestid.error} role="alert">
          {t(actionError)}
        </p>
      )}

      <div className="play-layout">
        <div className="play-side">
          {picking ? (
            <SeriesPicker
              view={view}
              secondsLeft={secondsLeft}
              busy={busy}
              onLockIn={(slot) => {
                onPick(slot, view.gameNo);
              }}
            />
          ) : null}

          {view.status === "playing" && view.currentMatchId !== null ? (
            <section className="lobby-card play-panel series-panel" aria-labelledby="series-game-heading">
              <h2 id="series-game-heading" className="play-panel__heading">
                {t("Game {game} is on", { game: String(view.gameNo) })}
              </h2>
              <a
                className="button-primary play-cta series-open"
                href={paths.match(view.currentMatchId)}
                data-testid={seriesTestid.openMatch}
                onClick={followInApp(paths.match(view.currentMatchId))}
              >
                {t("Open game {game}", { game: String(view.gameNo) })}
              </a>
              {/* R334: a game is conceded on the board; the series can only be forfeited between games. */}
              <p className="lobby-note">{t("To give up this game, concede it on the board.")}</p>
            </section>
          ) : null}

          {view.result === null ? null : (
            <section
              className="lobby-card play-panel series-panel series-result"
              data-testid={seriesTestid.result}
              data-outcome={view.result.outcome}
            >
              <span className="series-result__word">{t(SERIES_OUTCOME_HEADLINE[view.result.outcome])}</span>
              <p>{endReasonWords(view.result, view.winsNeeded, view.maxGames, { you: you.wins, opponent: opponent.wins })}</p>
              <p>{ratingWords(view.result)}</p>
              {view.result.ranked && view.result.outcome !== "abandoned" ? (
                <p className="lobby-note" data-testid={seriesTestid.endReminder}>
                  {conquestText("seriesEndRatingReminder")}
                </p>
              ) : null}
              <a
                className="button-primary play-cta"
                href={paths.play}
                data-testid={seriesTestid.backToPlay}
                onClick={followInApp(paths.play)}
              >
                {t("Back to the lobby")}
              </a>
            </section>
          )}
        </div>

        <div className="play-side">
          <section className="lobby-card play-panel series-panel" aria-labelledby="series-standing-heading">
            <h2 id="series-standing-heading" className="play-panel__heading">
              {t("The series")}
            </h2>
            <p
              className="series-score"
              data-testid={seriesTestid.score}
              data-you={you.wins}
              data-opponent={opponent.wins}
            >
              <span>{t("You {you} – {them} Opponent", { you: String(you.wins), them: String(opponent.wins) })}</span>
              <span className="series-score__first">
                {t("Win with all {count} decks · game {game} of at most {max}", {
                  count: String(view.winsNeeded),
                  game: String(view.gameNo),
                  max: String(view.maxGames),
                })}
              </span>
            </p>

            <h3 className="series-panel__label">
              {t("Your decks")} · {you.trioName}
            </h3>
            <ul className="play-trio-decks" aria-label={t("Your decks")}>
              {yourDecks.map((deck) => {
                const isPick = you.pick === deck.slot;
                return (
                  <li
                    key={deck.slot}
                    className="play-trio-deck series-deck"
                    data-testid={seriesTestid.deck(deck.slot)}
                    data-won={deck.won ? "true" : "false"}
                    data-picked={isPick ? "true" : "false"}
                    title={deckStanding(deck)}
                  >
                    <span className="series-deck__name">{deck.name}</span>
                    <span className="series-deck__standing">{deck.won ? t("won · locked") : isPick ? t("your pick") : deckStanding(deck).toLowerCase()}</span>
                  </li>
                );
              })}
            </ul>
            <p className="lobby-note">{t("A deck that wins a game is locked for the rest of the series.")}</p>

            <h3 className="series-panel__label">{t("Their decks")}</h3>
            <ul className="play-trio-decks" aria-label={t("Your opponent's decks")}>
              {theirDecks.map((deck) => (
                <li
                  key={deck.slot}
                  className="play-trio-deck series-deck"
                  data-testid={seriesTestid.opponentDeck(deck.slot)}
                  data-won={deck.won ? "true" : "false"}
                >
                  <span className="series-deck__name">{t("Deck {number}", { number: String(deck.slot + 1) })}</span>
                  <span className="series-deck__standing">{deck.won ? t("won · locked") : t("not won yet")}</span>
                </li>
              ))}
            </ul>
          </section>

          {view.games.length === 0 ? null : (
            <section className="lobby-card play-panel series-panel" aria-labelledby="series-games-heading">
              <h2 id="series-games-heading" className="play-panel__heading play-panel__heading--sub">
                {t("Games")}
              </h2>
              <ol className="series-history">
                {view.games.map((game) => (
                  <li key={game.gameNo} data-testid={seriesTestid.game(game.gameNo)} data-result={game.result ?? "pending"}>
                    {t("Game {game}: {deck} vs their deck {theirs} —", {
                      game: String(game.gameNo),
                      deck: deckName(game.yourSlot),
                      theirs: String(game.opponentSlot + 1),
                    })}{" "}
                    <span className="series-history__result">
                      {game.result === null ? t("in progress") : t(GAME_RESULT_WORD[game.result])}
                    </span>{" "}
                    · {game.youWentFirst ? t("you went first") : t("they went first")}
                  </li>
                ))}
              </ol>
            </section>
          )}

          {picking ? (
            <section className="lobby-card play-panel series-panel" aria-label={t("Leave the series")}>
              {confirming ? (
                <div className="row" role="alertdialog" aria-label={t("Forfeit the series?")}>
                  <p>
                    {view.ranked
                      ? t("Forfeit the series? Your opponent wins it, and your rating moves as for a loss.")
                      : t("Forfeit the series? Your opponent wins it.")}
                  </p>
                  <button type="button" data-testid={seriesTestid.forfeitConfirm} disabled={busy} onClick={onForfeit}>
                    {t("Forfeit")}
                  </button>
                  <button
                    type="button"
                    className="link-button"
                    data-testid={seriesTestid.forfeitCancel}
                    onClick={() => {
                      setConfirming(false);
                    }}
                  >
                    {t("Keep playing")}
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  className="link-button"
                  data-testid={seriesTestid.forfeit}
                  disabled={busy}
                  onClick={() => {
                    setConfirming(true);
                  }}
                >
                  {t("Forfeit the series")}
                </button>
              )}
            </section>
          ) : null}
        </div>
      </div>
    </div>
  );
}
