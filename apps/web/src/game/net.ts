// CLAUDE.md rule 7 / SPEC §9.1: this client sends intent and renders server views; it never computes legality (BUILD M5-T2).
// The handshake uses query parameters (SURFACE §11.3) and `hello` refreshes the view (§9.5).
// `MatchClocks` is structural across the deployable boundary (§9.2); view frames carry legal actions (BUILD §1, M6-T4).

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";

import type { ActionBody, Aim, EmoteId, PlayerId, PlayerView, PortraitId, PromptKind } from "@jackioh/shared";
import { isEmoteId, isPortraitId, parseAim } from "@jackioh/shared";
import { WS_PING_INTERVAL_SECONDS } from "@jackioh/server-config";

import { isEmoteHand } from "../emotes/hand.ts";
import { matchSocketUrl } from "../net/api.ts";

/** BUILD M5-T3: the dev handle exists only outside a production build. */
const DEV_ONLY = import.meta.env.MODE !== "production";

// The wire, restated structurally

/** The server's `MatchClocks`, restated across the deployable boundary. */
export type MatchClocks = {
  /** Epoch ms the active player's turn clock expires, or null while it is paused. */
  turnDeadline: number | null;
  /** Epoch ms the open prompt's own clock expires (R79), or null. */
  promptDeadline: number | null;
  /** Per-player disconnect grace deadlines (§9.5). */
  graceDeadline: { p1: number | null; p2: number | null };
  /** Epoch ms the hard ceiling is reached (R79). */
  ceilingAt: number;
};

/** `PromptMessage`, split on `forYou` exactly as §10.6 splits it. */
export type PromptFrame =
  | { forYou: true; pendingFor: PlayerId; choiceId: string; kind: PromptKind; deadline: number | null }
  | { forYou: false; pendingFor: PlayerId; deadline: number | null };

/**
 * A `clock` frame plus the monotonic reading when it landed; deadline calculations never use wall time.
 */
export type ClockReading = { now: number; clocks: MatchClocks; receivedAt: number };

/**
 * Milliseconds left on an absolute server deadline, or null when there is none.
 */
export function remainingMs(
  deadline: number | null | undefined,
  clock: ClockReading | null,
  monotonic: () => number = defaultMonotonic,
): number | null {
  if (deadline === null || deadline === undefined) return null;
  if (clock === null) return null;
  const elapsed = Math.max(0, monotonic() - clock.receivedAt);
  return Math.max(0, deadline - clock.now - elapsed);
}

function defaultMonotonic(): number {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
}

// The socket seam

/**
 * The WebSocket subset used here, for frame-level tests.
 */
