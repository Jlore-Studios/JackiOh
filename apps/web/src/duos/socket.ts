// The `/ws/duos` client (#936 SPEC S6): one socket per co-op match, the way `game/net.ts` is one per
// 1v1 match. CLAUDE.md rule 7: it sends intent (a `DuosActionBody`, a ping) and keeps what the server
// pushed — the seat's `DuosView` and its `legal` list, never computed here — for the board to render.
//
// The handshake is `/ws/match`'s (auth as `/ws/match`, S6): the token and the match id ride in the
// query string, since a browser cannot set a socket's headers, and `hello` asks for a fresh view. The
// token is the gate's, given through `setToken` as `game/net.ts` takes it (the tab's session until one
// is given, `net/session.ts`), so a reconnect after a renewal carries the new one, and a 4401 is final
// only for the token it refused (R1115000).
//
// A drop reconnects with `game/net.ts`'s backoff and jitter (R1441); a refusal (4401/4403/4404) and
// a voided match (4410) do not. A woken page (the tab turned visible, the network came back) asks a
// socket quiet past the server's ping interval for a view with `hello`, and replaces it if nothing
// answers within `WAKE_PROBE_MS`, as `game/net.ts` does (R1441). Pings are cosmetic (B39): one sent while the socket is down is dropped,
// and one past the server's own limit (PING_RATE_LIMIT per PING_RATE_WINDOW_MS) is not sent at all,
// as the server would drop it silently. The seat's own ping is kept beside the relayed ones, so the
// board marks it for the sender too.

import { useEffect, useMemo, useSyncExternalStore } from "react";

import { PING_RATE_LIMIT, PING_RATE_WINDOW_MS, PING_SHOW_MS } from "@jackioh/server-config";

import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import {
  RECONNECT_DELAYS_MS,
  RECONNECT_JITTER,
  REFUSAL_CLOSE_CODES,
  SILENT_SOCKET_MS,
  UNAUTHORIZED_CLOSE_CODE,
  VOIDED_CLOSE_CODE,
  WAKE_PROBE_MS,
  defaultTimers,
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
import { DUOS_PINGS_KEPT } from "./config.ts";
import { isDuosSeat, isPingKind, parsePingTarget } from "./pings.ts";

/** `WebSocket.OPEN`, spelled out so this module needs no DOM constant at runtime. */
const OPEN = 1;

/** The Duos socket's path, beside `/ws/match` (S6). */
const DUOS_PATH = "/ws/duos";

/**
 * The `/ws/duos` URL on the same server as `/ws/match` (`VITE_SERVER_WS_URL`, `net/api.ts`): the
 * configured URL with its path swapped, so one environment variable still names the server.
 */
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
  | { type: "view"; view: DuosView; legal: readonly DuosActionBody[] }
  | { type: "ack"; nonce: string; seq: number }
  | { type: "error"; code: string; message: string; nonce?: string }
  | ({ type: "clock" } & DuosClockFrame)
  | { type: "teammate"; connected: boolean; aiPlaying: boolean }
  | { type: "ping"; from: DuosSeat; ping: PingKind; target: PingTarget };

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

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
      return { type: "view", view: parsed.view as unknown as DuosView, legal: parseLegal(parsed.legal) };
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
  /** The seat's last `DuosView`, or null before the first one. */
  view: DuosView | null;
  /** The seat's legal actions as the server last listed them; never computed here (rule 7). */
  legal: readonly DuosActionBody[];
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
  /** Defaults to `duosSocketUrl()`. */
  baseUrl?: string;
  /** Test seam. Defaults to the browser's `WebSocket`. */
  socketFactory?: SocketFactory;
  /** Test seam for the monotonic reading stamped on every clock frame. */
  monotonic?: () => number;
  /** Test seam for a ping's landing time (epoch ms). */
  now?: () => number;
  /** Test seam. Defaults to `setTimeout` / `clearTimeout`. */
  timers?: Timers;
  /**
   * Test seam for what wakes a sleeping page (R1441): takes the handler, returns the function that
   * stops listening. Defaults to the tab turning visible and the `online` event.
   */
  wake?: (handler: () => void) => () => void;
  /** Test seam for the reconnect jitter (R1441), a draw in [0, 1). Defaults to `Math.random`. */
  random?: () => number;
};

const INITIAL: DuosSnapshot = {
  connection: "connecting",
  view: null,
  legal: [],
  error: null,
  errorCode: null,
  clock: null,
  teammate: null,
  pings: [],
};

function browserSocket(url: string): SocketLike {
  return new WebSocket(url) as unknown as SocketLike;
}

