// `routes/Rematch.tsx`: the death screen's rematch offers (SPEC §9.5, R672).
//
// The buttons show only while both sockets are open — ours (`connection`) and theirs
// (`opponentHere`) — and each one sends its stakes. Matching an incoming offer navigates to the
// game the server made; double-or-nothing stays disabled outside ranked matches.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { navigate, paths } from "../net/navigate.ts";
import { rematchOffer, rematchStatus, type RematchStatusResponse } from "../net/api.ts";
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

  it("double-or-nothing is disabled outside ranked matches", async () => {
    renderButtons("open", false);
    const double = await screen.findByTestId(rematchTestid.double);
    expect(double).toBeDisabled();
    expect(screen.getByText(/needs a ranked match/i)).toBeInTheDocument();
    // The normal offer still works there.
    expect(screen.getByTestId(rematchTestid.offer)).not.toBeDisabled();
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
