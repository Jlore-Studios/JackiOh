// R3016000 (#3016): a mouse resting on an owned-cards grid card, or the keyboard's focus, opens its hover
// preview after HOVER_DELAY_MS, the card's look as the card, in hand and on the board with a Radiant
// toggle (LookViews.tsx), drawn by the game's own faces; leaving closes it. Touch has no hover: its
// long press still opens the fullscreen preview (R2730000), which shows the same views.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS } from "../cards/inspect/constants.ts";
import { resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { collectionTestid as T } from "./collection.ts";
import { collectionOwning } from "./fixtures.ts";
import { LOOK_HOVER_CLOSE_MS } from "./LookHover.tsx";
import LookViews, { boardFormOf } from "./LookViews.tsx";
import OwnedCards from "./OwnedCards.tsx";

/** Speciesist, a Core Unit: 4/1, Radiant 8/2. */
const UNIT = "core-002";
/** Stockpile, a Core Spell. */
const SPELL = "core-005";
/** Bread and Butter, a Core Field Trap. */
const TRAP = "core-018";
/** Big D-fender, a Core Unit that prints full art (R2723000's Collection card). */
const FULL_ART = "core-001";

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

/** The grid of `ids`, each owned, Core's set opened. */
function grid(ids: readonly string[]): void {
  render(<OwnedCards collection={collectionOwning(ids)} defs={ids.map(defOf)} />);
  fireEvent.click(screen.getByTestId(T.setToggle("Core")));
}

function tile(id: string): HTMLElement {
  return screen.getByTestId(T.card(id));
}

function wait(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

function mouseOver(element: HTMLElement): void {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
}

function mouseOff(element: HTMLElement): void {
  fireEvent.pointerLeave(element, { pointerType: "mouse" });
}

/** The keyboard's Tab onto `element`, so `:focus-visible` matches it (jsdom reads the Tab's keydown off the body). */
function tabTo(element: HTMLElement): void {
  fireEvent.keyDown(document.body, { key: "Tab" });
  act(() => {
    element.focus();
  });
}

function boardView(root: HTMLElement): HTMLElement {
  return within(root).getByTestId(T.lookView("board"));
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
});

describe("R3016000 the Collection's hover preview", () => {
  it("R3016000 a resting mouse opens it after HOVER_DELAY_MS with the three views, and leaving closes it", () => {
    grid([UNIT]);
    mouseOver(tile(UNIT));
    wait(HOVER_DELAY_MS - 1);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    wait(1);
    const panel = screen.getByTestId(T.lookHover);
    expect(panel.getAttribute("data-item")).toBe(UNIT);
    for (const view of ["card", "hand", "board"]) expect(within(panel).getByTestId(T.lookView(view))).toBeTruthy();
    expect(within(panel).getByRole("button", { name: "Show Radiant" }).getAttribute("aria-pressed")).toBe("false");

    mouseOff(tile(UNIT));
    wait(LOOK_HOVER_CLOSE_MS - 1);
    expect(screen.queryByTestId(T.lookHover)).not.toBeNull();
    wait(1);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 the pointer may move onto the panel to flip it Radiant; leaving the panel closes it", () => {
    grid([UNIT]);
    mouseOver(tile(UNIT));
    wait(HOVER_DELAY_MS);
    const panel = screen.getByTestId(T.lookHover);
    mouseOff(tile(UNIT));
    mouseOver(panel);
    wait(LOOK_HOVER_CLOSE_MS);
    expect(screen.getByTestId(T.lookHover)).toBe(panel);

    fireEvent.click(within(panel).getByRole("button", { name: "Show Radiant" }));
    expect(within(panel).getByTestId(T.look).getAttribute("data-radiant")).toBe("true");
    expect(boardView(panel).querySelector(".stat-attack")?.getAttribute("data-attack")).toBe(String(defOf(UNIT).radiant.attack));

    mouseOff(panel);
    wait(LOOK_HOVER_CLOSE_MS);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 the keyboard's focus opens it the same way, and the focus leaving closes it", () => {
    grid([UNIT, SPELL]);
    tabTo(tile(UNIT));
    wait(HOVER_DELAY_MS);
    expect(screen.getByTestId(T.lookHover).getAttribute("data-item")).toBe(UNIT);
    tabTo(tile(SPELL));
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    wait(HOVER_DELAY_MS);
    expect(screen.getByTestId(T.lookHover).getAttribute("data-item")).toBe(SPELL);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016000 a press closes it, and a click still opens the card's view", () => {
    grid([UNIT]);
    mouseOver(tile(UNIT));
    wait(HOVER_DELAY_MS);
    fireEvent.pointerDown(tile(UNIT), { pointerType: "mouse", button: 0, pointerId: 1 });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    fireEvent.pointerUp(tile(UNIT), { pointerType: "mouse", button: 0, pointerId: 1 });
    fireEvent.click(tile(UNIT));
    expect(screen.getByTestId(T.cardView)).toBeTruthy();
  });

  it("R3016000 touch has no hover; its long press opens the fullscreen preview with the views and the toggle", () => {
    grid([UNIT]);
    fireEvent.pointerEnter(tile(UNIT), { pointerType: "touch" });
    wait(HOVER_DELAY_MS);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();

    fireEvent.pointerDown(tile(UNIT), { pointerType: "touch", button: 0, pointerId: 1, clientX: 100, clientY: 100 });
    wait(LONG_PRESS_MS);
    fireEvent.pointerUp(tile(UNIT), { pointerType: "touch", button: 0, pointerId: 1, clientX: 100, clientY: 100 });
    const preview = screen.getByTestId(T.preview);
    for (const view of ["card", "hand", "board"]) expect(within(preview).getByTestId(T.lookView(view))).toBeTruthy();
    const toggle = within(preview).getByRole("button", { name: "Show Radiant" });
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-pressed")).toBe("true");
    expect(within(preview).getByTestId(T.look).getAttribute("data-radiant")).toBe("true");
  });

  it("R3016000 with hover previews off, neither a mouse nor the focus opens it", () => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, hoverPreviews: false });
    grid([UNIT]);
    mouseOver(tile(UNIT));
    tabTo(tile(UNIT));
    wait(HOVER_DELAY_MS);
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });
});

