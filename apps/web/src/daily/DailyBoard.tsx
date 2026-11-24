// `/daily/:matchId` (#3727): one daily game's board, the ordinary `game/Game.tsx` fed from the server's read
// (`GET /api/daily/games/:matchId`: the seat's `view` and `legal`). No socket and no engine runs here: each
// action is one `POST …/actions` sent `after` the last action the board read, under a fresh nonce (the same
// nonce sent again applies once), and the board draws the answer. A refusal reads the game again: `stale`
// means it moved on meanwhile (the opponent, or this game in another tab), and `time_up`, `not_your_move`
// or `illegal` say so in the server's words over the game as it now stands.
//
// Only the player holding the move sends anything but a concede (§9.13: the server answers anything else
// `not_your_move`). "End turn automatically" (R345), which `Game.tsx` sends once whenever the setting and the
// view differ, whoever's turn it is, is held until the move is the player's and sent then if it still differs.
// It is held through the mulligan too, where both seats hold the move (R3727002): sent on the board's own, it would
// add a row to the game's log under the other seat's keep, which would answer `stale`. It goes out just before the
// player's own keep or mulligan instead, so it is in the game before the mulligan's last answer starts a turn (#3841).
//
// The clock pill (`DailyClockPill.tsx`) says who holds the move and how long it has left. Once the move has
// passed on and another game waits on the player (`nextGame`), Next game opens it. The board reads again
// when the social read says the games waiting on the player changed (the opponent moved), and every
// `DAILY_REREAD_MS` while the tab shows and when it shows again, which shows an end the count does not
// (R3840000). Once the game is over, Rematch sends its opponent a new daily challenge at the same mode and time per move (§9.13).
// An opponent whose account was deleted after the game reads "a deleted account" and gets no Rematch (R4621000).
// A KY's Competition question shows no 60-second countdown here (R3727003): the move's deadline is its window.
//
// The board shows what every table shows (R3855000): its hero names the opponent, its rail lists the modifiers
// in play (R169), the seat's forecast of its attacks rides the read and draws an aimed attack's outcomes
// (R3477000), and a finished game's Watch the replay opens the server's replay of it.
//
// A finished game whose moves the retention purge took reads `expired` (R3934000): no view, so no board, only its
// result with both heroes' life, when it ended, a line saying its moves are no longer kept and Back to daily
// games. It offers no replay (the log is gone) and no Rematch, and it is read no more.
//
// CLAUDE.md rule 7: who holds the move, its deadline, what is legal and every refusal are the server's.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import type { ActionBody } from "@jackioh/shared";

import Game from "../game/Game.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { ModifierList } from "../game/ModifierList.tsx";
import { pollWhileVisible } from "../game/net.ts";
import { outcomeFor } from "../game/Result.tsx";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { Loading, ShellPanel } from "../main.tsx";
import { ApiRequestError } from "../net/api.ts";
import { getDailyGame, sendDailyAction, type DailyExpiredGame, type DailyGame } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { withSession } from "../net/gate.ts";
import { navigate, paths } from "../net/navigate.ts";
import OpenedReplay from "../replay/OpenedReplay.tsx";
import { openOnlineMatchReplay } from "../replay/sources.ts";
import { BackLink } from "../routes/nav.tsx";
import { playerName } from "../routes/replayWords.ts";
import { useCatalogLookup } from "../together/catalog.ts";
import { useSocialReads } from "../together/socialPoll.ts";
import { DailyClockPill, resultLine, useNow } from "./DailyClockPill.tsx";
import { DAILY_REREAD_MS } from "./config.ts";
import { NewDailyDialog } from "./NewDailyDialog.tsx";
import { dailyTestid } from "./testids.ts";

import "./daily.css";

/** The words a finished game's result says when the move's time ran out (R3727000), else none. */
export function onTimeWords(game: DailyGame): string | undefined {
  const { result, viewer } = game.view;
  if (!game.lostOnTime || result === null) return undefined;
  return outcomeFor(result, viewer) === "win" ? t("Won on time") : t("Lost on time");
}

