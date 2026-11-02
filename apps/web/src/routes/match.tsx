// `/match/<id>` — the networked board (BUILD M6-T4, SPEC §9.5): open the socket (`game/net.ts`),
// render whatever `PlayerView` came back through the hotseat's `Game.tsx`, and send clicks straight
// back out. CLAUDE.md rule 7: no rule is decided here, and nothing on screen comes from anywhere but
// the view.
//
// WHERE LEGALITY COMES FROM. The client never computes legality (BUILD M5-T2): the actor sends
// `legalActions` for this socket's own seat on the `view` frame (`ViewMessage` in
// `crates/server/src/actor/protocol.rs`). When it has never arrived (`net.ts` reports it) this file
// shows a loud notice, since prompts would still work and nothing else would: a silently dead board.
//
// THE CATALOG. A `CardView` carries no name (§10.8), and the specs find cards by name, so the board's
// `CatalogContext` is the catalog the client ships (`game/currentCatalog.ts`, §9.4, #1378), which asks
// the server nothing. A card the catalog lacks shows its `defId`, never a guessed name.

import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { ActionBody, PlayerId, PlayerView } from "@jackioh/shared";

import Clock, { turnKeyOf, useFrameFor } from "../game/Clock.tsx";
import ConfirmLeaveMatch from "../game/ConfirmLeaveMatch.tsx";
import { connectionWords } from "../game/connectionWords.ts";
import Game from "../game/Game.tsx";
import { ModifierList } from "../game/ModifierList.tsx";
import { OnceRevealed } from "../game/Result.tsx";
import { getAudioEngine } from "../audio/index.ts";
import { announceWallet } from "../economy/wallet.ts";
import XpReveal from "../economy/XpReveal.tsx";
import { useEmotes } from "../emotes/useEmotes.ts";
import { useSetting } from "../settings/index.ts";
import { CatalogContext } from "../game/catalog.ts";
import { currentCatalogLookup } from "../game/currentCatalog.ts";
import {
  installDevHandle,
  awaitsYou,
  remainingMs,
  useMatch,
  viewDerivedState,
  type ConnectionState,
  type SocketFactory,
} from "../game/net.ts";
import { Loading, SITE_NAME, ShellPanel, documentTitleFor } from "../main.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import Username from "../auth/Username.tsx";
import { getMatchRanks, type MatchRanksResponse } from "../net/api.ts";
import { guardLeaving, navigate, paths } from "../net/navigate.ts";
import OpenedReplay from "../replay/OpenedReplay.tsx";
import { openOnlineMatchReplay } from "../replay/sources.ts";
import { rankCaption, rankShortWords, rankWords } from "../rank/rank.ts";
import { MATCH_RANKS_READ_ATTEMPTS, MATCH_RANKS_RETRY_MS } from "../game/config.ts";
import RankCrest from "../rank/RankCrest.tsx";
import StreakFlame from "../rank/StreakFlame.tsx";
import { ShareWithFriend } from "../together/ShareReplay.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import RematchButtons, { RematchWatcher } from "./Rematch.tsx";
import { SeriesBanner, SeriesContinue, useMatchSeries } from "./SeriesBanner.tsx";

const DEV_ONLY = import.meta.env.MODE !== "production";

/** R79: the seat an open prompt waits on, as the view says it (`PlayerView.pending`), or null. */
export function promptHolderOf(pending: PlayerView["pending"], viewer: PlayerId): PlayerId | null {
  if (pending === null) return null;
  return pending.forYou ? viewer : pending.pendingFor;
}

/** Chrome this route invented. None of it is in `e2e/support/testids.ts`; see the hand-off report. */
export const matchTestid = {
  /** The connection line in a player's words; the raw state rides on its `data-connection`. */
  status: "match-status",
  missingLegal: "missing-legal-frame",
  /** The panel shown before the first `view` frame lands. */
  connecting: "match-connecting",
  refused: "match-refused",
  /** #1599, #1899: the sentence that stands in for a socket closed for good (a voided match). */
  gone: "match-gone",
  /** The button that retries at once while the connection line says Reconnecting…. */
  reconnect: "match-reconnect",
  /** Both seats' visible ranks and whether this game moves them (R604, R612). */
  ranks: "match-ranks",
  /** R2388000: one seat's rank before → after in a ranked match's result panel (`data-seat`). */
  rankMove: "match-rank-move",
  /** R768000: in a finished match's result panel, its replay from step 0. */
  watchReplay: "match-watch-replay",
  /** R768000: in a finished match's result panel, its last turn's GIF. */
  shareLastTurn: "match-share-last-turn",
  /** #936 B51: how many friends watch this match (`data-count`), shown while any do. */
  watchers: "match-watchers",
} as const;

