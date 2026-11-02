// `/watch/:matchId?friend=<id>` — a friend's live game, watched from the friend's seat (#936, B50).
// The watch socket (authenticated as `/ws/match` is) is told whom to watch in its hello, and answers the
// friend's `PlayerView` frames, each `SPECTATE_DELAY_MS` after the state it shows, with `legal: []`.
// The board is `Game.tsx` as a replay draws it. Nothing a watcher sends after the hello reaches the
// game, and this screen sends nothing but the hello (CLAUDE.md rule 7).
// Whether the friend may be watched is the server's: a refusal is a `forbidden` frame and a closed
// socket. A tab that wakes probes a quiet socket with the hello again and replaces one that does not
// answer (R1441, R1103000), within the reopen attempts.
// R1834003, R1834004: both seats' frozen loadouts are kept merged as the match socket keeps them
// (game/net.ts); a watched board wears nobody's device picks, so both seats wear their own.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import { createLoadoutsHolder, type SeatLoadouts } from "../cards/loadouts.ts";
import Game from "../game/Game.tsx";
import { ModifierList } from "../game/ModifierList.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { createWakeProbe, parseServerFrame } from "../game/net.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { Loading, ShellPanel } from "../main.tsx";
import { matchSocketUrl, serverSocketUrl } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { socketProtocols } from "../net/socketProtocols.ts";
import { getFriends } from "../net/social.ts";
import { BackLink, followInApp } from "../routes/nav.tsx";
import { useCatalogLookup } from "./catalog.ts";
import { WATCH_RECONNECT_ATTEMPTS, WATCH_RECONNECT_MS } from "./config.ts";
import { usePolled } from "./session.ts";
import { watchTestid } from "./testids.ts";
import { Named } from "./words.tsx";

import "./together.css";

/** `WS_PATH`'s sibling on the server: the watch socket. */
const WATCH_PATH = "/ws/watch";

export type WatchConnection = "connecting" | "open" | "closed" | "refused";

/** A watched board is offered nothing, and sends nothing. */
const NOTHING_LEGAL: readonly ActionBody[] = [];
const IGNORE = (): void => undefined;

/** The watch socket's address beside the match socket's, with no token: it is offered as a subprotocol (R1379000). */
export function watchSocketUrl(matchId: string, friendId: string, base: string = matchSocketUrl()): string {
  const query = new URLSearchParams({ matchId, friendId }).toString();
  return `${serverSocketUrl(WATCH_PATH, base).replace(/\?.*$/u, "")}?${query}`;
}

/** The newest delayed view of the friend's seat, both seats' loadouts (R1834003), how the socket stands, and the watcher's own retry. */
function useWatch(
  matchId: string,
  friendId: string,
): { view: PlayerView | null; loadouts: SeatLoadouts | null; connection: WatchConnection; reconnect: () => void } {
  const [view, setView] = useState<PlayerView | null>(null);
  const [loadouts, setLoadouts] = useState<SeatLoadouts | null>(null);
  const [connection, setConnection] = useState<WatchConnection>("connecting");
  /** The running effect's retry (#1570); a no-op until the effect sets it. */
  const reconnectRef = useRef<() => void>(IGNORE);

  useEffect(() => {
    let stopped = false;
    let socket: WebSocket | null = null;
    let timer: number | undefined;
    let tries = 0;
    let refused = false;
    let over = false;
    const held = createLoadoutsHolder();

    const replace = (stale: WebSocket): void => {
      if (stopped || refused || over) return;
      window.clearTimeout(timer);
      socket = null;
      wakeProbe.cancel();
      const closeStale = (): void => {
        try {
          stale.close(1000, "replaced");
        } catch {
          // Already gone.
        }
      };
      if (tries >= WATCH_RECONNECT_ATTEMPTS) {
        closeStale();
        setConnection("closed");
        return;
      }
      tries += 1;
      setConnection("connecting");
      open();
      closeStale();
    };

    const wakeProbe = createWakeProbe<WebSocket>({
      socket: () => socket,
      sendProbe: (live) => live.send(JSON.stringify({ type: "hello", matchId, friendId })),
      replace,
      disabled: () => stopped || refused || over,
    });

    const open = (): void => {
      const token = readSession()?.accessToken ?? null;
      if (token === null || friendId.length === 0) {
        refused = true;
        setConnection("refused");
        return;
      }
      let live: WebSocket;
      try {
        live = new WebSocket(watchSocketUrl(matchId, friendId), socketProtocols(token));
      } catch {
        setConnection("closed");
        return;
      }
      socket = live;
      wakeProbe.hear();
      live.addEventListener("open", () => {
        if (socket !== live) return;
        tries = 0;
        wakeProbe.hear();
        setConnection("open");
        live.send(JSON.stringify({ type: "hello", matchId, friendId }));
      });
      live.addEventListener("message", (event: MessageEvent) => {
        if (socket !== live) return;
        wakeProbe.hear();
        wakeProbe.cancel();
        if (typeof event.data !== "string") return;
        const frame = parseServerFrame(event.data);
        if (frame === null) return;
        if (frame.type === "view") {
          over = frame.view.result !== null;
          setView(frame.view);
        } else if (frame.type === "loadouts") {
          setLoadouts(held.seats(frame.seats));
        } else if (frame.type === "cardLoadouts") {
          setLoadouts(held.cards(frame.cards));
        } else if (frame.type === "error" && frame.code === "forbidden") {
          refused = true;
          setConnection("refused");
        }
      });
      live.addEventListener("close", () => {
        if (stopped || socket !== live) return;
        socket = null;
        wakeProbe.cancel();
        if (refused) return;
        if (over || tries >= WATCH_RECONNECT_ATTEMPTS) {
          setConnection("closed");
          return;
        }
        tries += 1;
        setConnection("connecting");
        timer = window.setTimeout(open, WATCH_RECONNECT_MS);
      });
    };

    // #1570: once the reopens run out on a game still running, the watcher's own retry opens the
    // socket again and starts the attempts over.
    reconnectRef.current = () => {
      if (stopped || refused || over || socket !== null) return;
      window.clearTimeout(timer);
      tries = 0;
      setConnection("connecting");
      open();
    };

    wakeProbe.listen();
    open();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      wakeProbe.dispose();
      socket?.close();
    };
  }, [matchId, friendId]);

  const reconnect = useCallback((): void => {
    reconnectRef.current();
  }, []);

  return { view, loadouts, connection, reconnect };
}

