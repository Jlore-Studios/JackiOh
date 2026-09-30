/**
 * The `ws` adapter: the only file in `src/match` that knows a WebSocket library exists.
 *
 * Everything the protocol guarantees is proven against the in-memory `Socket` (test/fakes/socket.ts),
 * because that is the same interface the actor talks to (src/match/contracts.ts). This file adds
 * two things and no rules:
 *
 *  - `socketFromWs`: a `ws` connection as a `Socket`. Text frames only — every protocol message is
 *    JSON (contracts.ts), so a binary frame is answered with an `error` and dropped.
 *  - `createMatchSocketHandler`: the upgrade. It authenticates the bearer token through
 *    `deps.auth`, resolves the profile, applies §9.4's gate (`assertActive`) and refuses unless
 *    that profile is in the match it asked for (§9.1: the client may only read its own view of a
 *    match it is playing). Only then does the registry get the socket.
 *
 * `attachWebSocketServer` also bounds what one client can cost before it has authenticated: a
 * frame over `MAX_FRAME_BYTES` closes the socket with 1009 before `ws` buffers it, and one client
 * address holds at most `WS_MAX_CONNECTIONS_PER_ADDRESS` sockets, handshakes included.
 */

import { WebSocketServer, type WebSocket } from "ws";
import { ApiError, assertActive, bearerToken, clientAddress, rateLimitAddress } from "../api/http";
import type { AuthProvider, Logger, Store } from "../api/ports";
import { DEFAULT_TRUSTED_PROXY_HOPS, WS_MAX_CONNECTIONS_PER_ADDRESS } from "../config";
import type { Socket, SocketHandlers } from "./contracts";
import { MAX_FRAME_BYTES, encode, errorMessage } from "./protocol";

/** SPEC §9.2: one WebSocket per player, upgraded on the same listener the API serves. */
export const WS_PATH = "/ws/match";

/**
 * The subprotocol a browser names on the handshake, with its access token as the second entry:
 * `new WebSocket(url, [WS_SUBPROTOCOL, token])`. A browser cannot set an `authorization` header on
 * a WebSocket, and a token in the URL is written to every access log on the way. The server echoes
 * only this name back, never the token.
 */
export const WS_SUBPROTOCOL = "jackioh.v1";

/**
 * SPEC §11 R148: "4401, 4403 and 4404 are private-use mirrors of the HTTP statuses the REST side
 * returns for the same three refusals, with 1011 for an internal fault, so a client reuses one
 * table." R148 also fixes what the socket may learn: every refusal answers with the same error
 * code and only the close code varies, so a socket learns that it may not have this match and
 * never which check said so (§9.1).
 */
export const WS_CLOSE = {
  unauthorized: 4401,
  forbidden: 4403,
  notFound: 4404,
  internal: 1011,
} as const;

export function socketFromWs(ws: WebSocket): Socket {
  let handlers: SocketHandlers | null = null;

  const socket: Socket = {
    get isOpen() {
      // 1 === WebSocket.OPEN. Compared as a number so this module needs no runtime import of `ws`.
      return ws.readyState === 1;
    },
    send: (text) => {
      ws.send(text);
    },
    close: (code, reason) => {
      ws.close(code ?? 1000, reason ?? "");
    },
    attach: (next) => {
      handlers = next;
    },
  };

  ws.on("message", (data: unknown, isBinary: boolean) => {
    if (isBinary) {
      ws.send(encode(errorMessage("malformed", "text frames only: every message is JSON")));
      return;
    }
    handlers?.message(String(data));
  });
  ws.on("close", () => {
    handlers?.close();
  });
  ws.on("error", () => {
    // A transport error is a disconnect; §9.5's grace is what handles it.
    handlers?.close();
  });

  return socket;
}

/** Enough of Node's `IncomingMessage` to read the token and the match id. */
export type UpgradeRequest = {
  url?: string | undefined;
  headers: Record<string, string | string[] | undefined>;
};

export type MatchSocketDeps = {
  auth: AuthProvider;
  store: Store;
  log: Logger;
  /** The registry, narrowed to the one method the upgrade needs. */
  registry: { attach: (matchId: string, profileId: string, socket: Socket) => Promise<unknown> };
};

/**
 * The token offered in `Sec-WebSocket-Protocol` next to `WS_SUBPROTOCOL`, or null. The header is a
 * comma-separated list; the token is the one entry that is not the protocol name.
 */
