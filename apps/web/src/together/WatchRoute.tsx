// `/watch/:matchId?friend=<id>` — a friend's live game, watched from the friend's seat (#936, B50).
// The watch socket (authenticated as `/ws/match` is) is told whom to watch in its hello, and answers the
// friend's `PlayerView` frames, each `SPECTATE_DELAY_MS` after the state it shows, with `legal: []`.
// The board is `Game.tsx` as a replay draws it. Nothing a watcher sends after the hello reaches the
// game, and this screen sends the hello and its emotes, nothing else (CLAUDE.md rule 7).
// R2990000, R2990001: the socket deals the watcher its own emote hand (`spectatorHand`), which the
// sideline strip offers while the socket is open; a pick goes out as `{type:"emote"}` and shows at
// once, since the server never echoes it, and the other watchers' relays (`spectatorEmote`) show by name.
// Whether the friend may be watched is the server's: a refusal is a `forbidden` frame and a closed
// socket. A tab that wakes probes a quiet socket with the hello again and replaces one that does not
// answer (R1441, R1103000), within the reopen attempts.
// R1834003, R1834004: both seats' frozen loadouts are kept merged as the match socket keeps them
// (game/net.ts); a watched board wears nobody's device picks, so both seats wear their own.
// R2881000: a co-op game (`coop`, the link's `&coop=1`) is watched over the same socket and hello; its
// frames are the friend's seat's `DuosView`s, drawn on the co-op board as its replay draws a step.
// #2974: the board's turn clock counts down from each frame's `clockMs`, silently, and a finished game's
// status line says "Game over" in place of the connection's words.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import type { ActionBody, EmojiEmoteId, PlayerView } from "@jackioh/shared";
import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import { getAudioEngine } from "../audio/index.ts";
import { createLoadoutsHolder, type SeatLoadouts } from "../cards/loadouts.ts";
import Game from "../game/Game.tsx";
import { ModifierList } from "../game/ModifierList.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { createWakeProbe, parseServerFrame } from "../game/net.ts";
import { DuosChromeContext } from "../duos/context.ts";
import { DuosBoard } from "../duos/DuosBoard.tsx";
import { parseDuosFrame } from "../duos/socket.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { Loading, ShellPanel } from "../main.tsx";
import { matchSocketUrl, serverSocketUrl } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import type { ReceivedPing } from "../net/duos.ts";
import { readSession } from "../net/session.ts";
import { socketProtocols } from "../net/socketProtocols.ts";
import { getFriends } from "../net/social.ts";
import { BackLink, followInApp } from "../routes/nav.tsx";
import { useSetting } from "../settings/index.ts";
import { useSideline } from "../sideline/useSideline.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { useCatalogLookup } from "./catalog.ts";
import { WATCH_RECONNECT_ATTEMPTS, WATCH_RECONNECT_MS } from "./config.ts";
import { usePolled } from "./session.ts";
import { watchTestid } from "./testids.ts";
import { WatchClock, watchClockOf, type WatchClockReading } from "./WatchClock.tsx";
import { Named } from "./words.tsx";

import "./together.css";

/** `WS_PATH`'s sibling on the server: the watch socket. */
const WATCH_PATH = "/ws/watch";

export type WatchConnection = "connecting" | "open" | "closed" | "refused";

/** A watched board is offered nothing, and sends nothing. */
const NOTHING_LEGAL: readonly ActionBody[] = [];
const IGNORE = (): void => undefined;
/** R2881000: and a watched co-op board is offered nothing and hears no ping, as `DuosReplay.tsx`'s. */
const NO_COOP_ACTIONS: readonly DuosActionBody[] = [];
const NO_PINGS: readonly ReceivedPing[] = [];
/** R2990001: no hand dealt yet, or none to offer while the socket is not open. */
const NO_HAND: readonly EmojiEmoteId[] = [];

/** The watch socket's address beside the match socket's, with no token: it is offered as a subprotocol (R1379000). */
export function watchSocketUrl(matchId: string, friendId: string, base: string = matchSocketUrl()): string {
  const query = new URLSearchParams({ matchId, friendId }).toString();
  return `${serverSocketUrl(WATCH_PATH, base).replace(/\?.*$/u, "")}?${query}`;
}

/**
 * The newest delayed view of the friend's seat (a co-op seat's in `coopView`, R2881000), both seats' loadouts
 * (R1834003), how the socket stands, and the watcher's own retry.
 */
