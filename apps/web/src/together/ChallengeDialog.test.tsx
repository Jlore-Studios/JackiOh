// #1568: the challenge dialog's deck read offers Try again when it fails, and reads again.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { refusal, stubServer } from "../test/stubServer.ts";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { togetherTestid } from "./testids.ts";

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

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
