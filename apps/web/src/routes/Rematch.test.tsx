// `routes/Rematch.tsx`: the death screen's rematch offers (SPEC §9.5, R672).
//
// The buttons show only while both sockets are open — ours (`connection`) and theirs
// (`opponentHere`) — and each one sends its stakes. Matching an incoming offer navigates to the
// game the server made; double-or-nothing stays disabled outside ranked matches.
//
// #477: in the result panel the block is part of the panel's row of ways on, its buttons sized and
// coloured as the panel's (Rematch its gold primary), an offer on its way says so on its button,
// and the wait names the offer. e2e/cypress/component/rematch-buttons.cy.tsx measures the layout in
// a real browser; jsdom has none, so the stylesheet rules it rests on are read as text here.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GameResult } from "../game/Result.tsx";
import { navigate, paths } from "../net/navigate.ts";
import { rematchOffer, rematchStatus, type RematchOfferResponse, type RematchStatusResponse } from "../net/api.ts";
import { PLAY_LEAN_NEWEST_KEY, leanNewestLabel } from "../game/LeanNewest.tsx";
import RematchButtons, { RematchWatcher, rematchTestid } from "./Rematch.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, rematchOffer: vi.fn(), rematchStatus: vi.fn() };
});
vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

const TOKEN = "token-1";
const MATCH_ID = "match-1";

function statusOf(overrides: Partial<RematchStatusResponse> = {}): RematchStatusResponse {
  return { youOffered: null, opponentOffer: null, opponentHere: true, matchId: null, ...overrides };
}

function renderButtons(connection: "open" | "closed" = "open", ranked = true) {
  return render(<RematchButtons token={TOKEN} matchId={MATCH_ID} connection={connection} ranked={ranked} />);
}

beforeEach(() => {
  vi.mocked(rematchStatus).mockResolvedValue(statusOf());
  vi.mocked(rematchOffer).mockResolvedValue({ matchId: null });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("RematchButtons", () => {
  it("renders nothing while our own socket is not open", async () => {
    renderButtons("closed");
    await waitFor(() => {
      expect(vi.mocked(rematchStatus)).toHaveBeenCalled();
    });
    expect(screen.queryByTestId(rematchTestid.offer)).toBeNull();
    expect(screen.queryByTestId(rematchTestid.double)).toBeNull();
  });

  it("renders nothing once the opponent is gone", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ opponentHere: false }));
    renderButtons();
    await waitFor(() => {
      expect(vi.mocked(rematchStatus)).toHaveBeenCalled();
    });
    expect(screen.queryByTestId(rematchTestid.offer)).toBeNull();
    expect(screen.queryByTestId(rematchTestid.double)).toBeNull();
  });

  it("both buttons offer with their stakes, and an offer waits for the opponent", async () => {
    renderButtons();
    await waitFor(() => {
      expect(screen.getByTestId(rematchTestid.offer)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId(rematchTestid.offer));
    await waitFor(() => {
      expect(vi.mocked(rematchOffer)).toHaveBeenCalledWith(TOKEN, MATCH_ID, 1);
    });

    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 1 }));
    fireEvent.click(screen.getByTestId(rematchTestid.double));
    await waitFor(() => {
      expect(vi.mocked(rematchOffer)).toHaveBeenCalledWith(TOKEN, MATCH_ID, 2);
    });
    expect(await screen.findByTestId(rematchTestid.status)).toHaveTextContent(/waiting for your opponent/i);
  });

  it("an incoming double offer renders, and meeting it navigates to the new game", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ opponentOffer: 2 }));
    renderButtons();
    expect(await screen.findByTestId(rematchTestid.incoming)).toHaveTextContent(/double-or-nothing/i);

    vi.mocked(rematchOffer).mockResolvedValue({ matchId: "match-2" });
    fireEvent.click(screen.getByTestId(rematchTestid.double));
    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.match("match-2"));
    });
  });

  it("navigates when a poll finds the game equal offers made", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 1, opponentOffer: 1, matchId: "match-9" }));
    renderButtons();
    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.match("match-9"));
    });
  });

  it("R1372 an All Random rematch offers the switch, starts it from the lobby's pick and sends leanNewest with the offer", async () => {
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "true");
    try {
      vi.mocked(rematchStatus).mockResolvedValue(statusOf({ mode: "random" }));
      renderButtons();
      const toggle = await screen.findByTestId(rematchTestid.leanNewest);
      expect(toggle).toBeChecked();
      expect(toggle.closest("label")).toHaveTextContent(leanNewestLabel());

      fireEvent.click(screen.getByTestId(rematchTestid.offer));
      await waitFor(() => {
        expect(vi.mocked(rematchOffer)).toHaveBeenLastCalledWith(TOKEN, MATCH_ID, 1, true);
      });

      // Off, the offer carries no lean, and the device remembers the change for the lobby too.
      fireEvent.click(screen.getByTestId(rematchTestid.leanNewest));
      expect(window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY)).toBe("false");
      fireEvent.click(screen.getByTestId(rematchTestid.double));
      await waitFor(() => {
        expect(vi.mocked(rematchOffer)).toHaveBeenLastCalledWith(TOKEN, MATCH_ID, 2);
      });
    } finally {
      window.localStorage.removeItem(PLAY_LEAN_NEWEST_KEY);
    }
  });

  it("R1372 a Best-of-1 rematch replays its decks, so it shows no switch and sends no lean", async () => {
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "true");
    try {
      vi.mocked(rematchStatus).mockResolvedValue(statusOf({ mode: "bo1" }));
      renderButtons();
      await screen.findByTestId(rematchTestid.offer);
      expect(screen.queryByTestId(rematchTestid.leanNewest)).toBeNull();
      fireEvent.click(screen.getByTestId(rematchTestid.offer));
      await waitFor(() => {
        expect(vi.mocked(rematchOffer)).toHaveBeenLastCalledWith(TOKEN, MATCH_ID, 1);
      });
    } finally {
      window.localStorage.removeItem(PLAY_LEAN_NEWEST_KEY);
    }
  });

  it("double-or-nothing is disabled outside ranked matches", async () => {
    renderButtons("open", false);
    const double = await screen.findByTestId(rematchTestid.double);
    expect(double).toBeDisabled();
    expect(screen.getByText(/needs a ranked match/i)).toBeInTheDocument();
    // The normal offer still works there.
    expect(screen.getByTestId(rematchTestid.offer)).not.toBeDisabled();
  });
});

