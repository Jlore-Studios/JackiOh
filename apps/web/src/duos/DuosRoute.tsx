// `/duos/<matchId>` (#936 S8, B61–B64): the networked co-op board over `/ws/duos`.
//
// As `/match/<id>` is (routes/match.tsx), the whole route is intent in and views out: open the socket
// (`socket.ts`), render the `DuosView` and `legal` the server pushed through `DuosBoard` — Game.tsx
// for the own duel — and send what the board reports straight back. CLAUDE.md rule 7: nothing on
// screen comes from anywhere but the view, and no move is offered that `legal` does not list.
//
// The catalog comes from `GET /api/catalog`, as on the match screen, so cards wear their names; the
// provider is always rendered, `null` included, so its arrival never remounts the board.
//
// A finished game's result panel offers "Watch the replay" (#965, §13.9): the route then swaps the
// board for `DuosReplay.tsx`, which reads the server's replay once, and "back" returns to the result.
//
// The route takes the bearer the gate resolved, as `/match/<id>` does, and hands each renewal to the
// socket (R194, R1115000); the gate in main.tsx has already let a signed-in, active account through,
// and keys the route by the provider session, so a new session opens a new socket.

import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { ActionBody } from "@jackioh/shared";

import { CatalogContext } from "../game/catalog.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { installDevHandle, viewDerivedState } from "../game/net.ts";
import { formatSeconds, t, useLocale } from "../i18n/i18n.ts";
import { Loading, SITE_NAME, ShellPanel, documentTitleFor } from "../main.tsx";
import { navigate, paths } from "../net/navigate.ts";
import { BackLink, followInApp } from "../routes/nav.tsx";
import { useCatalogLookup } from "../together/catalog.ts";
import { DUOS_CLOCK_TICK_MS } from "./config.ts";
import { DuosChromeContext, type DuosChrome } from "./context.ts";
import { DuosBoard } from "./DuosBoard.tsx";
import DuosReplay from "./DuosReplay.tsx";
import { duelOf } from "./seats.ts";
import { useDuosMatch, type DuosClockReading, type DuosConnection } from "./socket.ts";
import { duosTestid } from "./testids.ts";

const DEV_ONLY = import.meta.env.MODE !== "production";

/** A connection state in a player's words. */
function connectionWords(state: DuosConnection): string {
  switch (state) {
    case "open":
      return t("Connected");
    case "connecting":
      return t("Connecting…");
    case "reconnecting":
      return t("Reconnecting…");
    case "refused":
      return t("This match isn’t available");
    case "closed":
      return t("Disconnected");
  }
}

/** The connection line: the words for a player, the raw state for tests, and a retry while reconnecting. */
function ConnectionLine({ state, onReconnect }: { state: DuosConnection; onReconnect: () => void }): ReactElement {
  return (
    <>
      <span data-testid={duosTestid.connection} data-connection={state}>
        {connectionWords(state)}
      </span>
      {state === "reconnecting" ? (
        <>
          {" "}
          <button type="button" className="link-button" data-testid={duosTestid.reconnect} onClick={onReconnect}>
            {t("Reconnect now")}
          </button>
        </>
      ) : null}
    </>
  );
}

function monotonic(): number {
  return typeof performance !== "undefined" && typeof performance.now === "function" ? performance.now() : Date.now();
}

/** Milliseconds left on a server deadline, counted on this tab's monotonic clock from the frame. */
function remainingMs(deadline: number | null, clock: DuosClockReading): number | null {
  if (deadline === null) return null;
  return Math.max(0, deadline - clock.now - Math.max(0, monotonic() - clock.receivedAt));
}

