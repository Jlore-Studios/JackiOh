// R3016000 (#3016, part 1): a card of the Collection's owned-cards grid previews its look three ways, as
// the card, in the viewer's hand and on the board, drawn by the game's own components: a mouse resting on
// it or a keyboard's focus opens a hover preview after HOVER_DELAY_MS, which leaving closes; a hold or the
// inspect key opens the fullscreen preview (R2730000), which draws the same three with a Radiant toggle.
// A Unit stands on the board as its lane's minion and a Spell in its face-up field form; an owned
// full-art card's hand view is the art-only face of R2723000. The pixels (no horizontal scroll at phone
// width) were checked in a browser; jsdom has no layout.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS } from "../cards/inspect/constants.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { collectionTestid as T } from "./collection.ts";
import { collectionOwning } from "./fixtures.ts";
import { LOOK_VIEWS } from "./LookPreview.tsx";
import OwnedCards from "./OwnedCards.tsx";

/** Big D-fender, a Core Unit whose full art the Collection draws. */
const UNIT = "core-001";
/** League of Losers, a Classic+ Spell. */
const SPELL = "classicplus-019";
/** Speciesist, a Core Unit the account does not own here. */
const LOCKED = "core-002";

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

/** The grid of `ids`, owning `owned`, with each card's set opened. */
function grid(ids: readonly string[], owned: readonly string[] = ids): void {
  const defs = ids.map(defOf);
  render(<OwnedCards collection={collectionOwning(owned)} defs={defs} />);
  for (const set of new Set(defs.map((def) => def.set))) fireEvent.click(screen.getByTestId(T.setToggle(set)));
}

function tile(id: string): HTMLElement {
  return screen.getByTestId(T.card(id));
}

function wait(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** The game card a view draws: its root, `.card` with the card's def id. */
function gameCard(look: HTMLElement, view: "hand" | "board", id: string): HTMLElement {
  const root = within(look).getByTestId(T.lookView(view)).querySelector<HTMLElement>(`.card[data-def-id="${id}"]`);
  if (root === null) throw new Error(`no ${id} card in the ${view} view`);
  return root;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  localStorage.clear();
  resetCosmeticsForTests();
});