export function subprotocolToken(raw: string | string[] | undefined): string | null {
  const joined = Array.isArray(raw) ? raw.join(",") : raw;
  if (typeof joined !== "string") return null;
  const entries = joined
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
  if (!entries.includes(WS_SUBPROTOCOL)) return null;
  return entries.find((entry) => entry !== WS_SUBPROTOCOL) ?? null;
}

/**
 * Browsers cannot set headers on a WebSocket handshake, so a browser sends the token as the second
 * `Sec-WebSocket-Protocol` entry (`WS_SUBPROTOCOL`). A Node client (the e2e `wsPlayer` task) may
 * send an `authorization` header instead. `?token=` is still read for clients that have not moved
 * off it; it puts the token in access logs, so nothing new should use it.
 */
function tokenFrom(request: UpgradeRequest, url: URL): string | null {
  const header = request.headers.authorization;
  if (typeof header === "string") {
    const fromHeader = bearerToken(new Headers({ authorization: header }));
    if (fromHeader !== null) return fromHeader;
  }
  const offered = subprotocolToken(request.headers["sec-websocket-protocol"]);
  if (offered !== null) return offered;
  const query = url.searchParams.get("token");
  return query === null || query.length === 0 ? null : query;
}

export function createMatchSocketHandler(
  deps: MatchSocketDeps,
): (ws: WebSocket, request: UpgradeRequest) => Promise<void> {
  return async (ws, request) => {
    const socket = socketFromWs(ws);

    const refuse = (code: number, message: string): void => {
      try {
        // One protocol code for every refusal: a socket learns that it may not have this match,
        // never which of the checks said so.
        socket.send(encode(errorMessage("forbidden", message)));
      } catch {
        // The peer may already be gone; the close below is what matters.
      }
      socket.close(code, message);
    };

    try {
      const url = new URL(request.url ?? "/", "http://match.invalid");
      const token = tokenFrom(request, url);
      if (token === null) {
        refuse(WS_CLOSE.unauthorized, "sign in first");
        return;
      }

      const user = await deps.auth.verifyAccessToken(token);
      if (user === null) {
        refuse(WS_CLOSE.unauthorized, "sign in first");
        return;
      }

      const profile = await deps.store.profiles.getByUserId(user.userId);
      if (profile === null) {
        refuse(WS_CLOSE.unauthorized, "sign in first");
        return;
      }

      // §9.4's gate, from the same function the HTTP router uses.
      assertActive(profile);

      const asked = url.searchParams.get("matchId");
      const matchId = asked ?? profile.inMatchId;
      if (matchId === null) {
        refuse(WS_CLOSE.notFound, "you are not in a match");
        return;
      }
      // §9.1: a socket is only ever opened onto a match this profile is playing.
      if (profile.inMatchId !== matchId) {
        refuse(WS_CLOSE.forbidden, "you are not in that match");
        return;
      }

      await deps.registry.attach(matchId, profile.id, socket);
    } catch (error: unknown) {
      if (error instanceof ApiError) {
        refuse(error.status === 404 ? WS_CLOSE.notFound : WS_CLOSE.forbidden, error.message);
        return;
      }
      deps.log.alert("ws.upgrade.threw", {
        message: error instanceof Error ? error.message : String(error),
      });
      refuse(WS_CLOSE.internal, "something went wrong");
    }
  };
}

/** Enough of Node's `http.Server` to take over an upgrade, so this file needs no `node:http` type. */
type UpgradableServer = {
  on: (event: "upgrade", listener: (request: UpgradeRequest, socket: Duplexish, head: Buffer) => void) => unknown;
};

/**
 * The raw TCP socket an upgrade hands over. The refusal path writes to it; the connection count
 * reads its peer address and waits for it to close, which covers both a finished socket and a
 * handshake that never completed.
 */
type Duplexish = {
  write: (chunk: string) => unknown;
  destroy: () => unknown;
  once: (event: "close", listener: () => void) => unknown;
  remoteAddress?: string | undefined;
};

export type AttachedSockets = { close: () => Promise<void> };

export type AttachOptions = {
  path?: string;
  /**
   * §9.8: a browser attaches credentials to a cross-origin WebSocket handshake automatically, so an
   * unchecked upgrade is a CSRF surface. An empty or absent list allows any origin, which is what a
   * Node client (the e2e `wsPlayer` task) sends — it has no `Origin` at all.
   */
  allowedOrigins?: readonly string[];
  /**
   * R190's hop count, so a socket is counted against the same client address the API's per-IP
   * limits read (`clientAddress` in api/http.ts). Defaults to `DEFAULT_TRUSTED_PROXY_HOPS`.
   */
  trustedProxyHops?: number;
  /** Defaults to `WS_MAX_CONNECTIONS_PER_ADDRESS`; a test lowers it. */
  maxConnectionsPerAddress?: number;
};

