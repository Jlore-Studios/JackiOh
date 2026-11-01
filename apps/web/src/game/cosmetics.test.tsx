// R1295000: on the board, only the viewer's own cards wear the device's card cosmetics — the hand,
// the units and face-up backrow cards it owns, a unit the opponent has taken included — and only on
// a board that wears them (`wearCosmetics`). The opponent's cards, and every card on a board that
// wears none, draw the defaults whatever the device picked. R1953001: what a card wears is its own
// picks, never the account's synced foil, finish and border, which every test here holds non-default.
//
// R1297000: the same holds for the deck slots: the playmat under the viewer's half of the field, the
// backdrop behind it, and the back of the viewer's face-down cards and deck pile. A saved deck's
// picks beat the account's; the opponent's side and a board that wears none draw as they always did.
//
// R1834004: an online board carries both seats' frozen loadouts (R1834002), and every seat whose cards
// do not wear the device's picks wears its own: its half (playmat, backdrop, card back) and its cards,
// by owner, each its card's own picks over its kinds' defaults (R1953001: never the seat's foil, finish
// and border), under the same rarity rules. The viewer's "Show opponents' cosmetics" off leaves only
// the seat the board is seen from dressed.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";

import {
  CARD_SLOT_KINDS,
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  deckCosmeticsOf,
  resetCosmeticsForTests,
  writeCardPick,
  writeCosmetics,
  type CardCosmetics,
  type CardPicks,
  type CosmeticId,
  type CosmeticKind,
} from "../cards/cosmetics.ts";
import { FullArtAllowed } from "../cards/fullArt.ts";
import type { SeatLoadout, SeatLoadouts } from "../cards/loadouts.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { INSPECT_HOVER, INSPECT_SHEET } from "../cards/inspect/testids.ts";
import { baseView, card, emptySide, faceUpBackrow, fullBoardView, pendingFor, resetIds, unit, withEvents } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { BACKDROP_PARTICLE_MAX } from "./config.ts";
import Prompt from "./Prompt.tsx";
import CardShowcase from "./showcase/CardShowcase.tsx";
import { showcaseTestid } from "./showcase/constants.ts";
import { revealTestid } from "./reveal.ts";
import { TheirHand } from "./TheirHand.tsx";
import { WearerProvider } from "./wearer.ts";

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
  beforeEach(() => {
    // The view's cards (core-004 in hand and as units, core-020 in the backrow) each pick their own.
    for (const defId of ["core-004", "core-020"]) {
      for (const kind of CARD_SLOT_KINDS) writeCardPick(defId, kind, item(kind));
    }
  });

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
    const deck = deckCosmeticsOf({ tableCosmetic: "table-unknown", fieldCosmetic: "field-unknown", cardBackCosmetic: "card-back-gold" });
    render(<Board view={fullBoardView()} wearCosmetics deckCosmetics={deck} />);

    expect(document.querySelector(".field")?.hasAttribute("data-table")).toBe(false);
    expect(document.querySelector(".board-backdrop")).toBeNull();
    expect(document.querySelector(".board")?.hasAttribute("data-backdrop")).toBe(false);
    expect(document.querySelectorAll("[data-card-back]")).toHaveLength(0);
    expect(DEFAULT_CARD_COSMETICS.table).not.toBe("table-unknown");
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

  it("R1655000 the viewer's copy wears its card's picks, another card and the opponent's copy the defaults", () => {
    writeCardPick(OWN, "border", CARD_BORDER);
    render(<Board view={picksView()} wearCosmetics />);

    const hand = screen.getByTestId("hand-you");
    expect(frameOf(hand, OWN)).toBe(CARD_BORDER);
    // R1953001: a card with no pick of its own draws the default, not the account's border.
    expect(frameOf(hand, OTHER)).toBeNull();
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

  it("R1655000 the hover preview and the sheet of the viewer's own card wear its picks; the opponent's copy's do not", () => {
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

    // The I key opens the sheet (a touch hold shows the preview, R2617000).
    fireEvent.keyDown(hand, { key: "i" });
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

  it("R1655000 the viewer's mulligan wears its cards' picks on a board that wears them; a Discover's options do not", () => {
    writeCardPick(OWN, "border", CARD_BORDER);
    const hand = { you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: OWN })] }) };
    const mulligan = baseView({ ...hand, pending: pendingFor("mulligan", [{ key: "h1", label: "One", instanceId: "h1" }], { min: 0, max: 1 }) });
    const optionFrame = (): string | null =>
      document.querySelector<HTMLElement>('[data-testid^="prompt-option-"] .cf')?.getAttribute("data-frame") ?? null;

    render(
      <WearerProvider seat="p1">
        <Prompt view={mulligan} onAction={vi.fn()} />
      </WearerProvider>,
    );
    expect(optionFrame()).toBe(CARD_BORDER);
    cleanup();

    // A board that wears none (a replay, a watched game) draws the defaults.
    render(<Prompt view={mulligan} onAction={vi.fn()} />);
    expect(optionFrame()).toBeNull();
    cleanup();

    // A Discover's cards are offered, not the viewer's copies yet.
    const discover = baseView({ ...hand, pending: pendingFor("discover", [{ key: `mode:${OWN}`, label: "Own", defId: OWN }]) });
    render(
      <WearerProvider seat="p1">
        <Prompt view={discover} onAction={vi.fn()} />
      </WearerProvider>,
    );
    expect(optionFrame()).toBeNull();
  });

  it("R1655000 the showcase of the viewer's own cast on draw wears its card's picks; the opponent's wears the defaults", () => {
    writeCardPick(OWN, "border", CARD_BORDER);
    const turn: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
    const lookup = lookupFromDefs(CATALOG);
    const showcaseFrame = (player: "p1" | "p2"): string | null => {
      const castOnDraw: GameEvent[] = [
        { type: "drawn", player, instanceId: "c9", defId: OWN },
        { type: "cardPlayed", player, instanceId: "c9", defId: OWN, costPaid: 0 },
      ];
      const tree = (shown: PlayerView): ReactElement => (
        <CatalogContext.Provider value={lookup}>
          <WearerProvider seat="p1">
            <CardShowcase view={shown} />
          </WearerProvider>
        </CatalogContext.Provider>
      );
      const { rerender } = render(tree(withEvents(baseView(), [turn])));
      rerender(tree(withEvents(baseView(), [turn, ...castOnDraw])));
      return screen.getByTestId(showcaseTestid.face).querySelector<HTMLElement>(".cf")?.getAttribute("data-frame") ?? null;
    };

    expect(showcaseFrame("p1")).toBe(CARD_BORDER);
    cleanup();
    expect(showcaseFrame("p2")).toBeNull();
  });
});