describe("R3016000 the Collection previews a card's look as a card, in hand and on the board", () => {
  it("R3016000 a mouse resting on a grid card opens the hover preview after HOVER_DELAY_MS, and leaving closes it", () => {
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS - 1);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    wait(1);

    const hover = screen.getByTestId(T.lookHover);
    expect(hover).toHaveAttribute("aria-hidden", "true");
    const look = within(hover).getByTestId(T.look);
    expect(look).toHaveAttribute("data-card", UNIT);
    expect(look).toHaveAttribute("data-radiant", "false");
    expect(LOOK_VIEWS.map((view) => within(look).getByTestId(T.lookView(view)).querySelector(".collection-look__caption")?.textContent)).toEqual([
      "The card",
      "In your hand",
      "On the board",
    ]);
    // Nothing fullscreen opened, and no pick was written.
    expect(screen.queryByTestId(T.preview)).toBeNull();

    fireEvent.pointerLeave(tile(UNIT), { pointerType: "mouse" });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 a Unit is drawn as its full face, as the hand's card and as the lane's minion, by the game's own components", () => {
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS);
    const look = screen.getByTestId(T.look);

    const face = within(look).getByTestId(T.lookView("card")).querySelector(".cf");
    expect(face).toHaveAttribute("data-layout", "full");
    expect(face).toHaveAttribute("data-card-type", "Unit");

    const hand = gameCard(look, "hand", UNIT);
    expect(hand).toHaveClass("card-hand");
    expect(hand.closest(".hand-you")).not.toBeNull();

    const board = gameCard(look, "board", UNIT);
    expect(board).toHaveAttribute("data-face", "minion");
    expect(board.closest(".zone")).toHaveAttribute("data-row", "units");
    const def = defOf(UNIT);
    expect(board.querySelector(".stat-attack")).toHaveAttribute("data-attack", String(def.base.attack));
    expect(board.querySelector(".stat-health")).toHaveAttribute("data-health", String(def.base.health));
  });

  it("R3016000 a Spell stands on the board in its face-up field form, never as a minion", () => {
    grid([SPELL]);
    fireEvent.pointerEnter(tile(SPELL), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS);
    const look = screen.getByTestId(T.look);

    expect(within(look).getByTestId(T.lookView("card")).querySelector(".cf")).toHaveAttribute("data-card-type", "Spell");
    expect(gameCard(look, "hand", SPELL)).toHaveAttribute("data-face", "full");
    const board = gameCard(look, "board", SPELL);
    expect(board).toHaveAttribute("data-face", "compact");
    expect(board).toHaveAttribute("data-card-type", "Spell");
    expect(board.closest(".zone")).toHaveAttribute("data-row", "backrow");
    expect(look.querySelector(".cf-host--minion")).toBeNull();
  });

  it("R3016000 R2723000 an owned full-art card's hand view is the art-only face, and its card view prints its name", () => {
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS);
    const look = screen.getByTestId(T.look);
    expect(look).toHaveAttribute("data-worn", "true");

    const handFace = gameCard(look, "hand", UNIT).querySelector(":scope > .cf");
    expect(handFace).toHaveAttribute("data-full-art-hand", "true");
    expect(handFace?.querySelector(".card-name")).toBeNull();

    const cardFace = within(look).getByTestId(T.lookView("card")).querySelector(".cf");
    expect(cardFace).toHaveAttribute("data-full-art", "true");
    expect(cardFace?.querySelector(".card-name")?.textContent).toBe(defOf(UNIT).name);
  });

  it("R3016000 a card the account does not own previews unworn, its hand view keeping its banner", () => {
    grid([LOCKED], []);
    fireEvent.pointerEnter(tile(LOCKED), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS);
    const look = screen.getByTestId(T.look);
    expect(look).toHaveAttribute("data-worn", "false");
    const handFace = gameCard(look, "hand", LOCKED).querySelector(":scope > .cf");
    expect(handFace?.hasAttribute("data-full-art-hand")).toBe(false);
    expect(handFace?.querySelector(".card-name")?.textContent).toBe(defOf(LOCKED).name);
  });

  it("R3016000 a keyboard's focus opens the hover preview after HOVER_DELAY_MS and leaving the card closes it; a press's focus opens none", () => {
    grid([UNIT]);
    act(() => {
      tile(UNIT).focus();
    });
    wait(HOVER_DELAY_MS - 1);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    wait(1);
    expect(screen.getByTestId(T.lookHover)).toBeInTheDocument();

    act(() => {
      tile(UNIT).blur();
    });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();

    // A press focuses the card on its way to the click: that is the click's, and no preview opens.
    fireEvent.pointerDown(tile(UNIT), { pointerType: "mouse", button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      tile(UNIT).focus();
    });
    wait(HOVER_DELAY_MS);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 with hover previews off, neither a resting mouse nor a focus opens one", () => {
    writeCardSettings({ hoverPreviews: false });
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "mouse" });
    wait(HOVER_DELAY_MS);
    act(() => {
      tile(UNIT).focus();
    });
    wait(HOVER_DELAY_MS);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 R2730000 a touch hold opens the fullscreen preview with the three views and a Radiant toggle, which turns all three", () => {
    grid([UNIT]);
    // A touch has no hover: its hold is the preview.
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "touch" });
    fireEvent.pointerDown(tile(UNIT), { pointerType: "touch", button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    wait(LONG_PRESS_MS);
    fireEvent.pointerUp(tile(UNIT), { pointerType: "touch", button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    fireEvent.click(tile(UNIT));
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    expect(screen.queryByTestId(T.cardView)).toBeNull();

    const preview = screen.getByTestId(T.preview);
    expect(preview).toHaveAttribute("data-item", UNIT);
    const look = within(preview).getByTestId(T.look);
    expect(look).toHaveAttribute("data-radiant", "false");
    expect(within(look).getByTestId(T.lookView("card")).querySelector(".cf")?.hasAttribute("data-radiant-face")).toBe(false);

    const toggle = within(preview).getByTestId(T.lookRadiant);
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(look).toHaveAttribute("data-radiant", "true");
    expect(within(look).getByTestId(T.lookView("card")).querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
    expect(gameCard(look, "hand", UNIT)).toHaveAttribute("data-radiant", "true");
    expect(gameCard(look, "board", UNIT)).toHaveAttribute("data-radiant", "true");

    // The toggle is a button inside the trapped dialog; Close still closes it.
    fireEvent.click(within(preview).getByTestId(T.previewClose));
    expect(screen.queryByTestId(T.preview)).toBeNull();
  });

  it("R3016000 the inspect key closes the hover preview and opens the fullscreen one", () => {
    grid([SPELL]);
    act(() => {
      tile(SPELL).focus();
    });
    wait(HOVER_DELAY_MS);
    expect(screen.getByTestId(T.lookHover)).toBeInTheDocument();
    fireEvent.keyDown(tile(SPELL), { key: "i" });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    const preview = screen.getByTestId(T.preview);
    expect(gameCard(within(preview).getByTestId(T.look), "board", SPELL)).toHaveAttribute("data-face", "compact");
  });

  it("R3016000 a click still opens the card's view and no preview", () => {
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "mouse" });
    fireEvent.pointerDown(tile(UNIT), { pointerType: "mouse", button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    fireEvent.pointerUp(tile(UNIT), { pointerType: "mouse", button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    fireEvent.click(tile(UNIT));
    wait(HOVER_DELAY_MS);
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    expect(screen.queryByTestId(T.preview)).toBeNull();
  });
});
