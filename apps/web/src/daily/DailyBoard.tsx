// `/daily/:matchId` (#3727): one daily game's board, the ordinary `game/Game.tsx` fed from the server's read
// (`GET /api/daily/games/:matchId`: the seat's `view` and `legal`). No socket and no engine runs here: each
// action is one `POST …/actions` sent `after` the last action the board read, under a fresh nonce (the same
// nonce sent again applies once), and the board draws the answer. A refusal reads the game again: `stale`
// means it moved on meanwhile (the opponent, or this game in another tab), and `time_up`, `not_your_move`
// or `illegal` say so in the server's words over the game as it now stands.
//
// The clock pill (`DailyClockPill.tsx`) says who holds the move and how long it has left. Once the move has
// passed on and another game waits on the player (`nextGame`), Next game opens it. The board reads again
// when the social read says the games waiting on the player changed (the opponent moved).
//
// CLAUDE.md rule 7: who holds the move, its deadline, what is legal and every refusal are the server's.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import type { ActionBody } from "@jackioh/shared";

import Game from "../game/Game.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { outcomeFor } from "../game/Result.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { Loading, ShellPanel } from "../main.tsx";
import { ApiRequestError } from "../net/api.ts";
import { getDailyGame, sendDailyAction, type DailyGame } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { navigate, paths } from "../net/navigate.ts";
import { BackLink } from "../routes/nav.tsx";
import { useCatalogLookup } from "../together/catalog.ts";
import { useSocialReads } from "../together/socialPoll.ts";
import { DailyClockPill, useNow } from "./DailyClockPill.tsx";
import { dailyTestid } from "./testids.ts";

import "./daily.css";

/** The words a finished game's result says when the move's time ran out (R3727000), else none. */
export function onTimeWords(game: DailyGame): string | undefined {
  const { result, viewer } = game.view;
  if (!game.lostOnTime || result === null) return undefined;
  return outcomeFor(result, viewer) === "win" ? t("Won on time") : t("Lost on time");
}

/** A fresh nonce per action: the server applies an action sent twice under one nonce once. */
function nonce(): string {
  return crypto.randomUUID();
}

export default function DailyBoard({ matchId }: { matchId: string }): ReactElement {
  useLocale();
  const lookup = useCatalogLookup();
  const now = useNow();
  const [game, setGame] = useState<DailyGame | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reads, setReads] = useState(0);
  const current = useRef<DailyGame | null>(null);
  current.current = game;
  const sending = useRef(false);
  const { dailyWaiting } = useSocialReads();

  const reread = useCallback((): void => {
    setReads((before) => before + 1);
  }, []);

  useEffect(() => {
    let canceled = false;
    attempt(() => getDailyGame(matchId)).then(
      (answer) => {
        if (canceled) return;
        setGame(answer);
        setLoadError(null);
      },
      (cause: unknown) => {
        if (!canceled) setLoadError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [matchId, reads]);

  // The games waiting on the player changed: the opponent may have moved here.
  const waitingSeen = useRef(dailyWaiting);
  useEffect(() => {
    if (waitingSeen.current === dailyWaiting) return;
    waitingSeen.current = dailyWaiting;
    if (current.current !== null && !current.current.hasMove) reread();
  }, [dailyWaiting, reread]);

  const send = useCallback(
    (action: ActionBody): void => {
      const shown = current.current;
      // One action at a time: the next is sent after the answer, which names the seq it follows.
      if (shown === null || sending.current) return;
      sending.current = true;
      setError(null);
      attempt(() => sendDailyAction(matchId, { after: shown.seq, nonce: nonce(), action }))
        .then(
          (answer) => {
            setGame(answer);
          },
          (cause: unknown) => {
            // Behind: the game moved on meanwhile, so the board reads it and says nothing.
            if (!(cause instanceof ApiRequestError && cause.status === 409 && cause.code === "stale")) setError(messageOf(cause));
            if (cause instanceof ApiRequestError) reread();
          },
        )
        .finally(() => {
          sending.current = false;
        });
    },
    [matchId, reread],
  );

  if (game === null) {
    if (loadError !== null) {
      return (
        <ShellPanel testId={dailyTestid.board}>
          <p className="notice" data-testid={dailyTestid.boardError} role="alert">
            {t("This daily game didn't load: {message}", { message: t(loadError) })}
          </p>
          <div className="row">
            <button type="button" className="button-primary" data-testid={dailyTestid.boardRetry} onClick={reread}>
              {t("Try again")}
            </button>
            <button type="button" data-testid={dailyTestid.backToList} data-route={paths.daily} onClick={() => navigate(paths.daily)}>
              {t("Back to daily games")}
            </button>
          </div>
        </ShellPanel>
      );
    }
    return <Loading what="Loading your game…" slow skeleton="panel" />;
  }

  const next = game.nextGame;
  const nextButton =
    !game.hasMove && next !== null ? (
      <button
        type="button"
        className="button-primary"
        data-testid={dailyTestid.nextGame}
        data-route={paths.dailyGame(next)}
        onClick={() => {
          navigate(paths.dailyGame(next));
        }}
      >
        {t("Next game")}
      </button>
    ) : null;

  return (
    <div
      className="app-shell app-shell--wide daily-board"
      data-testid={dailyTestid.board}
      data-match-id={game.matchId}
      data-has-move={game.hasMove ? "true" : "false"}
    >
      <CatalogContext.Provider value={lookup}>
        <Game
          view={game.view}
          legal={game.legal}
          onAction={send}
          error={error}
          wearCosmetics
          trackStats
          endTurnLabel={t("End turn and send")}
          resultReason={onTimeWords(game)}
          turnClock={game.over ? undefined : <DailyClockPill holds={game.holds} seat={game.seat} now={now} />}
          phoneRailTop
          boardRail={<BackLink to={paths.daily} label="Back to daily games" showSettings={false} />}
          boardNotice={
            <>
              {game.olderCatalog ? (
                <p className="daily-board-notice" data-testid={dailyTestid.olderCatalog} role="note">
                  {t("This game plays on the card version it started on, an older one than today's.")}
                </p>
              ) : null}
              {game.over ? null : nextButton}
            </>
          }
          resultActions={
            <>
              {nextButton}
              <button type="button" data-testid={dailyTestid.backToList} data-route={paths.daily} onClick={() => navigate(paths.daily)}>
                {t("Back to daily games")}
              </button>
            </>
          }
        />
      </CatalogContext.Provider>
    </div>
  );
}
