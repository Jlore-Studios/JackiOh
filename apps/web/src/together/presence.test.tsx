// R3210000: the presence heartbeat goes only once PRESENCE_HEARTBEAT_MS passes with no signed-in request, since
// the server counts every signed-in request as presence. The server is a stubbed `fetch` (`test/stubServer.ts`)
// and time is vitest's fake clock. R3576000: a tab hidden for HIDDEN_QUIET_MS sends none.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PRESENCE_HEARTBEAT_MS } from "@jackioh/server-config";

import { apiRequest } from "../net/api.ts";
import { getFriends } from "../net/social.ts";
import { settle } from "../test/settle.ts";
import { resetVisibility, setVisibility } from "../test/setup.ts";
import { meBody, signIn, stubServer, type StubServer } from "../test/stubServer.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { HIDDEN_QUIET_MS } from "./config.ts";
import { usePresenceHeartbeat } from "./presence.ts";

function serve(): StubServer {
  return stubServer((call) => {
    const { method, path } = call;
    if (path === "/api/auth/me") return { status: 200, body: meBody() };
    if (method === "GET" && path === "/api/social") return { status: 200, body: { incoming: [], outgoing: [], unread: {} } };
    if (method === "GET" && path === "/api/friends") return { status: 200, body: { friends: [], incoming: [], outgoing: [], blocked: [] } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    if (method === "GET" && path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    return undefined;
  });
}

/** The shell's heartbeat for a signed-in tab, alone. */
function Heartbeat(): null {
  usePresenceHeartbeat(true);
  return null;
}

function beats(server: StubServer): number {
  return server.to("POST", "/api/presence").length;
}

beforeEach(() => {
  window.localStorage.clear();
  signIn("tok-presence");
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  resetVisibility();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("R3210000 the presence heartbeat as a fallback", () => {
  it("R3210000 while the social poll's signed-in reads flow, no heartbeat is sent", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <Heartbeat />
      </>,
    );
    await settle({ advance: 3 * PRESENCE_HEARTBEAT_MS });
    expect(beats(server), "every social read counted as presence").toBe(0);
    expect(server.to("GET", "/api/social").length).toBeGreaterThan(1);
  });

  it("R3210000 after PRESENCE_HEARTBEAT_MS with no signed-in request, one heartbeat is sent", async () => {
    const server = serve();
    render(<Heartbeat />);
    await settle({ advance: 0 });
    expect(beats(server), "none at mount: the shell's social poll reads at once").toBe(0);

    await settle({ advance: PRESENCE_HEARTBEAT_MS - 1 });
    expect(beats(server)).toBe(0);
    await settle({ advance: 1 });
    expect(beats(server)).toBe(1);
    expect(server.to("POST", "/api/presence")[0]?.headers.authorization).toBe("Bearer tok-presence");

    await settle({ advance: PRESENCE_HEARTBEAT_MS - 1 });
    expect(beats(server)).toBe(1);
    await settle({ advance: 1 });
    expect(beats(server)).toBe(2);
  });

  it("R3210000 a signed-in request puts the next heartbeat a full PRESENCE_HEARTBEAT_MS after it", async () => {
    const server = serve();
    render(<Heartbeat />);
    await settle({ advance: PRESENCE_HEARTBEAT_MS / 2 });
    void getFriends({ keep: false });
    await settle({ advance: PRESENCE_HEARTBEAT_MS - 1 });
    expect(beats(server)).toBe(0);
    await settle({ advance: 1 });
    expect(beats(server)).toBe(1);
  });

  it("R3210000 a request with no token counts for nothing, so it leaves the next heartbeat where it was", async () => {
    const server = serve();
    render(<Heartbeat />);
    await settle({ advance: PRESENCE_HEARTBEAT_MS / 2 });
    void apiRequest("/api/catalog");
    await settle({ advance: PRESENCE_HEARTBEAT_MS / 2 });
    expect(server.to("GET", "/api/catalog")).toHaveLength(1);
    expect(beats(server)).toBe(1);
  });

  it("R3210000 a hidden tab, its social poll paused, still sends the heartbeat every PRESENCE_HEARTBEAT_MS", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <Heartbeat />
      </>,
    );
    await settle({ advance: 0 });
    expect(server.to("GET", "/api/social")).toHaveLength(1);
    setVisibility("hidden");

    await settle({ advance: PRESENCE_HEARTBEAT_MS });
    expect(beats(server)).toBe(1);
    await settle({ advance: PRESENCE_HEARTBEAT_MS });
    expect(beats(server)).toBe(2);
    expect(server.to("GET", "/api/social"), "the hidden tab's social poll stays paused").toHaveLength(1);
  });
});

describe("R3576000 a quiet tab sends no heartbeat", () => {
  it("R3576000 a tab hidden for HIDDEN_QUIET_MS stops beating, and beats again once it shows", async () => {
    const server = serve();
    render(<Heartbeat />);
    await settle({ advance: 0 });
    setVisibility("hidden");

    await settle({ advance: HIDDEN_QUIET_MS });
    const quietAt = beats(server);
    expect(quietAt, "it beats at its pace until it goes quiet").toBe(Math.floor(HIDDEN_QUIET_MS / PRESENCE_HEARTBEAT_MS));
    await settle({ advance: 10 * PRESENCE_HEARTBEAT_MS });
    expect(beats(server), "quiet: no beat, so the server can sleep").toBe(quietAt);

    setVisibility("visible");
    await settle({ advance: PRESENCE_HEARTBEAT_MS - 1 });
    expect(beats(server)).toBe(quietAt);
    await settle({ advance: 1 });
    expect(beats(server), "shown: a heartbeat later it beats again").toBe(quietAt + 1);
  });

  it("R3576000 a tab that shows again before HIDDEN_QUIET_MS goes on beating", async () => {
    const server = serve();
    render(<Heartbeat />);
    await settle({ advance: 0 });
    setVisibility("hidden");
    await settle({ advance: HIDDEN_QUIET_MS - 1 });
    setVisibility("visible");
    setVisibility("hidden");
    await settle({ advance: HIDDEN_QUIET_MS - 1 });
    expect(beats(server)).toBe(Math.floor((2 * HIDDEN_QUIET_MS - 2) / PRESENCE_HEARTBEAT_MS));
  });
});