describe("R1956001 the board's Created cards and their generator's picks", () => {
  /** The card that generated the Created card, and the Created card. */
  const GENERATOR = "core-004";
  const CREATED = "core-002";
  const [, GENERATOR_BORDER, CREATED_BORDER] = cosmeticsOf("border")
    .filter((cosmetic) => !cosmetic.default)
    .map((cosmetic) => cosmetic.id);

  /** The viewer's Created card in hand and on the field, and the opponent's copy of it. */
  function createdView(): PlayerView {
    resetIds();
    const made = { defId: CREATED, created: true, createdBy: GENERATOR } as const;
    return baseView({
      you: emptySide("p1", { hand: [card(made)], units: [unit("p1", made), null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 3 }, units: [unit("p2", made), null, null, null, null] }),
    });
  }

  it("R1956001 the viewer's Created card wears its generator's picks; the opponent's copy and a board that wears none draw the defaults", () => {
    if (GENERATOR_BORDER === undefined || CREATED_BORDER === undefined) throw new Error("too few borders");
    writeCardPick(GENERATOR, "border", GENERATOR_BORDER);
    // The Created card's own picks give way to its generator's.
    writeCardPick(CREATED, "border", CREATED_BORDER);
    render(<Board view={createdView()} wearCosmetics />);

    const hand = screen.getByTestId("hand-you").querySelector<HTMLElement>(`.card[data-def-id="${CREATED}"] > .cf`);
    expect(hand?.getAttribute("data-frame")).toBe(GENERATOR_BORDER);
    const yours = facesOwnedBy("p1");
    expect(yours).toHaveLength(1);
    expect(yours[0]?.getAttribute("data-frame")).toBe(GENERATOR_BORDER);
    const theirs = facesOwnedBy("p2");
    expect(theirs).toHaveLength(1);
    expect(theirs[0]?.getAttribute("data-frame")).toBeNull();

    // A board that wears none draws the defaults.
    cleanup();
    render(<Board view={createdView()} />);
    expect(document.querySelectorAll(".cf[data-frame]")).toHaveLength(0);

    // R1834004: a replayed or watched board dresses each seat in its loadout, which reaches a Created
    // card by its own card's entry alone, never its generator's.
    cleanup();
    const generators: SeatLoadout = { seat: { ...DEFAULT_CARD_COSMETICS }, cards: { [GENERATOR]: { border: GENERATOR_BORDER } } };
    render(<Board view={createdView()} loadouts={{ p1: generators, p2: generators }} />);
    expect(facesOwnedBy("p1")).toHaveLength(1);
    expect(document.querySelectorAll(".cf[data-frame]")).toHaveLength(0);
  });
});

