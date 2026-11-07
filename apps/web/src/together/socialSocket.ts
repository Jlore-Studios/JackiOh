// The social socket (R3214000): one `/ws/social` per signed-in tab, which the server writes to when the
// tab's challenges, unread counts, chat or friends list change, so the social poll and an open chat's
// poll (`socialPoll.ts`, `ChatPanel.tsx`) send nothing while it is up. A `social` frame is the tab's own
// `GET /api/social` answer; a `chat` frame names the friend a message came from or went to, never its
// words; a `friends` frame says the friends list changed. The tab sends only `hello`, which the server
// answers with a `social` frame.
//
// The handshake is `/ws/match`'s: the token as an offered subprotocol (R1379000), never in the
// address. A drop reopens on the match socket's backoff and a woken page probes a quiet socket (R1441);
// a 4401 reopens at once with a renewed token (R1115000), and a refusal waits for another token.
// CLAUDE.md rule 7: nothing here is a rule, and the server sends a tab only what it may read.

import {
  REFUSAL_CLOSE_CODES,
  UNAUTHORIZED_CLOSE_CODE,
  browserSocket,
  createBackoff,
  createWakeProbe,
  defaultMonotonic,
  defaultTimers,
  letGo,
  pageWake,
  type SocketFactory,
  type SocketLike,
  type Timers,
} from "../game/net.ts";
import { serverSocketUrl } from "../net/api.ts";
import type { Challenge, SocialRead } from "../net/social.ts";
import { socketProtocols } from "../net/socketProtocols.ts";
import { isRecord } from "../wire/json.ts";

/** Must match `crates/server/src/actor/ws_server.rs`'s `SOCIAL_PATH`. */
export const SOCIAL_SOCKET_PATH = "/ws/social";

/** The one frame the tab sends: the server answers it with a `social` frame (R1441's wake probe hears it). */
const HELLO = JSON.stringify({ type: "hello" });

/** What the server pushed, as far as the client reads it. */
export type SocialPush =
  /** The tab's own `GET /api/social` answer; `socialPoll.ts` checks each entry as it checks the read's. */
  | { kind: "social"; read: SocialRead }
  /** A chat message to or from `friendId`: the open conversation with them reads its newest page. */
  | { kind: "chat"; friendId: string }
  /** The friends list changed: an open list reads again. */
  | { kind: "friends" };

/** One text frame, or null for anything this client does not read (an error frame, junk). */
export function parseSocialPush(data: unknown): SocialPush | null {
  if (typeof data !== "string") return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(data);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  switch (parsed.type) {
    case "social": {
      const { incoming, outgoing, unread } = parsed;
      if (!Array.isArray(incoming) || !Array.isArray(outgoing) || !isRecord(unread)) return null;
      return {
        kind: "social",
        read: { incoming: incoming as Challenge[], outgoing: outgoing as Challenge[], unread: unread as Record<string, number> },
      };
    }
    case "chat":
      return typeof parsed.friendId === "string" ? { kind: "chat", friendId: parsed.friendId } : null;
    case "friends":
      return { kind: "friends" };
    default:
      return null;
  }
}

/**
 * The factory the tab's social socket is opened with; null opens none. It starts off under vitest
 * (`MODE` is `"test"` only there, as `main.tsx` says), since some tests stub `WebSocket` and count the
 * sockets a page opens; a test that drives the socket installs its own and puts null back. Not set in
 * `test/setup.ts`: a module a setup file imports is cached before a test file's `vi.mock` applies, so
 * this module's imports (`cards/cosmetics.ts` through `game/net.ts`) would never see a mocked one.
 */
let factory: SocketFactory | null = import.meta.env.MODE === "test" ? null : browserSocket;

/** Sets the factory the next `socialPoll.ts` start opens its socket with, or turns the socket off (null). */
export function setSocialSocketFactory(next: SocketFactory | null): void {
  factory = next;
}

export function socialSocketFactory(): SocketFactory | null {
  return factory;
}

