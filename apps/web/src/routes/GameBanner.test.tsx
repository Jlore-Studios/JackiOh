// R765: the one banner the menus show while the player has a game to go back to (GameBanner.tsx),
// and the "Match found!" notice a pairing shows on any screen; R3729001: that notice names the opponent and their rank.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";
import { MATCH_FOUND_STATUS } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { refusal, signIn, stubServer, type Reply } from "../test/stubServer.ts";
import { GameBanner, LiveGameBanner, OpponentLine, QueueFound, gameBannerTestid } from "./GameBanner.tsx";

vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

afterEach(() => {
  cleanup();
  vi.mocked(navigate).mockReset();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("R765 the menus' game banner", () => {
  it("R765 is a notice named by its title, with its line and one way back: a button, or a link", () => {
    const onPress = vi.fn();
    render(
      <GameBanner
        testId="banner"
        title="Game in progress"
        action={{ label: "Resume", testId: "banner-go", onPress }}
        data={{ difficulty: "hard" }}
      >
        Your Hard game is saved.
      </GameBanner>,
    );
    const banner = screen.getByRole("region", { name: "Game in progress" });
    expect(banner).toHaveAttribute("data-testid", "banner");
    expect(banner).toHaveClass("notice");
    expect(banner).toHaveAttribute("data-difficulty", "hard");
    expect(banner).toHaveTextContent("Your Hard game is saved.");
    const go = screen.getByTestId("banner-go");
    expect(go.tagName).toBe("BUTTON");
    fireEvent.click(go);
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it("R765 the live banner names a match, its grace and Rejoin, a real link to the board that moves in place", () => {
    render(<LiveGameBanner game={{ kind: "match", id: "m-1", path: paths.match("m-1") }} />);
    const banner = screen.getByTestId(gameBannerTestid.live);
    expect(banner).toHaveAttribute("data-kind", "match");
    expect(banner).toHaveTextContent("You're in a game");
    expect(banner).toHaveTextContent(`${String(DISCONNECT_GRACE_SECONDS)} seconds`);
    const rejoin = screen.getByTestId(gameBannerTestid.rejoin);
    expect(rejoin.tagName).toBe("A");
    expect(rejoin).toHaveAttribute("href", "/match/m-1");
    expect(rejoin).toHaveTextContent("Rejoin");
    fireEvent.click(rejoin);
    expect(navigate).toHaveBeenCalledWith("/match/m-1");
  });

  it("R765 between the games of a series it names the series and rejoins it on the series screen", () => {
    render(<LiveGameBanner game={{ kind: "series", id: "s-1", path: paths.series("s-1") }} />);
    expect(screen.getByTestId(gameBannerTestid.live)).toHaveAttribute("data-kind", "series");
    expect(screen.getByTestId(gameBannerTestid.live)).toHaveTextContent("Conquest series");
    expect(screen.getByTestId(gameBannerTestid.rejoin)).toHaveAttribute("href", "/series/s-1");
  });

  it("R1566000 a live co-op game names the co-op game and the grace, and Rejoin links /duos/<id>", () => {
    render(<LiveGameBanner game={{ kind: "duos", id: "d-1", path: paths.duos("d-1") }} />);
    const banner = screen.getByTestId(gameBannerTestid.live);
    expect(banner).toHaveAttribute("data-kind", "duos");
    expect(banner).toHaveTextContent("You're in a co-op game");
    expect(banner).toHaveTextContent(`${String(DISCONNECT_GRACE_SECONDS)} seconds`);
    const rejoin = screen.getByTestId(gameBannerTestid.rejoin);
    expect(rejoin.tagName).toBe("A");
    expect(rejoin).toHaveAttribute("href", "/duos/d-1");
    fireEvent.click(rejoin);
    expect(navigate).toHaveBeenCalledWith("/duos/d-1");
  });

  it("R765 no live game, no banner; no pairing, no notice", () => {
    const { container } = render(
      <>
        <LiveGameBanner game={null} />
        <QueueFound game={null} />
      </>,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("R765 a pairing is announced in /play's own words, as a status", () => {
    render(<QueueFound game={{ kind: "match", id: "m-2", path: paths.match("m-2") }} />);
    const found = screen.getByTestId(gameBannerTestid.queueFound);
    expect(found).toHaveAttribute("role", "status");
    expect(found).toHaveTextContent(MATCH_FOUND_STATUS);
  });

  it("R765 the testids are the strings e2e/support/testids.ts mirrors", () => {
    expect(gameBannerTestid).toEqual({
      practice: "practice-resume-banner",
      practiceResume: "practice-resume",
      live: "live-game-banner",
      rejoin: "live-game-rejoin",
      queueFound: "queue-found",
      queueFoundOpponent: "queue-found-opponent",
    });
  });
});

describe("R3729001 the reveal names the opponent and their rank", () => {
  const RANKS_PATH = "/api/matches/m-2/ranks";
  const GAME = { kind: "match", id: "m-2", path: paths.match("m-2") } as const;
  const you = {
    profileId: "11111111-1111-4111-8111-111111111111",
    username: "Ana",
    rank: { tier: "golden", division: 1, pips: 1, pipsPerDivision: 3, floor: "large" },
    streak: 0,
    you: true,
  };
  const them = {
    profileId: "22222222-2222-4222-8222-222222222222",
    username: "Bo#2",
    rank: { tier: "normal", division: 2, pips: 1, pipsPerDivision: 3, floor: "rotten" },
    streak: 0,
    you: false,
  };

  /** A server that answers the ranks read with `reply`. */
  function serve(reply: Reply) {
    signIn("tok-reveal");
    return stubServer((call) => (call.method === "GET" && call.path === RANKS_PATH ? reply : undefined));
  }

  /** Let a read that failed or answered something unusable settle: nothing appears to wait for. */
  async function settle(): Promise<void> {
    await act(async () => {
      for (let i = 0; i < 20; i += 1) await Promise.resolve();
    });
  }

  it("R3729001 names the seat that is not the player's, with its rank, whichever seat it holds", async () => {
    const server = serve({ status: 200, body: { ranked: true, ladder: "constructed", seats: { p1: you, p2: them } } });
    render(<QueueFound game={GAME} />);
    const opponent = await screen.findByTestId(gameBannerTestid.queueFoundOpponent);
    expect(opponent).toHaveTextContent("You face Bo#2, Normal Grape II.");
    expect(opponent.querySelector("bdi.username")).not.toBeNull();
    // It stands in the notice that says the match was found, and the player is not named.
    expect(screen.getByTestId(gameBannerTestid.queueFound)).toHaveTextContent(`${MATCH_FOUND_STATUS} You face Bo#2, Normal Grape II.`);
    expect(screen.getByTestId(gameBannerTestid.queueFound)).not.toHaveTextContent("Ana");
    expect(server.to("GET", RANKS_PATH)).toHaveLength(1);
    expect(server.to("GET", RANKS_PATH)[0]?.headers["authorization"]).toBe("Bearer tok-reveal");
    cleanup();

    // The seats the other way round: the opponent is p1 this time.
    serve({ status: 200, body: { ranked: true, ladder: "constructed", seats: { p1: them, p2: you } } });
    render(<QueueFound game={GAME} />);
    expect(await screen.findByTestId(gameBannerTestid.queueFoundOpponent)).toHaveTextContent("You face Bo#2, Normal Grape II.");
  });

  it("R3729001 an unranked match names the opponent alone, and a Raisin is named as one", async () => {
    serve({ status: 200, body: { ranked: false, ladder: "constructed", seats: { p1: you, p2: them } } });
    render(<OpponentLine matchId="m-2" />);
    const plain = await screen.findByTestId(gameBannerTestid.queueFoundOpponent);
    expect(plain).toHaveTextContent(/^You face Bo#2\.$/);
    cleanup();

    const raisin = { ...them, rank: { tier: "raisin", placementsPlayed: 1, placementGames: 5 } };
    serve({ status: 200, body: { ranked: true, ladder: "random", seats: { p1: you, p2: raisin } } });
    render(<OpponentLine matchId="m-2" />);
    expect(await screen.findByTestId(gameBannerTestid.queueFoundOpponent)).toHaveTextContent("You face Bo#2, Raisin.");
  });

  it("R3729001 falls back to the plain line when the read fails, answers nonsense or has no session", async () => {
    const failed = serve(refusal(404, "not_found", "no such match"));
    render(<QueueFound game={GAME} />);
    await waitFor(() => {
      expect(failed.to("GET", RANKS_PATH)).toHaveLength(1);
    });
    await settle();
    expect(screen.queryByTestId(gameBannerTestid.queueFoundOpponent)).toBeNull();
    expect(screen.getByTestId(gameBannerTestid.queueFound)).toHaveTextContent(MATCH_FOUND_STATUS);
    cleanup();

    for (const body of [{ ranked: true }, "nope", null, { ranked: true, seats: { p1: you, p2: { ...you } } }, { ranked: true, seats: { p1: { ...them, username: "" }, p2: you } }]) {
      const odd = serve({ status: 200, body });
      render(<QueueFound game={GAME} />);
      await waitFor(() => {
        expect(odd.to("GET", RANKS_PATH)).toHaveLength(1);
      });
      await settle();
      expect(screen.queryByTestId(gameBannerTestid.queueFoundOpponent), JSON.stringify(body)).toBeNull();
      expect(screen.getByTestId(gameBannerTestid.queueFound)).toHaveTextContent(MATCH_FOUND_STATUS);
      cleanup();
    }

    // Signed out: no read is made, and nothing throws.
    window.localStorage.clear();
    const signedOut = stubServer(() => undefined);
    render(<QueueFound game={GAME} />);
    await settle();
    expect(signedOut.calls).toHaveLength(0);
    expect(screen.getByTestId(gameBannerTestid.queueFound)).toHaveTextContent(MATCH_FOUND_STATUS);
  });

  it("R3729001 a series or a co-op game has no match to read, and the line is never drawn before the answer", async () => {
    const server = serve({ status: 200, body: { ranked: true, ladder: "constructed", seats: { p1: you, p2: them } } });
    render(<QueueFound game={{ kind: "series", id: "s-1", path: paths.series("s-1") }} />);
    await settle();
    expect(server.calls).toHaveLength(0);
    expect(screen.queryByTestId(gameBannerTestid.queueFoundOpponent)).toBeNull();
    cleanup();

    render(<QueueFound game={GAME} />);
    // The notice is up at once; the opponent joins it when the server has answered.
    expect(screen.getByTestId(gameBannerTestid.queueFound)).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(screen.queryByTestId(gameBannerTestid.queueFoundOpponent)).toBeNull();
    expect(await screen.findByTestId(gameBannerTestid.queueFoundOpponent)).toBeInTheDocument();
  });
});
