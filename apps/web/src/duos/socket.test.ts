// `duos/socket.ts`'s reconnect half through a fake WebSocket, as `game/net.test.ts` covers the 1v1 client's.
// Injected socket, timer, wake and clock seams keep it deterministic without a server.

import { describe, expect, it } from "vitest";

import { WS_PING_INTERVAL_SECONDS } from "@jackioh/server-config";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import {
  RECONNECT_DELAYS_MS,
  RECONNECT_JITTER,
  WAKE_PROBE_MS,
  type SocketLike,
  type Timers,
} from "../game/net.ts";
import { createDuosClient, type DuosClient } from "./socket.ts";

// Fakes

class FakeSocket implements SocketLike {
  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;

  readonly sent: string[] = [];
  closedWith: { code?: number; reason?: string } | null = null;

  constructor(readonly url: string) {}

  send(data: string): void {
    this.sent.push(data);
  }

  close(code?: number, reason?: string): void {
    this.readyState = 3;
    this.closedWith = { ...(code === undefined ? {} : { code }), ...(reason === undefined ? {} : { reason }) };
  }

  // Test drivers

  open(): void {
    this.readyState = 1;
    this.onopen?.({});
  }

  deliver(frame: unknown): void {
    this.onmessage?.({ data: typeof frame === "string" ? frame : JSON.stringify(frame) });
  }

  drop(code = 1006, reason = ""): void {
    this.readyState = 3;
    this.onclose?.({ code, reason, wasClean: false });
  }

  /** What the client sent, parsed. */
  frames(): Record<string, unknown>[] {
    return this.sent.map((text) => JSON.parse(text) as Record<string, unknown>);
  }
}

type Harness = {
  client: DuosClient;
  sockets: FakeSocket[];
  socket: () => FakeSocket;
  /** Run every pending backoff callback, in order. */
  runTimers: () => void;
  pending: () => number;
  /** Every delay a timer was set for, in order. */
  delays: () => number[];
  /** Time passes on both clocks: the page was awake, and the socket said nothing. */
  advance: (ms: number) => void;
  /** Time passes on the wall clock alone: the machine slept, and the monotonic clock stood still. */
  sleep: (ms: number) => void;
  /** R1441: the page wakes (its tab turns visible, the network comes back). */
  wake: () => void;
};

type HarnessOptions = {
  /** The reconnect jitter's draw; 0 adds nothing. */
  random?: () => number;
  /** Leave the client to listen for the page itself (`visibilitychange`, `online`). */
  pageEvents?: boolean;
};

const MATCH_ID = "dm-1";
const HELLO = { type: "hello", matchId: MATCH_ID };

/** How long a socket must have said nothing before a woken page doubts it (R1441). */
const SILENCE_MS = WS_PING_INTERVAL_SECONDS * MS_PER_SECOND;

const view = (round: number): unknown => ({ type: "view", view: { round }, legal: [] });

/** The round the client holds, or null before a view. */
function roundOf(h: Harness): number | null {
  const held = h.client.snapshot().view as { round?: number } | null;
  return held === null ? null : (held.round ?? null);
}

function harness(options: HarnessOptions = {}): Harness {
  const sockets: FakeSocket[] = [];
  let scheduled: { id: number; run: () => void }[] = [];
  const delays: number[] = [];
  let nextId = 1;
  let monotonicNow = 0;
  let wallNow = 0;
  let onWake: (() => void) | null = null;

  const timers: Timers = {
    setTimeout: (handler, ms) => {
      const id = nextId;
      nextId += 1;
      scheduled.push({ id, run: handler });
      delays.push(ms);
      return id;
    },
    clearTimeout: (handle) => {
      scheduled = scheduled.filter((entry) => entry.id !== handle);
    },
  };

  const client = createDuosClient({
    matchId: MATCH_ID,
    token: () => "tok",
    baseUrl: "ws://server.test/ws/duos",
    socketFactory: (url) => {
      const socket = new FakeSocket(url);
      sockets.push(socket);
      return socket;
    },
    timers,
    monotonic: () => monotonicNow,
    now: () => wallNow,
    random: options.random ?? (() => 0),
    ...(options.pageEvents === true
      ? {}
      : {
          wake: (handler: () => void) => {
            onWake = handler;
            return () => {
              onWake = null;
            };
          },
        }),
  });

  return {
    client,
    sockets,
    socket: () => {
      const last = sockets.at(-1);
      if (last === undefined) throw new Error("no socket was opened");
      return last;
    },
    runTimers: () => {
      const due = scheduled;
      scheduled = [];
      for (const entry of due) entry.run();
    },
    pending: () => scheduled.length,
    delays: () => [...delays],
    advance: (ms) => {
      monotonicNow += ms;
      wallNow += ms;
    },
    sleep: (ms) => {
      wallNow += ms;
    },
    wake: () => {
      onWake?.();
    },
  };
}