/** B38: the round clock, beside End turn, while a round is running on one. */
function RoundClock({ clock }: { clock: DuosClockReading | null }): ReactElement | null {
  const [, setTick] = useState(0);
  const deadline = clock?.roundDeadline ?? null;
  useEffect(() => {
    if (deadline === null) return undefined;
    const timer = window.setInterval(() => {
      setTick((count) => count + 1);
    }, DUOS_CLOCK_TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [deadline]);
  if (clock === null) return null;
  const left = remainingMs(deadline, clock);
  if (left === null) return null;
  return (
    <span
      className="clock turn-mechanism-clock duos-clock"
      data-testid={duosTestid.clock}
      data-ms={Math.round(left)}
      title={t("Time left in this round")}
    >
      {formatSeconds(Math.ceil(left / MS_PER_SECOND))}
    </span>
  );
}

export function DuosRoute({ matchId, token }: { matchId: string; token: string }): ReactElement {
  const locale = useLocale();
  const match = useDuosMatch({ matchId, token });
  // `GET /api/catalog`, once; with none the board shows def ids, never a guessed name.
  const lookup = useCatalogLookup();
  // #965: the result panel's replay is open. Held above the early returns, which a hook may not follow.
  const [watching, setWatching] = useState(false);

  // BUILD M5-T3's `window.__jackioh` outside production: the own duel's view-derived state, which is
  // what the shared Cypress commands read (`cy.keepMulligans` waits on it), and a dispatch of a duel
  // move as the board would send it.
  const latest = useRef(match);
  latest.current = match;
  useEffect(
    () =>
      installDevHandle({
        get state() {
          const view = latest.current.view;
          return view === null ? null : viewDerivedState(view.own);
        },
        get seed() {
          // The server never sends the seed (§9.1, §9.3).
          return "";
        },
        get seat() {
          return latest.current.view?.own.viewer ?? null;
        },
        dispatch: (action) => {
          // `playerId` is dropped: the server stamps the authenticated seat.
          const { playerId: _seat, ...rest } = action;
          const body = rest as ActionBody;
          latest.current.send(body.type === "concede" ? { kind: "concede" } : { kind: "duel", body });
        },
      }),
    [],
  );

  const view = match.view;
  const slot = view === null ? 0 : duelOf(view.seat);
  // The tab says when the seat's own turn is waiting on it, so a match in a background tab is found.
  const yourMove =
    view !== null &&
    view.result === null &&
    view.round > 0 &&
    view.own.active === view.own.viewer &&
    view.team.ended[slot] !== true;
  useEffect(() => {
    document.title = yourMove ? `${t("Your turn")} · ${SITE_NAME}` : documentTitleFor(paths.duos(matchId));
  }, [yourMove, matchId, locale]);

  const refusedWith = match.connection === "refused" ? (match.error ?? "no reason given") : null;
  useEffect(() => {
    if (refusedWith !== null) console.error(`The server refused the co-op socket for match ${matchId}: ${refusedWith}`);
  }, [refusedWith, matchId]);

  const chrome: DuosChrome = useMemo(
    () => ({
      back: <BackLink to={paths.together} label="← Play together" showSettings={false} />,
      status: (
        <>
          {t("Co-op match")} ·{" "}
          <ConnectionLine state={match.connection} onReconnect={match.connect} />
          {DEV_ONLY ? (
            <>
              {" "}· {t("match")} <code>{matchId}</code>
            </>
          ) : null}
        </>
      ),
      clock: <RoundClock clock={match.clock} />,
      teammate: match.teammate,
      onLeave: () => {
        navigate(paths.together);
      },
      onWatchReplay: () => {
        setWatching(true);
      },
    }),
    // `locale`: the words are drawn in the reader's language, and follow a switch (#711).
    [match.connection, match.connect, match.clock, match.teammate, matchId, locale],
  );

  if (view === null) {
    if (match.connection === "refused") {
      return (
        <ShellPanel testId={duosTestid.connecting}>
          <p className="notice" data-testid={duosTestid.refused} role="alert">
            {t("This co-op match isn’t available. It may have ended, or it may not be yours to join.")}
          </p>
          <div className="row">
            <a className="button-primary" href={paths.together} onClick={followInApp(paths.together)}>
              {t("Back to Play together")}
            </a>
          </div>
        </ShellPanel>
      );
    }
    return (
      <Loading what="Joining the co-op match…" slow testId={duosTestid.connecting}>
        <p className="auth-hint">
          <ConnectionLine state={match.connection} onReconnect={match.connect} />
        </p>
      </Loading>
    );
  }

  if (watching && view.result !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <DuosReplay
          matchId={matchId}
          onLeave={() => {
            setWatching(false);
          }}
        />
      </CatalogContext.Provider>
    );
  }

  return (
    <div className="app-shell app-shell--wide">
      <CatalogContext.Provider value={lookup}>
        <DuosChromeContext.Provider value={chrome}>
          <DuosBoard
            view={view}
            legal={match.legal}
            onAction={match.send}
            pings={match.pings}
            onPing={match.sendPing}
            error={
              refusedWith === null
                ? match.error
                : t("{reason}. Head back to Play together.", { reason: connectionWords("refused") })
            }
          />
        </DuosChromeContext.Provider>
      </CatalogContext.Provider>
    </div>
  );
}

export default DuosRoute;
