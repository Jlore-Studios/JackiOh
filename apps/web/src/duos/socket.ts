// The `/ws/duos` client (#936 SPEC S6), one socket per co-op match as `game/net.ts` is per 1v1 match.
// CLAUDE.md rule 7: it sends intent (an action, a ping) and keeps the `DuosView` and `legal` list the
// server pushed, never computed here.
//
// The handshake is `/ws/match`'s (S6): the match id in the query string, the token as a subprotocol
// (R1379000), never in the address. The token is the gate's (`setToken`), so a 4401 is final only for
// the token it refused (R1115000). Drops and page wakes are handled as in `game/net.ts` (R1441); a
// refusal or a voided match never reconnects. A ping past the server's limit is not sent (B39: the
// server would drop it silently), and the seat's own is kept so the board marks it for the sender.

import { useEffect, useMemo, useSyncExternalStore } from "react";

import { PING_RATE_LIMIT, PING_RATE_WINDOW_MS, PING_SHOW_MS } from "@jackioh/server-config";

import { economyField, type GameEconomy } from "../economy/xp.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import {
  REFUSAL_CLOSE_CODES,
  UNAUTHORIZED_CLOSE_CODE,
  VOIDED_CLOSE_CODE,
  browserSocket,
  createBackoff,
  createWakeProbe,
  defaultMonotonic,
  defaultTimers,
  letGo,
  nextNonce,
  pageWake,
  socketUrlFor,
  type SocketFactory,
  type SocketLike,
  type Timers,
} from "../game/net.ts";
import { matchSocketUrl, serverSocketUrl } from "../net/api.ts";
import type { PingKind, PingTarget, ReceivedPing } from "../net/duos.ts";
import { readSession } from "../net/session.ts";
import { socketProtocols } from "../net/socketProtocols.ts";
import { isRecord } from "../wire/json.ts";
import { DUOS_PINGS_KEPT } from "./config.ts";
import { isDuosSeat, isPingKind, parsePingTarget } from "./pings.ts";
import { messageOf } from "../net/errors.ts";

/** `WebSocket.OPEN`, spelled out so this module needs no DOM constant at runtime. */
const OPEN = 1;

const DUOS_PATH = "/ws/duos";

/** `/ws/match`'s URL (`VITE_SERVER_WS_URL`, `net/api.ts`) with its path swapped, so one variable names the server. */
export function duosSocketUrl(base: string = matchSocketUrl()): string {
  return serverSocketUrl(DUOS_PATH, base);
}

// Frames (S6), parsed shallowly: the server shapes the view, the client only checks it is there.

/** The round clock and the human seats' disconnect graces, as absolute server times (epoch ms). */
export type DuosClockFrame = {
  now: number;
  roundDeadline: number | null;
  graceDeadlines: { a1: number | null; a2: number | null };
};

/** A clock frame and the local monotonic reading it landed at, so countdowns never read wall time. */
export type DuosClockReading = DuosClockFrame & { receivedAt: number };

export type DuosServerFrame =
  /** `jloreCapped` and `economy` as `DuosSnapshot` reads them; the server leaves either off when it has none. */
  | {
      type: "view";
      view: DuosView;
      legal: readonly DuosActionBody[];
      jloreCapped: boolean;
      economy?: GameEconomy;
    }
  | { type: "ack"; nonce: string; seq: number }
  | { type: "error"; code: string; message: string; nonce?: string }
  | ({ type: "clock" } & DuosClockFrame)
  | { type: "teammate"; connected: boolean; aiPlaying: boolean }
  | { type: "ping"; from: DuosSeat; ping: PingKind; target: PingTarget };

const isTimeOrNull = (value: unknown): value is number | null => value === null || typeof value === "number";

function parseLegal(value: unknown): readonly DuosActionBody[] {
  if (!Array.isArray(value)) return [];
  return value.filter((entry): entry is DuosActionBody => isRecord(entry) && typeof entry.kind === "string");
}

