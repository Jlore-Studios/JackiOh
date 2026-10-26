// #1568: the challenge dialog's deck read offers Try again when it fails, and reads again.
// #1831: it opens on the last mode and pick, with Send focused, and remembers what it sent.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { refusal, stubServer } from "../test/stubServer.ts";
import { CHALLENGE_CHOICE_KEY, readChallengeChoice } from "./challengeChoice.ts";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { challengeModeTestid, togetherTestid } from "./testids.ts";

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

const MANY = {
  catalogVersion: "v1",
  decks: [
    { id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 },
    { id: "d-control", name: "Control", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 },
  ],
  trios: [
    { id: "t-one", name: "Trio One", deckIds: ["d-aggro", "d-control", null], createdAt: 0, updatedAt: 0 },
    { id: "t-two", name: "Trio Two", deckIds: ["d-aggro", "d-control", "d-aggro"], createdAt: 0, updatedAt: 0 },
  ],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

const ANN = { profileId: "f-ann", username: "Ann#2" };

function serveDecks(): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body: MANY };
    if (call.method === "POST" && call.path === "/api/challenges") {
      return { status: 200, body: { challengeId: "ch-1", roomCode: "HJK234" } };
    }
    return undefined;
  });
}

beforeEach(() => {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-challenge" }));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("#1568 the challenge dialog's deck read", () => {
  it("#1568 decks that could not be read offer Try again, which reads them again and offers the deck", async () => {
    let reads = 0;
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/decks") {
        reads += 1;
        return reads === 1 ? refusal(500, "internal", "The decks are out of reach.") : { status: 200, body: DECKS };
      }
      return undefined;
    });
    render(<ChallengeDialog friend={{ profileId: "f-ann", username: "Ann#2" }} onClose={vi.fn()} onSent={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Your decks could not be loaded: The decks are out of reach.",
    );

    fireEvent.click(screen.getByTestId(togetherTestid.challengeDecksRetry));
    const picker = await screen.findByTestId(togetherTestid.challengeDeck);
    expect([...(picker as HTMLSelectElement).options].map((option) => option.textContent)).toEqual(["Aggro"]);
    expect(reads).toBe(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

describe("#1831 the challenge dialog opens on the last pick", () => {
  it("#1831 nothing remembered: Best of 1, the first deck, Send focused", async () => {
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    // Until the decks are read there is nothing to send: the focus starts on the mode.
    expect(screen.getByTestId(challengeModeTestid("bo1"))).toBeChecked();
    expect(screen.getByTestId(challengeModeTestid("bo1"))).toHaveFocus();

    const picker = (await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement;
    expect(picker.value).toBe("d-aggro");
    await waitFor(() => {
      expect(screen.getByTestId(togetherTestid.challengeSend)).toHaveFocus();
    });
  });

  it("#1831 the last mode and trio, Send focused", async () => {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo3", deckId: "d-control", trioId: "t-two" }));
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    expect(screen.getByTestId(challengeModeTestid("bo3"))).toBeChecked();
    const picker = (await screen.findByTestId(togetherTestid.challengeTrio)) as HTMLSelectElement;
    expect(picker.value).toBe("t-two");
    await waitFor(() => {
      expect(screen.getByTestId(togetherTestid.challengeSend)).toHaveFocus();
    });
    // The deck remembered for Best of 1 is waiting there too.
    fireEvent.click(screen.getByTestId(challengeModeTestid("bo1")));
    expect(((await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement).value).toBe("d-control");
  });

  it("#1831 All Random, remembered, needs no deck: Send is focused at once", () => {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "random", deckId: null, trioId: null }));
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    expect(screen.getByTestId(challengeModeTestid("random"))).toBeChecked();
    expect(screen.getByTestId(togetherTestid.challengeSend)).toHaveFocus();
  });

  it("#1831 a deck no longer saved falls back to the first", async () => {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo1", deckId: "d-deleted", trioId: "t-gone" }));
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    const picker = (await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement;
    expect(picker.value).toBe("d-aggro");
    fireEvent.click(screen.getByTestId(challengeModeTestid("bo3")));
    // The first trio with three decks, as the lobby picks one with nothing remembered ("Trio One" has two).
    expect(((await screen.findByTestId(togetherTestid.challengeTrio)) as HTMLSelectElement).value).toBe("t-two");
  });

  it("#1831 a sent challenge is remembered", async () => {
    const server = serveDecks();
    const onSent = vi.fn();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={onSent} />);

    const picker = await screen.findByTestId(togetherTestid.challengeDeck);
    fireEvent.change(picker, { target: { value: "d-control" } });
    fireEvent.click(screen.getByTestId(togetherTestid.challengeSend));

    await waitFor(() => {
      expect(onSent).toHaveBeenCalledTimes(1);
    });
    expect(server.to("POST", "/api/challenges")[0]?.body).toEqual({ mode: "bo1", deckId: "d-control", friendId: "f-ann" });
    expect(readChallengeChoice()).toEqual({ mode: "bo1", deckId: "d-control", trioId: null });
  });
});