/** The `X-Forwarded-For` of an upgrade, as the `Headers` that `clientAddress` reads. */
function forwardedHeaders(request: UpgradeRequest): Headers {
  const raw = request.headers["x-forwarded-for"];
  const value = Array.isArray(raw) ? raw.join(",") : raw;
  return new Headers(typeof value === "string" ? { "x-forwarded-for": value } : {});
}

/**
 * Wire the upgrade onto the listener the API already serves (§9.2). `ws` runs in `noServer` mode so
 * this owns the routing decision: a handshake off `path` is left alone rather than refused, because
 * another handler may want it, while a disallowed origin is refused before any token is read.
 *
 * The returned `close` shuts every live socket, so `start()`'s own `close` really does release the
 * port — a test that started a server and did not get its sockets closed would hang the run.
 */
export function attachWebSocketServer(
  server: UpgradableServer,
  deps: { auth: AuthProvider; store: Store; log: Logger },
  registry: MatchSocketDeps["registry"],
  options: AttachOptions = {},
): AttachedSockets {
  const path = options.path ?? WS_PATH;
  const allowed = options.allowedOrigins ?? [];
  const trustedProxyHops = options.trustedProxyHops ?? DEFAULT_TRUSTED_PROXY_HOPS;
  const maxPerAddress = options.maxConnectionsPerAddress ?? WS_MAX_CONNECTIONS_PER_ADDRESS;
  const wss = new WebSocketServer({
    noServer: true,
    // §9.8: every client message is a small JSON frame. Without this `ws` buffers up to 100 MiB of
    // one frame before `parseClientMessage` could refuse it; with it the frame is refused with 1009
    // as its length arrives.
    maxPayload: MAX_FRAME_BYTES,
    // Echo only the fixed name, never the token that was offered beside it.
    handleProtocols: (protocols: Set<string>) => (protocols.has(WS_SUBPROTOCOL) ? WS_SUBPROTOCOL : false),
  });
  const handle = createMatchSocketHandler({ ...deps, registry });

  /** Open and in-progress sockets per client address (`rateLimitAddress` form). Never logged. */
  const perAddress = new Map<string, number>();

  /**
   * R162 makes this list the same one the REST layer reads, "so the two doors cannot diverge" — so
   * the comparison has to match too. `api/cors.ts` canonicalises a trailing slash before comparing;
   * a raw `includes` here meant a hand-written `PUBLIC_ORIGINS=https://play.example/` was accepted
   * by CORS and refused at the upgrade.
   */
  function sameOrigin(a: string, b: string): boolean {
    const strip = (value: string): string => value.trim().replace(/\/+$/, "").toLowerCase();
    return strip(a) === strip(b);
  }

  function originAllowed(request: UpgradeRequest): boolean {
    if (allowed.length === 0) return true;
    const origin = request.headers.origin;
    // No Origin header at all is a non-browser client, which the CSRF concern does not reach.
    if (typeof origin !== "string" || origin.length === 0) return true;
    return allowed.some((entry) => sameOrigin(entry, origin));
  }

  server.on("upgrade", (request, socket, head) => {
    const url = new URL(request.url ?? "/", "http://match.invalid");
    // Not our path: leave the handshake for another handler rather than destroying it.
    if (url.pathname !== path) return;

    if (!originAllowed(request)) {
      deps.log.warn("ws.upgrade.origin_refused", { origin: String(request.headers.origin) });
      socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
      socket.destroy();
      return;
    }

    const address = rateLimitAddress(
      clientAddress(forwardedHeaders(request), socket.remoteAddress ?? null, trustedProxyHops),
    );
    const held = perAddress.get(address) ?? 0;
    if (held >= maxPerAddress) {
      // No address in the log line: the count is the signal.
      deps.log.warn("ws.upgrade.too_many", { limit: maxPerAddress });
      socket.write("HTTP/1.1 429 Too Many Requests\r\n\r\n");
      socket.destroy();
      return;
    }
    perAddress.set(address, held + 1);
    socket.once("close", () => {
      const left = (perAddress.get(address) ?? 1) - 1;
      if (left <= 0) perAddress.delete(address);
      else perAddress.set(address, left);
    });

    wss.handleUpgrade(request as never, socket as never, head, (ws: WebSocket) => {
      void handle(ws, request);
    });
  });

  return {
    close: async () => {
      for (const client of wss.clients) client.close(1001, "server closing");
      await new Promise<void>((resolve) => {
        wss.close(() => {
          resolve();
        });
      });
    },
  };
}