export type SocialSocketOptions = {
  /** The bearer for each open: the tab's session, read again every time. */
  token: () => string | null;
  onPush: (push: SocialPush) => void;
  /** The socket turned live (its first `social` frame) or stopped being (it dropped or was replaced). */
  onLive: (live: boolean) => void;
  /** Test seam. Defaults to the browser's `WebSocket`. */
  socketFactory?: SocketFactory;
  url?: string;
  /** Test seam. Defaults to `setTimeout` / `clearTimeout`. */
  timers?: Timers;
  /** The reconnect jitter's draw (R1441), in [0, 1). */
  random?: () => number;
  /** Test seam for what wakes a page (R1441), returning its unlisten. */
  wake?: (handler: () => void) => () => void;
  monotonic?: () => number;
  now?: () => number;
};

export type SocialSocket = {
  /** Whether a `social` frame has come over the socket open now. */
  live: () => boolean;
  /**
   * Opens a socket when none is open, unless the session's token is the one last refused (R1115000), and
   * replaces one opened with a token the session no longer holds.
   */
  retry: () => void;
  /** Lets go of the socket for good; `onLive` is not called. */
  close: () => void;
};

export function openSocialSocket(options: SocialSocketOptions): SocialSocket {
  const {
    token,
    onPush,
    onLive,
    socketFactory = browserSocket,
    url = serverSocketUrl(SOCIAL_SOCKET_PATH),
    timers = defaultTimers,
    random = Math.random,
    wake = pageWake,
    monotonic = defaultMonotonic,
    now = () => Date.now(),
  } = options;

  let socket: SocketLike | null = null;
  /** The token `socket` was opened with: its frames are that session's, and no other's. */
  let openedWith: string | null = null;
  let disposed = false;
  let live = false;
  /** The token a refusal closed the socket for; nothing opens again until the session holds another. */
  let refusedToken: string | null = null;

  const backoff = createBackoff(open, () => disposed, timers, random);
  /** R1441: a woken page asks a quiet socket for its read, and replaces it if nothing answers. */
  const probe = createWakeProbe({
    socket: () => socket,
    sendProbe: (current) => {
      current.send(HELLO);
    },
    replace: (stale) => {
      socket = null;
      setLive(false);
      letGo(stale, "no answer", open);
    },
    disabled: () => disposed,
    timers,
    monotonic,
    now,
    wake,
  });

  function setLive(next: boolean): void {
    if (next === live) return;
    live = next;
    if (!disposed) onLive(next);
  }

  /** The session moved on (renewed, signed out, another account): its socket opens again for the new one. */
  function replace(stale: SocketLike): void {
    socket = null;
    probe.cancel();
    letGo(stale, "session changed");
    setLive(false);
    open();
  }

  function open(): void {
    backoff.cancel();
    if (disposed || socket !== null) return;
    const bearer = token();
    if (bearer === null || bearer === refusedToken) return;
    refusedToken = null;
    let created: SocketLike;
    try {
      created = socketFactory(url, socketProtocols(bearer));
    } catch {
      backoff.schedule();
      return;
    }
    socket = created;
    openedWith = bearer;
    probe.hear();
    created.onopen = () => {
      if (socket === created) probe.hear();
    };
    created.onmessage = (event) => {
      if (socket !== created) return;
      // A frame for a token the session no longer holds is never shown (an account switched under it).
      if (token() !== bearer) {
        replace(created);
        return;
      }
      probe.hear();
      probe.cancel();
      const push = parseSocialPush(event.data);
      if (push === null) return;
      if (push.kind === "social") {
        backoff.reset();
        setLive(true);
      }
      onPush(push);
    };
    created.onerror = () => {};
    created.onclose = (event) => {
      if (socket !== created) return;
      socket = null;
      probe.cancel();
      setLive(false);
      if (disposed) return;
      if (event.code === UNAUTHORIZED_CLOSE_CODE) {
        // R1115000: a 4401 refuses this handshake's token. One renewed meanwhile opens again now.
        const renewed = token();
        if (renewed !== null && renewed !== bearer) {
          open();
          return;
        }
      }
      if (REFUSAL_CLOSE_CODES.includes(event.code)) {
        refusedToken = bearer;
        return;
      }
      backoff.schedule();
    };
  }

  probe.listen();
  open();

  return {
    live: () => live,
    retry: () => {
      if (disposed) return;
      if (socket === null) open();
      else if (token() !== openedWith) replace(socket);
    },
    close: () => {
      disposed = true;
      live = false;
      backoff.cancel();
      probe.dispose();
      const current = socket;
      socket = null;
      if (current !== null) letGo(current, "leaving");
    },
  };
}
