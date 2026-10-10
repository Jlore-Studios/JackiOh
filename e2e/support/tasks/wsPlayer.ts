// Node second-player client for BUILD M6 and BUILD M6-T4 spec 06 (e2e/README.md A8).
// `protocol.rs` fixes hello (fresh full view, §9.5), action, view (SPEC §10.8), ack (nonce/seq,
// SPEC §9.3), error, prompt (record only, §10.6) and clock (R79, §9.5). HTTP joins atomically;
// the actor stamps `playerId`'s authenticated seat.
// BUILD M8 runs these specs individually and as one suite.

import { WebSocket } from "ws";

export type WsMessage = { type: string; [key: string]: unknown };

type Waiter = { match: (message: WsMessage) => boolean; resolve: (message: WsMessage) => void; reject: (error: Error) => void; timer: NodeJS.Timeout };

type Client = {
  name: string;
  socket: WebSocket;
  seat: "p1" | "p2";
  messages: WsMessage[];
  lastView: Record<string, unknown> | null;
  waiters: Waiter[];
  nonce: number;
  matchId: string | null;
};

export type WsPlayerCommand =
  | { action: "connect"; name: string; url?: string; token?: string; matchId?: string; roomCode?: string; seat?: "p1" | "p2" }
  | { action: "send"; name: string; body: Record<string, unknown> }
  | { action: "awaitView"; name: string; where?: ViewPredicate }
  | { action: "view"; name: string }
  | { action: "messages"; name: string }
  | { action: "disconnect"; name: string }
  | { action: "concede"; name: string; url?: string; token: string; matchId: string; seat?: "p1" | "p2" }
  | { action: "reset" };

export type ViewPredicate = {
  active?: string;
  phase?: string;
  promptKind?: string;
  hasResult?: boolean;
  turnAtLeast?: number;
  /** R265, R266: `view.mulligan.opponentReady`; a view outside the mulligan window never matches. */
  mulliganOpponentReady?: boolean;
  /** R269: `view.drawOffer.by`, or `null` for "no offer stands". */
  drawOfferBy?: string | null;
};

export type WsPlayerResult = {
  ok: boolean;
  name?: string;
  seat?: string;
  view?: Record<string, unknown> | null;
  messages?: WsMessage[];
  /** §9.3: the reducer's reason, relayed verbatim — never restated. */
  error?: string;
  /** The `SocketErrorCode` that came with it (`illegal_action`, `rate_limited`, …). */
  code?: string;
  /** §9.3: the append-only log seq an accepted action was written at (`ack.seq`). */
  seq?: number;
};

const TIMEOUT_MS = 15_000;
const clients = new Map<string, Client>();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function deliver(client: Client, message: WsMessage): void {
  client.messages.push(message);
  if (message.type === "view" && isRecord(message.view)) client.lastView = message.view;
  for (const waiter of [...client.waiters]) {
    if (!waiter.match(message)) continue;
    clearTimeout(waiter.timer);
    client.waiters = client.waiters.filter((other) => other !== waiter);
    waiter.resolve(message);
  }
}

function waitFor(client: Client, match: (message: WsMessage) => boolean, what: string): Promise<WsMessage> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      client.waiters = client.waiters.filter((waiter) => waiter.timer !== timer);
      const seen = client.messages.map((message) => message.type).join(", ");
      reject(new Error(`wsPlayer(${client.name}): timed out waiting for ${what}; saw [${seen}]`));
    }, TIMEOUT_MS);
    client.waiters.push({ match, resolve, reject, timer });
  });
}

