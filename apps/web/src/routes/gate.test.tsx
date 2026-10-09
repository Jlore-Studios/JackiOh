// `main.tsx`: the route table and SPEC §9.4's gate, asserted through the real modules — a session
// in `localStorage`, `GET /api/auth/me` over a stubbed `fetch`, and `window.location.pathname` as
// the answer, because that is what specs 09 and 10 read.
//
// The gate enforces nothing (CLAUDE.md rule 7). The server 403s `account_pending` at every door and
// `wsServer.ts` runs the same check on the upgrade; what is asserted here is only that the browser
// is sent to the screen §9.4 says the account may see.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

import { announceAccountChange } from "../net/gate.ts";
import { readQueued, rememberQueued } from "../net/liveGame.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";

// `main.tsx` is the app's entry point and mounts itself, except under vitest — see the guard at the
// foot of that file. This assertion is what makes the guard's condition explicit: if vitest ever
// stops reporting `MODE === "test"`, this fails instead of the suite silently double-mounting.
it("vitest runs in MODE=test, which is what main.tsx's mount guard keys on", () => {
  expect(import.meta.env.MODE).toBe("test");
});

const { App, Gated, redirectFor, resetUsernamePromptForTests } = await import("../main.tsx");

// ---------------------------------------------------------------------------------------------
// the stubbed server
// ---------------------------------------------------------------------------------------------

type Status = "pending" | "active" | "banned";

function jsonResponse(status: number, body: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: () => Promise.resolve(JSON.stringify(body)),
  } as unknown as Response;
}

/** R1435: the caller's own username on `/api/auth/me`; omitted, as a server before it would. */
type OwnUsername = { name: string; nextChangeAt: number | null; promptOwed: boolean };

function meBody(status: Status, username?: OwnUsername) {
  return {
    profile: { id: "profile-1", status },
    needsInviteCode: status === "pending",
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
    ...(username === undefined ? {} : { username }),
  };
}

/** Stub `/api/auth/me` (and the catalog the match route reads) for one account status. */
function serveAs(status: Status | "unauthorized", username?: OwnUsername): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/api/auth/me")) {
        if (status === "unauthorized") {
          return Promise.resolve(
            jsonResponse(401, { error: { code: "unauthorized", message: "sign in first" } }),
          );
        }
        return Promise.resolve(jsonResponse(200, meBody(status, username)));
      }
      if (url.endsWith("/api/catalog")) {
        return Promise.resolve(jsonResponse(200, { version: "v1", defs: {} }));
      }
      // The reads `/decks` makes for itself (another task owns that screen); stubbed so this file
      // asserts the gate rather than a deckbuilder failing to load.
      if (url.endsWith("/api/loadout")) {
        return Promise.resolve(jsonResponse(200, { catalogVersion: "v1", loadout: null }));
      }
      if (url.endsWith("/api/collection")) {
        return Promise.resolve(jsonResponse(200, { catalogVersion: "v1", entries: [] }));
      }
      if (url.endsWith("/api/codes/status")) {
        return Promise.resolve(jsonResponse(200, { redemptionEnabled: true, retryAfterMs: 0 }));
      }
      return Promise.resolve(
        jsonResponse(404, { error: { code: "not_found", message: `no stub for ${url}` } }),
      );
    }),
  );
}

/** The key the M8 specs seed in `cy.visit`'s `onBeforeLoad` (`net/session.ts`). */
function signedIn(token = "e2e-token"): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
}

/**
 * A lazily-imported route chunk plus a stubbed round trip can outrun testing-library's 1 s default
 * when the whole suite runs at once. Still an assertion retried against the DOM, never a sleep.
 */
const SLOW = { timeout: 5_000 } as const;

/**
 * The gate has answered and the gated screen has mounted. `gate-loading` is both the gate's own
 * holding panel and the route table's Suspense fallback, so its absence covers the lazy chunk too.
 *
 * Deliberately not an assertion on the screen's own markup: `/decks` and `/invite` belong to other
 * tasks and their contents are theirs to change. What this file owns is which screen the browser
 * ends up at, which is exactly what specs 09 and 10 assert.
 */