function defaultMonotonic(): number {
  return typeof performance !== "undefined" && typeof performance.now === "function" ? performance.now() : Date.now();
}

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
  let retry = 0;
  let pendingRetry: unknown = null;
  /** R1441: the monotonic reading at which the open socket was created or last heard from. */
  let heardAt = 0;
  /**
   * R1441: the wall clock at the same instant. The monotonic clock leaves a sleeping machine's time
   * out on most platforms, so a socket heard just before a laptop slept would read as recent on
   * waking; the wall clock keeps counting.
   */
  let heardWall = 0;
  /** R1441: the timer of a wake probe still waiting for its answer. */
  let probe: unknown = null;
  /** R1441: stops listening for the page waking; null while it is not. */
  let unwake: (() => void) | null = null;
  let stopped = false;
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

  function urlWith(token: string): string {
    return socketUrlFor(base, token, options.matchId);
  }

  /** The pings still worth marking, plus `ping`, at most DUOS_PINGS_KEPT. */
  function withPing(ping: ReceivedPing): readonly ReceivedPing[] {
    const live = snapshot.pings.filter((kept) => ping.at - kept.at < PING_SHOW_MS);
    return [...live, ping].slice(-DUOS_PINGS_KEPT);
  }

  function cancelRetry(): void {
    if (pendingRetry === null) return;
    timers.clearTimeout(pendingRetry);
    pendingRetry = null;
  }

  function cancelProbe(): void {
    if (probe === null) return;
    timers.clearTimeout(probe);
    probe = null;
  }

  function hear(): void {
    heardAt = monotonic();
    heardWall = now();
  }

  /** How long the open socket has been quiet: the larger of the two clocks' readings, so a sleep counts. */
  function silentFor(): number {
    return Math.max(monotonic() - heardAt, now() - heardWall);
  }

  function scheduleRetry(): void {
    if (stopped || pendingRetry !== null) return;
    const step = RECONNECT_DELAYS_MS[Math.min(retry, RECONNECT_DELAYS_MS.length - 1)] ?? 0;
    const delay = Math.floor(step * (1 + RECONNECT_JITTER * random()));
    retry += 1;
    pendingRetry = timers.setTimeout(() => {
      pendingRetry = null;
      open();
    }, delay);
  }

  function handle(frame: DuosServerFrame): void {
    switch (frame.type) {
      case "view":
        patch({ view: frame.view, legal: frame.legal });
        return;
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
      created = factory(urlWith(token));
    } catch (cause) {
      patch({
        connection: "reconnecting",
        error: cause instanceof Error ? cause.message : String(cause),
        errorCode: "transport",
      });
      scheduleRetry();
      return;
    }
    socket = created;
    hear();
    if (snapshot.connection !== "reconnecting") patch({ connection: "connecting" });

    created.onopen = () => {
      if (socket !== created) return;
      hear();
      retry = 0;
      patch({ connection: "open" });
      try {
        created.send(JSON.stringify({ type: "hello", matchId: options.matchId }));
      } catch {
        // Gone between `onopen` and here; `onclose` is about to run.
      }
    };
    created.onmessage = (event) => {
      if (socket !== created) return;
      hear();
      cancelProbe();
      if (typeof event.data !== "string") return;
      const frame = parseDuosFrame(event.data);
      if (frame !== null) handle(frame);
    };
    created.onerror = () => {};
    created.onclose = (event) => {
      if (socket !== created) return;
      socket = null;
      cancelProbe();
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
      scheduleRetry();
    };
  }

  /**
   * R1441: lets go of `stale` without waiting for it (a dead peer never answers a close) and opens
   * its successor, which sends `hello` as any new socket does. The new socket opens before the old
   * one is closed, so the server meets the successor first when both reach it.
   */
  function replace(stale: SocketLike): void {
    socket = null;
    stale.onopen = null;
    stale.onmessage = null;
    stale.onerror = null;
    stale.onclose = null;
    cancelProbe();
    patch({ connection: "reconnecting" });
    open();
    try {
      stale.close(1000, "replaced");
    } catch {
      // Already gone.
    }
  }

  /**
   * R1441: the page woke (its tab turned visible, the network came back). A phone that changed
   * networks or a laptop that slept can leave a socket that never reports its own end, and the
   * browser answers the server's pings without telling the page, so a quiet socket looks the same as
   * a dead one. A socket heard from lately is left alone. One that is not is asked for a full view
   * (`hello`, §9.5), and replaced if nothing comes back within `WAKE_PROBE_MS`: a healthy one answers
   * and is kept, so a tab switch never flashes the teammate's disconnect grace.
   */
  function onWake(): void {
    const current = socket;
    if (stopped || current === null || probe !== null) return;
    if (silentFor() <= SILENT_SOCKET_MS) return;
    if (current.readyState !== OPEN) {
      replace(current);
      return;
    }
    try {
      current.send(JSON.stringify({ type: "hello", matchId: options.matchId }));
    } catch {
      replace(current);
      return;
    }
    probe = timers.setTimeout(() => {
      probe = null;
      if (socket === current) replace(current);
    }, WAKE_PROBE_MS);
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
      cancelRetry();
      unwake ??= wake(onWake);
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
        patch({ error: cause instanceof Error ? cause.message : String(cause), errorCode: "transport" });
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
      cancelRetry();
      cancelProbe();
      unwake?.();
      unwake = null;
      const current = socket;
      socket = null;
      if (current !== null) {
        current.onopen = null;
        current.onmessage = null;
        current.onerror = null;
        current.onclose = null;
        try {
          current.close(1000, "leaving the match");
        } catch {
          // Already gone.
        }
      }
      patch({ connection: "closed" });
    },
    url: () => base,
    setToken: (next) => {
      given = next;
      // R1115000: after a 4401, a token other than the one it refused opens one more socket.
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
  /** Open a socket now rather than at the end of the backoff (the route's Reconnect now). */
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