function matchesView(view: Record<string, unknown>, where: ViewPredicate): boolean {
  if (where.active !== undefined && view.active !== where.active) return false;
  if (where.phase !== undefined && view.phase !== where.phase) return false;
  if (where.hasResult !== undefined && (view.result !== null && view.result !== undefined) !== where.hasResult) return false;
  if (where.turnAtLeast !== undefined && !(typeof view.turn === "number" && view.turn >= where.turnAtLeast)) return false;
  if (where.mulliganOpponentReady !== undefined) {
    const mulligan = isRecord(view.mulligan) ? view.mulligan : null;
    if (mulligan === null || mulligan.opponentReady !== where.mulliganOpponentReady) return false;
  }
  if (where.drawOfferBy !== undefined) {
    const by = isRecord(view.drawOffer) && typeof view.drawOffer.by === "string" ? view.drawOffer.by : null;
    if (by !== where.drawOfferBy) return false;
  }
  if (where.promptKind !== undefined) {
    // §10.6, §10.8: only the responding player receives a pending prompt's kind and options.
    const pending = isRecord(view.pending) ? view.pending : null;
    if (pending === null || pending.forYou !== true || pending.kind !== where.promptKind) return false;
  }
  return true;
}

function client(name: string): Client {
  const found = clients.get(name);
  if (found === undefined) throw new Error(`wsPlayer: no client named "${name}"; connect first`);
  return found;
}

async function connect(command: Extract<WsPlayerCommand, { action: "connect" }>): Promise<WsPlayerResult> {
  const existing = clients.get(command.name);
  if (existing !== undefined) existing.socket.close();

  // `WS_PATH`: an off-path handshake never reaches the upgrade.
  const base = command.url ?? "ws://localhost:8787/ws/match";
  const url = new URL(base);
  // The upgrade reads only `?token=` and `?matchId=`.
  if (command.token !== undefined) url.searchParams.set("token", command.token);
  if (command.matchId !== undefined) url.searchParams.set("matchId", command.matchId);

  const socket = new WebSocket(url.toString());
  const record: Client = {
    name: command.name,
    socket,
    seat: command.seat ?? "p2",
    messages: [],
    lastView: null,
    waiters: [],
    nonce: 0,
    matchId: command.matchId ?? null,
  };
  clients.set(command.name, record);

  socket.on("message", (data) => {
    const text = typeof data === "string" ? data : data.toString();
    try {
      const parsed: unknown = JSON.parse(text);
      deliver(record, isRecord(parsed) && typeof parsed.type === "string" ? (parsed as WsMessage) : { type: "unknown", raw: text });
    } catch {
      deliver(record, { type: "unparsed", raw: text });
    }
  });

  // Reject waiters with a refusal's close code instead of waiting for the timeout.
  socket.on("close", (code: number, reason: Buffer) => {
    const why = reason.length > 0 ? `: ${reason.toString()}` : "";
    for (const waiter of record.waiters.splice(0)) {
      clearTimeout(waiter.timer);
      waiter.reject(new Error(`wsPlayer(${command.name}): the socket closed (${String(code)}${why})`));
    }
  });

  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`wsPlayer(${command.name}): no open on ${url.toString()}`)), TIMEOUT_MS);
    socket.once("open", () => {
      clearTimeout(timer);
      resolve();
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      reject(error instanceof Error ? error : new Error(String(error)));
    });
  });

  const hello: WsMessage = { type: "hello" };
  if (command.token !== undefined) hello.token = command.token;
  if (command.matchId !== undefined) hello.matchId = command.matchId;
  if (command.roomCode !== undefined) hello.roomCode = command.roomCode;
  socket.send(JSON.stringify(hello));

  // §9.5 permits the pushed view to pre-date `hello`; match refusal errors rather than timing out.
  const first = await waitFor(
    record,
    (message) => message.type === "view" || message.type === "error",
    "the first view push",
  );
  if (first.type === "error") {
    return { ok: false, name: command.name, ...errorOf(first) };
  }
  return { ok: true, name: command.name, seat: record.seat, view: isRecord(first.view) ? first.view : null };
}

/** §9.3: `ErrorMessage.message` is the reducer's verbatim reason; it has no `error` field. */
function errorOf(message: WsMessage): { error: string; code?: string } {
  const code = typeof message.code === "string" ? message.code : undefined;
  const text = typeof message.message === "string" ? message.message : "";
  return {
    error: text.length > 0 ? text : (code ?? "the server refused it without saying why"),
    ...(code === undefined ? {} : { code }),
  };
}

