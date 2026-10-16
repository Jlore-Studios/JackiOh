// #936 S7 B50: `SpectatorSwitch`, "Let friends watch my games". It draws nothing until the account's
// `allowSpectators` is in (signed out, or an account whose settings carry none), shows it as a switch
// with the delay in its hint, writes a change with `PUT /api/settings` carrying `allowSpectators` alone
// and keeps the server's answer, and puts the switch back with the server's words when it refuses.
//
// The server is a stubbed `fetch` (`test/stubServer.ts`); the signed-in session is put where
// `net/session.ts` reads it.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { must, refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { SpectatorSwitch } from "./SpectatorSwitch.tsx";

/** A stubbed round trip can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const SWITCH = "setting-allowSpectators";

/** The account's copy: `allow` as the server stores it, or no such setting at all (`undefined`). */
function serve(allow: boolean | undefined, put?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/settings") {
      return { status: 200, body: { settings: { groups: {}, ...(allow === undefined ? {} : { allowSpectators: allow }) } } };
    }
    if (call.method === "PUT" && call.path === "/api/settings") {
      return (
        put?.(call) ?? {
          status: 200,
          body: { settings: { groups: {}, allowSpectators: (call.body as { allowSpectators?: unknown } | undefined)?.allowSpectators } },
        }
      );
    }
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-spectators" }));
}

function theSwitch(): HTMLInputElement {
  return screen.getByTestId(SWITCH) as HTMLInputElement;
}

beforeEach(() => {
  window.localStorage.clear();
  signedIn();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("B50 the spectator switch", () => {
  it("B50 draws nothing signed out, and reads nothing", async () => {
    window.localStorage.clear();
    const server = serve(true);
    const { container } = render(<SpectatorSwitch />);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    expect(container).toBeEmptyDOMElement();
    expect(server.calls).toHaveLength(0);
  });

  it("B50 draws nothing when the account's settings carry no allowSpectators", async () => {
    const server = serve(undefined);
    const { container } = render(<SpectatorSwitch />);
    await waitFor(() => {
      expect(server.to("GET", "/api/settings").length).toBeGreaterThan(0);
    }, SLOW);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    expect(container).toBeEmptyDOMElement();
  });

  it("B50 shows allowSpectators as a switch, the delay in its hint", async () => {
    serve(true);
    render(<SpectatorSwitch />);
    const toggle = await screen.findByTestId(SWITCH, undefined, SLOW);
    expect(toggle.getAttribute("role")).toBe("switch");
    expect(theSwitch().checked).toBe(true);
    expect(document.body).toHaveTextContent(String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND)));
  });

  it("B50 turning it off sends PUT /api/settings with allowSpectators alone, and keeps the server's answer", async () => {
    const server = serve(true);
    render(<SpectatorSwitch />);
    await screen.findByTestId(SWITCH, undefined, SLOW);
    fireEvent.click(theSwitch());
    await waitFor(() => {
      expect(server.to("PUT", "/api/settings")).toHaveLength(1);
    }, SLOW);
    const put = must(server.to("PUT", "/api/settings")[0], "the PUT");
    expect(put.body).toEqual({ allowSpectators: false });
    expect(put.headers.authorization).toBe("Bearer tok-spectators");
    await waitFor(() => {
      expect(theSwitch().disabled).toBe(false);
    }, SLOW);
    expect(theSwitch().checked).toBe(false);
  });

  it("B50 a refused change puts the switch back and shows the server's words", async () => {
    const words = "Slow down a little.";
    serve(true, () => refusal(429, "rate_limited", words));
    render(<SpectatorSwitch />);
    await screen.findByTestId(SWITCH, undefined, SLOW);
    fireEvent.click(theSwitch());
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(words);
    await waitFor(() => {
      expect(theSwitch().checked).toBe(true);
    }, SLOW);
  });
});