/** One text frame, or null for anything this client does not understand (it must not take the board down). */
export function parseDuosFrame(text: string): DuosServerFrame | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isRecord(parsed) || typeof parsed.type !== "string") return null;
  switch (parsed.type) {
    case "view":
      if (!isRecord(parsed.view)) return null;
      // R1483000: absent is false; a value that is not a boolean is no frame this client knows.
      if (parsed.jloreCapped !== undefined && typeof parsed.jloreCapped !== "boolean") return null;
      return {
        type: "view",
        view: parsed.view as unknown as DuosView,
        legal: parseLegal(parsed.legal),
        jloreCapped: parsed.jloreCapped === true,
        ...economyField(parsed.economy),
      };
    case "ack":
      if (typeof parsed.nonce !== "string" || typeof parsed.seq !== "number") return null;
      return { type: "ack", nonce: parsed.nonce, seq: parsed.seq };
    case "error":
      if (typeof parsed.code !== "string" || typeof parsed.message !== "string") return null;
      return {
        type: "error",
        code: parsed.code,
        message: parsed.message,
        ...(typeof parsed.nonce === "string" ? { nonce: parsed.nonce } : {}),
      };
    case "clock": {
      const grace = parsed.graceDeadlines;
      if (typeof parsed.now !== "number" || !isTimeOrNull(parsed.roundDeadline) || !isRecord(grace)) return null;
      if (!isTimeOrNull(grace.a1) || !isTimeOrNull(grace.a2)) return null;
      return {
        type: "clock",
        now: parsed.now,
        roundDeadline: parsed.roundDeadline,
        graceDeadlines: { a1: grace.a1, a2: grace.a2 },
      };
    }
    case "teammate":
      if (typeof parsed.connected !== "boolean" || typeof parsed.aiPlaying !== "boolean") return null;
      return { type: "teammate", connected: parsed.connected, aiPlaying: parsed.aiPlaying };
    case "ping": {
      const target = parsePingTarget(parsed.target);
      if (!isDuosSeat(parsed.from) || !isPingKind(parsed.ping) || target === null) return null;
      return { type: "ping", from: parsed.from, ping: parsed.ping, target };
    }
    default:
      return null;
  }
}

// The client

export type DuosConnection = "connecting" | "open" | "reconnecting" | "refused" | "closed";

export type DuosSnapshot = {
  connection: DuosConnection;
  view: DuosView | null;
  /** The seat's legal actions as the server last listed them; never computed here (rule 7). */
  legal: readonly DuosActionBody[];
  /** R1483000: the last view said the daily Jlore cap held back this seat's Jlore; never worked out here. */
  jloreCapped: boolean;
  /** #1664: what the finished match paid this seat's person (`economy/xp.ts`); null if the last view lacks it. */
  economy: GameEconomy | null;
  /** The last `error` frame's message, relayed verbatim, or null. */
  error: string | null;
  errorCode: string | null;
  clock: DuosClockReading | null;
  /** B37: whether the teammate's socket is up, and whether the AI is playing their seat meanwhile. */
  teammate: { connected: boolean; aiPlaying: boolean } | null;
  /** The pings to mark, the seat's own and the teammate's, newest last (B64). */
  pings: readonly ReceivedPing[];
};

export type DuosClient = {
  subscribe: (listener: () => void) => () => void;
  snapshot: () => DuosSnapshot;
  /** Open the socket now, cutting short a waiting backoff (idempotent while a socket is open or opening). */
  connect: () => void;
  /** One action, under a fresh nonce; the server stamps the seat. */
  send: (body: DuosActionBody) => void;
  /** One ping to the teammate (B39); dropped while the socket is down or past the rate limit. */
  sendPing: (ping: PingKind, target: PingTarget) => void;
  close: () => void;
  url: () => string;
  /** R1115000: the gate's token for the next handshake; after a 4401, a token other than the refused one opens one more socket. */
  setToken: (token: string) => void;
};

export type DuosClientOptions = {
  matchId: string;
  /** The bearer for the next open until `setToken` gives one; defaults to the tab's session. */
  token?: () => string | null;
  baseUrl?: string;
  /** Test seam. Defaults to the browser's `WebSocket`. */
  socketFactory?: SocketFactory;
  /** Test seam for the monotonic reading stamped on every clock frame. */
  monotonic?: () => number;
  /** Test seam for a ping's landing time (epoch ms). */
  now?: () => number;
  /** Test seam. Defaults to `setTimeout` / `clearTimeout`. */
  timers?: Timers;
  /** Test seam for what wakes a page (R1441), returning its unlisten; defaults to visibility and `online`. */
  wake?: (handler: () => void) => () => void;
  /** Test seam for the reconnect jitter (R1441), a draw in [0, 1). Defaults to `Math.random`. */
  random?: () => number;
};

const INITIAL: DuosSnapshot = {
  connection: "connecting",
  view: null,
  legal: [],
  jloreCapped: false,
  economy: null,
  error: null,
  errorCode: null,
  clock: null,
  teammate: null,
  pings: [],
};

function sessionToken(): string | null {
  return readSession()?.accessToken ?? null;
}

