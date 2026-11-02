// R2723000: a full-art card (R1017001, any tier of R2655000) in the viewer's own hand on a board they
// play draws its art alone: its cost gem and a Unit's attack and health stay, and no name banner,
// signature, rarity gem, set mark or crest is drawn. Its name stays in its words, visually hidden,
// and in its tooltip with hover previews off. This file holds the hand against every other view that
// draws the same card whole: the hover preview, the inspect sheet, the board, a mulligan and a
// Discover, the Collection, and a hand nobody wears. The pixels are e2e's
// cypress/component/full-art-hand.cy.tsx.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import OwnedCards from "../collection/OwnedCards.tsx";
import { collectionTestid } from "../collection/collection.ts";
import { collectionOwning } from "../collection/fixtures.ts";
import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import Prompt from "../game/Prompt.tsx";
import { WearerProvider } from "../game/wearer.ts";
import { baseView, card, emptySide, pendingFor, resetIds, unit } from "../test/fixtures.ts";
import { resetCosmeticsForTests } from "./cosmetics.ts";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_HOVER, INSPECT_SHEET } from "./inspect/testids.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";

/** Felinor Flagbearer, a Legendary Unit that prints full art: a gem, a crest, a set mark and stats. */
const UNIT = "classicplus-046";
/** Bigot, a card that prints none. */
const PLAIN = "core-002";
/** Big D-fender, whose full art the Collection draws. */
const COLLECTED = "core-001";

/** Longer than any inspect delay (hover or long-press), as in game/Card.test.tsx. */
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

function nameOf(id: string): string {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def.name;
}

/** A board whose hand holds `ids`, worn as the viewer's own unless `wearCosmetics` is false. */
function board(view: PlayerView, wearCosmetics = true): ReactElement {
  return (
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <Board view={view} wearCosmetics={wearCosmetics} />
    </CatalogContext.Provider>
  );
}

function handView(...ids: string[]): PlayerView {
  resetIds();
  return baseView({
    you: emptySide("p1", { hand: ids.map((defId) => card({ defId })) }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
}

function handRoot(defId: string): HTMLElement {
  const root = screen.getByTestId("hand-you").querySelector<HTMLElement>(`.card[data-def-id="${defId}"]`);
  if (root === null) throw new Error(`no ${defId} in hand`);
  return root;
}

function handFace(defId: string): HTMLElement {
  const face = handRoot(defId).querySelector<HTMLElement>(":scope > .cf");
  if (face === null) throw new Error(`no face for ${defId} in hand`);
  return face;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  localStorage.clear();
});

describe("R2723000: a full-art card in the viewer's own hand", () => {
  it("R2723000 on a board you play, your hand's full-art card draws its art alone and keeps its name in its words; a normal card keeps its banner", () => {
    render(board(handView(UNIT, PLAIN)));

    const face = handFace(UNIT);
    expect(face.getAttribute("data-full-art-hand")).toBe("true");
    for (const part of [".card-name", ".cf-gem", ".cf-set", ".cf-crest"]) expect(face.querySelector(part), part).toBeNull();
    for (const part of [".cost-gem", ".cf-atk", ".cf-hp"]) expect(face.querySelector(part), part).not.toBeNull();
    expect(within(handRoot(UNIT)).getByText(nameOf(UNIT))).toHaveClass("cf-hand-name");

    const plain = handFace(PLAIN);
    expect(plain.hasAttribute("data-full-art-hand")).toBe(false);
    expect(plain.querySelector(".card-name")?.textContent).toBe("Bigot");
  });

  it("R2723000 with hover previews off, your hand's full-art card's tooltip names it", () => {
    writeCardSettings({ hoverPreviews: false });
    render(board(handView(UNIT)));
    expect(handRoot(UNIT).getAttribute("title")).toBe(nameOf(UNIT));
  });

  it("R2723000 the same card prints its name in its hover preview, its inspect sheet and on the board", () => {
    vi.useFakeTimers();
    resetIds();
    const view = baseView({
      you: emptySide("p1", { hand: [card({ defId: UNIT })], units: [unit("p1", { defId: UNIT }), null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    });
    render(board(view));
    const root = handRoot(UNIT);

    fireEvent.pointerEnter(root, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".cf .card-name")?.textContent).toBe(nameOf(UNIT));
    fireEvent.pointerLeave(root, { pointerType: "mouse" });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();

    // The I key opens the sheet (a touch hold shows the preview, R2617000).
    fireEvent.keyDown(root, { key: "i" });
    const sheet = screen.getByTestId(INSPECT_SHEET).querySelector(".cf");
    expect(sheet?.getAttribute("data-full-art")).toBe("true");
    expect(sheet?.querySelector(".card-name")?.textContent).toBe(nameOf(UNIT));

    const onBoard = document.querySelector(`.card[data-face="minion"][data-def-id="${UNIT}"] .card-name`);
    expect(onBoard?.textContent).toBe(nameOf(UNIT));
  });

  it("R2723000 a full-art card offered in your mulligan or a Discover prints its name", () => {
    const hand = { you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: UNIT })] }) };
    const options = (view: PlayerView): void => {
      render(
        <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
          <WearerProvider seat="p1">
            <Prompt view={view} onAction={vi.fn()} />
          </WearerProvider>
        </CatalogContext.Provider>,
      );
      const face = document.querySelector('[data-testid^="prompt-option-"] .cf');
      expect(face?.getAttribute("data-full-art")).toBe("true");
      expect(face?.querySelector(".card-name")?.textContent).toBe(nameOf(UNIT));
      expect(face?.hasAttribute("data-full-art-hand")).toBe(false);
      cleanup();
    };

    options(baseView({ ...hand, pending: pendingFor("mulligan", [{ key: "h1", label: "One", instanceId: "h1" }], { min: 0, max: 1 }) }));
    options(baseView({ ...hand, pending: pendingFor("discover", [{ key: `mode:${UNIT}`, label: "Own", defId: UNIT }]) }));
  });

  it("R2723000 the Collection's tile of a full-art card prints its name", () => {
    const def = CATALOG[COLLECTED];
    if (def === undefined) throw new Error(`the catalog has no ${COLLECTED}`);
    render(<OwnedCards collection={collectionOwning([COLLECTED])} defs={[def]} />);
    fireEvent.click(screen.getByTestId(collectionTestid.setToggle("Core")));
    const face = screen.getByTestId(collectionTestid.card(COLLECTED)).querySelector(".cf");
    expect(face?.getAttribute("data-full-art")).toBe("true");
    expect(face?.querySelector(".card-name")?.textContent).toBe("Big D-fender");
  });

  it("R2723000 a hand nobody wears (a watched game's or a replay's) keeps its names", () => {
    render(board(handView(UNIT), false));
    const face = handFace(UNIT);
    expect(face.getAttribute("data-full-art")).toBe("true");
    expect(face.querySelector(".card-name")?.textContent).toBe(nameOf(UNIT));
    expect(face.hasAttribute("data-full-art-hand")).toBe(false);
  });
});
