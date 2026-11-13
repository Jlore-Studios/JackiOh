// #936 S6, S8: the Duos specs' second seats (41, 55). Cypress drives one browser, so every other seat of a
// co-op or a 2v2 game is a client the spec drives over `/ws/duos` itself, as the networked specs drive their
// second seat over `/ws/match` (spec 06's `wsPlayer`, whose frames are the 1v1 protocol's, not S6's). The
// views are S2's `DuosView`, camelCase, restated as far as the specs read them, since e2e type-checks alone.

import { socketProtocols } from "../../apps/web/src/net/socketProtocols.ts";
import { apiUrl, bearerHeaders, server, timeouts, type E2EAccount } from "./config.ts";

/** S8's paths; support/config.ts's `routes` predate them. */
export const TOGETHER_PATH = "/together";

export function duosPath(matchId: string): string {
  return `/duos/${matchId}`;
}

/** `/ws/duos` beside the configured `/ws/match` (S6), as the client's `duosSocketUrl` finds it. */
function duosSocketUrl(): string {
  return server.ws().replace(/\/ws\/match\/?$/u, "/ws/duos");
}

export type Frame = { type: string; [key: string]: unknown };
type CardLike = { instanceId: string };
export type PlayerViewLike = {
  active?: string;
  viewer?: string;
  you?: { player?: string; hand?: CardLike[] | { count: number } };
  pending?: { forYou?: boolean; kind?: string } | null;
};
type TeamLike = { team?: string; pool?: number; passesLeft?: number[]; ended?: boolean[] };
export type DuosViewLike = {
  seat?: string;
  own?: PlayerViewLike;
  teammate?: PlayerViewLike;
  team?: TeamLike;
  enemy?: TeamLike;
  round?: number;
  concedeOfferedBy?: string | null;
  result?: { outcome?: string; winner?: string | null; reason?: string } | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

type Waiter = { match: (frame: Frame) => boolean; resolve: (frame: Frame) => void; timer: ReturnType<typeof setTimeout> };

/** A seat on `/ws/duos` the spec drives by hand: S6's hello, actions under nonces, pings. */
export class DuosSocket {
  readonly frames: Frame[] = [];
  view: DuosViewLike | null = null;
  private socket: WebSocket | null = null;
  private waiters: Waiter[] = [];
  private sent = 0;

  constructor(private readonly name: string) {}

  private deliver(frame: Frame): void {
    this.frames.push(frame);
    if (frame.type === "view" && isRecord(frame.view)) this.view = frame.view as DuosViewLike;
    for (const waiter of [...this.waiters]) {
      if (!waiter.match(frame)) continue;
      clearTimeout(waiter.timer);
      this.waiters = this.waiters.filter((other) => other !== waiter);
      waiter.resolve(frame);
    }
  }

  private waitFor(match: (frame: Frame) => boolean, what: string): Promise<Frame> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiters = this.waiters.filter((waiter) => waiter.timer !== timer);
        reject(new Error(`${this.name}: no ${what} on /ws/duos`));
      }, timeouts.view);
      this.waiters.push({ match, resolve, timer });
    });
  }

  /** Opens the socket (auth as `/ws/match`: the match in the query, the token offered as a subprotocol, R1379000) and says hello. */
  open(token: string, matchId: string): Promise<Frame> {
    const url = new URL(duosSocketUrl());
    url.searchParams.set("matchId", matchId);
    const socket = new WebSocket(url.toString(), socketProtocols(token));
    this.socket = socket;
    socket.addEventListener("message", (event: MessageEvent) => {
      if (typeof event.data !== "string") return;
      let parsed: unknown;
      try {
        parsed = JSON.parse(event.data);
      } catch {
        return;
      }
      if (isRecord(parsed) && typeof parsed.type === "string") this.deliver(parsed as Frame);
    });
    const first = this.waitFor((frame) => frame.type === "view" || frame.type === "error", "first view");
    socket.addEventListener("open", () => {
      socket.send(JSON.stringify({ type: "hello", matchId }));
    });
    return first;
  }

  /** One action; resolves with its `ack`, or with the `error` that refused it. */
  act(body: Record<string, unknown>): Promise<Frame> {
    this.sent += 1;
    const nonce = `${this.name}-${String(this.sent)}`;
    const reply = this.waitFor(
      (frame) => (frame.type === "ack" || frame.type === "error") && (frame.nonce === nonce || frame.nonce === undefined),
      `answer to ${nonce}`,
    );
    this.socket?.send(JSON.stringify({ type: "action", nonce, body }));
    return reply;
  }

  ping(ping: string, target: Record<string, unknown>): void {
    this.socket?.send(JSON.stringify({ type: "ping", ping, target }));
  }

  close(): void {
    this.socket?.close();
    this.socket = null;
  }
}

/**
 * A Duos match an earlier run left live for these two teammates ends here: both concede (B11), each from
 * a socket of its own. Nothing to do when they are not in one together.
 */
export function endLiveDuos(first: E2EAccount, second: E2EAccount): void {
  const live: (string | null)[] = [];
  for (const account of [first, second]) {
    cy.request<{ matchId: string | null }>({ method: "GET", url: apiUrl("/api/duos/live"), headers: bearerHeaders(account.token) }).then(
      (answer) => {
        live.push(answer.body.matchId);
      },
    );
  }
  cy.then(() => {
    const [one, two] = live;
    if (one === null || one === undefined || one !== two) return;
    const firstSeat = new DuosSocket("cleanup-a");
    const secondSeat = new DuosSocket("cleanup-b");
    cy.wrap(Promise.all([firstSeat.open(first.token, one), secondSeat.open(second.token, one)]), { timeout: timeouts.view });
    cy.then(() => cy.wrap(firstSeat.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.then(() => cy.wrap(secondSeat.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.then(() => {
      firstSeat.close();
      secondSeat.close();
    });
  });
}
