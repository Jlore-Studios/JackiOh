// `/decks` with the account's custom cards (SPEC §12.6, R915005; #921): shelved beside the catalog
// under a "Custom" set chip, drawn from their frames, owned once each, and a deck that names one is
// marked practice only in the validator's words (L7).

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DECK_AUTOSAVE_DEBOUNCE_MS } from "@jackioh/server-config";
import type { CustomCard } from "@jackioh/shared";

import { fixtureCatalog, fixtureCollection, legalDecks } from "../game/deckbuilder/fixtures.ts";
import {
  CARD_POOL,
  DECK_PRACTICE_ONLY,
  DECKS_OPEN_CREATOR,
  addPoolId,
  deckRowId,
  filterSetId,
  loadoutErrorId,
  poolCardId,
} from "../game/deckbuilder/testids.ts";
import { decksResponse, savedDeck } from "../game/deckbuilder/testkit.ts";
import { getCatalog, getCollection, getDecks, getMe, putDeck } from "../net/api.ts";
import { getCustomCards } from "../net/customCards.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import DecksRoute from "./decks.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getMe: vi.fn(),
    getDecks: vi.fn(),
    getCatalog: vi.fn(),
    getCollection: vi.fn(),
    putDeck: vi.fn(),
    deleteDeck: vi.fn(),
    putTrio: vi.fn(),
    deleteTrio: vi.fn(),
  };
});
vi.mock("../net/customCards.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/customCards.ts")>();
  return { ...actual, getCustomCards: vi.fn() };
});

const TOKEN = "e2e-token-p1";
const PROFILE = "profile-p1";
const catalog = fixtureCatalog();
const collection = fixtureCollection();
const [ONE = [], TWO = []] = legalDecks();