export type SocketLike = {
  readonly readyState: number;
  send: (data: string) => void;
  close: (code?: number, reason?: string) => void;
  onopen: ((event: unknown) => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null;
  onerror: ((event: unknown) => void) | null;
};

export type SocketFactory = (url: string) => SocketLike;

/** `WebSocket.OPEN`, spelled out so this module needs no DOM constant at runtime. */
const OPEN = 1;

function browserSocket(url: string): SocketLike {
  return new WebSocket(url) as unknown as SocketLike;
}

/**
 * Refusal close codes; retrying one would loop against a settled server answer.
 */
const REFUSAL_CLOSE_CODES: readonly number[] = [4401, 4403, 4404];

/**
 * R679: a voided match has no socket to reconnect to.
 */
const VOIDED_CLOSE_CODE = 4410;

/** A short backoff; the last entry repeats. Spec 05 reloads the page, so a resume is a fresh boot. */
const RECONNECT_DELAYS_MS: readonly number[] = [250, 500, 1000, 2000, 5000];

/**
 * R1441: each backoff gains up to this fraction of itself, drawn at random, so every client a
 * server restart or a network blip dropped at once does not come back on the same beat. It only
 * adds time, so the first retry never comes before its 250 ms.
 */
const RECONNECT_JITTER = 0.25;

/**
 * R1441: how long a socket may have said nothing before a woken page (the tab turned visible, the
 * network came back) doubts it: the server's ping interval, since the server pings that often and
 * the browser answers without telling the page. A socket that is still there but quiet (a long
 * turn) is no reason to drop it; this is only the point past which it is worth asking.
 */
const SILENT_SOCKET_MS = WS_PING_INTERVAL_SECONDS * 1000;

/**
 * R1441: how long a woken page waits for the answer to its probe (a `hello`, which the actor
 * answers with a full view) before it replaces the socket.
 */
const WAKE_PROBE_MS = 5000;

export type Timers = {
  setTimeout: (handler: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
};

const defaultTimers: Timers = {
  setTimeout: (handler, ms) => setTimeout(handler, ms),
  clearTimeout: (handle) => {
    clearTimeout(handle as ReturnType<typeof setTimeout>);
  },
};

/**
 * R1441: calls `handler` when the page may have been asleep or offline: its tab turns visible or the
 * network comes back (as `settings/accountSync.ts` listens for). Returns the function that stops
 * listening.
 */
function pageWake(handler: () => void): () => void {
  if (typeof window === "undefined" || typeof document === "undefined") return () => undefined;
  const onVisibility = (): void => {
    if (document.visibilityState === "visible") handler();
  };
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("online", handler);
  return () => {
    document.removeEventListener("visibilitychange", onVisibility);
    window.removeEventListener("online", handler);
  };
}

// Frame parsing: total and protocol-shaped

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isString = (value: unknown): value is string => typeof value === "string";

const isNumberOrNull = (value: unknown): value is number | null =>
  value === null || typeof value === "number";

/**
 * Deliberately shallow (§9.3): validation here would duplicate the engine's action union.
 */
function parseLegal(value: unknown): readonly ActionBody[] | null {
  if (!Array.isArray(value)) return null;
  const out: ActionBody[] = [];
  for (const entry of value) {
    if (isRecord(entry) && isString(entry.type)) out.push(entry as unknown as ActionBody);
  }
  return out;
}

function parseClocks(value: unknown): MatchClocks | null {
  if (!isRecord(value)) return null;
  const grace = value.graceDeadline;
  if (!isRecord(grace)) return null;
  if (!isNumberOrNull(value.turnDeadline) || !isNumberOrNull(value.promptDeadline)) return null;
  if (!isNumberOrNull(grace.p1) || !isNumberOrNull(grace.p2)) return null;
  if (typeof value.ceilingAt !== "number") return null;
  return {
    turnDeadline: value.turnDeadline,
    promptDeadline: value.promptDeadline,
    graceDeadline: { p1: grace.p1, p2: grace.p2 },
    ceilingAt: value.ceilingAt,
  };
}

/** Every server frame this client understands, after parsing. */
export type ServerFrame =
  | { type: "view"; view: PlayerView; legal: readonly ActionBody[] | null }
  | { type: "ack"; nonce: string; seq: number }
  | { type: "error"; code: string; message: string; nonce?: string }
  | { type: "prompt"; prompt: PromptFrame }
  | { type: "clock"; now: number; clocks: MatchClocks }
  /** R642: both portraits. R1342: the account's dealt emote hand, if any. */
  | { type: "portraits"; p1: PortraitId; p2: PortraitId; emotes: EmoteId[] | null }
  /** R643: an emote the opponent sent, relayed by the actor. */
  | { type: "emote"; from: PlayerId; emote: EmoteId }
  /** R738: what the opponent is aiming at now, or null when its aim has ended. */
  | { type: "aim"; from: PlayerId; aim: Aim | null };

/**
 * Parse one text frame; unknown frames must not take the board down.
 */
export function parseServerFrame(text: string): ServerFrame | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isRecord(parsed) || !isString(parsed.type)) return null;

  switch (parsed.type) {
    case "view": {
      // §10.8: the server shapes PlayerView; the client only checks its presence.
      if (!isRecord(parsed.view)) return null;
      return {
        type: "view",
        view: parsed.view as unknown as PlayerView,
        // `legal` riding alongside the view, which is what the actor sends (see the header).
        legal: parsed.legal === undefined ? null : parseLegal(parsed.legal),
      };
    }
    case "ack": {
      if (!isString(parsed.nonce) || typeof parsed.seq !== "number") return null;
      return { type: "ack", nonce: parsed.nonce, seq: parsed.seq };
    }
    case "error": {
      if (!isString(parsed.code) || !isString(parsed.message)) return null;
      return {
        type: "error",
        code: parsed.code,
        message: parsed.message,
        ...(isString(parsed.nonce) ? { nonce: parsed.nonce } : {}),
      };
    }
    case "prompt": {
      const pendingFor = parsed.pendingFor;
      if (pendingFor !== "p1" && pendingFor !== "p2") return null;
      const deadline = isNumberOrNull(parsed.deadline) ? parsed.deadline : null;
      if (parsed.forYou === true) {
        if (!isString(parsed.choiceId) || !isString(parsed.kind)) return null;
        return {
          type: "prompt",
          prompt: {
            forYou: true,
            pendingFor,
            choiceId: parsed.choiceId,
            kind: parsed.kind as PromptKind,
            deadline,
          },
        };
      }
      return { type: "prompt", prompt: { forYou: false, pendingFor, deadline } };
    }
    case "clock": {
      const clocks = parseClocks(parsed.clocks);
      if (clocks === null || typeof parsed.now !== "number") return null;
      return { type: "clock", now: parsed.now, clocks };
    }
    case "portraits": {
      if (!isPortraitId(parsed.p1) || !isPortraitId(parsed.p2)) return null;
      // R1342: absent or null means no dealt hand.
      const emotes = parsed.emotes ?? null;
      if (emotes !== null && !isEmoteHand(emotes)) return null;
      return { type: "portraits", p1: parsed.p1, p2: parsed.p2, emotes };
    }
    case "emote": {
      if (!isEmoteId(parsed.emote)) return null;
      const from = parsed.from;
      if (from !== "p1" && from !== "p2") return null;
      return { type: "emote", from, emote: parsed.emote };
    }
    case "aim": {
      const aim = parseAim(parsed.aim);
      const from = parsed.from;
      if (aim === undefined || (from !== "p1" && from !== "p2")) return null;
      return { type: "aim", from, aim };
    }
    default:
      return null;
  }
}