export function createDuosClient(options: DuosClientOptions): DuosClient {
  const factory = options.socketFactory ?? browserSocket;
  const monotonic = options.monotonic ?? defaultMonotonic;
  const now = options.now ?? Date.now;
  const timers = options.timers ?? defaultTimers;
  const wake = options.wake ?? pageWake;
  const random = options.random ?? Math.random;
  const tokenOf = options.token ?? sessionToken;
  const base = options.baseUrl ?? duosSocketUrl();

  const listeners = new Set<() => void>();
  let snapshot: DuosSnapshot = INITIAL;
  let socket: SocketLike | null = null;
  let stopped = false;
  const backoff = createBackoff(open, () => stopped, timers, random);
  /**
   * R1441: the page woke (its tab turned visible, the network came back). A phone that changed
   * networks or a laptop that slept can leave a socket that never reports its own end, and the
   * browser answers the server's pings without telling the page, so a quiet socket looks the same as
   * a dead one. A socket heard from lately is left alone. One that is not is asked for a full view
   * (`hello`, §9.5), and replaced if nothing comes back within `WAKE_PROBE_MS`: a healthy one answers
   * and is kept, so a tab switch never flashes the teammate's disconnect grace.
   */
  const wakeProbe = createWakeProbe({
    socket: () => socket,
    sendProbe: (current) => current.send(JSON.stringify({ type: "hello", matchId: options.matchId })),
    replace: (current) => replace(current),
    disabled: () => stopped,
    timers,
    monotonic,
    now,
    wake,
  });
  /** Set when a socket is created; its first `view` proves the server answered (§9.5, §13), so an error from before it is stale. */
  let awaitingFirstView = false;
  /** When this seat's last pings went out, for the server's own limit (B39). */
  let pingTimes: number[] = [];
  /** R1115000: the token `setToken` last gave, which every open carries in place of `tokenOf()`'s. */
  let given: string | null = null;
  /** R1115000: the token a 4401 refused while the client stays refused for it; null otherwise. */
  let refusedToken: string | null = null;

  function patch(next: Partial<DuosSnapshot>): void {
    snapshot = { ...snapshot, ...next };
    for (const listener of [...listeners]) listener();
  }

  function withPing(ping: ReceivedPing): readonly ReceivedPing[] {
    const live = snapshot.pings.filter((kept) => ping.at - kept.at < PING_SHOW_MS);
    return [...live, ping].slice(-DUOS_PINGS_KEPT);
  }

  function handle(frame: DuosServerFrame): void {
    switch (frame.type) {
      case "view": {
        // A new socket's first view clears an error from before it (a send while the socket was down).
        const fresh = awaitingFirstView;
        awaitingFirstView = false;
        patch({
          view: frame.view,
          legal: frame.legal,
          jloreCapped: frame.jloreCapped,
          economy: frame.economy ?? null,
          ...(fresh ? { error: null, errorCode: null } : {}),
        });
        return;
      }
      case "ack":
        patch({ error: null, errorCode: null });
        return;
      case "error":
        patch({ error: frame.message, errorCode: frame.code });
        return;
      case "clock":
        patch({
          clock: {
            now: frame.now,
            roundDeadline: frame.roundDeadline,
            graceDeadlines: frame.graceDeadlines,
            receivedAt: monotonic(),
          },
        });
        return;
      case "teammate":
        patch({ teammate: { connected: frame.connected, aiPlaying: frame.aiPlaying } });
        return;
      case "ping":
        patch({ pings: withPing({ from: frame.from, ping: frame.ping, target: frame.target, at: now() }) });
        return;
    }
  }

  function open(): void {
    if (stopped || socket !== null) return;
    const token = given ?? tokenOf();
    if (token === null) {
      patch({ connection: "refused", error: "Sign in again to play together.", errorCode: "forbidden" });
      return;
    }
    let created: SocketLike;
    try {
      created = factory(socketUrlFor(base, options.matchId), socketProtocols(token));
    } catch (cause) {
      patch({
        connection: "reconnecting",
        error: messageOf(cause),
        errorCode: "transport",
      });
      backoff.schedule();
      return;
    }
    socket = created;
    awaitingFirstView = true;
    wakeProbe.hear();
    if (snapshot.connection !== "reconnecting") patch({ connection: "connecting" });

    created.onopen = () => {
      if (socket !== created) return;
      wakeProbe.hear();
      backoff.reset();
      patch({ connection: "open" });
      try {
        created.send(JSON.stringify({ type: "hello", matchId: options.matchId }));
      } catch {
        // Gone between `onopen` and here; `onclose` is about to run.
      }
    };
    created.onmessage = (event) => {
      if (socket !== created) return;
      wakeProbe.hear();
      wakeProbe.cancel();
      if (typeof event.data !== "string") return;
      const frame = parseDuosFrame(event.data);
      if (frame !== null) handle(frame);
    };
    created.onerror = () => {};
    created.onclose = (event) => {
      if (socket !== created) return;
      socket = null;
      wakeProbe.cancel();
      if (stopped || event.code === VOIDED_CLOSE_CODE) {
        patch({ connection: "closed" });
        return;
      }
      if (REFUSAL_CLOSE_CODES.includes(event.code)) {
        // R1115000: a 4401 refuses this handshake's token, not the match. A token renewed before
        // the close arrived opens one more socket now; one `setToken` brings later opens it then.
        if (event.code === UNAUTHORIZED_CLOSE_CODE && given !== null && given !== token) {
          open();
          return;
        }
        refusedToken = event.code === UNAUTHORIZED_CLOSE_CODE ? token : null;
        patch({
          connection: "refused",
          error: event.reason.length > 0 ? event.reason : "the server refused this co-op socket",
          errorCode: "forbidden",
        });
        return;
      }
      patch({ connection: "reconnecting" });
      backoff.schedule();
    };
  }

  /** R1441: lets go of `stale` and opens its successor, which sends `hello` as any new socket does. */
  function replace(stale: SocketLike): void {
    socket = null;
    letGo(stale, "replaced", () => {
      wakeProbe.cancel();
      patch({ connection: "reconnecting" });
      open();
    });
  }

  /** The open socket, or null (with the board told why) when an intent cannot go out. */
  function live(): SocketLike | null {
    const current = socket;
    return current !== null && current.readyState === OPEN ? current : null;
  }

  return {
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    snapshot: () => snapshot,
    connect: () => {
      stopped = false;
      // A backoff still waiting is cut short: the socket opens now (the route's Reconnect now).
      backoff.cancel();
      wakeProbe.listen();
      open();
    },
    send: (body) => {
      const current = live();
      if (current === null) {
        // An intent formed against an older view is not replayed later.
        patch({ error: "not connected to the match", errorCode: "offline" });
        return;
      }
      patch({ error: null, errorCode: null });
      try {
        current.send(JSON.stringify({ type: "action", nonce: nextNonce(), body }));
      } catch (cause) {
        patch({ error: messageOf(cause), errorCode: "transport" });
      }
    },
    sendPing: (ping, target) => {
      const current = live();
      const seat = snapshot.view?.seat;
      if (current === null || seat === undefined) return;
      const at = now();
      pingTimes = pingTimes.filter((sent) => at - sent < PING_RATE_WINDOW_MS);
      if (pingTimes.length >= PING_RATE_LIMIT) return;
      pingTimes.push(at);
      try {
        current.send(JSON.stringify({ type: "ping", ping, target }));
      } catch {
        // Cosmetic: a ping lost with its socket is simply not seen.
        return;
      }
      patch({ pings: withPing({ from: seat, ping, target, at }) });
    },
    close: () => {
      stopped = true;
      backoff.cancel();
      wakeProbe.dispose();
      const current = socket;
      socket = null;
      if (current !== null) letGo(current, "leaving the match");
      patch({ connection: "closed" });
    },
    url: () => base,
    setToken: (next) => {
      given = next;
      if (refusedToken === null || next === refusedToken) return;
      refusedToken = null;
      // A view does not clear an error here, so the refusal's reason goes now.
      patch({ error: null, errorCode: null });
      open();
    },
  };
}

