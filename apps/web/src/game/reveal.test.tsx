// R434, the game's end reveals both hands: the one adapter the client reads it through
// (`reveal.ts`), and the game-over screen's "Their hand" (Result.tsx, TheirHand.tsx), on fixture
// views. The board's hand row turning face up is Hand.test.tsx's.

import { CATALOG } from "@jackioh/cards";
import type { CardView, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { INSPECT_FACE, INSPECT_HOVER, INSPECT_LIST_CARD, INSPECT_LIST_SHEET, INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/index.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { revealTestid, revealedOpponentHand } from "./reveal.ts";
import { THEIR_HAND_TITLE } from "./TheirHand.tsx";

const lookup = lookupFromDefs(CATALOG);
const nameOf = (defId: string): string => CATALOG[defId]?.name ?? defId;

const THEIRS = ["core-002", "core-019", "core-055"];

function theirCards(): CardView[] {
  return THEIRS.map((defId, index) => card({ instanceId: `t${String(index)}`, defId, radiant: index === 1 }));
}

/** A finished game, as the engine will show it: the opponent's hand as cards (R434). */
function finished(hand: CardView[] = theirCards()): PlayerView {
  return baseView({
    phase: "over",
    result: { winner: "p2", reason: "hero-death" },
    opponent: emptySide("p2", { hand }),
  });
}

function renderGame(view: PlayerView, resultForm: "panel" | "chip" = "panel"): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={[]} onAction={vi.fn()} resultForm={resultForm} />
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

describe("R434 the adapter: the opponent's hand once the view reveals it", () => {
  it("R434 a running game reveals nothing; a finished one shows the opponent's hand as cards", () => {
    expect(revealedOpponentHand(baseView())).toBeNull();
    expect(revealedOpponentHand(baseView({ opponent: emptySide("p2", { hand: { count: 4 } }) }))).toBeNull();
    const hand = theirCards();
    expect(revealedOpponentHand(finished(hand))).toBe(hand);
    // A result is a finished game even before a view says `phase: "over"`.
    expect(revealedOpponentHand({ ...finished(hand), phase: "main" })).toBe(hand);
    expect(revealedOpponentHand(finished([]))).toEqual([]);
  });

  it("R434 cards in a running game's opponent hand are not read as a reveal", () => {
    const view = baseView({ opponent: emptySide("p2", { hand: theirCards() }) });
    expect(revealedOpponentHand(view)).toBeNull();
  });

  it("R434 a field of the engine's own, `revealedHand`, is read as the reveal", () => {
    const hand = theirCards();
    const view = finished();
    const named: PlayerView = {
      ...view,
      opponent: { ...view.opponent, hand: { count: hand.length }, revealedHand: hand } as PlayerView["opponent"],
    };
    expect(revealedOpponentHand(named)).toBe(hand);
  });
});

describe("R434 the game-over screen lists 'Their hand'", () => {
  it("R434 the result panel lists the opponent's hand as faces, each opening large", () => {
    renderGame(finished());
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveAttribute("data-form", "panel");
    const section = within(overlay).getByTestId(revealTestid.theirHand);
    expect(section).toHaveAttribute("data-count", String(THEIRS.length));
    expect(section).toHaveAccessibleName(THEIR_HAND_TITLE);
    expect(section).toHaveTextContent(`${THEIR_HAND_TITLE} · ${String(THEIRS.length)} cards`);
    THEIRS.forEach((defId, index) => {
      const face = within(section).getByTestId(revealTestid.theirHandCard(`t${String(index)}`));
      expect(face).toHaveAttribute("data-def-name", nameOf(defId));
      expect(face).toHaveAccessibleName(`${nameOf(defId)}: show it large`);
    });
    // A Radiant card is shown on its Radiant face, as the view lists it.
    expect(within(section).getByTestId(revealTestid.theirHandCard("t1")).querySelector('[data-radiant-face="true"]')).not.toBeNull();
    expect(within(section).getByTestId(revealTestid.theirHandCard("t0")).querySelector('[data-radiant-face="true"]')).toBeNull();

    // A click opens the card large in the inspect sheet.
    fireEvent.click(within(section).getByTestId(revealTestid.theirHandCard("t2")));
    const sheet = screen.getByTestId(INSPECT_SHEET);
    expect(within(sheet).getByTestId(INSPECT_FACE)).toHaveTextContent(nameOf("core-055"));
  });

  it("R434 a resting mouse on a face shows its preview", () => {
    renderGame(finished());
    const face = screen.getByTestId(revealTestid.theirHandCard("t0"));
    fireEvent.pointerEnter(face, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER)).toHaveTextContent(nameOf("core-002"));
  });

  it("R434 an empty hand says so, and a game without a reveal shows no section", () => {
    renderGame(finished([]));
    expect(screen.getByTestId(revealTestid.theirHand)).toHaveAttribute("data-count", "0");
    expect(screen.getByTestId(revealTestid.theirHand)).toHaveTextContent("Their hand was empty.");
    cleanup();

    renderGame(baseView({ phase: "over", result: { winner: "p1", reason: "concede" } }));
    expect(screen.getByTestId("result-overlay")).toBeInTheDocument();
    expect(screen.queryByTestId(revealTestid.theirHand)).toBeNull();
    expect(screen.queryByTestId(revealTestid.theirHandOpen)).toBeNull();
  });

  it("R434 the chip (practice's, or a folded panel) offers 'Their hand', which opens every card in the list dialog", () => {
    renderGame(finished(), "chip");
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveAttribute("data-form", "chip");
    expect(within(overlay).queryByTestId(revealTestid.theirHand)).toBeNull();
    const open = within(overlay).getByTestId(revealTestid.theirHandOpen);
    expect(open).toHaveAttribute("data-count", String(THEIRS.length));
    expect(open.tagName).toBe("BUTTON");
    // The chip is blind to the pointer but its own buttons (animations.css): this one is a direct child.
    expect(open.parentElement).toBe(overlay);
    fireEvent.click(open);
    const sheet = screen.getByTestId(INSPECT_LIST_SHEET);
    expect(sheet).toHaveAttribute("aria-label", THEIR_HAND_TITLE);
    expect(within(sheet).getAllByTestId(INSPECT_LIST_CARD).map((tile) => tile.getAttribute("data-def-name"))).toEqual(THEIRS.map(nameOf));
  });

  it("R434 a folded panel's chip keeps both ways back: Result and Their hand", () => {
    renderGame(finished());
    fireEvent.click(screen.getByTestId("result-view-board"));
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveAttribute("data-form", "chip");
    expect(within(overlay).getByTestId("result-reopen")).toBeInTheDocument();
    expect(within(overlay).getByTestId(revealTestid.theirHandOpen)).toBeInTheDocument();
    // The empty hand's chip has nothing to open.
    cleanup();
    renderGame(finished([]), "chip");
    expect(screen.queryByTestId(revealTestid.theirHandOpen)).toBeNull();
  });
});