// Nonces (SPEC §9.3)

let nonceCounter = 0;

/** Unique within a tab and across reconnects; well under `MAX_NONCE_LENGTH` (128). */
export function nextNonce(): string {
  nonceCounter += 1;
  const random = Math.random().toString(36).slice(2, 10);
  return `c${Date.now().toString(36)}-${String(nonceCounter)}-${random}`;
}

// The client

export type ConnectionState =
  /** A socket is being opened, or a backoff is waiting to open one. */
  | "connecting"
  /** The socket is open and `hello` has been sent. */
  | "open"
  /** The socket dropped and a reconnect is pending. */
  | "reconnecting"
  /** The server refused this socket (4401/4403/4404); no reconnect will be attempted. */
  | "refused"
  /** `close()` was called. */
  | "closed";

/**
 * `"none"` leaves the board read-only until a server view provides legal actions.
 */
export type LegalSource = "none" | "view";

export type MatchSnapshot = {
  connection: ConnectionState;
  /** The last `PlayerView` pushed, or null before the first one. */
  view: PlayerView | null;
  /** Empty until a server sends one; never computed here (CLAUDE.md rule 7, BUILD M5-T2). */
  legal: readonly ActionBody[];
  legalSource: LegalSource;
  /** The last `error` frame's message, relayed verbatim (§9.3), or null. */
  error: string | null;
  /** The last `error` frame's code, for a caller that wants to branch without parsing prose. */
  errorCode: string | null;
  /** The last `clock` frame with the monotonic reading it landed at. */
  clock: ClockReading | null;
  /** The last `prompt` frame. The board renders `view.pending`; this carries R79's deadline. */
  prompt: PromptFrame | null;
  /** The last `ack`: the nonce the actor accepted and the log seq it wrote it at. */
  ack: { nonce: string; seq: number } | null;
  /** R642: both seats' portraits, null until the first `portraits` frame arrives. */
  portraits: { p1: PortraitId; p2: PortraitId } | null;
  /** R1342: this account's own emote hand, from the last `portraits` frame; null until one deals it. */
  emoteHand: EmoteId[] | null;
  /** R643: the last opponent emote; `seq` makes repeats notify. */
  emote: { from: PlayerId; emote: EmoteId; seq: number } | null;
  /** R738: the opponent's aim as last relayed; null when none is up or the socket has dropped. */
  aim: { from: PlayerId; aim: Aim } | null;
};