async function send(command: Extract<WsPlayerCommand, { action: "send" }>): Promise<WsPlayerResult> {
  const record = client(command.name);
  record.nonce += 1;
  const nonce = `${command.name}-${record.nonce}`;
  // The actor stamps the authenticated seat; frozen specs still put `playerId` on the body.
  const action = { playerId: record.seat, ...command.body, nonce };
  record.socket.send(JSON.stringify({ type: "action", action }));
  const reply = await waitFor(
    record,
    (message) =>
      (message.type === "ack" && message.nonce === nonce) ||
      // An un-nonced malformed error is this frame's reply; another action's error is not.
      (message.type === "error" && (message.nonce === undefined || message.nonce === nonce)),
    `an ack for ${String(command.body.type)} (${nonce})`,
  );
  if (reply.type === "error") {
    return { ok: false, name: command.name, ...errorOf(reply), view: record.lastView };
  }
  return {
    ok: true,
    name: command.name,
    view: record.lastView,
    ...(typeof reply.seq === "number" ? { seq: reply.seq } : {}),
  };
}

async function awaitView(command: Extract<WsPlayerCommand, { action: "awaitView" }>): Promise<WsPlayerResult> {
  const record = client(command.name);
  const where = command.where ?? {};
  if (record.lastView !== null && matchesView(record.lastView, where)) {
    return { ok: true, name: command.name, view: record.lastView };
  }
  const message = await waitFor(
    record,
    (msg) => msg.type === "view" && isRecord(msg.view) && matchesView(msg.view, where),
    `a view matching ${JSON.stringify(where)}`,
  );
  return { ok: true, name: command.name, view: isRecord(message.view) ? message.view : null };
}

/** §2.5 concede clears both seats' in-match state (§9.5); cleanup is best-effort for closed or over matches. */
async function concedeIfLive(record: Client): Promise<void> {
  if (record.matchId === null) return;
  if (record.socket.readyState !== WebSocket.OPEN) return;
  // §10.8: only a non-null result proves the match ended; a missing view still concedes.
  const view = record.lastView;
  if (view !== null && view.result !== null && view.result !== undefined) return;
  try {
    await send({ action: "send", name: record.name, body: { type: "concede" } });
  } catch {
    // The socket went away, the actor had already stopped, or the match was over after all.
  }
}

/** Concede in one task so browser reconnect cannot reclaim the seat between Cypress commands. */
async function concede(command: Extract<WsPlayerCommand, { action: "concede" }>): Promise<WsPlayerResult> {
  try {
    const opened = await connect({
      action: "connect",
      name: command.name,
      token: command.token,
      matchId: command.matchId,
      ...(command.url === undefined ? {} : { url: command.url }),
      ...(command.seat === undefined ? {} : { seat: command.seat }),
    });
    if (!opened.ok) return opened;
    const view = opened.view ?? null;
    if (view !== null && view.result !== null && view.result !== undefined) {
      return { ok: true, name: command.name, view };
    }
    return await send({ action: "send", name: command.name, body: { type: "concede" } });
  } finally {
    const record = clients.get(command.name);
    if (record !== undefined) {
      record.socket.close();
      clients.delete(command.name);
    }
  }
}

export async function wsPlayer(command: WsPlayerCommand): Promise<WsPlayerResult> {
  try {
    switch (command.action) {
      case "connect":
        return await connect(command);
      case "send":
        return await send(command);
      case "awaitView":
        return await awaitView(command);
      case "view":
        return { ok: true, name: command.name, view: client(command.name).lastView };
      case "messages":
        return { ok: true, name: command.name, messages: client(command.name).messages };
      case "disconnect": {
        const record = client(command.name);
        // Spec 05 models a dropped player: only `reset` cleans up, not this disconnect.
        record.socket.close();
        clients.delete(command.name);
        return { ok: true, name: command.name };
      }
      case "concede":
        return await concede(command);
      case "reset": {
        for (const record of clients.values()) await concedeIfLive(record);
        for (const record of clients.values()) record.socket.close();
        clients.clear();
        return { ok: true };
      }
    }
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  }
}
