// R1956000: the deck builder's pool and deck list, and the Almanac's read-only grid, draw the
// viewer's own copies wearing their picks (R1655000's foil, finish and border), behind ownership.
// The browse pane's rules are browse.test.tsx's; the Almanac's own route is almanac.test.tsx's.

import type { CatalogSnapshot, Collection } from "@jackioh/validator";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { resetCosmeticsForTests, writeCardPick, writeCosmetics } from "../../cards/cosmetics.ts";
import { OwnCopy } from "../../cards/fullArt.ts";
import { TERM_HOVER_DELAY_MS } from "../../cards/constants.ts";
import { LONG_PRESS_MS } from "../../cards/inspect/constants.ts";
import {
  CARD_SETTINGS_DEFAULTS,
  GLOSSARY,
  INSPECT_FACE_BASE,
  TERM_TOOLTIP_TESTID,
  closeInspect,
  writeCardSettings,
} from "../../cards/index.ts";
import { must } from "../../test/stubServer.ts";
import CardBrowser from "./CardBrowser.tsx";
import DeckSidebar from "./DeckSidebar.tsx";
import { DEFAULT_FILTER, DEFAULT_SORT } from "./filters.ts";
import PoolGrid, { type OwnCopies } from "./PoolGrid.tsx";
import { deckCardId, poolCardId } from "./testids.ts";

const CATALOG_HERE: CatalogSnapshot = { version: "test", cards: CATALOG };
const PICKED = "core-002";
const OTHER = "core-004";
const IDS = [PICKED, OTHER] as const;
const ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

function noop(): void {}

/** The deck editor's half of the grid. */
function deckProps(collection: Collection | null) {
  return {
    deckName: "Deck",
    collection,
    inDeck: new Set<string>(),
    holders: new Map(),
    refusedCardId: null,
    onAdd: noop,
    onDragStart: noop,
    onDragEnd: noop,
  };
}

function owning(...ids: string[]): Collection {
  return Object.fromEntries(ids.map((id) => [id, 1]));
}

function poolFace(cardId: string): Element {
  const face = screen.getByTestId(poolCardId(cardId)).querySelector(".cf");
  if (face === null) throw new Error(`${cardId}'s tile draws no face`);
  return face;
}

function wearsNothing(element: Element | null): void {
  for (const attribute of ATTRIBUTES) expect(element?.getAttribute(attribute), attribute).toBeNull();
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  closeInspect();
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
  vi.useRealTimers();
});