export type MatchClient = {
  subscribe: (listener: () => void) => () => void;
  snapshot: () => MatchSnapshot;
  /** Open the socket (idempotent while one is open or pending). */
  connect: () => void;
  /** Send one action. The nonce is minted here; `playerId` is never sent (the actor stamps it). */
  send: (body: ActionBody) => void;
  /** R643: send one emote without nonce or ack. */
  sendEmote: (emote: EmoteId) => void;
  /** R738: tell the opponent what this seat is aiming at (null: the aim ended). Never answered. */
  sendAim: (aim: Aim | null) => void;
  /** Close for good: no reconnect until `connect()` is called again. */
  close: () => void;
  /** The URL the next socket will open, for a diagnostic panel. */
  url: () => string;
  /** R194: changes the token for the next handshake without reopening a healthy socket. */
  setToken: (token: string) => void;
};

export type MatchClientOptions = {
  matchId: string;
  token: string;
  /** Defaults to `matchSocketUrl()` (`apps/web/src/net/api.ts`). */
  baseUrl?: string;
  /** Test seam. Defaults to the browser's `WebSocket`. */
  socketFactory?: SocketFactory;
  /** Test seam. Defaults to `setTimeout` / `clearTimeout`. */
  timers?: Timers;
  /** Test seam for the monotonic reading stamped on every `clock` frame. */
  monotonic?: () => number;
  /**
   * Test seam for what wakes a sleeping page (R1441): takes the handler, returns the function that
   * stops listening. Defaults to the tab turning visible and the `online` event.
   */
  wake?: (handler: () => void) => () => void;
  /** Test seam for the reconnect jitter (R1441), a draw in [0, 1). Defaults to `Math.random`. */
  random?: () => number;
};

const EMPTY_LEGAL: readonly ActionBody[] = [];

const INITIAL: MatchSnapshot = {
  connection: "connecting",
  view: null,
  legal: EMPTY_LEGAL,
  legalSource: "none",
  error: null,
  errorCode: null,
  clock: null,
  prompt: null,
  ack: null,
  portraits: null,
  emoteHand: null,
  emote: null,
  aim: null,
};

/**
 * Browsers cannot set a WebSocket authorization header, so the handshake uses query parameters.
 */
export function socketUrlFor(baseUrl: string, token: string, matchId: string): string {
  try {
    const url = new URL(baseUrl);
    url.searchParams.set("token", token);
    url.searchParams.set("matchId", matchId);
    return url.toString();
  } catch {
    // Relative test URLs still need their query string.
    const separator = baseUrl.includes("?") ? "&" : "?";
    return `${baseUrl}${separator}token=${encodeURIComponent(token)}&matchId=${encodeURIComponent(matchId)}`;
  }
}