function connected(options: HarnessOptions = {}): Harness {
  const h = harness(options);
  h.client.connect();
  h.socket().open();
  return h;
}

// a quiet socket on a woken page (R1441)

describe("a quiet co-op socket on a woken page", () => {
  it("R1441 a co-op socket quiet past the ping interval is sent hello on wake, and replaced when nothing answers", () => {
    const h = connected();
    const old = h.socket();
    h.advance(SILENCE_MS + 1);

    h.wake();
    // It is asked first: a healthy socket answers with a view, and nothing else changes.
    expect(old.frames()).toEqual([HELLO, HELLO]);
    expect(h.sockets).toHaveLength(1);
    expect(h.delays()).toEqual([WAKE_PROBE_MS]);

    // Nothing came back, so it is let go and a new socket takes its place.
    h.runTimers();
    expect(old.closedWith).toEqual({ code: 1000, reason: "replaced" });
    expect(h.sockets).toHaveLength(2);
    expect(h.client.snapshot().connection).toBe("reconnecting");
    h.socket().open();
    expect(h.socket().frames()).toEqual([HELLO]);
    expect(h.client.snapshot().connection).toBe("open");

    // Whatever the old socket still says is no longer the match's.
    old.deliver(view(9));
    expect(h.client.snapshot().view).toBeNull();
    h.socket().deliver(view(3));
    expect(roundOf(h)).toBe(3);
  });

  it("R1441 a quiet co-op socket that answers the probe is kept", () => {
    const h = connected();
    const current = h.socket();
    h.advance(SILENCE_MS + 1);
    h.wake();
    current.deliver(view(3));

    // The answer cancelled the probe: nothing is left to run, and the socket stays.
    expect(h.pending()).toBe(0);
    h.runTimers();
    expect(h.sockets).toHaveLength(1);
    expect(current.closedWith).toBeNull();
    expect(h.client.snapshot().connection).toBe("open");

    // It was just heard from, so waking again asks nothing.
    h.wake();
    expect(current.frames()).toEqual([HELLO, HELLO]);
  });

  it("R1441 a co-op socket heard within the ping interval is left alone", () => {
    const h = connected();
    h.advance(SILENCE_MS);
    h.wake();
    expect(h.socket().frames()).toEqual([HELLO]);
    expect(h.pending()).toBe(0);
  });

  it("R1441 a co-op socket whose monotonic clock stood still through a sleep is still probed", () => {
    const h = connected();
    // `performance.now()` leaves a sleeping machine's time out; the wall clock does not.
    h.sleep(SILENCE_MS + 1);
    h.wake();
    expect(h.socket().frames()).toEqual([HELLO, HELLO]);
  });

  it("R1441 a co-op socket that is not open is replaced at once", () => {
    const h = harness();
    h.client.connect();
    const stuck = h.socket();
    h.advance(SILENCE_MS + 1);
    h.wake();
    expect(stuck.closedWith).not.toBeNull();
    expect(h.sockets).toHaveLength(2);
    expect(h.pending()).toBe(0);
  });

  it("R1441 by default the tab turning visible and the network coming back wake it", () => {
    const h = connected({ pageEvents: true });
    const current = h.socket();
    const visibility = Object.getOwnPropertyDescriptor(document, "visibilityState");
    const show = (state: "hidden" | "visible"): void => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
      document.dispatchEvent(new Event("visibilitychange"));
    };
    try {
      h.advance(SILENCE_MS + 1);
      show("hidden");
      expect(current.frames()).toEqual([HELLO]);
      show("visible");
      expect(current.frames()).toEqual([HELLO, HELLO]);

      current.deliver(view(1));
      h.advance(SILENCE_MS + 1);
      window.dispatchEvent(new Event("online"));
      expect(current.frames()).toEqual([HELLO, HELLO, HELLO]);
    } finally {
      h.client.close();
      if (visibility === undefined) Reflect.deleteProperty(document, "visibilityState");
      else Object.defineProperty(document, "visibilityState", visibility);
    }

    // A client that was closed no longer listens.
    h.advance(2 * SILENCE_MS);
    window.dispatchEvent(new Event("online"));
    expect(current.frames()).toHaveLength(3);
    expect(h.sockets).toHaveLength(1);
  });

  it("R1441 close() drops a waiting probe and stops listening", () => {
    const h = connected();
    h.advance(SILENCE_MS + 1);
    h.wake();
    expect(h.pending()).toBe(1);

    h.client.close();
    expect(h.pending()).toBe(0);
    expect(h.client.snapshot().connection).toBe("closed");

    h.advance(SILENCE_MS + 1);
    h.wake();
    expect(h.pending()).toBe(0);
    expect(h.sockets).toHaveLength(1);
  });
});

// the reconnect

