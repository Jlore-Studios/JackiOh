// R3214000: the tab's `/ws/social` client through a fake WebSocket, with manual timers, a manual clock
// and a manual page wake, as `game/net.test.ts` drives the match socket.

import { describe, expect, it } from "vitest";

import { RECONNECT_DELAYS_MS, SILENT_SOCKET_MS, type Timers } from "../game/net.ts";
import { serverSocketUrl } from "../net/api.ts";
import { socketProtocols, tokenFromProtocols } from "../net/socketProtocols.ts";
import { fakeSockets, type FakeSocket } from "../test/fakeSocket.ts";
import { SOCIAL_SOCKET_PATH, openSocialSocket, parseSocialPush, type SocialPush, type SocialSocket } from "./socialSocket.ts";

const SOCIAL = { type: "social", incoming: [], outgoing: [], unread: { "f-ann": 2 } };

type Harness = {
  client: SocialSocket;
  sockets: FakeSocket[];
  socket: () => FakeSocket;
  pushes: SocialPush[];
  lives: boolean[];
  setToken: (token: string | null) => void;
  runTimers: () => void;
  pending: () => number;
  delays: () => number[];
  /** Moves the clock the wake probe reads, monotonic and wall alike. */
  now: (value: number) => void;
  /** R1441: the page wakes (its tab turns visible, the network comes back). */
  wake: () => void;
};

/** `retryOnDown`: `onLive(false)` asks the socket to retry, as `socialPoll.ts`'s poll does through its tick. */
function harness(token: string | null = "tok-a", { retryOnDown = false }: { retryOnDown?: boolean } = {}): Harness {
  const { factory, sockets } = fakeSockets();
  let scheduled: { id: number; run: () => void }[] = [];
  const delays: number[] = [];
  let nextId = 1;
  let clock = 0;
  let current = token;
  let onWake: (() => void) | null = null;
  const pushes: SocialPush[] = [];
  const lives: boolean[] = [];
  let retrying: SocialSocket | null = null;

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

  const client = openSocialSocket({
    token: () => current,
    onPush: (push) => pushes.push(push),
    onLive: (live) => {
      lives.push(live);
      if (!live && retryOnDown) retrying?.retry();
    },
    socketFactory: factory,
    url: serverSocketUrl(SOCIAL_SOCKET_PATH, "ws://server.test/ws/match"),
    timers,
    random: () => 0,
    monotonic: () => clock,
    now: () => clock,
    wake: (handler) => {
      onWake = handler;
      return () => {
        onWake = null;
      };
    },
  });
  retrying = client;

  return {
    client,
    sockets,
    socket: () => {
      const last = sockets.at(-1);
      if (last === undefined) throw new Error("no socket was opened");
      return last;
    },
    pushes,
    lives,
    setToken: (next) => {
      current = next;
    },
    runTimers: () => {
      const due = scheduled;
      scheduled = [];
      for (const entry of due) entry.run();
    },
    pending: () => scheduled.length,
    delays: () => [...delays],
    now: (value) => {
      clock = value;
    },
    wake: () => {
      onWake?.();
    },
  };
}

/** The harness with its first socket open and live. */
function live(token = "tok-a", options: { retryOnDown?: boolean } = {}): Harness {
  const h = harness(token, options);
  h.socket().open();
  h.socket().deliver(SOCIAL);
  return h;
}