type AutoEndTurn = Extract<ActionBody, { type: "setAutoEndTurn" }>;

/** A fresh nonce per action: the server applies an action sent twice under one nonce once. */
function nonce(): string {
  return crypto.randomUUID();
}

/** R3934000: a finished game whose moves are no longer kept, as its result: the list's row, from the stored board. */
function ExpiredGame({ game }: { game: DailyExpiredGame }): ReactElement {
  return (
    <ShellPanel testId={dailyTestid.expired}>
      <h2 data-testid={dailyTestid.expiredResult} data-outcome={game.result?.outcome ?? "none"}>
        {resultLine(game)}
      </h2>
      <p>
        {t("vs {opponent}", { opponent: playerName(game.opponent.username) })} · {t("Turn {n}", { n: game.turn })}
      </p>
      <p>{t("You {me} – {them} Opponent", { me: game.life.me, them: game.life.them })}</p>
      {game.endedAt === null ? null : <p>{t("Ended {when}", { when: formatDateTime(game.endedAt) })}</p>}
      <p className="notice" role="note">
        {t("This game's moves are no longer kept, so it can't be shown or replayed.")}
      </p>
      <div className="row">
        <button type="button" className="button-primary" data-testid={dailyTestid.backToList} data-route={paths.daily} onClick={() => navigate(paths.daily)}>
          {t("Back to daily games")}
        </button>
      </div>
    </ShellPanel>
  );
}