/** The connection in a watcher's words. */
function connectionWords(connection: WatchConnection): string {
  switch (connection) {
    case "open":
      return t("Connected");
    case "connecting":
      return t("Reconnecting…");
    case "closed":
      return t("Disconnected");
    case "refused":
      return t("This game can't be watched");
  }
}

function BackToTogether(): ReactElement {
  return (
    <a className="button-secondary" href={paths.together} data-testid={watchTestid.back} onClick={followInApp(paths.together)}>
      {t("Back to Play together")}
    </a>
  );
}

/** #1570: the match bar's "Reconnect now", offered once the screen's own reopens have run out and, #1691, while one waits. */
function ReconnectNow({ className, onReconnect }: { className: string; onReconnect: () => void }): ReactElement {
  return (
    <button type="button" className={className} data-testid={watchTestid.reconnect} onClick={onReconnect}>
      {t("Reconnect now")}
    </button>
  );
}

export function WatchRoute({ matchId, friendId }: { matchId: string; friendId: string }): ReactElement {
  useLocale();
  const { view, loadouts, connection, reconnect } = useWatch(matchId, friendId);
  const lookup = useCatalogLookup();
  const [friends] = usePolled(() => getFriends({ keep: false }), null);
  const name = friends.kind === "ready" ? friends.value.friends.find((friend) => friend.profileId === friendId)?.username : undefined;
  const seconds = String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND));
  // #1570: the reopens ran out on a game still running (a finished game's socket closes as "closed" too).
  const exhausted = connection === "closed" && view?.result == null;
  // #1691: and while a reopen waits, as the match bar offers it; a click opens the socket at once.
  const offerReconnect = exhausted || (connection === "connecting" && view?.result == null);

  let screen: ReactElement;
  if (connection === "refused") {
    screen = (
      <ShellPanel>
        <p className="notice" data-testid={watchTestid.refused} role="alert">
          {t("You can't watch this game. It may be over or full, or you may no longer be friends.")}
        </p>
        <div className="row">
          <BackToTogether />
        </div>
      </ShellPanel>
    );
  } else if (view === null && connection === "closed") {
    // #1599: the reopens ran out before any view; a spinner here would be a dead end.
    screen = (
      <ShellPanel testId={watchTestid.waiting}>
        <p className="notice" data-testid={watchTestid.gone} role="alert">
          {t("This game can no longer be reached. It may be over, or the connection gave up.")}
        </p>
        <div className="row">
          <ReconnectNow className="button-primary" onReconnect={reconnect} />
          <BackToTogether />
        </div>
      </ShellPanel>
    );
  } else if (view === null) {
    screen = (
      <Loading what="Joining as a watcher…" slow testId={watchTestid.waiting}>
        <p className="auth-hint">{t("What you watch runs {seconds} seconds behind the game.", { seconds })}</p>
        <div className="row">
          <BackToTogether />
        </div>
      </Loading>
    );
  } else {
    screen = (
      <div className="app-shell app-shell--wide watch-screen">
        <CatalogContext.Provider value={lookup}>
          <Game
            view={view}
            legal={NOTHING_LEGAL}
            onAction={IGNORE}
            replay
            resultForm="chip"
            loadouts={loadouts}
            matchStatus={
              <>
                <span data-testid={watchTestid.status} data-connection={connection}>
                  {name === undefined ? t("Watching your friend") : <Named text={t("Watching {name}")} name={name} />} ·{" "}
                  {t("{seconds} s behind", { seconds })} · {connectionWords(connection)}
                </span>
                {offerReconnect ? (
                  <>
                    {" "}
                    <ReconnectNow className="link-button" onReconnect={reconnect} />
                  </>
                ) : null}
              </>
            }
            boardRail={
              <>
                <BackLink to={paths.together} label="← Play together" showSettings={false} />
                {/* R169: the friend's seat's live modifiers in full, one tap away, from the newest view. */}
                <ModifierList view={view} opponentLabel={t("Opponent")} />
              </>
            }
          />
        </CatalogContext.Provider>
      </div>
    );
  }

  return (
    <div className="watch-root" data-testid={watchTestid.screen} data-connection={connection}>
      {screen}
    </div>
  );
}
