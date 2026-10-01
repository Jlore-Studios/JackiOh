// R433, a dealt deck lists only the cards its owner has been shown. The engine lists a deck the
// player did not build (All Random, practice's random deck) in `SideView.ownLibrary` with the cards
// its owner has seen and counts the rest as `unknown`, which the deck pile already draws as backs
// (R312). These prove the client's half on fixture views: the pile of a mostly unknown deck, and that
// nothing before the game (practice's preview of its random deck) names a card of a deck still to be
// dealt. The All Random lobby's half is in routes/play.test.tsx.

import { CATALOG } from "@jackioh/cards";
import type { CardDefs, LibraryView, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { INSPECT_LIST_CARD, INSPECT_LIST_COUNT, INSPECT_LIST_HOVER, INSPECT_LIST_SHEET, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/index.ts";
import { DeckPreview } from "../practice/DeckPreview.tsx";
import { RANDOM_DECK_IDENTITY } from "../practice/decks.ts";
import { practiceTestid } from "../practice/testids.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";

const lookup = lookupFromDefs(CATALOG);
const nameOf = (defId: string): string => CATALOG[defId]?.name ?? defId;

/** A dealt deck at the start of a game: two cards its owner has been shown, the rest unknown. */
const DEALT: LibraryView = {
  cards: [
    { defId: "core-010", radiant: false, count: 1 },
    { defId: "core-002", radiant: true, count: 1 },
  ],
  unknown: 18,
};
const DEALT_TOTAL = 20;

function dealtView(library: LibraryView = DEALT): PlayerView {
  const count = library.cards.reduce((sum, entry) => sum + entry.count, 0) + library.unknown;
  return baseView({ you: emptySide("p1", { libraryCount: count, ownLibrary: library }) });
}

function renderBoard(view: PlayerView): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} />
    </CatalogContext.Provider>,
  );
}

beforeEach(() => {
  vi.useFakeTimers();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  __resetSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

describe("R433 a dealt deck lists only the cards its owner has been shown", () => {
  it("R433 the pile of a mostly unknown deck shows the cards seen, then backs for the rest, under 'Your deck'", () => {
    renderBoard(dealtView());
    const pile = screen.getByTestId("library-you");
    expect(pile).toHaveAttribute("aria-label", `Your deck: ${String(DEALT_TOTAL)} cards, order hidden. Show them`);
    expect(screen.getByTestId("library-count-you")).toHaveTextContent(String(DEALT_TOTAL));

    fireEvent.pointerEnter(pile, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    const preview = screen.getByTestId(INSPECT_LIST_HOVER);
    expect(preview).toHaveTextContent("Your deck");
    expect(within(preview).getByTestId(INSPECT_LIST_COUNT)).toHaveAttribute("data-count", String(DEALT_TOTAL));
    const tiles = within(preview).getAllByTestId(INSPECT_LIST_CARD);
    expect(tiles.map((tile) => tile.getAttribute("data-def-name"))).toEqual([nameOf("core-010"), nameOf("core-002"), ""]);
    expect(tiles.map((tile) => tile.getAttribute("data-count"))).toEqual(["1", "1", String(DEALT.unknown)]);
    const backs = tiles[2];
    expect(backs).toHaveAttribute("data-unknown", "true");
    // A back names nothing: its only text is how many cards it stands for.
    expect(backs?.textContent).toBe(`×${String(DEALT.unknown)}`);
  });

  it("R433 a click opens the list: the unknown cards are one back that opens nothing", () => {
    renderBoard(dealtView());
    fireEvent.click(screen.getByTestId("library-you"));
    const sheet = screen.getByTestId(INSPECT_LIST_SHEET);
    expect(sheet).toHaveAttribute("aria-label", "Your deck");
    expect(within(sheet).getByTestId(INSPECT_LIST_COUNT)).toHaveTextContent(`${String(DEALT_TOTAL)} cards`);
    const back = within(sheet).getByRole("img", { name: `${String(DEALT.unknown)} × Unknown card` });
    expect(back.tagName).not.toBe("BUTTON");
    expect(within(sheet).getAllByRole("button", { name: /show it large/ })).toHaveLength(DEALT.cards.length);
  });

  it("R433 a dealt deck its owner has been shown nothing of is backs alone", () => {
    renderBoard(dealtView({ cards: [], unknown: DEALT_TOTAL }));
    fireEvent.click(screen.getByTestId("library-you"));
    const sheet = screen.getByTestId(INSPECT_LIST_SHEET);
    const tiles = within(sheet).getAllByTestId(INSPECT_LIST_CARD);
    expect(tiles).toHaveLength(1);
    expect(tiles[0]).toHaveAttribute("data-unknown", "true");
    expect(tiles[0]).toHaveAttribute("data-count", String(DEALT_TOTAL));
    expect(within(sheet).queryAllByRole("button", { name: /show it large/ })).toHaveLength(0);
  });

  it("R433 before the game, practice's preview of its random deck names no card, however much of the catalog it holds", () => {
    const defs: CardDefs = CATALOG;
    render(<DeckPreview title="Random deck" identity={RANDOM_DECK_IDENTITY} cards={null} defs={defs} defsFailed={false} />);
    const preview = screen.getByTestId(practiceTestid.deckPreview);
    expect(preview).toHaveTextContent("Random deck");
    expect(preview).toHaveTextContent(RANDOM_DECK_IDENTITY);
    expect(screen.queryByTestId(practiceTestid.deckCurve)).toBeNull();
    expect(preview.querySelectorAll('[data-testid^="practice-deck-card-"]')).toHaveLength(0);
    expect(within(preview).queryByRole("list")).toBeNull();
  });
});