export function createMatchClient(options: MatchClientOptions): MatchClient {
  const factory = options.socketFactory ?? browserSocket;
  const timers = options.timers ?? defaultTimers;
  const monotonic = options.monotonic ?? defaultMonotonic;
  const wake = options.wake ?? pageWake;
  const random = options.random ?? Math.random;
  const base = options.baseUrl ?? matchSocketUrl();
  /** Every subsequently opened socket carries this token. */
  let token = options.token;

  const listeners = new Set<() => void>();
  let snapshot: MatchSnapshot = INITIAL;
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
  /** A deliberate close must not reconnect. */
  let stopped = false;
  /** R643: repeated emotes still notify. */
  let emoteSeq = 0;

  function emit(): void {
    for (const listener of [...listeners]) listener();
  }

  function patch(next: Partial<MatchSnapshot>): void {
    snapshot = { ...snapshot, ...next };
    emit();
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

  function scheduleRetry(): void {
    if (stopped || pendingRetry !== null) return;
    const step = RECONNECT_DELAYS_MS[Math.min(retry, RECONNECT_DELAYS_MS.length - 1)] ?? 5000;
    const delay = Math.floor(step * (1 + RECONNECT_JITTER * random()));
    retry += 1;
    pendingRetry = timers.setTimeout(() => {
      pendingRetry = null;
      open();
    }, delay);
  }

  function handleFrame(frame: ServerFrame): void {
    switch (frame.type) {
      case "view":
        patch({
          view: frame.view,
          // A view without `legal` does not blank an earlier legal-action array.
          ...(frame.legal === null ? {} : { legal: frame.legal, legalSource: "view" as const }),
        });
        return;
      case "ack":
        patch({ ack: { nonce: frame.nonce, seq: frame.seq }, error: null, errorCode: null });
        return;
      case "error":
        // §9.3: relay the reducer's reason verbatim.
        patch({ error: frame.message, errorCode: frame.code });
        return;
      case "prompt":
        patch({ prompt: frame.prompt });
        return;
      case "clock":
        patch({ clock: { now: frame.now, clocks: frame.clocks, receivedAt: monotonic() } });
        return;
      case "portraits":
        patch({ portraits: { p1: frame.p1, p2: frame.p2 }, emoteHand: frame.emotes });
        return;
      case "emote":
        emoteSeq += 1;
        patch({ emote: { from: frame.from, emote: frame.emote, seq: emoteSeq } });
        return;
      case "aim":
        patch({ aim: frame.aim === null ? null : { from: frame.from, aim: frame.aim } });
        return;
    }
  }

  function open(): void {
    if (stopped || socket !== null) return;

    let created: SocketLike;
    try {
      created = factory(socketUrlFor(base, token, options.matchId));
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
      // §9.5: `hello` requests a fresh full view.
      try {
        created.send(JSON.stringify({ type: "hello", token, matchId: options.matchId }));
      } catch {
        // The peer went away between `onopen` and here; `onclose` is about to run.
      }
    };

    created.onmessage = (event) => {
      if (socket !== created) return;
      hear();
      cancelProbe();
      if (typeof event.data !== "string") return; // Text frames only (`socketFromWs`).
      const frame = parseServerFrame(event.data);
      if (frame !== null) handleFrame(frame);
    };

    created.onerror = () => {
    };

    created.onclose = (event) => {
      if (socket !== created) return;
      socket = null;
      cancelProbe();
      // R738: clear an aim from before the drop.
      if (snapshot.aim !== null) patch({ aim: null });
      if (stopped) {
        patch({ connection: "closed" });
        return;
      }
      if (event.code === VOIDED_CLOSE_CODE) {
        patch({ connection: "closed" });
        return;
      }
      if (REFUSAL_CLOSE_CODES.includes(event.code)) {
        // Retrying a settled refusal would loop.
        patch({
          connection: "refused",
          error: event.reason.length > 0 ? event.reason : "the server refused this match socket",
          errorCode: "forbidden",
        });
        return;
      }
      patch({ connection: "reconnecting" });
      scheduleRetry();
    };
  }

  /**
   * R1441: lets go of `live` without waiting for it (a dead peer never answers a close) and opens
   * its successor, which sends `hello` as any new socket does. The new socket opens before the old
   * one is closed, so the server meets the successor first when both reach it.
   */
  function replace(live: SocketLike): void {
    socket = null;
    live.onopen = null;
    live.onmessage = null;
    live.onerror = null;
    live.onclose = null;
    cancelProbe();
    patch({ connection: "reconnecting", ...(snapshot.aim === null ? {} : { aim: null }) });
    open();
    try {
      live.close(1000, "replaced");
    } catch {
      // Already gone.
    }
  }

  function hear(): void {
    heardAt = monotonic();
    heardWall = Date.now();
  }

  /** How long the open socket has been quiet: the larger of the two clocks' readings, so a sleep counts. */
  function silentFor(): number {
    return Math.max(monotonic() - heardAt, Date.now() - heardWall);
  }

  /**
   * R1441: the page woke (its tab turned visible, the network came back). A phone that changed
   * networks or a laptop that slept can leave a socket that never reports its own end, and the
   * browser answers the server's pings without telling the page, so a quiet socket looks the same as
   * a dead one. A socket heard from lately is left alone. One that is not is asked for a full view
   * (`hello`, §9.5), and replaced if nothing comes back within `WAKE_PROBE_MS`: a healthy one answers
   * and is kept, so a tab switch never flashes the opponent's disconnect grace.
   */
  function onWake(): void {
    const live = socket;
    if (stopped || live === null || probe !== null) return;
    if (silentFor() <= SILENT_SOCKET_MS) return;
    if (live.readyState !== OPEN) {
      replace(live);
      return;
    }
    try {
      live.send(JSON.stringify({ type: "hello", token, matchId: options.matchId }));
    } catch {
      replace(live);
      return;
    }
    probe = timers.setTimeout(() => {
      probe = null;
      if (socket === live) replace(live);
    }, WAKE_PROBE_MS);
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
      unwake ??= wake(onWake);
      open();
    },
    send: (body) => {
      const live = socket;
      if (live === null || live.readyState !== OPEN) {
        // Do not replay an intent formed against an older view.
        patch({ error: "not connected to the match", errorCode: "offline" });
        return;
      }
      // The actor stamps the seat; never send `playerId`.
      const frame = JSON.stringify({ type: "action", action: { ...body, nonce: nextNonce() } });
      patch({ error: null, errorCode: null });
      try {
        live.send(frame);
      } catch (cause) {
        patch({ error: cause instanceof Error ? cause.message : String(cause), errorCode: "transport" });
      }
    },
    sendEmote: (emote) => {
      const live = socket;
      // R643: a dead socket drops cosmetic emotes.
      if (live === null || live.readyState !== OPEN) return;
      try {
        live.send(JSON.stringify({ type: "emote", emote }));
      } catch {
        // Gone between the check and the send; `onclose` reports the connection, not the emote.
      }
    },
    sendAim: (aim) => {
      const live = socket;
      // R738: a dead socket drops cosmetic aim updates.
      if (live === null || live.readyState !== OPEN) return;
      try {
        live.send(JSON.stringify({ type: "aim", aim }));
      } catch {
        // Gone between the check and the send; `onclose` reports the connection.
      }
    },
    close: () => {
      stopped = true;
      cancelRetry();
      cancelProbe();
      unwake?.();
      unwake = null;
      const live = socket;
      socket = null;
      if (live !== null) {
        live.onopen = null;
        live.onmessage = null;
        live.onerror = null;
        live.onclose = null;
        try {
          live.close(1000, "leaving the match");
        } catch {
          // Already gone.
        }
      }
      patch({ connection: "closed" });
    },
    url: () => socketUrlFor(base, token, options.matchId),
    setToken: (next) => {
      token = next;
    },
  };
}

