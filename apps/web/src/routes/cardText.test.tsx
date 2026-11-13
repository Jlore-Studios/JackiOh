// R3838000: the Almanac's "Show card text" button prints every face's words, no full art, kept on the device and never
// in the Full-art cards setting (the Statistics page's is stats.test.tsx's). R3838001: on the Almanac a mouse resting on
// any card with words opens the hover preview with its whole text beside it.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { faceText, printsText } from "../cards/CardFace.tsx";
import { isFullArtCard, wearsFullArt } from "../cards/fullArt.ts";
import { INSPECT_HOVER, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { INSPECT_EXTENDED } from "../cards/inspect/testids.ts";
import { faceModel } from "../cards/model.ts";
import { readCardSettings } from "../cards/settings.ts";
import { DEFAULT_FILTER, DEFAULT_SORT, almanacPool } from "../game/deckbuilder/filters.ts";
import { CARD_POOL, poolCardId } from "../game/deckbuilder/testids.ts";
import { at } from "../test/stubServer.ts";
import AlmanacRoute, { ALMANAC_CATALOG } from "./almanac.tsx";
import { CARD_TEXT_KEY, cardTextTestid } from "./cardText.tsx";

/** The whole grid (710 cards and tokens since #1884) renders slowly under jsdom. */
const SLOW_GRID = { timeout: 30_000 } as const;

const SHOWN = almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, DEFAULT_SORT);

function faceOf(id: string) {
  const def = ALMANAC_CATALOG.cards[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return faceModel({ defId: id, def, radiant: false });
}

/** A full-art card with words, which its face does not print. */
const FULL_ART_ID = SHOWN.find((id) => isFullArtCard(id) && printsText(faceOf(id)));
/** A card with no full art whose face prints its whole text: before R3838001 a mouse on it opened nothing. */
const PLAIN_ID = SHOWN.find((id) => !wearsFullArt(id) && printsText(faceOf(id)) && !faceText(faceOf(id), undefined, false).less);

function fullArtFaces(): number {
  return screen.getByTestId(CARD_POOL).querySelectorAll('.cf[data-full-art="true"]').length;
}

function mouse(): PointerEventInit {
  return { pointerType: "mouse", clientX: 40, clientY: 40, pointerId: 1, isPrimary: true, button: 0 };
}

beforeEach(() => {
  window.localStorage.clear();
  at("/almanac");
  // An account read that never answers: a screen that asked the server would sit on it.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  act(() => closeInspect());
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("R3838000 the Almanac's Show card text button", SLOW_GRID, () => {
  it("R3838000 prints every full-art face in the grid with its words, keeps the choice and leaves the setting alone", () => {
    expect(FULL_ART_ID).toBeDefined();
    render(<AlmanacRoute />);
    const toggle = screen.getByTestId(cardTextTestid.toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(toggle).toHaveTextContent("Show card text");
    expect(toggle.closest("header")).not.toBeNull();
    expect(fullArtFaces()).toBeGreaterThan(0);
    const face = (): Element | null => screen.getByTestId(poolCardId(FULL_ART_ID!)).querySelector(".cf");
    expect(face()).toHaveAttribute("data-full-art", "true");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(fullArtFaces()).toBe(0);
    expect(face()).not.toHaveAttribute("data-full-art");
    expect(window.localStorage.getItem(CARD_TEXT_KEY)).toBe("1");
    expect(readCardSettings().fullArt).toBe(true);

    // The device keeps it: the next visit opens with every card's words.
    cleanup();
    render(<AlmanacRoute />);
    expect(screen.getByTestId(cardTextTestid.toggle)).toHaveAttribute("aria-pressed", "true");
    expect(fullArtFaces()).toBe(0);

    fireEvent.click(screen.getByTestId(cardTextTestid.toggle));
    expect(screen.getByTestId(cardTextTestid.toggle)).toHaveAttribute("aria-pressed", "false");
    expect(face()).toHaveAttribute("data-full-art", "true");
    expect(window.localStorage.getItem(CARD_TEXT_KEY)).toBe("0");
  });
});

describe("R3838001 the Almanac's hover text box", SLOW_GRID, () => {
  it("R3838001 a mouse resting on a grid card with words opens the preview with its whole text, full art or not", () => {
    expect(FULL_ART_ID).toBeDefined();
    expect(PLAIN_ID).toBeDefined();
    vi.useFakeTimers();
    render(<AlmanacRoute />);
    for (const id of [FULL_ART_ID!, PLAIN_ID!]) {
      const card = screen.getByTestId(poolCardId(id));
      fireEvent.pointerEnter(card, mouse());
      act(() => {
        vi.advanceTimersByTime(HOVER_DELAY_MS);
      });
      const preview = screen.getByTestId(INSPECT_HOVER);
      expect(within(preview).getByTestId(INSPECT_EXTENDED), id).toBeInTheDocument();
      fireEvent.pointerLeave(card, mouse());
      act(() => closeInspect());
    }
  });
});