function useWatch(
  matchId: string,
  friendId: string,
  coop: boolean,
): {
  view: PlayerView | null;
  coopView: DuosView | null;
  loadouts: SeatLoadouts | null;
  /** #2974: the newest frame's `clockMs`, stamped as it arrived; a new frame is a new reading. */
  clock: WatchClockReading | null;
  connection: WatchConnection;
  reconnect: () => void;
  /** R2990001: the watcher's own emote hand, from the last `spectatorHand` frame. */
  hand: readonly EmojiEmoteId[];
  /** R2990000: another watcher's last relayed emote; `seq` makes repeats notify. */
  relayed: { watcher: string; emote: EmojiEmoteId; seq: number } | null;
  /** R2990000: one emote out on the open socket; dropped while there is none. */
  sendEmote: (emote: EmojiEmoteId) => void;
} {
  const [view, setView] = useState<PlayerView | null>(null);
  const [coopView, setCoopView] = useState<DuosView | null>(null);
  const [loadouts, setLoadouts] = useState<SeatLoadouts | null>(null);
  const [clock, setClock] = useState<WatchClockReading | null>(null);
  const [connection, setConnection] = useState<WatchConnection>("connecting");
  const [hand, setHand] = useState<readonly EmojiEmoteId[]>(NO_HAND);
  const [relayed, setRelayed] = useState<{ watcher: string; emote: EmojiEmoteId; seq: number } | null>(null);
  /** The running effect's retry (#1570); a no-op until the effect sets it. */
  const reconnectRef = useRef<() => void>(IGNORE);
  /** The running effect's emote send, as `reconnectRef` is its retry. */
  const sendEmoteRef = useRef<(emote: EmojiEmoteId) => void>(IGNORE);
  const relaySeq = useRef(0);

  useEffect(() => {
    let stopped = false;
    let socket: WebSocket | null = null;
    let timer: number | undefined;
    let tries = 0;
    let refused = false;
    let over = false;
    const held = createLoadoutsHolder();
    const relay = (watcher: string, emote: EmojiEmoteId): void => {
      relaySeq.current += 1;
      setRelayed({ watcher, emote, seq: relaySeq.current });
    };

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
        if (coop) {
          // R2881000: a co-op game's watcher is sent the seat's views and a refusal, and, R2990000, the
          // emote frames.
          const frame = parseDuosFrame(event.data);
          if (frame?.type === "view") {
            over = frame.view.result !== null;
            setCoopView(frame.view);
            setClock(watchClockOf(frame.view.own.clockMs));
          } else if (frame?.type === "spectatorHand") {
            setHand(frame.emotes);
          } else if (frame?.type === "spectatorEmote") {
            relay(frame.watcher, frame.emote);
          } else if (frame?.type === "error" && frame.code === "forbidden") {
            refused = true;
            setConnection("refused");
          }
          return;
        }
        const frame = parseServerFrame(event.data);
        if (frame === null) return;
        if (frame.type === "view") {
          over = frame.view.result !== null;
          setView(frame.view);
          setClock(watchClockOf(frame.view.clockMs));
        } else if (frame.type === "loadouts") {
          setLoadouts(held.seats(frame.seats));
        } else if (frame.type === "cardLoadouts") {
          setLoadouts(held.cards(frame.cards));
        } else if (frame.type === "spectatorHand") {
          setHand(frame.emotes);
        } else if (frame.type === "spectatorEmote") {
          relay(frame.watcher, frame.emote);
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
    // R2990000: cosmetic, as a seat's emote is: with no socket, or one not yet open, it is dropped.
    sendEmoteRef.current = (emote) => {
      try {
        socket?.send(JSON.stringify({ type: "emote", emote }));
      } catch {
        // Not open yet; its `open` brings the strip's hand back.
      }
    };

    wakeProbe.listen();
    open();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      wakeProbe.dispose();
      socket?.close();
    };
  }, [matchId, friendId, coop]);

  const reconnect = useCallback((): void => {
    reconnectRef.current();
  }, []);
  const sendEmote = useCallback((emote: EmojiEmoteId): void => {
    sendEmoteRef.current(emote);
  }, []);

  return { view, coopView, loadouts, clock, connection, reconnect, hand, relayed, sendEmote };
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

export function WatchRoute({
  matchId,
  friendId,
  coop = false,
}: {
  matchId: string;
  friendId: string;
  /** R2881000: the game is a co-op one, drawn on the co-op board. */
  coop?: boolean;
}): ReactElement {
  useLocale();
  const { view, coopView, loadouts, clock, connection, reconnect, hand, relayed, sendEmote } = useWatch(matchId, friendId, coop);
  // R2990000–R2990002, R2990004: the watcher's own picks while the socket is open, the others' by `seq`.
  const sideline = useSideline({
    hand: connection === "open" ? hand : NO_HAND,
    emit: sendEmote,
    engine: getAudioEngine(),
    globalMute: useSetting("muteSpectatorEmotes"),
  });
  const lastRelaySeq = useRef(0);
  useEffect(() => {
    if (relayed === null || relayed.seq === lastRelaySeq.current) return;
    lastRelaySeq.current = relayed.seq;
    sideline.receive(relayed.watcher, relayed.emote);
  }, [relayed, sideline]);
  const lookup = useCatalogLookup();
  const [friends] = usePolled(() => getFriends({ keep: false }), null);
  const name = friends.kind === "ready" ? friends.value.friends.find((friend) => friend.profileId === friendId)?.username : undefined;
  const seconds = String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND));
  const shown = coop ? coopView : view;
  // #2974: a game with a result is over, whatever its socket does after (it closes as "closed" too).
  const finished = shown?.result != null;
  // #1570: the reopens ran out on a game still running.
  const exhausted = connection === "closed" && !finished;
  // #1691: and while a reopen waits, as the match bar offers it; a click opens the socket at once.
  const offerReconnect = exhausted || (connection === "connecting" && !finished);
  // #2974: the friend's turn clock, counted down silently from the newest frame; a finished game has none.
  const turnClock = <WatchClock reading={finished ? null : clock} />;
  const status = (
    <>
      <span data-testid={watchTestid.status} data-connection={connection}>
        {name === undefined ? t("Watching your friend") : <Named text={t("Watching {name}")} name={name} />} ·{" "}
        {t("{seconds} s behind", { seconds })} · {finished ? t("Game over") : connectionWords(connection)}
      </span>
      {offerReconnect ? (
        <>
          {" "}
          <ReconnectNow className="link-button" onReconnect={reconnect} />
        </>
      ) : null}
    </>
  );

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
  } else if (shown === null && connection === "closed") {
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
  } else if (coopView !== null) {
    // R2881000: the friend's co-op seat, read-only, as the co-op replay draws a step.
    screen = (
      <div className="app-shell app-shell--wide watch-screen">
        <CatalogContext.Provider value={lookup}>
          <DuosChromeContext.Provider value={{ back: <BackLink to={paths.together} label="Back to Play together" showSettings={false} />, status, clock: turnClock }}>
            <DuosBoard view={coopView} legal={NO_COOP_ACTIONS} onAction={IGNORE} pings={NO_PINGS} onPing={IGNORE} jloreCapped={false} replay drainWhenHidden sideline={sideline} />
          </DuosChromeContext.Provider>
        </CatalogContext.Provider>
      </div>
    );
  } else if (view !== null) {
    screen = (
      <div className="app-shell app-shell--wide watch-screen">
        <CatalogContext.Provider value={lookup}>
          <Game
            view={view}
            legal={NOTHING_LEGAL}
            onAction={IGNORE}
            replay
            drainWhenHidden
            resultForm="chip"
            loadouts={loadouts}
            sideline={sideline}
            matchStatus={status}
            phoneRailTop
            turnClock={turnClock}
            boardRail={
              <>
                <BackLink to={paths.together} label="Back to Play together" showSettings={false} />
                {/* R169: the friend's seat's live modifiers in full, one tap away, from the newest view. */}
                <ModifierList view={view} opponentLabel={t("Opponent")} />
              </>
            }
          />
        </CatalogContext.Provider>
      </div>
    );
  } else {
    screen = (
      <Loading what="Joining as a watcher…" slow testId={watchTestid.waiting}>
        <p className="auth-hint">{t("What you watch runs {seconds} seconds behind the game.", { seconds })}</p>
        <div className="row">
          <BackToTogether />
        </div>
      </Loading>
    );
  }

  return (
    <div className="watch-root" data-testid={watchTestid.screen} data-connection={connection}>
      {screen}
    </div>
  );
}