describe("the co-op socket's reconnect", () => {
  it.each([4401, 4403, 4404])("does not retry a refusal: %i is a settled answer", (code) => {
    const h = connected();
    h.socket().drop(code, "not your match");
    expect(h.client.snapshot().connection).toBe("refused");
    expect(h.client.snapshot().error).toBe("not your match");
    expect(h.pending()).toBe(0);
  });

  it("a voided match's close (4410) is final: no reconnect, the last view kept", () => {
    const h = connected();
    h.socket().deliver(view(4));
    h.socket().drop(4410, "voided");
    expect(h.client.snapshot().connection).toBe("closed");
    expect(roundOf(h)).toBe(4);
    expect(h.pending()).toBe(0);
  });

  it("R1441 a reconnect waits its backoff plus up to RECONNECT_JITTER of it more", () => {
    const h = connected({ random: () => 0.5 });
    h.socket().drop(1006);
    h.runTimers();
    expect(h.sockets).toHaveLength(2);
    h.socket().drop(1006);

    const steps = RECONNECT_DELAYS_MS.slice(0, 2);
    expect(h.delays()).toEqual(steps.map((step) => Math.floor(step * (1 + RECONNECT_JITTER * 0.5))));
    h.delays().forEach((delay, index) => {
      const step = steps[index] ?? 0;
      expect(delay).toBeGreaterThanOrEqual(step);
      expect(delay).toBeLessThanOrEqual(step * (1 + RECONNECT_JITTER));
    });
  });

  it("connect() during a backoff opens a socket at once and drops the pending retry", () => {
    const h = connected();
    h.socket().drop(1006);
    expect(h.pending()).toBe(1);

    h.client.connect();
    expect(h.sockets).toHaveLength(2);
    expect(h.pending()).toBe(0);

    h.socket().open();
    expect(h.client.snapshot().connection).toBe("open");
    expect(h.socket().frames()).toEqual([HELLO]);

    h.client.connect();
    expect(h.sockets).toHaveLength(2);
  });
});

// a refusal for the sign-in (R1115000)

/** The token a socket's handshake carried. */
function handshakeToken(socket: FakeSocket): string | null {
  return new URL(socket.url).searchParams.get("token");
}

describe("a co-op socket refused for its sign-in", () => {
  it("R1115000 a co-op 4401, then a renewed token, opens one more socket that carries it", () => {
    const h = connected();
    h.socket().drop(4401, "sign in first");
    expect(h.client.snapshot().connection).toBe("refused");
    expect(h.client.snapshot().error).toBe("sign in first");

    h.client.setToken("new");
    expect(h.sockets).toHaveLength(2);
    expect(handshakeToken(h.socket())).toBe("new");
    expect(h.client.snapshot().error).toBeNull();

    h.socket().open();
    expect(h.client.snapshot().connection).toBe("open");
    expect(h.socket().frames()).toEqual([HELLO]);
  });

  it("R1115000 a co-op token the 4401 refused opens no socket", () => {
    const h = connected();
    h.socket().drop(4401, "sign in first");
    h.client.setToken("tok");
    expect(h.sockets).toHaveLength(1);
    expect(h.client.snapshot().connection).toBe("refused");
    expect(h.pending()).toBe(0);
  });

  it("R1115000 a co-op 4401 on the socket a renewed token opened is final for that token", () => {
    const h = connected();
    h.socket().drop(4401, "sign in first");
    h.client.setToken("new");
    h.socket().drop(4401, "sign in first");
    expect(h.client.snapshot().connection).toBe("refused");
    expect(h.pending()).toBe(0);

    h.client.setToken("new");
    expect(h.sockets).toHaveLength(2);
    // One refusal per token: the next renewal gets its one socket.
    h.client.setToken("newer");
    expect(h.sockets).toHaveLength(3);
    expect(handshakeToken(h.socket())).toBe("newer");
  });

  it("R1115000 a co-op token renewed before the 4401 arrives opens one more socket at once", () => {
    const h = connected();
    // R194: a healthy socket is not reopened by a renewal.
    h.client.setToken("new");
    expect(h.sockets).toHaveLength(1);

    h.socket().drop(4401, "sign in first");
    expect(h.sockets).toHaveLength(2);
    expect(handshakeToken(h.socket())).toBe("new");
    expect(h.pending()).toBe(0);

    h.socket().drop(4401, "sign in first");
    expect(h.client.snapshot().connection).toBe("refused");
    expect(h.sockets).toHaveLength(2);
  });

  it("R1115000 a co-op 4403 or 4404 is not reopened by a renewed token, even after a 4401's retry", () => {
    for (const code of [4403, 4404]) {
      const direct = connected();
      direct.socket().drop(code, "not your match");
      direct.client.setToken("new");
      expect(direct.sockets).toHaveLength(1);

      const afterRetry = connected();
      afterRetry.socket().drop(4401, "sign in first");
      afterRetry.client.setToken("new");
      afterRetry.socket().drop(code, "not your match");
      afterRetry.client.setToken("newer");
      expect(afterRetry.sockets).toHaveLength(2);
      expect(afterRetry.client.snapshot().connection).toBe("refused");
    }
  });
});
