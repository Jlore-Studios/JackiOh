// #1568: the challenge dialog's deck read offers Try again when it fails, and reads again.
// #1831: it opens on the last mode and pick, with Send focused, and remembers what it sent.
// R2031000: All Random offers "More cards from" on the device's online pick, sent as `leanSet`.
// R915005: a deck with a custom card, or a trio holding one, is listed off and never picked.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LEAN_NEWEST } from "@jackioh/shared";

import { PLAY_LEAN_SET_KEY, leanLabel, readPlayLean } from "../game/LeanNewest.tsx";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectSkeleton } from "../test/skeleton.ts";
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

// R915005: Imps holds a card from the creator; Core holds none.
const IMPS = { id: "d-imps", name: "Imps", cards: ["custom-ember-imp"], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 };
const CORE = { id: "d-core", name: "Core", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 };
const PRACTICE = {
  ...MANY,
  decks: [IMPS, CORE],
  trios: [
    { id: "t-imps", name: "Imp trio", deckIds: ["d-core", "d-imps", "d-core"], createdAt: 0, updatedAt: 0 },
    { id: "t-core", name: "Core trio", deckIds: ["d-core", "d-core", "d-core"], createdAt: 0, updatedAt: 0 },
  ],
};

const ANN = { profileId: "f-ann", username: "Ann#2" };

function serveDecks(body: object = MANY): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body };
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

  it("#2115 the decks are a panel skeleton while they are read, then the deck picker", async () => {
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    // Right after render the stubbed read has not answered yet.
    expectSkeleton(screen.getByText("Loading your decks…"), "panel");

    expect(await screen.findByTestId(togetherTestid.challengeDeck)).toBeInTheDocument();
    expect(screen.queryByText("Loading your decks…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
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

describe("R2031000 an All Random challenge's more cards from", () => {
  it("R2031000 the picker is All Random's alone and starts on the device's online pick", async () => {
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Classic");
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    await screen.findByTestId(togetherTestid.challengeDeck);
    expect(screen.queryByTestId(togetherTestid.challengeLean)).toBeNull();
    fireEvent.click(screen.getByTestId(challengeModeTestid("random")));
    const picker = await screen.findByTestId<HTMLSelectElement>(togetherTestid.challengeLean);
    expect(picker.value).toBe("Classic");
    expect(picker.closest("label")).toHaveTextContent(leanLabel());
    fireEvent.click(screen.getByTestId(challengeModeTestid("bo3")));
    expect(screen.queryByTestId(togetherTestid.challengeLean)).toBeNull();
  });

  it("R2031000 with a pick the challenge carries leanSet and the device keeps it; with none it carries no lean", async () => {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "random", deckId: null, trioId: null }));
    const server = serveDecks();
    const sent = vi.fn();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={sent} />);

    const picker = await screen.findByTestId<HTMLSelectElement>(togetherTestid.challengeLean);
    expect(picker.value).toBe("");
    fireEvent.change(picker, { target: { value: LEAN_NEWEST } });
    expect(readPlayLean()).toBe(LEAN_NEWEST);
    fireEvent.click(screen.getByTestId(togetherTestid.challengeSend));
    await waitFor(() => {
      expect(sent).toHaveBeenCalledTimes(1);
    });
    expect(server.to("POST", "/api/challenges")[0]?.body).toEqual({ mode: "random", leanSet: LEAN_NEWEST, friendId: "f-ann" });
    cleanup();

    // The next challenge opens on the pick; none sends no lean at all.
    const sentAgain = vi.fn();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={sentAgain} />);
    const again = await screen.findByTestId<HTMLSelectElement>(togetherTestid.challengeLean);
    expect(again.value).toBe(LEAN_NEWEST);
    fireEvent.change(again, { target: { value: "" } });
    fireEvent.click(screen.getByTestId(togetherTestid.challengeSend));
    await waitFor(() => {
      expect(sentAgain).toHaveBeenCalledTimes(1);
    });
    expect(server.to("POST", "/api/challenges")[1]?.body).toEqual({ mode: "random", friendId: "f-ann" });
    expect(readPlayLean()).toBeNull();
  });
});

describe("R915005 the challenge dialog never offers a deck or trio with a custom card", () => {
  it("R915005 Imps is listed off as practice only, the dialog starts on Core, says why, and Send posts Core", async () => {
    const server = serveDecks(PRACTICE);
    const onSent = vi.fn();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={onSent} />);

    const picker = (await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement;
    expect(picker.value).toBe("d-core");
    expect(within(picker).getByRole("option", { name: "Imps (practice only)" })).toBeDisabled();
    expect(within(picker).getByRole("option", { name: "Core" })).toBeEnabled();
    expect(screen.getByTestId(togetherTestid.challengePracticeOnly)).toHaveTextContent(
      "A deck with a custom card plays only in practice, against the AI.",
    );

    fireEvent.click(screen.getByTestId(togetherTestid.challengeSend));
    await waitFor(() => {
      expect(onSent).toHaveBeenCalledTimes(1);
    });
    expect(server.to("POST", "/api/challenges")[0]?.body).toEqual({ mode: "bo1", deckId: "d-core", friendId: "f-ann" });
  });

  it("R915005 a remembered pick naming Imps or its trio falls back to the first that plays online", async () => {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo1", deckId: "d-imps", trioId: "t-imps" }));
    serveDecks(PRACTICE);
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    expect(((await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement).value).toBe("d-core");
    fireEvent.click(screen.getByTestId(challengeModeTestid("bo3")));
    const trios = (await screen.findByTestId(togetherTestid.challengeTrio)) as HTMLSelectElement;
    expect(trios.value).toBe("t-core");
    expect(within(trios).getByRole("option", { name: "Imp trio (practice only)" })).toBeDisabled();
    expect(screen.getByTestId(togetherTestid.challengePracticeOnly)).toHaveTextContent("cannot play Conquest");
  });

  it("R915005 with nothing practice-only there is no note", async () => {
    serveDecks();
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    await screen.findByTestId(togetherTestid.challengeDeck);
    expect(screen.queryByTestId(togetherTestid.challengePracticeOnly)).toBeNull();
    fireEvent.click(screen.getByTestId(challengeModeTestid("bo3")));
    await screen.findByTestId(togetherTestid.challengeTrio);
    expect(screen.queryByTestId(togetherTestid.challengePracticeOnly)).toBeNull();
  });

  it("R915005 with only practice decks nothing is picked and Send stays off", async () => {
    serveDecks({ ...PRACTICE, decks: [IMPS], trios: [] });
    render(<ChallengeDialog friend={ANN} onClose={vi.fn()} onSent={vi.fn()} />);

    const picker = (await screen.findByTestId(togetherTestid.challengeDeck)) as HTMLSelectElement;
    expect(picker.value).toBe("");
    expect(within(picker).getByRole("option", { name: "No deck plays online yet" })).toBeInTheDocument();
    expect(screen.getByTestId(togetherTestid.challengeSend)).toBeDisabled();
    expect(screen.getByTestId(togetherTestid.challengePracticeOnly)).toBeInTheDocument();
  });
});
