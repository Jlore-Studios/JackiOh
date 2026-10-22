// R1295000: on the board, only the viewer's own cards wear the device's card cosmetics — the hand,
// the units and face-up backrow cards it owns, a unit the opponent has taken included — and only on
// a board that wears them (`wearCosmetics`). The opponent's cards, and every card on a board that
// wears none, draw the defaults whatever the device picked.
//
// R1297000: the same holds for the deck slots: the playmat under the viewer's half of the field, the
// backdrop behind it, and the back of the viewer's face-down cards and deck pile. A saved deck's
// picks beat the account's; the opponent's side and a board that wears none draw as they always did.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import {
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  deckCosmeticsOf,
  resetCosmeticsForTests,
  writeCardPick,
  writeCosmetics,
  type CosmeticId,
  type CosmeticKind,
} from "../cards/cosmetics.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { INSPECT_HOVER, INSPECT_SHEET } from "../cards/inspect/testids.ts";
import { baseView, card, emptySide, faceUpBackrow, fullBoardView, resetIds, unit } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { BACKDROP_PARTICLE_MAX } from "./config.ts";

/** The registry's first entry of `kind` that is not its default. */
function item(kind: CosmeticKind): CosmeticId {
  const found = cosmeticsOf(kind).find((cosmetic) => !cosmetic.default);
  if (found === undefined) throw new Error(`no ${kind} item`);
  return found.id;
}

const BORDER = item("border");

function view(): PlayerView {
  resetIds();
  return baseView({
    you: emptySide("p1", {
      hand: [card({ defId: "core-004" })],
      // Your own unit, and the opponent's unit you have taken: owned by p2, controlled by you.
      units: [unit("p1"), unit("p2", { controller: "p1" }), null, null, null],
      backrow: [faceUpBackrow("p1"), null, null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      // Their own unit, and yours they have taken: owned by you still.
      units: [unit("p2"), unit("p1", { controller: "p2" }), null, null, null],
      backrow: [faceUpBackrow("p2"), null, null, null, null],
    }),
  });
}

/** The face inside each card the board draws with this owner. */
function facesOwnedBy(owner: string): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(`.card[data-owner="${owner}"] > .cf`)];
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCosmetics({ foil: item("foil"), finish: item("finish"), border: BORDER });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
});

describe("R1295000 the board's cards and the device's cosmetics", () => {
  it("R1295000 the opponent's cards draw the defaults whatever the device picked; the viewer's own wear them", () => {
    render(<Board view={view()} wearCosmetics />);

    const hand = [...screen.getByTestId("hand-you").querySelectorAll<HTMLElement>(".cf")];
    expect(hand).toHaveLength(1);
    // Two units, one on each side of the board, and a face-up backrow card are the viewer's own.
    const yours = facesOwnedBy("p1");
    expect(yours).toHaveLength(3);
    for (const face of [...hand, ...yours]) expect(face.getAttribute("data-frame")).toBe(BORDER);

    const theirs = facesOwnedBy("p2");
    expect(theirs).toHaveLength(3);
    for (const face of theirs) {
      expect(face.getAttribute("data-frame")).toBeNull();
      expect(face.getAttribute("data-finish")).toBeNull();
      expect(face.getAttribute("data-cosmetic-foil")).toBeNull();
    }
  });

  it("R1295000 a board that wears none draws every card with the defaults", () => {
    render(<Board view={view()} />);
    expect(document.querySelectorAll(".cf").length).toBeGreaterThan(0);
    expect(document.querySelectorAll(".cf[data-frame], .cf[data-finish], .cf[data-cosmetic-foil]")).toHaveLength(0);
  });
});

const TABLE = item("table");
const FIELD = item("field");
const BACK = item("cardBack");

/** The `data-card-back` of every `.cf-back` inside `root`, one entry each (null for the default). */
function backs(root: ParentNode): (string | null)[] {
  return [...root.querySelectorAll<HTMLElement>(".cf-back")].map((back) => back.getAttribute("data-card-back"));
}

function ownBacks(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>('.zone[data-side="you"] .cf-back')];
}

function theirBacks(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>('.zone[data-side="opponent"] .cf-back, .hand-opponent .cf-back')];
}