/** A stylesheet of `game/` as one line, so a rule reads the same however it is wrapped. */
function gameCss(file: string): string {
  return readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "game", file), "utf8").replace(/\s+/g, " ");
}

/** The match route's result panel (routes/match.tsx `resultActions`): the offers, then Back to lobby. */
function renderPanel(ranked = true) {
  return render(
    <GameResult
      result={{ winner: "p1", reason: "hero-death" }}
      viewer="p1"
      form="panel"
      actions={
        <>
          <RematchButtons token={TOKEN} matchId={MATCH_ID} connection="open" ranked={ranked} />
          <button type="button" data-testid="result-back">
            Back to lobby
          </button>
        </>
      }
    />,
  );
}

describe("RematchButtons in the result panel (#477)", () => {
  it("is the panel's first way on, its two offers plain buttons for the panel to size and colour", async () => {
    renderPanel();
    const offer = await screen.findByTestId(rematchTestid.offer);
    const double = screen.getByTestId(rematchTestid.double);

    const actions = document.querySelector(".result-overlay__actions");
    const block = actions?.firstElementChild;
    expect(block).toHaveClass("rematch");
    expect(block?.querySelector(":scope > .rematch-buttons > :first-child")).toBe(offer);
    expect(double.parentElement).toBe(offer.parentElement);
    // Before #477 Rematch wore the shell's `button-primary`: the lobby's blue call to action, beside
    // the panel's gold one. Neither offer carries a class; the panel's rules draw them.
    expect(offer.className).toBe("");
    expect(double.className).toBe("");
    // The route's other ways on follow the block, and the panel's own View the board comes last.
    expect(screen.getByTestId("result-back").previousElementSibling).toBe(block);
    expect(actions?.lastElementChild).toBe(screen.getByTestId("result-view-board"));
  });

  it("puts the incoming offer before the buttons and the wait after them, inside the block", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 1, opponentOffer: 2 }));
    renderPanel();
    const incoming = await screen.findByTestId(rematchTestid.incoming);
    const status = screen.getByTestId(rematchTestid.status);
    const block = document.querySelector(".result-overlay__actions > .rematch");

    expect(incoming.parentElement).toBe(block);
    expect(status.parentElement).toBe(block);
    expect(block?.firstElementChild).toBe(incoming);
    expect(block?.lastElementChild).toBe(status);
  });

  it("the stylesheets lay the block into the panel's row and make Rematch the panel's primary", () => {
    const animations = gameCss("animations.css");
    const reveal = gameCss("reveal.css");

    // The panel's primary rule skips the block, which used to be painted gold whole, and lands on
    // its first button instead.
    expect(animations).toContain(
      ".result-overlay__actions > :first-child:not(.result-overlay__view, .rematch), .result-overlay__actions > .rematch:first-child .rematch-buttons > :first-child { border-color: var(--primary-edge",
    );
    // The offers are the panel's buttons: 44 px targets like Back to lobby and View the board.
    expect(animations).toMatch(
      /\.result-overlay__actions > \.rematch button, \.result-overlay__reopen \{ min-height: 44px; padding: 8px 18px; border-radius: 10px; font-weight: 700; \}/,
    );
    // The block draws no box: its buttons join the row, so nothing beside it is stretched to its
    // height, and its lines take rows of their own without widening the panel.
    expect(reveal).toContain(
      ".result-overlay__actions > .rematch, .result-overlay__actions > .rematch > .rematch-buttons { display: contents; }",
    );
    expect(reveal).toMatch(
      /\.rematch > \.rematch-incoming, \.rematch > \.rematch-note, \.rematch > \.rematch-status \{ flex: 1 0 100%; contain: inline-size; max-width: none;/,
    );
    expect(reveal).toMatch(/\.rematch > \.rematch-incoming \{ order: -1; color: var\(--text/);
  });

  it("an offer on its way says so on its button, and holds both buttons until it lands", async () => {
    let land: (answer: RematchOfferResponse) => void = () => undefined;
    vi.mocked(rematchOffer).mockReturnValue(
      new Promise<RematchOfferResponse>((resolve) => {
        land = resolve;
      }),
    );
    renderPanel();
    const offer = await screen.findByTestId(rematchTestid.offer);
    const double = screen.getByTestId(rematchTestid.double);
    expect(offer).toHaveAttribute("aria-busy", "false");

    fireEvent.click(double);
    expect(double).toHaveTextContent("Offering…");
    expect(double).toHaveAttribute("aria-busy", "true");
    expect(double).toBeDisabled();
    expect(offer).toHaveTextContent("Rematch");
    expect(offer).toHaveAttribute("aria-busy", "false");
    expect(offer).toBeDisabled();

    await act(async () => {
      land({ matchId: null });
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(double).toHaveTextContent("Double or nothing");
    });
    expect(double).toHaveAttribute("aria-busy", "false");
    expect(double).not.toBeDisabled();
    expect(offer).not.toBeDisabled();
  });

  it("the wait names the offer that is out", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 2 }));
    renderPanel();
    expect(await screen.findByTestId(rematchTestid.status)).toHaveTextContent(
      "You offered double-or-nothing. Waiting for your opponent…",
    );
    cleanup();

    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 1 }));
    renderPanel();
    expect(await screen.findByTestId(rematchTestid.status)).toHaveTextContent(
      "You offered a rematch. Waiting for your opponent…",
    );
  });

  it("a refused offer is an alert below the buttons, in the server's words", async () => {
    vi.mocked(rematchOffer).mockRejectedValue(new Error("finish your current match first"));
    renderPanel();
    fireEvent.click(await screen.findByTestId(rematchTestid.offer));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("finish your current match first");
    expect(alert).toHaveClass("rematch-status");
    expect(screen.getByTestId(rematchTestid.offer)).toHaveTextContent("Rematch");
  });
});

describe("RematchWatcher", () => {
  it("renders nothing, and takes the seat to the game the offers created", async () => {
    vi.mocked(rematchStatus).mockResolvedValue(statusOf({ youOffered: 1, opponentOffer: 1, matchId: "match-9" }));
    const { container } = render(<RematchWatcher token={TOKEN} matchId={MATCH_ID} />);

    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.match("match-9"));
    });
    expect(container).toBeEmptyDOMElement();
  });

  it("stays quiet while the seats disagree", async () => {
    render(<RematchWatcher token={TOKEN} matchId={MATCH_ID} />);

    await waitFor(() => {
      expect(vi.mocked(rematchStatus)).toHaveBeenCalled();
    });
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
  });
});