function ConnectionLine({ state, onReconnect }: { state: ConnectionState; onReconnect: () => void }): ReactElement {
  return (
    <>
      <span data-testid={matchTestid.status} data-connection={state}>
        {connectionWords(state)}
      </span>
      {state === "reconnecting" ? (
        <>
          {" "}
          <button type="button" className="link-button" data-testid={matchTestid.reconnect} onClick={onReconnect}>
            {t("Reconnect now")}
          </button>
        </>
      ) : null}
    </>
  );
}

export type MatchRouteProps = {
  matchId: string;
  /** The bearer token the gate already resolved; this route never reads storage itself. */
  token: string;
  /** Test seam, passed straight to `net.ts`. Unused in the app. */
  socketFactory?: SocketFactory;
};

/**
 * The socket URL with its query stripped, for display only, so no credential an address carries
 * reaches the screen (the token travels as a subprotocol, R1379000). Returns the input unchanged if
 * it will not parse: a status line must not be the thing that throws.
 */
export function withoutToken(raw: string): string {
  try {
    const url = new URL(raw);
    url.search = "";
    return url.toString();
  } catch {
    return raw;
  }
}

/** A `GET /api/matches/:id/ranks` body, or null when the answer is not one. */
function asMatchRanks(value: unknown): MatchRanksResponse | null {
  if (typeof value !== "object" || value === null) return null;
  const { ranked, seats } = value as { ranked?: unknown; seats?: unknown };
  if (typeof ranked !== "boolean" || typeof seats !== "object" || seats === null) return null;
  for (const side of ["p1", "p2"] as const) {
    const seat = (seats as Record<string, unknown>)[side];
    if (typeof seat !== "object" || seat === null) return null;
    const { profileId, username, rank, rankBefore, rankAfter } = seat as {
      profileId?: unknown;
      username?: unknown;
      rank?: unknown;
      rankBefore?: unknown;
      rankAfter?: unknown;
    };
    if (typeof profileId !== "string" || typeof username !== "string") return null;
    if (typeof rank !== "object" || rank === null) return null;
    if (typeof (rank as { tier?: unknown }).tier !== "string") return null;
    for (const moved of [rankBefore, rankAfter]) {
      if (moved === undefined) continue;
      if (typeof moved !== "object" || moved === null || typeof (moved as { tier?: unknown }).tier !== "string") return null;
    }
  }
  return value as MatchRanksResponse;
}

/** R2388000: whether the read carries the rated record's move (written for both seats at once). */
function recorded(ranks: MatchRanksResponse): boolean {
  return ranks.seats.p1.rankAfter !== undefined || ranks.seats.p2.rankAfter !== undefined;
}

/**
 * Both seats' ranks and win streaks for the match bar (R604, R612, R2101000), by username, never profile id (R1436).
 * Read when the match opens, so a promotion mid-match shows on the next one, and again once its result has come while a
 * ranked read carries no move (R2388000). An answer that is not a ranks body leaves no banner rather than breaking the board.
 */
function useMatchRanks(token: string, matchId: string, gameOver: boolean): MatchRanksResponse | null {
  const [ranks, setRanks] = useState<MatchRanksResponse | null>(null);
  useEffect(() => {
    let canceled = false;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const read = (reads: number): void => {
      getMatchRanks(token, matchId).then(
        (answer) => {
          if (canceled) return;
          const body = asMatchRanks(answer);
          setRanks(body);
          if (gameOver && body !== null && body.ranked && !recorded(body) && reads < MATCH_RANKS_READ_ATTEMPTS) {
            retry = setTimeout(() => {
              read(reads + 1);
            }, MATCH_RANKS_RETRY_MS);
          }
        },
        () => undefined,
      );
    };
    read(1);
    return () => {
      canceled = true;
      if (retry !== undefined) clearTimeout(retry);
    };
  }, [token, matchId, gameOver]);
  return ranks;
}