async function gateOpened(): Promise<void> {
  await waitFor(() => {
    expect(screen.queryByTestId("gate-loading")).toBeNull();
  }, SLOW);
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function pathname(): string {
  return window.location.pathname;
}

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------------------------
// redirectFor: the whole decision, with no DOM in the way
// ---------------------------------------------------------------------------------------------

describe("redirectFor", () => {
  it("sends an account with no session to /login", () => {
    expect(redirectFor({ kind: "anonymous" }, false)).toBe("/login");
    expect(redirectFor({ kind: "anonymous" }, true)).toBe("/login");
  });

  it("sends a session the provider refused to renew to /login?reason=expired, and nothing else in the query", () => {
    expect(redirectFor({ kind: "anonymous", reason: "expired" }, false)).toBe("/login?reason=expired");
    expect(redirectFor({ kind: "anonymous", reason: "expired" }, true)).toBe("/login?reason=expired");
  });

  it("sends a pending account to the code screen (§9.4)", () => {
    const pending = { kind: "ready", token: "t", me: meBody("pending") } as const;
    expect(redirectFor(pending, false)).toBe("/invite");
  });

  it("leaves a pending account on /invite: that is the one screen §9.4 allows it", () => {
    const pending = { kind: "ready", token: "t", me: meBody("pending") } as const;
    expect(redirectFor(pending, true)).toBeNull();
  });

  it("leaves an active account alone", () => {
    const active = { kind: "ready", token: "t", me: meBody("active") } as const;
    expect(redirectFor(active, false)).toBeNull();
    expect(redirectFor(active, true)).toBeNull();
  });

  it("holds a loading account where it is: a redirect on an unanswered /api/auth/me would flap", () => {
    expect(redirectFor({ kind: "loading" }, false)).toBeNull();
    expect(redirectFor({ kind: "error", message: "offline" }, false)).toBeNull();
  });
});

// ---------------------------------------------------------------------------------------------
// the three redirect outcomes, through the real route table
// ---------------------------------------------------------------------------------------------

describe("the gate, as specs 09 and 10 read it", () => {
  it("anonymous: /decks sends the browser to /login", async () => {
    serveAs("active"); // Never reached: there is no session to send.
    at("/decks");
    render(<App />);
    await waitFor(() => {
      expect(pathname()).toBe("/login");
    }, SLOW);
  });

  it("a token the server no longer accepts is the same as having none", async () => {
    signedIn("stale");
    serveAs("unauthorized");
    at("/play");
    render(<App />);
    await waitFor(() => {
      expect(pathname()).toBe("/login");
    }, SLOW);
  });

  it("spec 10: a pending account visiting /decks lands on /invite", async () => {
    signedIn();
    serveAs("pending");
    at("/decks");
    render(<App />);
    await waitFor(() => {
      expect(pathname()).toBe("/invite");
    }, SLOW);
    // And the code screen (another task's) is what renders there.
    await gateOpened();
  });

  it("spec 10: /invite is reachable by a pending account and stays put", async () => {
    signedIn();
    serveAs("pending");
    at("/invite");
    render(<App />);
    await gateOpened();
    expect(pathname()).toBe("/invite");
  });

  it("spec 09 and 10: an active account stays on /decks, neither on /login nor on /invite", async () => {
    signedIn();
    serveAs("active");
    at("/decks");
    render(<App />);
    await gateOpened();
    expect(pathname()).toBe("/decks");
    expect(pathname()).not.toBe("/login");
    expect(pathname()).not.toBe("/invite");
  });

  it("an active account may also open /invite", async () => {
    signedIn();
    serveAs("active");
    at("/invite");
    render(<App />);
    await gateOpened();
    expect(pathname()).toBe("/invite");
  });

  it("holds the screen while /api/auth/me is in flight, instead of guessing", () => {
    signedIn();
    serveAs("active");
    at("/decks");
    render(
      <Gated>
        {() => (
          <p>the gate opened</p>
        )}
      </Gated>,
    );
    expect(screen.getByTestId("gate-loading")).toBeInTheDocument();
    expect(pathname()).toBe("/decks");
  });
});

// ---------------------------------------------------------------------------------------------
// the username prompt (R1435)
// ---------------------------------------------------------------------------------------------

describe("R1435 the username prompt at the gate", () => {
  const OWED: OwnUsername = { name: "Player#7", nextChangeAt: null, promptOwed: true };

  it("R1435 an active account that owes the prompt meets it before the screen it asked for", async () => {
    signedIn();
    serveAs("active", OWED);
    at("/decks");
    render(<App />);

    const prompt = await screen.findByTestId("username-prompt", undefined, SLOW);
    expect(prompt).toHaveTextContent("You’re Player#7 for now.");
    expect(screen.getByTestId("username-prompt-skip")).toBeInTheDocument();
    expect(pathname(), "the prompt stands in place, it does not move the browser").toBe("/decks");
  });

  it("R1435 the prompt stands in front of a gated screen's own content", async () => {
    signedIn();
    serveAs("active", OWED);
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);

    expect(await screen.findByTestId("username-prompt", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("the gate opened")).toBeNull();
  });

  it("R1435 an active account that has picked or skipped goes straight through", async () => {
    signedIn();
    serveAs("active", { ...OWED, promptOwed: false });
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);

    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });

  it("R1435 a server that sends no username owes no prompt", async () => {
    signedIn();
    serveAs("active");
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);

    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });

  it("R1435 a re-read that finds the prompt owed once the screen is up leaves the screen in place", async () => {
    signedIn();
    serveAs("active");
    render(<Gated>{({ me }) => <p>{me.username === undefined ? "the gate opened" : "the gate read again"}</p>}</Gated>);
    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();

    // A server that now owes the prompt (one that gained usernames meanwhile), read in the background:
    // the screen takes the new read and stays.
    serveAs("active", OWED);
    announceAccountChange();
    expect(await screen.findByText("the gate read again", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });

  it("R1435 a screen shown to the account keeps the prompt off every screen after it on the page", async () => {
    signedIn();
    serveAs("active");
    render(<Gated>{() => <p>the first screen</p>}</Gated>);
    expect(await screen.findByText("the first screen", undefined, SLOW)).toBeInTheDocument();
    cleanup();

    // The server now owes the prompt, and the player moves on (the match a queue just paired): the
    // next screen's own gate reads it, and opens its screen all the same.
    serveAs("active", OWED);
    render(<Gated>{({ me }) => <p>{me.username?.promptOwed === true ? "the next screen" : "a stale read"}</p>}</Gated>);
    expect(await screen.findByText("the next screen", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });

  it("R1435 an account in a match or a Conquest series goes to its screen, prompt owed or not", async () => {
    for (const playing of [{ currentMatchId: "m-1" }, { currentSeriesId: "s-1" }]) {
      signedIn();
      vi.stubGlobal(
        "fetch",
        vi.fn((input: unknown) =>
          String(input).endsWith("/api/auth/me")
            ? Promise.resolve(jsonResponse(200, { ...meBody("active", OWED), ...playing }))
            : Promise.resolve(jsonResponse(404, { error: { code: "not_found", message: "no stub" } })),
        ),
      );
      render(<Gated>{() => <p>the board</p>}</Gated>);
      expect(await screen.findByText("the board", undefined, SLOW)).toBeInTheDocument();
      expect(screen.queryByTestId("username-prompt")).toBeNull();
      cleanup();
      resetUsernamePromptForTests();
    }
  });

  it("R1435 a skip opens the screen, even when the read of the account after it fails", async () => {
    signedIn();
    let skipped = false;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: unknown) => {
        const url = String(input);
        if (url.endsWith("/api/username/skip")) {
          skipped = true;
          return Promise.resolve(jsonResponse(200, { username: { ...OWED, promptOwed: false } }));
        }
        if (url.endsWith("/api/auth/me") && !skipped) return Promise.resolve(jsonResponse(200, meBody("active", OWED)));
        return Promise.resolve(jsonResponse(503, { error: { code: "internal", message: "down" } }));
      }),
    );
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);
    fireEvent.click(await screen.findByTestId("username-prompt-skip", undefined, SLOW));

    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();
    expect(skipped).toBe(true);
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });

  it("R1435 a redemption on a screen open to the pending account is followed by the prompt", async () => {
    signedIn();
    serveAs("pending");
    render(<Gated allowPending>{() => <p>the code screen</p>}</Gated>);
    expect(await screen.findByText("the code screen", undefined, SLOW)).toBeInTheDocument();

    serveAs("active", OWED);
    announceAccountChange();
    expect(await screen.findByTestId("username-prompt", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("the code screen")).toBeNull();
  });

  it("R1435 a pending account sees only the code screen, never the prompt", async () => {
    signedIn();
    serveAs("pending", OWED);
    at("/decks");
    render(<App />);

    await waitFor(() => {
      expect(pathname()).toBe("/invite");
    }, SLOW);
    await gateOpened();
    expect(await screen.findByTestId("invite-code-input", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("username-prompt")).toBeNull();
  });
});

// ---------------------------------------------------------------------------------------------
// routes that need no gate
// ---------------------------------------------------------------------------------------------

describe("the route table", () => {
  it("/login is ungated: it is the door the gate sends people to", async () => {
    at("/login");
    render(<App />);
    expect(await screen.findByTestId("login-submit", undefined, SLOW)).toBeInTheDocument();
    expect(pathname()).toBe("/login");
  });

  it("an unknown path is a 404 panel, not a redirect", async () => {
    at("/nope");
    render(<App />);
    expect(await screen.findByText(/that page doesn.t exist/i, undefined, SLOW)).toBeInTheDocument();
    expect(pathname()).toBe("/nope");
  });
});

// ---------------------------------------------------------------------------------------------
// /match/<id>
// ---------------------------------------------------------------------------------------------

describe("/match/<id>", () => {
  /** The match route opens a real `WebSocket`; jsdom has one and nothing to connect it to. */
  class StubSocket {
    static opened: string[] = [];
    readyState = 0;
    onopen: ((event: unknown) => void) | null = null;
    onmessage: ((event: { data: unknown }) => void) | null = null;
    onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
    onerror: ((event: unknown) => void) | null = null;
    constructor(url: string) {
      StubSocket.opened.push(url);
    }
    send(): void {}
    close(): void {}
  }

  beforeEach(() => {
    StubSocket.opened = [];
    vi.stubGlobal("WebSocket", StubSocket);
  });

  it("routes the id out of the path and opens a socket for it", async () => {
    signedIn("tok-1");
    serveAs("active");
    at("/match/m-42");
    render(<App />);

    expect(await screen.findByTestId("match-connecting", undefined, SLOW)).toBeInTheDocument();
    expect(pathname()).toBe("/match/m-42");
    await waitFor(() => {
      expect(StubSocket.opened.length).toBeGreaterThan(0);
    }, SLOW);
    // `wsServer.ts` `tokenFrom` reads `?token=`, and the match from `?matchId=`: a browser cannot
    // put either on a handshake header.
    const url = StubSocket.opened[0] ?? "";
    expect(url).toContain("token=tok-1");
    expect(url).toContain("matchId=m-42");
  });

  it("R765 a tab queued on /play is taken from any other screen to its game once the queue pairs it", async () => {
    signedIn("tok-1");
    rememberQueued("bo1");
    let paired = false;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: unknown) => {
        const url = String(input);
        if (url.endsWith("/api/auth/me")) {
          return Promise.resolve(jsonResponse(200, { ...meBody("active"), currentMatchId: paired ? "m-77" : null }));
        }
        if (url.endsWith("/api/catalog")) return Promise.resolve(jsonResponse(200, { version: "v1", defs: {} }));
        return Promise.resolve(jsonResponse(404, { error: { code: "not_found", message: `no stub for ${url}` } }));
      }),
    );
    at("/");
    render(<App />);
    expect(await screen.findByTestId("landing", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("queue-found")).toBeNull();

    paired = true;
    expect(await screen.findByTestId("queue-found", undefined, { timeout: 10_000 })).toHaveTextContent("Match found!");
    expect(readQueued(), "paired: out of the queue").toBeNull();
    await waitFor(() => {
      expect(pathname()).toBe("/match/m-77");
    }, SLOW);
    expect(await screen.findByTestId("match-connecting", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId("queue-found")).toBeNull();
  });

  it("a pending account cannot reach a match either (§9.4: 'no ... queue or match')", async () => {
    signedIn();
    serveAs("pending");
    at("/match/m-42");
    render(<App />);
    await waitFor(() => {
      expect(pathname()).toBe("/invite");
    }, SLOW);
    expect(StubSocket.opened).toEqual([]);
  });
});
