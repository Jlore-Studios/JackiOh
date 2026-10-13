// `/leaderboard`: Jlorious first, then the tiers, then the Raisins — and never a rating, nor a
// profile id (R1436).

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { getLeaderboard, type LeaderboardResponse } from "../net/api.ts";
import LeaderboardRoute from "./leaderboard.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getLeaderboard: vi.fn() };
});

const TOKEN = "token-1";

/** Profile ids as the server sends them: keys, never shown (R1436). */
const ID_MAX = "3f6b1c2e-9a4d-4e8f-b1a2-5c6d7e8f9a0b";
const ID_ME = "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d";
const ID_LENA = "9e8d7c6b-5a4f-4e3d-2c1b-0a9f8e7d6c5b";

function board(over: Partial<LeaderboardResponse> = {}): LeaderboardResponse {
  return {
    season: "v0.2",
    jlorious: [
      { position: 1, profileId: ID_MAX, username: "Max", you: false },
      { position: 2, profileId: ID_ME, username: "محمد#2", you: true },
    ],
    tiers: [
      { tier: "mythic", count: 0, players: [] },
      { tier: "golden", count: 1, players: [{ profileId: ID_LENA, username: "Lena#7", division: 1, pips: 2, you: false }] },
      { tier: "large", count: 0, players: [] },
      { tier: "normal", count: 0, players: [] },
      { tier: "rotten", count: 0, players: [] },
    ],
    raisins: 3,
    you: { tier: "jlorious", position: 2 },
    ...over,
  };
}

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("the leaderboard screen", () => {
  it("lists Jlorious by position, the tiers with their counts, and the Raisins", async () => {
    vi.mocked(getLeaderboard).mockResolvedValue(board());
    render(<LeaderboardRoute token={TOKEN} />);

    expect(await screen.findByTestId("leaderboard-season")).toHaveTextContent("Season v0.2");
    const jlorious = screen.getByTestId("leaderboard-jlorious");
    expect(jlorious.textContent).toMatch(/#1 Max/);
    expect(jlorious.textContent).toMatch(/#2 محمد#2 \(you\)/);
    const tiers = screen.getAllByTestId("leaderboard-tier");
    expect(tiers).toHaveLength(5);
    expect(tiers[1]?.textContent).toMatch(/Golden Grape \(1\)/);
    expect(tiers[1]?.textContent).toMatch(/Lena#7 · Division I · 2 pips/);
    expect(screen.getByTestId("leaderboard-raisins")).toHaveTextContent("3 players are still playing placements.");
    expect(screen.getByTestId("leaderboard-you")).toHaveTextContent("You are Jlorious #2");
  });

  it("R1436 shows every player by username, isolated, and no profile id nor any slice of one", async () => {
    vi.mocked(getLeaderboard).mockResolvedValue(board());
    const { container } = render(<LeaderboardRoute token={TOKEN} />);

    const jlorious = await screen.findByTestId("leaderboard-jlorious");
    const names = [...container.querySelectorAll("bdi.username")].map((name) => name.getAttribute("title"));
    expect(names).toEqual(["Max", "محمد#2", "Lena#7"]);
    expect(jlorious.querySelector("bdi.username .username__tag")?.textContent).toBe("#2");

    const text = container.textContent;
    for (const id of [ID_MAX, ID_ME, ID_LENA]) {
      expect(text).not.toContain(id);
      expect(text).not.toContain(id.slice(0, 8));
    }
  });

  it("says so when Jlorious is empty and nobody is placing", async () => {
    vi.mocked(getLeaderboard).mockResolvedValue(
      board({ jlorious: [], raisins: 0, you: { tier: "raisin", placementsPlayed: 1, placementGames: 5 } }),
    );
    render(<LeaderboardRoute token={TOKEN} />);

    expect(await screen.findByText(/no jlorious players yet/i)).toBeInTheDocument();
    expect(screen.getByTestId("leaderboard-raisins")).toHaveTextContent("Nobody is playing placements right now.");
    expect(screen.getByTestId("leaderboard-you")).toHaveTextContent("You are Raisin · 1/5 placements");
  });

  it("relays a failed read instead of rendering an empty ladder", async () => {
    vi.mocked(getLeaderboard).mockRejectedValue(new Error("the server refused"));
    render(<LeaderboardRoute token={TOKEN} />);

    expect(await screen.findByTestId("leaderboard-error")).toHaveTextContent("the server refused");
  });
});