describe("R1956000 the deck builder's pool and deck list wear the viewer's picks", () => {
  it("R1956000 a pool tile below an OwnCopy wears its card's picks, never the account's", () => {
    writeCosmetics({ finish: "finish-linen" });
    writeCardPick(PICKED, "border", "border-bone");
    writeCardPick(OTHER, "finish", "finish-parchment");
    render(
      <OwnCopy own>
        <PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={noop} {...deckProps(owning(PICKED, OTHER))} />
      </OwnCopy>,
    );
    const picked = poolFace(PICKED);
    expect(picked.getAttribute("data-frame")).toBe("border-bone");
    // R1953001: a kind the card has no pick of wears its default, not the account's pick.
    expect(picked.getAttribute("data-finish")).toBeNull();
    expect(poolFace(OTHER).getAttribute("data-frame")).toBeNull();
    expect(poolFace(OTHER).getAttribute("data-finish")).toBe("finish-parchment");

    // A pick reaches the same tile on the same render.
    act(() => {
      writeCardPick(PICKED, "border", "border-wood");
    });
    expect(poolFace(PICKED)).toBe(picked);
    expect(picked.getAttribute("data-frame")).toBe("border-wood");
  });

  it("R1956000 a pool tile with no OwnCopy, or one the collection does not own, draws the defaults", () => {
    writeCardPick(PICKED, "border", "border-bone");
    writeCardPick(OTHER, "finish", "finish-linen");
    render(<PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={noop} {...deckProps(owning(PICKED, OTHER))} />);
    for (const id of IDS) wearsNothing(poolFace(id));
    cleanup();

    render(
      <OwnCopy own>
        <PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={noop} {...deckProps(owning(OTHER))} />
      </OwnCopy>,
    );
    wearsNothing(poolFace(PICKED));
    expect(poolFace(OTHER).getAttribute("data-finish")).toBe("finish-linen");
    cleanup();

    // A collection that could not be read claims nothing unowned.
    render(
      <OwnCopy own>
        <PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={noop} {...deckProps(null)} />
      </OwnCopy>,
    );
    expect(poolFace(PICKED).getAttribute("data-frame")).toBe("border-bone");
  });

  it("R1956000 a deck list tile's art strip wears its card's picks below an OwnCopy", () => {
    writeCardPick(PICKED, "foil", "foil-rainbow");
    writeCardPick(PICKED, "border", "border-bone");
    writeCardPick(OTHER, "foil", "foil-rainbow");
    const sidebar = (collection: Collection | null): ReactElement => (
      <DeckSidebar
        deckName="Deck"
        cards={IDS}
        catalog={CATALOG_HERE}
        collection={collection}
        conflicts={new Map()}
        issues={new Map()}
        head={null}
        onDropCard={noop}
        onRemove={noop}
        onInspect={noop}
      />
    );
    const strip = (cardId: string): Element | null => screen.getByTestId(deckCardId(cardId)).querySelector(".db-tile-art");

    render(<OwnCopy own>{sidebar(owning(PICKED))}</OwnCopy>);
    expect(strip(PICKED)?.getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    // A strip's foil holds still, and its border rides along with no frame to show it on.
    expect(strip(PICKED)?.getAttribute("data-cosmetic-motion")).toBe("static");
    expect(strip(PICKED)?.getAttribute("data-frame")).toBe("border-bone");
    // A card the collection does not hold is no copy of the viewer's.
    wearsNothing(strip(OTHER));
    cleanup();

    render(sidebar(owning(PICKED, OTHER)));
    for (const id of IDS) wearsNothing(strip(id));
  });
});

describe("R1956000 a read-only grid's own copies", () => {
  it("R1956000 a read-only grid's own copy wears what the ledger owns on it; another card draws the defaults", () => {
    // A starter every account owns on every card, and a foil the ledger must list.
    writeCardPick(PICKED, "border", "border-bone");
    writeCardPick(PICKED, "foil", "foil-glitter");
    writeCardPick(OTHER, "border", "border-bone");
    const grid = (own: OwnCopies | undefined): ReactElement => <PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={noop} own={own} />;

    // Signed out (no own copies): every card draws the defaults.
    render(grid(undefined));
    for (const id of IDS) wearsNothing(poolFace(id));
    cleanup();

    render(grid({ cards: new Set([PICKED]), owned: new Map() }));
    expect(poolFace(PICKED).getAttribute("data-frame")).toBe("border-bone");
    expect(poolFace(PICKED).getAttribute("data-cosmetic-foil")).toBeNull();
    wearsNothing(poolFace(OTHER));
    cleanup();

    render(grid({ cards: new Set([PICKED]), owned: new Map([[PICKED, new Set(["foil-glitter"])]]) }));
    expect(poolFace(PICKED).getAttribute("data-cosmetic-foil")).toBe("foil-glitter");
  });

  it("R1956000 the detail view of an own copy wears its picks, and of any other card the defaults", () => {
    writeCardPick(PICKED, "border", "border-bone");
    writeCardPick(OTHER, "border", "border-bone");
    const own: OwnCopies = { cards: new Set([PICKED]), owned: new Map() };
    const browser = (cardId: string): ReactElement => (
      <CardBrowser
        catalog={CATALOG_HERE}
        pool={IDS}
        filter={DEFAULT_FILTER}
        onFilter={noop}
        sort={DEFAULT_SORT}
        onSort={noop}
        ownedControl={false}
        onInspect={noop}
        own={own}
        detail={{ cardId }}
        onCloseDetail={noop}
      />
    );
    const detailFrame = (): string | null =>
      screen.getByTestId(INSPECT_FACE_BASE).querySelector(".cf")?.getAttribute("data-frame") ?? null;

    render(browser(PICKED));
    expect(detailFrame()).toBe("border-bone");
    cleanup();
    closeInspect();
    render(browser(OTHER));
    expect(detailFrame()).toBeNull();
  });
});

describe("R2098000 a pool tile's term shows its rule", () => {
  it("R2098000 a pool tile's term (the Almanac's and the deck builder's grid) shows its rule on hover and on a long press, and a tap still opens the card", () => {
    vi.useFakeTimers();
    const onInspect = vi.fn();
    render(<PoolGrid ids={[PICKED]} catalog={CATALOG_HERE} onInspect={onInspect} />);
    const term = must(
      screen.getByTestId(poolCardId(PICKED)).querySelector<HTMLElement>('.cf-term[data-term="Cry"]'),
      "the pool tile's Cry term",
    );
    expect(term).not.toHaveAttribute("tabindex");

    fireEvent.pointerEnter(term, { pointerType: "mouse" });
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(TERM_HOVER_DELAY_MS);
    });
    expect(screen.getByRole("tooltip")).toHaveTextContent(`${GLOSSARY.Cry.label} ${GLOSSARY.Cry.rule}`);
    fireEvent.pointerLeave(term, { pointerType: "mouse" });
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();

    fireEvent.click(term);
    expect(onInspect).toHaveBeenCalledTimes(1);
    expect(onInspect).toHaveBeenLastCalledWith(PICKED);

    // A held touch shows the rule and opens no second detail; the lift's click goes nowhere.
    fireEvent.pointerDown(term, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS);
    });
    expect(screen.getByRole("tooltip")).toHaveTextContent(GLOSSARY.Cry.rule);
    fireEvent.pointerUp(term, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    fireEvent.click(term);
    expect(onInspect).toHaveBeenCalledTimes(1);
  });
});