// The hook

export type UseDuosMatchOptions = Omit<DuosClientOptions, "token" | "monotonic" | "now" | "timers" | "wake" | "random"> & {
  /** The bearer the gate resolved (R194), handed to the client as `useMatch` hands its own. */
  token: string;
};

export type UseDuosMatchResult = DuosSnapshot & {
  connect: () => void;
  send: (body: DuosActionBody) => void;
  sendPing: (ping: PingKind, target: PingTarget) => void;
  url: string;
};

/** One `/ws/duos` socket for the life of the route; snapshots through `useSyncExternalStore`. */
export function useDuosMatch(options: UseDuosMatchOptions): UseDuosMatchResult {
  const { matchId, token, baseUrl, socketFactory } = options;
  const client = useMemo(
    () =>
      createDuosClient({
        matchId,
        ...(baseUrl === undefined ? {} : { baseUrl }),
        ...(socketFactory === undefined ? {} : { socketFactory }),
      }),
    [matchId, baseUrl, socketFactory],
  );
  // Before connect, so the first socket carries the gate's token; later ones reach it here (R1115000).
  useEffect(() => {
    client.setToken(token);
  }, [client, token]);
  useEffect(() => {
    client.connect();
    return () => {
      client.close();
    };
  }, [client]);
  const snapshot = useSyncExternalStore(client.subscribe, client.snapshot, client.snapshot);
  return { ...snapshot, connect: client.connect, send: client.send, sendPing: client.sendPing, url: client.url() };
}