describe("R1297000 the board's deck slots and the device's cosmetics", () => {
  it("R1297000 the viewer's side wears the playmat, backdrop and card back; the opponent's ignores the local loadout", () => {
    writeCosmetics({ table: TABLE, field: FIELD, cardBack: BACK });
    render(<Board view={fullBoardView()} wearCosmetics />);

    expect(document.querySelector(".field")?.getAttribute("data-table")).toBe(TABLE);
    const backdrop = document.querySelector<HTMLElement>(".board-backdrop[aria-hidden]");
    expect(backdrop?.getAttribute("data-backdrop")).toBe(FIELD);
    expect(backdrop?.querySelectorAll(".board-backdrop-particle")).toHaveLength(BACKDROP_PARTICLE_MAX);
    expect(backdrop?.textContent).toBe("");
    // The layer is the board's first child, behind everything it holds, and the board names it.
    expect(document.querySelector(".board > :first-child")).toBe(backdrop);
    expect(document.querySelector(".board")?.getAttribute("data-backdrop")).toBe(FIELD);

    // Every face-down card of the viewer's own side, and their deck pile, wear one back.
    const own = ownBacks();
    expect(own.length).toBeGreaterThan(0);
    for (const back of own) expect(back.getAttribute("data-card-back")).toBe(BACK);
    const pile = screen.getByTestId("library-you");
    expect(pile.getAttribute("data-card-back")).toBe(BACK);

    // The opponent's backs and deck pile draw as they did.
    const theirs = theirBacks();
    expect(theirs.length).toBeGreaterThan(0);
    for (const back of theirs) expect(back.hasAttribute("data-card-back")).toBe(false);
    expect(screen.getByTestId("library-opponent").hasAttribute("data-card-back")).toBe(false);
    // A back says nothing of the card it hides: no text, whatever it looks like.
    for (const back of [...own, ...theirs]) expect(back.textContent).toBe("");
  });

  it("R1297000 a saved deck's picks beat the account's on the board", () => {
    const otherBack = cosmeticsOf("cardBack").filter((cosmetic) => !cosmetic.default)[1]?.id ?? "";
    expect(otherBack).not.toBe(BACK);
    writeCosmetics({ table: TABLE, cardBack: otherBack });
    const deck = deckCosmeticsOf({ cardBackCosmetic: BACK, fieldCosmetic: FIELD });

    render(<Board view={fullBoardView()} wearCosmetics deckCosmetics={deck} />);

    // The deck's backdrop and back; the account's table, since the deck picked none.
    expect(document.querySelector(".board")?.getAttribute("data-backdrop")).toBe(FIELD);
    expect(document.querySelector(".field")?.getAttribute("data-table")).toBe(TABLE);
    for (const back of ownBacks()) expect(back.getAttribute("data-card-back")).toBe(BACK);
    expect(screen.getByTestId("library-you").getAttribute("data-card-back")).toBe(BACK);
    for (const back of theirBacks()) expect(back.hasAttribute("data-card-back")).toBe(false);
  });

  it("R1297000 a deck's unknown pick draws the default, and the opponent's side is the default either way", () => {
    writeCosmetics({ table: TABLE, field: FIELD, cardBack: BACK });
    const deck = deckCosmeticsOf({ tableCosmetic: "table-marble", fieldCosmetic: "field-snow", cardBackCosmetic: "card-back-gold" });
    render(<Board view={fullBoardView()} wearCosmetics deckCosmetics={deck} />);

    expect(document.querySelector(".field")?.hasAttribute("data-table")).toBe(false);
    expect(document.querySelector(".board-backdrop")).toBeNull();
    expect(document.querySelector(".board")?.hasAttribute("data-backdrop")).toBe(false);
    expect(document.querySelectorAll("[data-card-back]")).toHaveLength(0);
    expect(DEFAULT_CARD_COSMETICS.table).not.toBe("table-marble");
  });

  it("R1297000 the deck sheet's cards the viewer was never shown wear the viewer's card back", () => {
    writeCosmetics({ cardBack: BACK });
    const view = fullBoardView();
    render(<Board view={{ ...view, you: { ...view.you, libraryCount: 3, ownLibrary: { cards: [], unknown: 3 } } }} wearCosmetics />);

    fireEvent.click(screen.getByTestId("library-you"));
    const sheet = screen.getByRole("dialog");
    const inside = [...sheet.querySelectorAll<HTMLElement>(".cf-back")];
    expect(inside).toHaveLength(1);
    expect(inside[0]?.getAttribute("data-card-back")).toBe(BACK);
    expect(inside[0]?.textContent).toBe("");
  });

  it("R1297000 a board that wears none draws the deck slots as today", () => {
    writeCosmetics({ table: TABLE, field: FIELD, cardBack: BACK });
    // Neither the account's picks nor a deck's reach a board nobody wears them on.
    render(<Board view={fullBoardView()} deckCosmetics={deckCosmeticsOf({ tableCosmetic: TABLE, fieldCosmetic: FIELD, cardBackCosmetic: BACK })} />);

    expect(document.querySelector(".field")?.hasAttribute("data-table")).toBe(false);
    expect(document.querySelector(".board-backdrop")).toBeNull();
    expect(document.querySelector(".board")?.hasAttribute("data-backdrop")).toBe(false);
    expect(document.querySelectorAll("[data-card-back]")).toHaveLength(0);
    expect(backs(document).every((back) => back === null)).toBe(true);
    expect(document.querySelectorAll(".cf-back").length).toBeGreaterThan(0);
  });
});

