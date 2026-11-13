// The lobby panel on /play and /together (R3730000, R3730004): Create, Join by code and the player's own lobbies.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { at, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { LobbyPanel } from "./LobbyPanel.tsx";
import { lobbyTestid } from "./testids.ts";

const SLOW = { timeout: 4000 };

function serve(lobbies: unknown[] = [], special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
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

describe("#1900 the way back to a live co-op game", () => {
  const LIVE_FAILED = "Your live co-op game could not be checked: The co-op games are out of reach.";

  it("B42 a co-op panel offers the way back to the live co-op game, and a 1v1 panel does not ask", async () => {
    const server = serve([], (call) =>
      call.method === "GET" && call.path === "/api/duos/live" ? { status: 200, body: { matchId: "dm-5" } } : undefined,
    );
    render(<LobbyPanel format="coop" />);
    expect(await screen.findByTestId(lobbyTestid.rejoin, undefined, SLOW)).toHaveAttribute("href", "/duos/dm-5");
    cleanup();

    const duel = serve();
    render(<LobbyPanel format="duel" />);
    await waitFor(() => {
      expect(duel.to("GET", "/api/lobbies")).toHaveLength(1);
    }, SLOW);
    expect(duel.to("GET", "/api/duos/live")).toHaveLength(0);
    expect(server.to("GET", "/api/duos/live")).toHaveLength(1);
  });

  it("#1900 a live co-op game that could not be read is named in an alert with Try again, and no way back is drawn", async () => {
    serve([], (call) =>
      call.method === "GET" && call.path === "/api/duos/live" ? refusal(500, "internal", "The co-op games are out of reach.") : undefined,
    );
    render(<LobbyPanel format="coop" />);
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(LIVE_FAILED);
    expect(screen.getByTestId(lobbyTestid.liveRetry)).toHaveTextContent("Try again");
    expect(screen.getByTestId(lobbyTestid.liveRetry)).toHaveAccessibleDescription(LIVE_FAILED);
    expect(screen.queryByTestId(lobbyTestid.rejoin)).toBeNull();
  });

  it("#1900 Try again reads the live co-op game again and offers the way back to it", async () => {
    let reads = 0;
    serve([], (call) => {
      if (call.method === "GET" && call.path === "/api/duos/live") {
        reads += 1;
        return reads === 1 ? refusal(500, "internal", "The co-op games are out of reach.") : { status: 200, body: { matchId: "dm-5" } };
      }
      return undefined;
    });
    render(<LobbyPanel format="coop" />);
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(LIVE_FAILED);
    fireEvent.click(screen.getByTestId(lobbyTestid.liveRetry));
    expect(await screen.findByTestId(lobbyTestid.rejoin, undefined, SLOW)).toHaveAttribute("href", "/duos/dm-5");
    expect(reads).toBe(2);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByTestId(lobbyTestid.liveRetry)).toBeNull();
  });
});