// The dev handle (BUILD M5-T3, consumed by e2e/support/commands.ts)

/**
 * Networked `window.__jackioh` is a PlayerView-derived shim, never GameState: §9.1 hides its state.
 * `e2e/support/types.ts` cites its `seat`; the empty seed protects §9.3 reconstruction, and log/decks stay absent (ASSUMPTION A2).
 */
export type ViewDerivedState = {
  seed: string;
  turn: number;
  active: PlayerId;
  phase: PlayerView["phase"];
  result: PlayerView["result"];
  pending: { choiceId?: string; kind?: PromptKind; player?: PlayerId } | null;
  players: Record<PlayerId, unknown>;
};

export function viewDerivedState(view: PlayerView): ViewDerivedState {
  const players: Record<PlayerId, unknown> = { p1: null, p2: null };
  players[view.you.player] = view.you;
  players[view.opponent.player] = view.opponent;

  const pending =
    view.pending === null
      ? null
      : view.pending.forYou
        ? { choiceId: view.pending.choiceId, kind: view.pending.kind, player: view.viewer }
        : { player: view.pending.pendingFor };

  return {
    seed: "",
    turn: view.turn,
    active: view.active,
    phase: view.phase,
    result: view.result,
    pending,
    players,
  };
}

export type NetDevHandle = {
  readonly state: ViewDerivedState | null;
  readonly seed: string;
  readonly seat: PlayerId | null;
  dispatch: (action: ActionBody & { playerId?: PlayerId }) => void;
};