describe("R1834004 the other seat wears its frozen loadout", () => {
  /** The registry's second and third entries of `kind` that are not its default (`item` is the first). */
  function others(kind: CosmeticKind): [CosmeticId, CosmeticId] {
    const [, second, third] = cosmeticsOf(kind).filter((cosmetic) => !cosmetic.default);
    if (second === undefined || third === undefined) throw new Error(`fewer than three ${kind} items`);
    return [second.id, third.id];
  }

  /** R1834002: a seat's loadout: every kind's default but those `picks` names, and these cards' own picks. */
  function loadout(picks: Partial<CardCosmetics> = {}, cards: Record<string, CardPicks> = {}): SeatLoadout {
    return { seat: { ...DEFAULT_CARD_COSMETICS, ...picks }, cards };
  }

  const [THEIR_TABLE, THIRD_TABLE] = others("table");
  const [THEIR_FIELD] = others("field");
  const [THEIR_BACK, THIRD_BACK] = others("cardBack");
  const [THEIR_BORDER, THIRD_BORDER] = others("border");
  const [THEIR_FOIL] = others("foil");
  const [THEIR_FINISH] = others("finish");
  const lookup = lookupFromDefs(CATALOG);

  /** The picks of the view's cards (core-004 as units, core-020 in the backrow), each this border. */
  function bordered(border: CosmeticId): Record<string, CardPicks> {
    return { "core-004": { border }, "core-020": { border } };
  }

  function faceOf(owner: string, defId: string): HTMLElement | null {
    return document.querySelector<HTMLElement>(`.card[data-owner="${owner}"][data-def-id="${defId}"] > .cf`);
  }

  beforeEach(() => {
    // The viewer's own copies of the view's cards pick their border (R1953001: never the account's).
    for (const defId of ["core-004", "core-020"]) writeCardPick(defId, "border", BORDER);
  });

  afterEach(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R1834004 the opponent's half wears their playmat, backdrop and back", () => {
    writeCosmetics({ table: TABLE, field: FIELD, cardBack: BACK });
    const loadouts: SeatLoadouts = { p1: loadout(), p2: loadout({ table: THEIR_TABLE, field: THEIR_FIELD, cardBack: THEIR_BACK }) };
    render(<Board view={fullBoardView()} wearCosmetics loadouts={loadouts} />);

    // The viewer's half wears the device's picks, as before; the other half its seat's.
    const field = document.querySelector<HTMLElement>(".field");
    expect(field?.getAttribute("data-table")).toBe(TABLE);
    expect(field?.getAttribute("data-their-table")).toBe(THEIR_TABLE);
    const mat = field?.firstElementChild;
    expect(mat?.classList.contains("field-their-mat")).toBe(true);
    expect(mat?.getAttribute("aria-hidden")).toBe("true");
    expect(mat?.textContent).toBe("");

    const board = document.querySelector(".board");
    expect(board?.getAttribute("data-backdrop")).toBe(FIELD);
    expect(board?.getAttribute("data-their-backdrop")).toBe(THEIR_FIELD);
    expect(document.querySelector(".board > :first-child")?.getAttribute("data-backdrop")).toBe(FIELD);
    const theirs = document.querySelector<HTMLElement>('.board-backdrop[data-half="theirs"]');
    expect(theirs?.getAttribute("data-backdrop")).toBe(THEIR_FIELD);
    expect(theirs?.getAttribute("aria-hidden")).toBe("true");
    expect(theirs?.querySelectorAll(".board-backdrop-particle")).toHaveLength(BACKDROP_PARTICLE_MAX);
    expect(theirs?.textContent).toBe("");

    // Their hand's and backrow's backs, and their deck pile, wear their back; the viewer's their own.
    expect(theirBacks().length).toBeGreaterThan(0);
    for (const back of theirBacks()) expect(back.getAttribute("data-card-back")).toBe(THEIR_BACK);
    expect(screen.getByTestId("library-opponent").getAttribute("data-card-back")).toBe(THEIR_BACK);
    for (const back of ownBacks()) expect(back.getAttribute("data-card-back")).toBe(BACK);
    cleanup();

    // A seat whose loadout is every default adds no attribute and no layer.
    render(<Board view={fullBoardView()} wearCosmetics loadouts={{ p1: loadout(), p2: loadout() }} />);
    expect(document.querySelector(".field")?.hasAttribute("data-their-table")).toBe(false);
    expect(document.querySelector(".field-their-mat")).toBeNull();
    expect(document.querySelector(".board")?.hasAttribute("data-their-backdrop")).toBe(false);
    expect(document.querySelector('.board-backdrop[data-half="theirs"]')).toBeNull();
    for (const back of theirBacks()) expect(back.hasAttribute("data-card-back")).toBe(false);
  });

  it("R1834004 a stolen unit keeps its owner's look", () => {
    const loadouts: SeatLoadouts = { p1: loadout({}, bordered(THIRD_BORDER)), p2: loadout({}, bordered(THEIR_BORDER)) };
    render(<Board view={view()} wearCosmetics loadouts={loadouts} />);

    // Their unit on the viewer's side wears their border; the viewer's on theirs, the device's.
    expect(document.querySelector('.zone[data-side="you"] .card[data-owner="p2"] > .cf')?.getAttribute("data-frame")).toBe(THEIR_BORDER);
    expect(document.querySelector('.zone[data-side="opponent"] .card[data-owner="p1"] > .cf')?.getAttribute("data-frame")).toBe(BORDER);
    expect(facesOwnedBy("p2")).toHaveLength(3);
    for (const face of facesOwnedBy("p2")) expect(face.getAttribute("data-frame")).toBe(THEIR_BORDER);
    for (const face of facesOwnedBy("p1")) expect(face.getAttribute("data-frame")).toBe(BORDER);
    cleanup();

    // A watched or replayed board wears no device picks: every card its owner's loadout, wherever it stands.
    render(<Board view={view()} loadouts={loadouts} />);
    for (const face of facesOwnedBy("p2")) expect(face.getAttribute("data-frame")).toBe(THEIR_BORDER);
    expect(facesOwnedBy("p1")).toHaveLength(3);
    for (const face of facesOwnedBy("p1")) expect(face.getAttribute("data-frame")).toBe(THIRD_BORDER);
  });

  it("R1834004 a card of theirs with no pick of its own wears its kind's default, never its seat's", () => {
    // R1953001: a seat's foil, finish and border dress no card, as its owner's own client draws it.
    const theirs = loadout({ foil: THEIR_FOIL, finish: THEIR_FINISH, border: THEIR_BORDER }, { "core-004": { finish: THEIR_FINISH } });
    render(<Board view={view()} wearCosmetics loadouts={{ p1: loadout(), p2: theirs }} />);

    expect(facesOwnedBy("p2")).toHaveLength(3);
    for (const face of facesOwnedBy("p2")) {
      expect(face.getAttribute("data-cosmetic-foil")).toBeNull();
      expect(face.getAttribute("data-frame")).toBeNull();
    }
    // Their units of core-004 wear its own finish; their backrow card, which picks none, the default.
    expect(faceOf("p2", "core-004")?.getAttribute("data-finish")).toBe(THEIR_FINISH);
    expect(faceOf("p2", "core-020")?.hasAttribute("data-finish")).toBe(false);
  });

  it("R1834004 Mythic, Radiant and B16 hold on the opponent's faces", () => {
    // Their cards pick a foil, a finish and a border of their own, the Mythic and the Radiant card among them.
    const theirs = loadout(
      { cardBack: THEIR_BACK },
      {
        "core-008": { foil: THEIR_FOIL, finish: THEIR_FINISH, border: THEIR_BORDER },
        "core-100": { foil: THEIR_FOIL, finish: THEIR_FINISH, border: THIRD_BORDER },
        "core-022": { foil: THEIR_FOIL, finish: THEIR_FINISH, border: THIRD_BORDER },
      },
    );
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={fullBoardView()} wearCosmetics loadouts={{ p1: loadout(), p2: theirs }} />
      </CatalogContext.Provider>,
    );

    const plain = faceOf("p2", "core-008");
    expect(plain?.getAttribute("data-cosmetic-foil")).toBe(THEIR_FOIL);
    expect(plain?.getAttribute("data-finish")).toBe(THEIR_FINISH);
    expect(plain?.getAttribute("data-frame")).toBe(THEIR_BORDER);
    // A Mythic face wears no cosmetic foil, but its finish and border.
    const mythic = faceOf("p2", "core-100");
    expect(mythic?.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(mythic?.getAttribute("data-finish")).toBe(THEIR_FINISH);
    expect(mythic?.getAttribute("data-frame")).toBe(THIRD_BORDER);
    // A Radiant face wears none of the three.
    const radiant = faceOf("p2", "core-022");
    expect(radiant?.getAttribute("data-radiant-face")).toBe("true");
    for (const attribute of ["data-cosmetic-foil", "data-finish", "data-frame"]) expect(radiant?.getAttribute(attribute), attribute).toBeNull();

    // B16: every back of theirs is the same back, its seat's, and says nothing of its card.
    const backsOfTheirs = theirBacks();
    expect(backsOfTheirs.length).toBeGreaterThan(1);
    expect(new Set(backsOfTheirs.map((back) => back.outerHTML)).size).toBe(1);
    expect(backsOfTheirs[0]?.getAttribute("data-card-back")).toBe(THEIR_BACK);
    for (const back of backsOfTheirs) expect(back.textContent).toBe("");
  });

  it("R1834004 the opponent's full art prints per their loadout", () => {
    const hand = [card({ instanceId: "h8", defId: "core-008" }), card({ instanceId: "h1", defId: "core-001" })];
    const over = baseView({ result: { winner: "p1", reason: "concede" }, opponent: emptySide("p2", { hand }) });
    const loadouts: SeatLoadouts = { p1: loadout(), p2: loadout({}, { "core-008": { fullArt: true }, "core-001": { border: THEIR_BORDER } }) };
    const fullArt = (testId: string): string | null => screen.getByTestId(testId).querySelector(".cf")?.getAttribute("data-full-art") ?? null;
    const board = (allowed = true): ReactElement => (
      <CatalogContext.Provider value={lookup}>
        <FullArtAllowed allowed={allowed}>
          <Board view={over} wearCosmetics loadouts={loadouts} />
        </FullArtAllowed>
      </CatalogContext.Provider>
    );

    // Their revealed hand (R434): Mr. Vanilla's full art is picked; Big D-fender's is not, though
    // every full art counts as owned in this build.
    const { rerender } = render(board());
    expect(fullArt(revealTestid.handCard("h8"))).toBe("true");
    expect(fullArt(revealTestid.handCard("h1"))).toBeNull();
    // The viewer's own Full-art cards setting, and a board that allows none, hold over it.
    act(() => {
      writeCardSettings({ fullArt: false });
    });
    expect(fullArt(revealTestid.handCard("h8"))).toBeNull();
    act(() => {
      writeCardSettings({ fullArt: true });
    });
    rerender(board(false));
    expect(fullArt(revealTestid.handCard("h8"))).toBeNull();
    cleanup();

    // The result panel's "Their hand" wears the same loadout.
    render(
      <CatalogContext.Provider value={lookup}>
        <WearerProvider seat="p1" loadouts={loadouts} viewer="p1">
          <TheirHand view={over} cards={hand} />
        </WearerProvider>
      </CatalogContext.Provider>,
    );
    expect(fullArt(revealTestid.theirHandCard("h8"))).toBe("true");
    expect(fullArt(revealTestid.theirHandCard("h1"))).toBeNull();
    expect(screen.getByTestId(revealTestid.theirHandCard("h1")).querySelector(".cf")?.getAttribute("data-frame")).toBe(THEIR_BORDER);
  });

  it("R1834004 the showcase of the opponent's play wears their loadout, and a card they set their card back", () => {
    const turn: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
    const loadouts: SeatLoadouts = { p1: loadout(), p2: loadout({ cardBack: THEIR_BACK }, { "core-032": { border: THEIR_BORDER } }) };
    const played: GameEvent[] = [
      { type: "cardPlayed", player: "p2", instanceId: "c7", defId: "core-032", costPaid: 1 },
      { type: "summoned", player: "p2", instanceId: "c7", defId: "core-032", row: "units", lane: 2 },
    ];
    const set: GameEvent[] = [
      { type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 1 },
      { type: "summoned", player: "p2", instanceId: "hidden", defId: "hidden", row: "backrow", lane: 3 },
    ];
    const showcased = (events: GameEvent[], testId: string, shownLoadouts: SeatLoadouts | null): HTMLElement => {
      const tree = (shown: PlayerView): ReactElement => (
        <CatalogContext.Provider value={lookup}>
          <WearerProvider seat="p1" loadouts={shownLoadouts} viewer="p1">
            <CardShowcase view={shown} />
          </WearerProvider>
        </CatalogContext.Provider>
      );
      const { rerender } = render(tree(withEvents(baseView(), [turn])));
      rerender(tree(withEvents(baseView(), [turn, ...events])));
      return screen.getByTestId(testId);
    };

    expect(showcased(played, showcaseTestid.face, loadouts).querySelector(".cf")?.getAttribute("data-frame")).toBe(THEIR_BORDER);
    cleanup();
    expect(backs(showcased(set, showcaseTestid.back, loadouts))).toEqual([THEIR_BACK]);
    cleanup();

    // Practice and hotseat carry no loadouts: the other seat's play and its back draw the defaults.
    expect(showcased(played, showcaseTestid.face, null).querySelector(".cf")?.hasAttribute("data-frame")).toBe(false);
    cleanup();
    expect(backs(showcased(set, showcaseTestid.back, null))).toEqual([null]);
  });

  it("R1834004 Show opponents' cosmetics off draws defaults everywhere but the viewer's own side", () => {
    writeCosmetics({ table: TABLE });
    const loadouts: SeatLoadouts = {
      p1: loadout({ table: THEIR_TABLE }, bordered(THIRD_BORDER)),
      p2: loadout(
        { table: THIRD_TABLE, field: THEIR_FIELD, cardBack: THIRD_BACK },
        { "core-004": { border: THEIR_BORDER, finish: THEIR_FINISH }, "core-020": { border: THEIR_BORDER } },
      ),
    };
    writeCardSettings({ opponentCosmetics: false });

    // A live board: the opponent's side is plain, and the viewer's own wears the device's picks.
    render(<Board view={view()} wearCosmetics loadouts={loadouts} />);
    for (const face of facesOwnedBy("p2")) {
      for (const attribute of ["data-cosmetic-foil", "data-finish", "data-frame"]) expect(face.getAttribute(attribute), attribute).toBeNull();
    }
    for (const face of facesOwnedBy("p1")) expect(face.getAttribute("data-frame")).toBe(BORDER);
    expect(document.querySelector(".field")?.getAttribute("data-table")).toBe(TABLE);
    expect(document.querySelector(".field")?.hasAttribute("data-their-table")).toBe(false);
    expect(document.querySelector(".board")?.hasAttribute("data-their-backdrop")).toBe(false);
    for (const back of theirBacks()) expect(back.hasAttribute("data-card-back")).toBe(false);
    cleanup();

    // A watched or replayed board: the seat it is seen from keeps its loadout, the other is plain.
    const { rerender } = render(<Board view={view()} loadouts={loadouts} />);
    for (const face of facesOwnedBy("p1")) expect(face.getAttribute("data-frame")).toBe(THIRD_BORDER);
    for (const face of facesOwnedBy("p2")) expect(face.hasAttribute("data-frame")).toBe(false);
    expect(document.querySelector(".field")?.getAttribute("data-table")).toBe(THEIR_TABLE);
    expect(document.querySelector(".field")?.hasAttribute("data-their-table")).toBe(false);

    // Turned back on, the other seat is dressed again.
    act(() => {
      writeCardSettings({ opponentCosmetics: true });
    });
    rerender(<Board view={view()} loadouts={loadouts} />);
    for (const face of facesOwnedBy("p2")) expect(face.getAttribute("data-frame")).toBe(THEIR_BORDER);
    expect(document.querySelector(".field")?.getAttribute("data-their-table")).toBe(THIRD_TABLE);
    expect(document.querySelector(".board")?.getAttribute("data-their-backdrop")).toBe(THEIR_FIELD);
    for (const back of theirBacks()) expect(back.getAttribute("data-card-back")).toBe(THIRD_BACK);
  });
});
