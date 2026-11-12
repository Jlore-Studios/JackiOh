// The lobby panel on /play and /together (R3730000, R3730004): Create, Join by code and the player's own lobbies.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { at, signIn, stubServer, type StubServer } from "../test/stubServer.ts";
import { LobbyPanel } from "./LobbyPanel.tsx";
import { lobbyTestid } from "./testids.ts";

const SLOW = { timeout: 4000 };

function serve(lobbies: unknown[] = []): StubServer {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/lobbies") return { status: 200, body: { lobbies } };
    if (call.method === "POST" && call.path === "/api/lobbies") return { status: 201, body: { code: "K7Q2XM" } };
    if (call.method === "GET" && call.path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    return undefined;
  });
}

beforeEach(() => {
  signIn("tok-panel");
  at("/play");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("the lobby panel", () => {
  it("R3730000 Create makes a lobby in the screen's format and mode, and opens it", async () => {
    const server = serve();
    render(<LobbyPanel format="duel" mode="bo3" />);
    fireEvent.click(screen.getByTestId(lobbyTestid.create));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/lobby/K7Q2XM");
    }, SLOW);
    expect(server.to("POST", "/api/lobbies")[0]?.body).toEqual({ format: "duel", mode: "bo3" });
  });

  it("R3730000 Join by code opens the code's lobby, and a code that is none is refused here", async () => {
    serve();
    render(<LobbyPanel format="coop" />);
    fireEvent.change(screen.getByTestId(lobbyTestid.joinInput), { target: { value: "abc123" } });
    fireEvent.click(screen.getByTestId(lobbyTestid.joinSubmit));
    expect(await screen.findByTestId(lobbyTestid.error, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe("/play");
    fireEvent.change(screen.getByTestId(lobbyTestid.joinInput), { target: { value: "k7q2xm" } });
    fireEvent.click(screen.getByTestId(lobbyTestid.joinSubmit));
    expect(window.location.pathname).toBe("/lobby/K7Q2XM");
  });

  it("R1075000 R3730004 the player's own lobbies of this format are listed, so a reload finds its lobby", async () => {
    serve([
      { code: "DUELAB", settings: { format: "duel", mode: "bo1" }, you: "host", expiresAt: 0 },
      { code: "COOPCD", settings: { format: "coop", difficulty: "easy" }, you: "guest", expiresAt: 0 },
    ]);
    render(<LobbyPanel format="duel" />);
    const mine = await screen.findByTestId(lobbyTestid.mine, undefined, SLOW);
    expect(mine).toHaveTextContent("DUELAB");
    expect(mine).not.toHaveTextContent("COOPCD");
  });
});