export default function DailyBoard({ matchId }: { matchId: string }): ReactElement {
  useLocale();
  const lookup = useCatalogLookup();
  const now = useNow();
  const [game, setGame] = useState<DailyGame | null>(null);
  const [expired, setExpired] = useState<DailyExpiredGame | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reads, setReads] = useState(0);
  const [rematching, setRematching] = useState(false);
  const [watching, setWatching] = useState(false);
  const current = useRef<DailyGame | null>(null);
  current.current = game;
  const sending = useRef(false);
  const [sends, setSends] = useState(0);
  const heldAutoEndTurn = useRef<AutoEndTurn | null>(null);
  const { dailyWaiting } = useSocialReads();

  const reread = useCallback((): void => {
    setReads((before) => before + 1);
  }, []);

  useEffect(() => {
    let canceled = false;
    attempt(() => getDailyGame(matchId)).then(
      (answer) => {
        if (canceled) return;
        // R3934000: the board is gone with the moves, so the slow poll, which reads a game not over, has none to read.
        if ("expired" in answer) {
          setExpired(answer);
          setGame(null);
        } else {
          setGame(answer);
          setExpired(null);
        }
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

  // R3840000: a game can end on the opponent's move, or while the move is the player's, with the count of games waiting on the player unchanged,
  // so the board also reads on a slow period and when the tab shows again (R3205000: a hidden tab reads nothing). Nothing changes once over.
  useEffect(
    () =>
      pollWhileVisible(() => {
        const shown = current.current;
        if (shown !== null && !shown.over && !sending.current) reread();
      }, DAILY_REREAD_MS),
    [reread],
  );

  const send = useCallback(
    (action: ActionBody): void => {
      const shown = current.current;
      if (action.type === "setAutoEndTurn" && (shown === null || !shown.hasMove || shown.view.phase === "mulligan" || sending.current)) {
        heldAutoEndTurn.current = action;
        return;
      }
      // One action at a time: the next is sent after the answer, which names the seq it follows.
      if (shown === null || sending.current) return;
      // #3841: the held setting goes just before the player's own mulligan answer, so it is in the game before the
      // mulligan's last answer can start a turn that ends itself, and adds no window the answer does not open.
      const held = action.type === "mulligan" && shown.view.phase === "mulligan" ? heldAutoEndTurn.current : null;
      const setting = held !== null && (shown.view.autoEndTurn !== false) !== held.enabled ? held : null;
      if (held !== null) heldAutoEndTurn.current = null;
      sending.current = true;
      setError(null);
      let after = shown.seq;
      let settingSent = setting === null;
      attempt(async () => {
        if (setting !== null) {
          const answer = await sendDailyAction(matchId, { after, nonce: nonce(), action: setting });
          setGame(answer);
          after = answer.seq;
          settingSent = true;
        }
        return sendDailyAction(matchId, { after, nonce: nonce(), action });
      })
        .then(
          (answer) => {
            setGame(answer);
          },
          (cause: unknown) => {
            // The setting did not land, so the next answer's board sends it again; the mulligan was never sent.
            if (!settingSent) heldAutoEndTurn.current = setting;
            // Behind: the game moved on meanwhile, so the board reads it and says nothing.
            if (!(cause instanceof ApiRequestError && cause.status === 409 && cause.code === "stale")) setError(messageOf(cause));
            if (cause instanceof ApiRequestError) reread();
          },
        )
        .finally(() => {
          sending.current = false;
          setSends((before) => before + 1);
        });
    },
    [matchId, reread],
  );

  // The held "End turn automatically", once the move is the player's and nothing is in flight.
  useEffect(() => {
    const held = heldAutoEndTurn.current;
    if (held === null || game === null || game.over || !game.hasMove || game.view.phase === "mulligan" || sending.current) return;
    heldAutoEndTurn.current = null;
    if ((game.view.autoEndTurn !== false) !== held.enabled) send(held);
  }, [game, sends, send]);

  if (expired !== null) return <ExpiredGame game={expired} />;

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

  const opponent = playerName(game.opponent.username);
  // R4621000: an account deleted after the game ended has no account to challenge.
  const opponentGone = game.opponent.username === "";

  if (watching && game.view.result !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <OpenedReplay
          load={() => withSession((token) => openOnlineMatchReplay(token, game.matchId))}
          opponentLabel={opponent}
          onLeave={() => {
            setWatching(false);
          }}
        />
      </CatalogContext.Provider>
    );
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
          forecast={game.forecast}
          enemyLabel={opponent}
          onAction={send}
          error={error}
          wearCosmetics
          trackStats
          endTurnLabel={t("End turn and send")}
          sequenceClock={false}
          resultReason={onTimeWords(game)}
          turnClock={game.over ? undefined : <DailyClockPill holds={game.holds} seat={game.seat} now={now} />}
          phoneRailTop
          boardRail={
            <>
              <BackLink to={paths.daily} label="Back to daily games" showSettings={false} />
              {/* R169: every live modifier in full, one tap away, as every other table's rail lists them. */}
              <ModifierList view={game.view} opponentLabel={t("Opponent")} />
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
              {opponentGone ? null : (
                <button
                  type="button"
                  data-testid={dailyTestid.rematch}
                  onClick={() => {
                    setRematching(true);
                  }}
                >
                  {t("Rematch")}
                </button>
              )}
              {game.view.result?.reason === "voided" ? null : (
                <button
                  type="button"
                  data-testid={dailyTestid.watchReplay}
                  onClick={() => {
                    setWatching(true);
                  }}
                >
                  {t("Watch the replay")}
                </button>
              )}
              <button type="button" data-testid={dailyTestid.backToList} data-route={paths.daily} onClick={() => navigate(paths.daily)}>
                {t("Back to daily games")}
              </button>
            </>
          }
        />
      </CatalogContext.Provider>
      {rematching ? (
        <NewDailyDialog
          rematch={{ opponent: game.opponent, mode: game.mode, perMoveDays: game.perMoveDays }}
          onClose={() => {
            setRematching(false);
          }}
          onDone={() => {
            setRematching(false);
            navigate(paths.daily);
          }}
        />
      ) : null}
    </div>
  );
}