export default function MatchRoute({ matchId, token, socketFactory }: MatchRouteProps): ReactElement {
  const locale = useLocale();
  const match = useMatch({
    matchId,
    token,
    ...(socketFactory === undefined ? {} : { socketFactory }),
  });
  const ranks = useMatchRanks(token, matchId, match.view?.result != null);
  // R336: a Conquest game shows its series' score and won decks, and once it is over the way to the next game.
  const series = useMatchSeries(token, matchId, match.view?.result != null);
  const lookup = currentCatalogLookup();
  /** R768000: the finished match's replay (`share` opens it at the last turn's GIF); null shows the board. */
  const [watching, setWatching] = useState<{ share: boolean } | null>(null);

  // BUILD M5-T3: `window.__jackioh` outside production builds, installed once and reading through a
  // ref, so a spec that re-reads `handle.state` sees the current view.
  const latest = useRef(match);
  latest.current = match;
  useEffect(
    () =>
      installDevHandle({
        get state() {
          const view = latest.current.view;
          return view === null ? null : viewDerivedState(view);
        },
        get seed() {
          // The server mints the match seed and never sends it: with the seed and the log a client
          // could reconstruct the library order (§9.1, §9.3). See `viewDerivedState`.
          return "";
        },
        get seat() {
          return latest.current.view?.viewer ?? null;
        },
        dispatch: (action) => {
          // `playerId` is dropped: the actor stamps the authenticated seat.
          const { playerId: _seat, ...body } = action;
          latest.current.send(body as ActionBody);
        },
      }),
    [],
  );

  const view = match.view;

  // #1664: the result's write paid this account (the Jlore, and maybe a level's or a quest's), so the
  // pill reads the wallet again, once however many views carry the answer.
  const paid = match.economy !== null;
  useEffect(() => {
    if (paid) announceWallet();
  }, [paid]);

  // R643–R644: the opponent's relays land through `match.emote`, whose `seq` bumps on every frame so
  // the same emote twice still notifies. R1342: this account's dealt hand follows the seat it plays.
  const globalMuteEmotes = useSetting("muteOpponentEmotes");
  const ownHand = match.emoteHand;
  const seatNow = view?.viewer;
  const hands = useMemo(
    () => (ownHand === null || seatNow === undefined ? null : { [seatNow]: ownHand }),
    [ownHand, seatNow],
  );
  const emotes = useEmotes({
    portraits: match.portraits,
    hands,
    emit: match.sendEmote,
    engine: getAudioEngine(),
    globalMute: globalMuteEmotes,
    you: view?.viewer ?? "p1",
  });
  const relayed = match.emote;
  const lastRelaySeq = useRef(0);
  useEffect(() => {
    if (relayed === null || relayed.seq === lastRelaySeq.current) return;
    lastRelaySeq.current = relayed.seq;
    emotes.receive(relayed.from, relayed.emote);
  }, [relayed, emotes]);

  // R738: aims both ways. This seat's go out through the socket; the opponent's relay is drawn,
  // and only ever the opponent's — a relay is never about the viewer's own seat.
  const relayedAim = match.aim;
  const viewer = view?.viewer;
  const aim = useMemo(
    () => ({
      emit: match.sendAim,
      opponent: relayedAim !== null && relayedAim.from !== viewer ? relayedAim.aim : null,
    }),
    [match.sendAim, relayedAim, viewer],
  );

  // R1834004: the viewer's side wears the deck slots its seat froze at the start (R1834002); the
  // account's choice until the first `loadouts` frame.
  const loadouts = match.loadouts;
  const ownSeat = viewer === undefined || loadouts === null ? null : loadouts[viewer].seat;
  const deckCosmetics = useMemo(
    () => (ownSeat === null ? null : { table: ownSeat.table, field: ownSeat.field, cardBack: ownSeat.cardBack }),
    [ownSeat],
  );

  // The clock frame of the view's turn: a view on the next turn never reads the last turn's deadline.
  const clock = useFrameFor(match.clock, turnKeyOf(view));

  // What the server said when it refused the socket is for a debugger, not the player (below).
  const refusedWith = match.connection === "refused" ? (match.error ?? "no reason given") : null;
  useEffect(() => {
    if (refusedWith === null) return;
    console.error(`The server refused the socket for match ${matchId}: ${refusedWith}`);
  }, [refusedWith, matchId]);

  // No frame has carried `legalActions` (WHERE LEGALITY COMES FROM): the console gets why.
  const readOnly = view !== null && match.legalSource === "none";
  useEffect(() => {
    if (!readOnly) return;
    // Worded without the build plan's task ids or repository paths, which ship in the bundle.
    console.error(
      "This board is read-only: nothing this socket has received carries the `legalActions` array, " +
        "so only prompts can be answered. The match server sends it as a `legal` field on the " +
        "`view` frame; a `legal` frame of its own would do as well.",
    );
  }, [readOnly]);

  // #1792: the tab says when the game waits on the player, so a match in a background tab can be found.
  const yourMove = view !== null && awaitsYou(view);
  useEffect(() => {
    document.title = yourMove ? `${t("Your turn")} · ${SITE_NAME}` : documentTitleFor(paths.match(matchId));
  }, [yourMove, matchId, locale]);

  // R2479000: while the match is live, every way off the board (its Back, the browser's Back, any
  // in-app move but a forced one) asks first and names the disconnect grace (§9.5). A finished,
  // voided or refused match has nothing to come back to, so it is left at once.
  const [leaving, setLeaving] = useState<{ leave: () => void } | null>(null);
  const atStake =
    view !== null && view.result === null && match.connection !== "closed" && match.connection !== "refused";
  useEffect(() => {
    if (!atStake) return;
    const release = guardLeaving((leave) => {
      setLeaving({ leave });
    });
    return () => {
      release();
      setLeaving(null);
    };
  }, [atStake]);

  if (view === null) {
    if (match.connection === "refused") {
      return (
        <ShellPanel testId={matchTestid.connecting}>
          <p className="notice" data-testid={matchTestid.refused} role="alert">
            {t("This match isn't available. It may have ended, or it may not be yours to join.")}
          </p>
          <div className="row">
            <a className="button-primary" href={paths.play} onClick={followInApp(paths.play)}>
              {t("Back to the lobby")}
            </a>
          </div>
        </ShellPanel>
      );
    }
    // #1599: a socket closed for good before any view (a voided match) has nothing left to wait for.
    if (match.connection === "closed") {
      return (
        <ShellPanel testId={matchTestid.connecting}>
          <p className="notice" data-testid={matchTestid.gone} role="alert">
            {t("This match is no longer available. It closed before the board could load.")}
          </p>
          <div className="row">
            <a className="button-primary" href={paths.play} onClick={followInApp(paths.play)}>
              {t("Back to the lobby")}
            </a>
          </div>
        </ShellPanel>
      );
    }
    // Render's free tier can take most of a minute to wake, which `slow` says once it has.
    return (
      <Loading what="Joining the match…" slow testId={matchTestid.connecting}>
        <p className="auth-hint">
          <ConnectionLine state={match.connection} onReconnect={match.connect} />
          {DEV_ONLY ? (
            <>
              {" "}
              · {t("match")} <code>{matchId}</code> · <code>{withoutToken(match.url)}</code>
            </>
          ) : null}
        </p>
      </Loading>
    );
  }

  if (watching !== null && view.result !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <OpenedReplay
          key={watching.share ? "share" : "watch"}
          load={() => openOnlineMatchReplay(token, matchId)}
          start={watching.share ? "last" : "first"}
          shareOnOpen={watching.share}
          // #936 B59: the finished match's replay, shared with a friend.
          actions={<ShareWithFriend matchId={matchId} />}
          onLeave={() => {
            setWatching(null);
          }}
        />
        {/*
          The rematch navigation that survives the replay, as the board rail's survives "View the
          board" (below): the result panel's buttons unmount with the board, so without this a seat
          that offered and opened the replay would never be taken to the game the offers created.
          Same series gate: series games offer no rematch.
        */}
        {series === null ? <RematchWatcher token={token} matchId={matchId} /> : null}
      </CatalogContext.Provider>
    );
  }

  const activeIsYou = view.active === view.viewer;
  // R79's turn clock belongs to the ACTIVE player. `clock` frames beat `PlayerView.clockMs`: they
  // carry the server's `now`, which makes a countdown immune to this tab's wall-clock skew.
  const turnMs = clock === null ? view.clockMs : remainingMs(clock.clocks.turnDeadline, clock);
  // R265, R268: while both mulligans are open one deadline (the prompt deadline) runs for both seats,
  // so both sides show it, the seat that has already answered included.
  const inMulligan = view.mulligan !== undefined;
  const mulliganMs = clock === null ? view.clockMs : remainingMs(clock.clocks.promptDeadline, clock);
  const opponent: PlayerId = view.opponent.player;
  const graceMs = {
    you: clock === null ? null : remainingMs(clock.clocks.graceDeadline[view.viewer], clock),
    opponent: clock === null ? null : remainingMs(clock.clocks.graceDeadline[opponent], clock),
  };
  // #1899: a socket closed for good once the board is up (a voided match; `close()` runs only as this
  // route unmounts) has nothing to reconnect to.
  const gone = match.connection === "closed";
  // R679: the server keeps no record of a voided match, so its result offers no replay to watch or share.
  const voided = view.result?.reason === "voided";

  // R2388000: each seat's recorded move, both ends present, for the result panel.
  const moves =
    view.result === null || ranks === null
      ? []
      : (["p1", "p2"] as const).flatMap((side) => {
          const seat = ranks.seats[side];
          return seat.rankBefore === undefined || seat.rankAfter === undefined
            ? []
            : [{ side, seat, before: seat.rankBefore, after: seat.rankAfter }];
        });

  const board = (
    <Game
      view={view}
      legal={match.legal}
      onAction={match.send}
      emotes={emotes}
      aim={aim}
      trackStats
      wearCosmetics
      deckCosmetics={deckCosmetics}
      loadouts={loadouts}
      // A refused socket's reason is the console's (above); the board says it in a player's words.
      error={
        refusedWith === null
          ? match.error
          : t("{reason}. Head back to the lobby.", { reason: connectionWords("refused") })
      }
      resultActions={
        // A rematch, never for a series game (the server refuses those, `series_game`, and the
        // Conquest continue flow owns what comes next), then the series' next game, then the lobby.
        <>
          {series === null ? (
            <RematchButtons
              token={token}
              matchId={matchId}
              connection={match.connection}
              ranked={ranks?.ranked ?? false}
            />
          ) : null}
          <SeriesContinue series={series} matchId={matchId} />
          {voided ? null : (
            <>
              <button type="button" data-testid={matchTestid.watchReplay} onClick={() => setWatching({ share: false })}>
                {t("Watch the replay")}
              </button>
              <button type="button" data-testid={matchTestid.shareLastTurn} onClick={() => setWatching({ share: true })}>
                {t("Share the last turn")}
              </button>
            </>
          )}
          <button type="button" data-testid="result-back" data-route={paths.play} onClick={() => navigate(paths.play)}>
            {t("Back to lobby")}
          </button>
          {/* R1483000: the server said the day's Jlore cap held this account's Jlore for the match back. */}
          {view.result !== null && match.jloreCapped ? (
            <p className="result-overlay__note" data-testid="result-daily-jlore">
              {t("Daily Jlore reached")}
            </p>
          ) : null}
          {/* R2388000: each seat's rank before → after this ranked game, once its record is written. */}
          {moves.map(({ side, seat, before, after }) => (
            <p key={side} className="result-overlay__note" data-testid={matchTestid.rankMove} data-seat={side}>
              <Username name={seat.username} />
              {seat.you ? t(" (you)") : ""}: {rankWords(before)} → {rankWords(after)}
            </p>
          ))}
          {/* #1664: the tracks the game moved, level by level with their rewards, and the quests it completed,
              played once the panel has come in. */}
          {view.result !== null && match.economy !== null ? (
            <OnceRevealed>
              <XpReveal economy={match.economy} />
            </OnceRevealed>
          ) : null}
        </>
      }
      turnClock={
        <Clock
          youMs={inMulligan ? mulliganMs : activeIsYou ? turnMs : null}
          opponentMs={inMulligan ? mulliganMs : activeIsYou ? null : turnMs}
          graceMs={graceMs}
          mulligan={inMulligan}
          frame={clock}
          viewer={view.viewer}
          activePlayer={view.result === null ? view.active : null}
          promptHolder={view.result === null ? promptHolderOf(view.pending, view.viewer) : null}
        />
      }
      matchStatus={
        <>
          {t("Online match")} · <ConnectionLine state={match.connection} onReconnect={match.connect} />
          {DEV_ONLY ? (
            <>
              {" "}· {t("match")} <code>{matchId}</code> · {t("seat")} <code>{view.viewer}</code>
            </>
          ) : null}
        </>
      }
      boardRail={
        <>
          <BackLink showSettings={false} />
          {/* R169: every live modifier in full, one tap away; on a phone the badges beside each hero are cut short. */}
          <ModifierList view={view} opponentLabel={t("Opponent")} />
          <SeriesBanner series={series} matchId={matchId} gameOver={view.result !== null} />
          {/*
            The rematch navigation that survives "View the board": the buttons above live in the
            result panel's actions and unmount with it, so without this a seat that offered and
            folded the panel would never be taken to the game the offers created (Rematch.tsx).
            Same series gate as the buttons: series games offer no rematch.
          */}
          {view.result !== null && series === null ? <RematchWatcher token={token} matchId={matchId} /> : null}
          {ranks !== null ? (
            <p className="match-ranks" data-testid={matchTestid.ranks}>
              <span className="match-ranks__full">
                {ranks.ranked ? t("Ranked match") : t("Unranked match")} ·{" "}
                <Username name={ranks.seats.p1.username} />
                {ranks.seats.p1.you ? t(" (you)") : ""}{ranks.ranked ? <StreakFlame streak={ranks.seats.p1.streak} size="small" /> : null} <RankCrest rank={ranks.seats.p1.rank} size="small" />{" "}
                {rankCaption(ranks.seats.p1.rank)} {t("vs")}{" "}
                <Username name={ranks.seats.p2.username} />
                {ranks.seats.p2.you ? t(" (you)") : ""}{ranks.ranked ? <StreakFlame streak={ranks.seats.p2.streak} size="small" /> : null} <RankCrest rank={ranks.seats.p2.rank} size="small" />{" "}
                {rankCaption(ranks.seats.p2.rank)}
              </span>
              <span className="match-ranks__short">
                {ranks.ranked ? t("Ranked") : t("Unranked")} · <Username name={ranks.seats.p1.username} />
                {ranks.seats.p1.you ? t(" (you)") : ""}{ranks.ranked ? <StreakFlame streak={ranks.seats.p1.streak} size="small" /> : null} <RankCrest rank={ranks.seats.p1.rank} size="small" />{" "}
                {rankShortWords(ranks.seats.p1.rank)} {t("vs")}{" "}
                <Username name={ranks.seats.p2.username} />
                {ranks.seats.p2.you ? t(" (you)") : ""}{ranks.ranked ? <StreakFlame streak={ranks.seats.p2.streak} size="small" /> : null} <RankCrest rank={ranks.seats.p2.rank} size="small" />{" "}
                {rankShortWords(ranks.seats.p2.rank)}
              </span>
            </p>
          ) : null}
          {/* #936 B51: the server's `watchers {count}` frame, while anyone is watching. */}
          {match.watchers > 0 ? (
            <p className="match-watchers" data-testid={matchTestid.watchers} data-count={match.watchers} role="status">
              {match.watchers === 1
                ? t("A friend is watching")
                : t("{count} friends are watching", { count: match.watchers })}
            </p>
          ) : null}
        </>
      }
      boardNotice={
        gone ? (
          <p className="board-read-only" data-testid={matchTestid.gone} role="alert">
            {t("This match is no longer available.")}{" "}
            <a href={paths.play} onClick={followInApp(paths.play)}>
              {t("Back to the lobby")}
            </a>
          </p>
        ) : readOnly ? (
          <p className="board-read-only" data-testid={matchTestid.missingLegal} role="alert">
            {t("Your moves can't be sent from this board right now. Reload the page to rejoin the match.")}
          </p>
        ) : null
      }
    />
  );

  return (
    <div className="app-shell app-shell--wide">
      {/*
        The provider is ALWAYS rendered: wrapping the board conditionally would change the element type
        here and remount the whole board, losing a resolved click or a half-finished play (spec 05 lost
        a mulligan to it). The lookup is the shipped catalog's (`game/currentCatalog.ts`), the same
        function from the first render on.
      */}
      <CatalogContext.Provider value={lookup}>{board}</CatalogContext.Provider>
      {leaving === null ? null : (
        <ConfirmLeaveMatch
          onStay={() => {
            setLeaving(null);
          }}
          onLeave={leaving.leave}
        />
      )}
    </div>
  );
}