/** The networked counterpart to the hotseat `window.__jackioh` handle. */
type DevWindow = { __jackioh?: unknown };

/**
 * Publish outside production; teardown avoids clobbering the hotseat handle.
 */
export function installDevHandle(handle: NetDevHandle): () => void {
  if (!DEV_ONLY || typeof window === "undefined") return () => {};
  const host = window as unknown as DevWindow;
  host.__jackioh = handle;
  return () => {
    if (host.__jackioh === handle) delete host.__jackioh;
  };
}

// The hook

export type UseMatchOptions = {
  matchId: string;
  token: string;
  baseUrl?: string;
  socketFactory?: SocketFactory;
  timers?: Timers;
  monotonic?: () => number;
};

export type UseMatchResult = MatchSnapshot & {
  /** Send one action to the actor. */
  send: (body: ActionBody) => void;
  /** R643: send one emote; the local board shows it at once and the server relays it on. */
  sendEmote: (emote: EmoteId) => void;
  /** R738: send this seat's aim (null: the aim ended); the server relays it to the opponent. */
  sendAim: (aim: Aim | null) => void;
  /** The handshake URL, for the diagnostic line on the match route. */
  url: string;
};

/**
 * One socket per match; snapshots use `useSyncExternalStore`. R194 tokens apply on the next handshake.
 */
export function useMatch(options: UseMatchOptions): UseMatchResult {
  const { matchId, token, baseUrl, socketFactory, timers, monotonic } = options;
  // Later tokens reach the client through `setToken`.
  const [firstToken] = useState(token);

  const client = useMemo(
    () =>
      createMatchClient({
        matchId,
        token: firstToken,
        ...(baseUrl === undefined ? {} : { baseUrl }),
        ...(socketFactory === undefined ? {} : { socketFactory }),
        ...(timers === undefined ? {} : { timers }),
        ...(monotonic === undefined ? {} : { monotonic }),
      }),
    [matchId, firstToken, baseUrl, socketFactory, timers, monotonic],
  );

  // This runs before connect, so a new match uses the current token.
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

  return { ...snapshot, send: client.send, sendEmote: client.sendEmote, sendAim: client.sendAim, url: client.url() };
}