describe("R3016000 a look as the card, in hand and on the board", () => {
  it("R3016000 a Unit is its whole face, the hand's face and a lane's minion, base and Radiant", () => {
    const def = defOf(UNIT);
    const { rerender } = render(<LookViews def={def} radiant={false} worn />);
    const root = screen.getByTestId(T.look);
    expect(within(root).getByTestId(T.lookView("card")).querySelector(".cf")?.getAttribute("data-layout")).toBe("full");
    expect(within(root).getByTestId(T.lookView("hand")).querySelector(".cf")?.getAttribute("data-layout")).toBe("full");
    const board = boardView(root);
    expect(board.getAttribute("data-form")).toBe("minion");
    expect(board.querySelector(".cf--minion")).not.toBeNull();
    expect(board.querySelector(".stat-attack")?.getAttribute("data-attack")).toBe(String(def.base.attack));
    expect(board.querySelector(".stat-health")?.getAttribute("data-health")).toBe(String(def.base.health));

    rerender(<LookViews def={def} radiant worn />);
    for (const view of ["card", "hand", "board"]) {
      expect(within(root).getByTestId(T.lookView(view)).querySelector(".cf")?.getAttribute("data-radiant-face")).toBe("true");
    }
    expect(boardView(root).querySelector(".stat-attack")?.getAttribute("data-attack")).toBe(String(def.radiant.attack));
  });

  it("R3016000 a Spell is on the board as it resolves, its whole face; a Trap as the backrow's face", () => {
    render(<LookViews def={defOf(SPELL)} radiant={false} worn />);
    const spell = boardView(screen.getByTestId(T.look));
    expect(spell.getAttribute("data-form")).toBe("resolving");
    expect(spell.querySelector(".cf")?.getAttribute("data-layout")).toBe("full");
    expect(spell.querySelector(".cf--minion")).toBeNull();
    cleanup();

    render(<LookViews def={defOf(TRAP)} radiant={false} worn />);
    const trap = boardView(screen.getByTestId(T.look));
    expect(trap.getAttribute("data-form")).toBe("backrow");
    expect(trap.querySelector(".cf")?.getAttribute("data-layout")).toBe("compact");
    expect(boardFormOf("Field Spell")).toBe("backrow");
    expect(boardFormOf("Trap")).toBe("backrow");
  });

  it("R3016000 an owned full-art card is its art alone in hand, and whole as the card", () => {
    grid([FULL_ART]);
    mouseOver(tile(FULL_ART));
    wait(HOVER_DELAY_MS);
    const panel = screen.getByTestId(T.lookHover);
    const card = within(panel).getByTestId(T.lookView("card")).querySelector(".cf");
    const hand = within(panel).getByTestId(T.lookView("hand")).querySelector(".cf");
    expect(card?.getAttribute("data-full-art")).toBe("true");
    expect(card?.hasAttribute("data-full-art-hand")).toBe(false);
    expect(hand?.getAttribute("data-full-art-hand")).toBe("true");
  });
});
