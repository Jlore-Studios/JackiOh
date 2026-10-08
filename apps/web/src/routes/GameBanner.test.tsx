// R765: the one banner the menus show while the player has a game to go back to (GameBanner.tsx),
// and the "Match found!" notice a pairing shows on any screen.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";
import { MATCH_FOUND_STATUS } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { GameBanner, LiveGameBanner, QueueFound, gameBannerTestid } from "./GameBanner.tsx";

vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

afterEach(() => {
  cleanup();
  vi.mocked(navigate).mockReset();
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
    });
  });
});