describe("R3214000 the social socket", () => {
  it("R3214000 offers the session's token on /ws/social and is live from its first social frame", () => {
    const h = harness("tok-a");
    expect(h.sockets).toHaveLength(1);
    const url = new URL(h.socket().url);
    expect(url.pathname).toBe("/ws/social");
    expect(h.socket().url).not.toContain("tok-a");
    expect(h.socket().protocols).toEqual(socketProtocols("tok-a"));
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-a");

    h.socket().open();
    h.socket().deliver({ type: "error", code: "forbidden", message: "sign in first" });
    expect(h.client.live(), "an open socket is not live before its read").toBe(false);
    expect(h.lives).toEqual([]);
    expect(h.pushes).toEqual([]);

    h.socket().deliver(SOCIAL);
    expect(h.client.live()).toBe(true);
    expect(h.lives).toEqual([true]);
    expect(h.pushes).toEqual([{ kind: "social", read: { incoming: [], outgoing: [], unread: { "f-ann": 2 } } }]);
    h.socket().deliver(SOCIAL);
    expect(h.lives, "live once, however many reads come").toEqual([true]);
    expect(h.socket().sent, "the tab says nothing unasked").toEqual([]);

    // With no address given, the socket is the match socket's server's.
    const { factory, sockets } = fakeSockets();
    const plain = openSocialSocket({ token: () => "tok-a", onPush: () => undefined, onLive: () => undefined, socketFactory: factory, wake: () => () => undefined });
    expect(new URL(sockets[0]?.url ?? "").pathname).toBe("/ws/social");
    plain.close();
  });

  it("R3214000 a dropped socket reopens on the match socket's backoff", () => {
    const h = live();
    const steps = RECONNECT_DELAYS_MS.length + 2;
    for (let drop = 0; drop < steps; drop += 1) {
      h.socket().drop(1006);
      expect(h.pending(), `drop ${String(drop)} waits one step`).toBe(1);
      h.runTimers();
      expect(h.sockets).toHaveLength(drop + 2);
    }
    const last = RECONNECT_DELAYS_MS.at(-1) ?? 0;
    expect(h.delays()).toEqual([...RECONNECT_DELAYS_MS, ...Array.from({ length: steps - RECONNECT_DELAYS_MS.length }, () => last)]);
    expect(h.lives, "the live socket's drop said so once").toEqual([true, false]);

    // A socket's first read starts the steps over.
    h.socket().open();
    h.socket().deliver(SOCIAL);
    h.socket().drop(1006);
    expect(h.delays().at(-1)).toBe(RECONNECT_DELAYS_MS[0]);
    expect(h.lives).toEqual([true, false, true, false]);

    // Closed, it reopens nothing and says nothing.
    h.client.close();
    h.runTimers();
    expect(h.sockets).toHaveLength(steps + 1);
    expect(h.lives).toEqual([true, false, true, false]);
  });

  it("R3214000 a drop waits its backoff step even when the poll retries as the socket goes down", () => {
    const h = live("tok-a", { retryOnDown: true });
    h.socket().drop(1006);
    expect(h.lives).toEqual([true, false]);
    expect(h.sockets, "no socket before the step runs").toHaveLength(1);
    expect(h.pending()).toBe(1);
    h.client.retry();
    expect(h.sockets, "nor on a later retry while the step waits").toHaveLength(1);
    h.runTimers();
    expect(h.sockets).toHaveLength(2);
    expect(h.delays()).toEqual([RECONNECT_DELAYS_MS[0]]);

    // A refusal for the same token, then a retry as it goes down: nothing opens.
    h.socket().open();
    h.socket().deliver(SOCIAL);
    h.socket().drop(4403, "too many open tabs");
    expect(h.sockets).toHaveLength(2);
    expect(h.pending()).toBe(0);
  });

  it("R3214000 a 4401 reopens at once with a renewed token, and a refusal for the same token waits for another", () => {
    const h = live("tok-a");
    h.setToken("tok-b");
    h.socket().drop(4401, "sign in first");
    expect(h.sockets, "renewed before the close came: a new socket at once").toHaveLength(2);
    expect(h.pending()).toBe(0);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-b");
    expect(h.lives).toEqual([true, false]);

    h.socket().open();
    h.socket().drop(4401, "sign in first");
    expect(h.sockets, "the same token: nothing opens").toHaveLength(2);
    expect(h.pending()).toBe(0);
    h.client.retry();
    expect(h.sockets, "nor on a retry while the session holds that token").toHaveLength(2);

    h.setToken("tok-c");
    h.client.retry();
    expect(h.sockets).toHaveLength(3);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-c");
    h.client.retry();
    expect(h.sockets, "a retry with a socket open opens no second").toHaveLength(3);
  });

  it("R3214000 4429 is a drop and 4403 is final for its token", () => {
    const h = live("tok-a");
    h.socket().drop(4429, "too many requests; slow down");
    expect(h.pending(), "past the account's budget: the backoff").toBe(1);
    h.runTimers();
    expect(h.sockets).toHaveLength(2);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-a");

    h.socket().open();
    h.socket().drop(4403, "too many open tabs");
    expect(h.pending()).toBe(0);
    h.client.retry();
    expect(h.sockets).toHaveLength(2);

    h.setToken("tok-b");
    h.client.retry();
    expect(h.sockets).toHaveLength(3);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-b");

    // No session: nothing opens, whatever closed the last socket.
    h.socket().open();
    h.setToken(null);
    h.socket().drop(1006);
    h.runTimers();
    h.client.retry();
    expect(h.sockets).toHaveLength(3);
  });

  it("R3214000 a frame after the session's token changed is never shown, and the socket opens again for the new one", () => {
    const h = live("tok-a");
    const first = h.socket();
    h.setToken("tok-b");
    first.deliver({ type: "chat", friendId: "f-ann" });
    expect(h.pushes, "a frame of the session the tab no longer holds is dropped").toHaveLength(1);
    expect(first.closedWith).toEqual({ code: 1000, reason: "session changed" });
    expect(h.sockets).toHaveLength(2);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-b");
    expect(h.lives).toEqual([true, false]);

    // A quiet socket is replaced by the poll's retry once the session holds another token.
    h.socket().open();
    h.socket().deliver(SOCIAL);
    h.setToken("tok-c");
    h.client.retry();
    expect(h.sockets).toHaveLength(3);
    expect(tokenFromProtocols(h.socket().protocols)).toBe("tok-c");
    expect(h.lives).toEqual([true, false, true, false]);

    // Signed out: the socket goes and none opens.
    h.socket().open();
    h.setToken(null);
    h.client.retry();
    expect(h.sockets).toHaveLength(3);
    expect(h.socket().closedWith).toEqual({ code: 1000, reason: "session changed" });
  });

  it("R3214000 a woken page probes a quiet socket with hello and replaces it if nothing answers", () => {
    const h = live();
    h.now(SILENT_SOCKET_MS);
    h.wake();
    expect(h.socket().sent, "heard from within the silence: no probe").toEqual([]);

    h.now(2 * SILENT_SOCKET_MS + 1);
    h.wake();
    expect(h.socket().frames()).toEqual([{ type: "hello" }]);
    expect(h.pending()).toBe(1);
    h.socket().deliver(SOCIAL);
    expect(h.pending(), "an answer keeps the socket").toBe(0);
    expect(h.sockets).toHaveLength(1);

    h.now(4 * SILENT_SOCKET_MS);
    h.wake();
    expect(h.socket().frames()).toEqual([{ type: "hello" }, { type: "hello" }]);
    const quiet = h.socket();
    h.runTimers();
    expect(quiet.closedWith).toEqual({ code: 1000, reason: "no answer" });
    expect(h.sockets, "replaced at once").toHaveLength(2);
    expect(h.lives).toEqual([true, false]);
    quiet.drop(1006);
    expect(h.pending(), "the stale socket's close is not heard").toBe(0);
  });

  it("R3214000 reads social, chat and friends frames and nothing else", () => {
    expect(parseSocialPush(JSON.stringify(SOCIAL))).toEqual({ kind: "social", read: { incoming: [], outgoing: [], unread: { "f-ann": 2 } } });
    expect(parseSocialPush(JSON.stringify({ type: "chat", friendId: "f-ann" }))).toEqual({ kind: "chat", friendId: "f-ann" });
    expect(parseSocialPush(JSON.stringify({ type: "friends" }))).toEqual({ kind: "friends" });
    for (const other of [
      JSON.stringify({ type: "chat" }),
      JSON.stringify({ type: "chat", friendId: 7 }),
      JSON.stringify({ type: "social", incoming: [], outgoing: [] }),
      JSON.stringify({ type: "social", incoming: {}, outgoing: [], unread: {} }),
      JSON.stringify({ type: "error", code: "forbidden", message: "sign in first" }),
      JSON.stringify({ type: "view" }),
      JSON.stringify([{ type: "friends" }]),
      "not json",
      42,
      null,
    ]) {
      expect(parseSocialPush(other), String(other)).toBeNull();
    }
  });
});