/** A 1-cost 1/2 Unit with no script: a vanilla custom card. */
const IMP: CustomCard = {
  id: "custom-ember-imp",
  version: 1,
  frame: { name: "Ember Imp", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 2, text: "" },
  source: "",
};

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
  window.history.replaceState(null, "", "/decks");
  vi.mocked(getMe).mockResolvedValue({
    profile: { id: PROFILE, status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
  });
  vi.mocked(getCatalog).mockResolvedValue({ version: catalog.version, defs: catalog.cards });
  vi.mocked(getCollection).mockResolvedValue({
    catalogVersion: catalog.version,
    entries: Object.entries(collection).map(([cardId, quantity]) => ({ cardId, quantity })),
  });
  vi.mocked(getCustomCards).mockResolvedValue({ cards: [IMP], limits: { cards: 50 } });
  vi.mocked(putDeck).mockImplementation(async (_token, id, input) => ({
    deck: { id, ...input, cards: [...input.cards], portrait: input.portrait ?? null, createdAt: 0, updatedAt: 0 },
  }));
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

async function mount(): Promise<void> {
  render(<DecksRoute />);
  await waitFor(() => {
    expect(screen.getByTestId("workshop")).toBeInTheDocument();
  });
}

describe("R915005 the deck builder shelves the account's custom cards", () => {
  it("R915005 a Custom set chip shows only the custom cards, each drawn from its frame, with a way to the creator", async () => {
    vi.mocked(getDecks).mockResolvedValue(decksResponse([savedDeck("d-tempo", "Tempo", TWO.slice(0, 4), 1, catalog.version)], [], catalog.version));
    await mount();
    expect(getCustomCards).toHaveBeenCalledWith(TOKEN);

    const chip = await screen.findByTestId(filterSetId("Custom"));
    expect(chip).toHaveTextContent("Custom");
    // Owned once, as the account's own: it is on the shelf beside the catalog's cards.
    const imp = screen.getByTestId(poolCardId(IMP.id));
    expect(imp).toHaveAttribute("data-owned", "true");
    expect(imp).toHaveAccessibleName(/Ember Imp/);

    fireEvent.click(chip);
    expect(chip).toHaveAttribute("aria-pressed", "true");
    const pool = screen.getByTestId(CARD_POOL);
    const shown = within(pool)
      .getAllByRole("button")
      .map((button) => button.getAttribute("data-card"))
      .filter((id): id is string => id !== null);
    expect(shown).toEqual([IMP.id]);

    const creator = screen.getByTestId(DECKS_OPEN_CREATOR);
    expect(creator).toHaveAttribute("href", paths.creator);
  });

  it("R915005 an account with no custom card gets no Custom chip, and the creator link still shows", async () => {
    vi.mocked(getCustomCards).mockResolvedValue({ cards: [], limits: { cards: 50 } });
    vi.mocked(getDecks).mockResolvedValue(decksResponse([savedDeck("d-tempo", "Tempo", TWO.slice(0, 4), 1, catalog.version)], [], catalog.version));
    await mount();
    await waitFor(() => {
      expect(getCustomCards).toHaveBeenCalled();
    });
    expect(screen.queryByTestId(filterSetId("Custom"))).toBeNull();
    expect(screen.getByTestId(DECKS_OPEN_CREATOR)).toBeInTheDocument();
  });
});

describe("R915005 a deck with a custom card is marked practice only", () => {
  it("R915005 a saved deck naming a custom card is Complete and practice only, in the row and in its verdict", async () => {
    const practice = [...ONE.slice(0, ONE.length - 1), IMP.id];
    vi.mocked(getDecks).mockResolvedValue(
      decksResponse(
        [savedDeck("d-custom", "Imps", practice, 1, catalog.version), savedDeck("d-plain", "Plain", TWO, 2, catalog.version)],
        [],
        catalog.version,
      ),
    );
    await mount();

    const row = screen.getByTestId(deckRowId("d-custom"));
    expect(row).toHaveAttribute("data-practice-only", "true");
    expect(row).toHaveAttribute("data-status", "complete");
    expect(within(row).getByTestId(DECK_PRACTICE_ONLY)).toHaveTextContent("Practice only");
    expect(row).toHaveAccessibleName(/practice only/);

    const plain = screen.getByTestId(deckRowId("d-plain"));
    expect(plain).not.toHaveAttribute("data-practice-only");
    expect(plain).toHaveAttribute("data-status", "ready");
    expect(within(plain).queryByTestId(DECK_PRACTICE_ONLY)).toBeNull();

    // The first deck is open: its verdict says why it cannot queue, in the validator's own L7.
    const editor = screen.getByTestId("deck-editor");
    expect(editor).toHaveAttribute("data-deck", "d-custom");
    expect(within(editor).getByTestId(DECK_PRACTICE_ONLY)).toBeInTheDocument();
    const l7 = within(editor).getByTestId(loadoutErrorId("L7"));
    expect(l7).toHaveAttribute("data-card", IMP.id);
  });

  it("R915005 adding a custom card to a deck makes it practice only, and the draft saves with it", async () => {
    vi.mocked(getDecks).mockResolvedValue(decksResponse([savedDeck("d-tempo", "Tempo", TWO.slice(0, 4), 1, catalog.version)], [], catalog.version));
    await mount();
    const row = screen.getByTestId(deckRowId("d-tempo"));
    expect(row).not.toHaveAttribute("data-practice-only");

    fireEvent.click(await screen.findByTestId(addPoolId(IMP.id)));
    await waitFor(() => {
      expect(screen.getByTestId(deckRowId("d-tempo"))).toHaveAttribute("data-practice-only", "true");
    });
    expect(screen.getByTestId(deckRowId("d-tempo"))).toHaveAttribute("data-status", "incomplete");
    expect(within(screen.getByTestId("deck-editor")).getByTestId(DECK_PRACTICE_ONLY)).toBeInTheDocument();
    // D3 keeps a well-formed custom id in a draft (R250), so the save carries it.
    await waitFor(
      () => {
        expect(putDeck).toHaveBeenCalled();
      },
      { timeout: DECK_AUTOSAVE_DEBOUNCE_MS * 4 },
    );
    const [, , input] = vi.mocked(putDeck).mock.calls.at(-1) ?? [];
    expect(input?.cards).toContain(IMP.id);
  });
});