/** Longer than any inspect delay (hover or long-press), as in Card.test.tsx. */
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

describe("R1655000 the board's cards and the picks of each card", () => {
  /** The card with picks of its own, and the card with none. */
  const OWN = "core-004";
  const OTHER = "core-005";
  const CARD_BORDER = (() => {
    const found = cosmeticsOf("border").filter((cosmetic) => !cosmetic.default)[1];
    if (found === undefined) throw new Error("no second border");
    return found.id;
  })();

  /** Both sides play the same card: the viewer's copies wear its picks, the opponent's draw the defaults. */
  function picksView(): PlayerView {
    resetIds();
    return baseView({
      you: emptySide("p1", {
        hand: [card({ defId: OWN }), card({ defId: OTHER })],
        units: [unit("p1", { defId: OWN }), null, null, null, null],
      }),
      opponent: emptySide("p2", { hand: { count: 3 }, units: [unit("p2", { defId: OWN }), null, null, null, null] }),
    });
  }

  function frameOf(root: ParentNode, defId: string): string | null {
    return root.querySelector<HTMLElement>(`.card[data-def-id="${defId}"] > .cf`)?.getAttribute("data-frame") ?? null;
  }

  afterEach(() => {
    closeInspect();
    vi.useRealTimers();
  });

  it("R1655000 the viewer's copy wears its card's picks, another card keeps the account's, the opponent's copy the defaults", () => {
    writeCardPick(OWN, "border", CARD_BORDER);
    render(<Board view={picksView()} wearCosmetics />);

    const hand = screen.getByTestId("hand-you");
    expect(frameOf(hand, OWN)).toBe(CARD_BORDER);
    expect(frameOf(hand, OTHER)).toBe(BORDER);
    // The viewer's unit of the same card wears them too; the opponent's copy draws the defaults.
    const yours = facesOwnedBy("p1");
    expect(yours).toHaveLength(1);
    expect(yours[0]?.getAttribute("data-frame")).toBe(CARD_BORDER);
    const theirs = facesOwnedBy("p2");
    expect(theirs).toHaveLength(1);
    expect(theirs[0]?.getAttribute("data-frame")).toBeNull();

    // A board that wears none draws the card's picks nowhere.
    cleanup();
    render(<Board view={picksView()} />);
    expect(document.querySelectorAll(".cf[data-frame]")).toHaveLength(0);
  });

  it("R1655000 the hover preview and the touch sheet of the viewer's own card wear its picks; the opponent's copy's do not", () => {
    vi.useFakeTimers();
    writeCardPick(OWN, "border", CARD_BORDER);
    render(<Board view={picksView()} wearCosmetics />);
    const hand = screen.getByTestId("hand-you").querySelector<HTMLElement>(`.card[data-def-id="${OWN}"]`);
    if (hand === null) throw new Error("no hand card");

    fireEvent.pointerEnter(hand, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".cf")?.getAttribute("data-frame")).toBe(CARD_BORDER);
    fireEvent.pointerLeave(hand, { pointerType: "mouse" });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();

    fireEvent.pointerDown(hand, { pointerType: "touch", clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_SHEET).querySelector(".cf")?.getAttribute("data-frame")).toBe(CARD_BORDER);
    cleanup();
    closeInspect();

    // The opponent's copy of the same card previews as its defaults.
    render(<Board view={picksView()} wearCosmetics />);
    const opposing = document.querySelector<HTMLElement>('.card[data-owner="p2"]');
    if (opposing === null) throw new Error("no opposing card");
    fireEvent.pointerEnter(opposing, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".cf")?.hasAttribute("data-frame")).toBe(false);
  });
});
