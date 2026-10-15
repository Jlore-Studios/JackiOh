// `/watch/:matchId?friend=<id>` — a friend's live game, watched from the friend's seat (#936, B50).
//
// The watch socket (`/ws/watch`, authenticated as `/ws/match` is: `?token=`) is told whom to watch
// in its hello, and answers the friend's `PlayerView` frames, each `SPECTATE_DELAY_MS` after the
// state it shows, with `legal: []`. The board is `Game.tsx` as a replay draws it: nothing legal, a
// no-op `onAction`, no prompt, drag or glow. Nothing a watcher sends after the hello reaches the
// game, and this screen sends nothing at all (CLAUDE.md rule 7).
//
// Whether the friend may be watched (an unranked game, the friend allows it, room for one more) is
// the server's: a refusal is a `forbidden` frame and a closed socket, and this screen says so with
// the way back. An unexpected close opens the socket again, `WATCH_RECONNECT_ATTEMPTS` times
// `WATCH_RECONNECT_MS` apart; a finished game is not reopened.

import { useEffect, useState, type ReactElement } from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import Game from "../game/Game.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { parseServerFrame } from "../game/net.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { Loading, ShellPanel } from "../main.tsx";
import { matchSocketUrl, serverSocketUrl } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
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

/** The watch socket's address beside the match socket's, its credentials in the query as `/ws/match` takes them. */
export function watchSocketUrl(token: string, matchId: string, friendId: string, base: string = matchSocketUrl()): string {
  const query = new URLSearchParams({ token, matchId, friendId }).toString();
  return `${serverSocketUrl(WATCH_PATH, base).replace(/\?.*$/u, "")}?${query}`;
}

/** The newest delayed view of the friend's seat, and how the socket stands. */
function useWatch(matchId: string, friendId: string): { view: PlayerView | null; connection: WatchConnection } {
  const [view, setView] = useState<PlayerView | null>(null);
  const [connection, setConnection] = useState<WatchConnection>("connecting");

  useEffect(() => {
    let stopped = false;
    let socket: WebSocket | null = null;
    let timer: number | undefined;
    let tries = 0;
    let refused = false;
    let over = false;

    const open = (): void => {
      const token = readSession()?.accessToken ?? null;
      if (token === null || friendId.length === 0) {
        refused = true;
        setConnection("refused");
        return;
      }
      let live: WebSocket;
      try {
        live = new WebSocket(watchSocketUrl(token, matchId, friendId));
      } catch {
        setConnection("closed");
        return;
      }
      socket = live;
      live.addEventListener("open", () => {
        tries = 0;
        setConnection("open");
        live.send(JSON.stringify({ type: "hello", matchId, friendId }));
      });
      live.addEventListener("message", (event: MessageEvent) => {
        if (typeof event.data !== "string") return;
        const frame = parseServerFrame(event.data);
        if (frame === null) return;
        if (frame.type === "view") {
          over = frame.view.result !== null;
          setView(frame.view);
        } else if (frame.type === "error" && frame.code === "forbidden") {
          refused = true;
          setConnection("refused");
        }
      });
      live.addEventListener("close", () => {
        if (stopped || socket !== live) return;
        socket = null;
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

    open();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      socket?.close();
    };
  }, [matchId, friendId]);

  return { view, connection };
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
      return t("This game can’t be watched");
  }
}

function BackToTogether(): ReactElement {
  return (
    <a className="button-secondary" href={paths.together} data-testid={watchTestid.back} onClick={followInApp(paths.together)}>
      {t("Back to Play together")}
    </a>
  );
}

export function WatchRoute({ matchId, friendId }: { matchId: string; friendId: string }): ReactElement {
  useLocale();
  const { view, connection } = useWatch(matchId, friendId);
  const lookup = useCatalogLookup();
  const [friends] = usePolled(getFriends, null);
  const name = friends.kind === "ready" ? friends.value.friends.find((friend) => friend.profileId === friendId)?.username : undefined;
  const seconds = String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND));

  let screen: ReactElement;
  if (connection === "refused") {
    screen = (
      <ShellPanel>
        <p className="notice" data-testid={watchTestid.refused} role="alert">
          {t("You can’t watch this game. It may be over, ranked or full, or your friend may have turned watching off.")}
        </p>
        <div className="row">
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
            matchStatus={
              <span data-testid={watchTestid.status} data-connection={connection}>
                {name === undefined ? t("Watching your friend") : <Named text={t("Watching {name}")} name={name} />} ·{" "}
                {t("{seconds} s behind", { seconds })} · {connectionWords(connection)}
              </span>
            }
            boardRail={<BackLink to={paths.together} label="← Play together" showSettings={false} />}
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
