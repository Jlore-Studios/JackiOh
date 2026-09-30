/**
 * `attachWebSocketServer` over a real listener and a real `ws` client: what one client can cost the
 * process before its token is even checked (§9.8), and how a browser hands over its token.
 *
 *  - a frame over `MAX_FRAME_BYTES` is refused by `ws` itself with 1009, so it is never buffered
 *    whole (the library's default ceiling is 100 MiB per frame);
 *  - one client address holds at most `maxConnectionsPerAddress` sockets, handshakes included;
 *  - the token may travel as the second `Sec-WebSocket-Protocol` entry, and only `WS_SUBPROTOCOL`
 *    is echoed back.
 */

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it, vi } from "vitest";
import { WebSocket } from "ws";

import { MAX_FRAME_BYTES } from "../../src/match/protocol";
import {
  WS_PATH,
  WS_SUBPROTOCOL,
  attachWebSocketServer,
  subprotocolToken,
  type AttachedSockets,
} from "../../src/match/wsServer";
import { createTestDeps } from "../fakes/deps";

type Running = { base: string; attach: ReturnType<typeof vi.fn>; token: string };

let server: Server | null = null;
let sockets: AttachedSockets | null = null;
const clients: WebSocket[] = [];

afterEach(async () => {
  for (const client of clients.splice(0)) client.terminate();
  await sockets?.close();
  await new Promise<void>((resolve) => {
    if (server === null) resolve();
    else server.close(() => resolve());
  });
  server = null;
  sockets = null;
});

/** A listener with one active player in `match-1`, whose socket the registry keeps open. */
async function listen(options: { maxConnectionsPerAddress?: number } = {}): Promise<Running> {
  const deps = createTestDeps();
  const token = deps.auth.addUser({ userId: "user-1", email: "a@example.test" });
  deps.store.seedProfile({ id: "profile-1", userId: "user-1", status: "active", inMatchId: "match-1" });
  const attach = vi.fn(async () => "p1");

  server = createServer();
  sockets = attachWebSocketServer(server, deps, { attach }, options);
  await new Promise<void>((resolve) => server?.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return { base: `ws://127.0.0.1:${String(port)}${WS_PATH}?matchId=match-1`, attach, token };
}

function open(url: string, protocols?: string[]): WebSocket {
  const client = new WebSocket(url, protocols);
  // A refused or terminated handshake is an `error` event; the tests read the outcome elsewhere.
  client.on("error", () => undefined);
  clients.push(client);
  return client;
}

function opened(client: WebSocket): Promise<void> {
  return new Promise((resolve, reject) => {
    client.once("open", () => resolve());
    client.once("error", reject);
  });
}

function closeCode(client: WebSocket): Promise<number> {
  return new Promise((resolve) => client.once("close", (code: number) => resolve(code)));
}

function refusedStatus(client: WebSocket): Promise<number> {
  return new Promise((resolve) => {
    client.once("unexpected-response", (_request, response) => resolve(response.statusCode ?? 0));
  });
}

describe("§9.8: a frame is capped before it is buffered", () => {
  it("closes with 1009 on a 65 KiB text frame", async () => {
    const { base, attach, token } = await listen();
    const client = open(base, [WS_SUBPROTOCOL, token]);
    await opened(client);
    await vi.waitFor(() => expect(attach).toHaveBeenCalledTimes(1));

    const closed = closeCode(client);
    client.send("x".repeat(65 * 1024));
    expect(65 * 1024).toBeGreaterThan(MAX_FRAME_BYTES);
    expect(await closed).toBe(1009);
  });

  it("still takes a frame at the cap", async () => {
    const { base, attach, token } = await listen();
    const client = open(base, [WS_SUBPROTOCOL, token]);
    await opened(client);
    await vi.waitFor(() => expect(attach).toHaveBeenCalledTimes(1));

    client.send("x".repeat(MAX_FRAME_BYTES));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(client.readyState).toBe(WebSocket.OPEN);
  });
});

describe("§9.8: sockets per client address", () => {
  it("refuses the upgrade past the cap with 429, and frees the slot when a socket closes", async () => {
    const { base, token } = await listen({ maxConnectionsPerAddress: 2 });
    const first = open(base, [WS_SUBPROTOCOL, token]);
    const second = open(base, [WS_SUBPROTOCOL, token]);
    await Promise.all([opened(first), opened(second)]);

    const third = open(base, [WS_SUBPROTOCOL, token]);
    expect(await refusedStatus(third)).toBe(429);

    const gone = closeCode(first);
    first.close();
    await gone;
    await vi.waitFor(async () => {
      const fourth = open(base, [WS_SUBPROTOCOL, token]);
      await opened(fourth);
    });
  });

  it("counts a socket refused for a bad token only until it closes", async () => {
    const { base } = await listen({ maxConnectionsPerAddress: 1 });
    const refused = open(base, [WS_SUBPROTOCOL, "not-a-token"]);
    expect(await closeCode(refused)).toBe(4401);
    await vi.waitFor(async () => {
      const next = open(base, [WS_SUBPROTOCOL, "not-a-token"]);
      await opened(next);
    });
  });
});

describe("the token in Sec-WebSocket-Protocol", () => {
  it("authenticates the socket and echoes only the protocol name", async () => {
    const { base, attach, token } = await listen();
    const client = open(base, [WS_SUBPROTOCOL, token]);
    await opened(client);
    expect(client.protocol).toBe(WS_SUBPROTOCOL);
    await vi.waitFor(() => expect(attach).toHaveBeenCalledTimes(1));
    expect(attach.mock.calls[0]?.slice(0, 2)).toEqual(["match-1", "profile-1"]);
  });

  it("reads the token next to the protocol name, in either order, and nothing without the name", () => {
    expect(subprotocolToken(`${WS_SUBPROTOCOL}, tok.en.sig`)).toBe("tok.en.sig");
    expect(subprotocolToken(`tok.en.sig,${WS_SUBPROTOCOL}`)).toBe("tok.en.sig");
    expect(subprotocolToken([WS_SUBPROTOCOL, "tok"])).toBe("tok");
    expect(subprotocolToken("chat, tok")).toBeNull();
    expect(subprotocolToken(WS_SUBPROTOCOL)).toBeNull();
    expect(subprotocolToken(undefined)).toBeNull();
  });
});
