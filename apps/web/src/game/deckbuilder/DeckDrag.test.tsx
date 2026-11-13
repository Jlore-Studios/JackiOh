// Issue #3734 (R3734001): a pool card is dragged into the deck with pointer events (`DeckDrag.tsx`), so a
// finger works. The box of the deck is where a test says it is: jsdom lays nothing out.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DRAG_THRESHOLD_PX } from "../drag/model.ts";
import DeckDrag from "./DeckDrag.tsx";
import { CARD_POOL, DECK_DRAG_GHOST, DECK_DROP } from "./testids.ts";

const DECK = { left: 100, top: 100, right: 300, bottom: 300 };
const INSIDE = { x: 200, y: 200 };

type Kind = "mouse" | "touch";

function setup(): { onDrop: ReturnType<typeof vi.fn>; onClick: ReturnType<typeof vi.fn>; card: HTMLElement; deck: HTMLElement } {
  const onDrop = vi.fn();
  const onClick = vi.fn();
  render(
    <>
      <section data-testid={CARD_POOL}>
        <button type="button" className="db-card" data-testid="c1" data-card="c1" data-legal="true" onClick={onClick}>
          Card one
        </button>
        <button type="button" className="db-card" data-testid="c2" data-card="c2" data-legal="false" onClick={onClick}>
          Card two
        </button>
        <button type="button" className="db-card" data-testid="c3" data-card="c3" onClick={onClick}>
          Card three
        </button>
      </section>
      <section data-testid={DECK_DROP} />
      <DeckDrag onDrop={onDrop} nameOf={(id) => `name of ${id}`} />
    </>,
  );
  const deck = screen.getByTestId(DECK_DROP);
  deck.getBoundingClientRect = () => ({ ...DECK, width: 200, height: 200, x: 100, y: 100, toJSON: () => ({}) });
  return { onDrop, onClick, card: screen.getByTestId("c1"), deck };
}

function event(kind: Kind, at: { x: number; y: number }): PointerEventInit {
  return { pointerType: kind, pointerId: 1, isPrimary: true, button: 0, clientX: at.x, clientY: at.y };
}

const press = (card: HTMLElement, kind: Kind, at = { x: 10, y: 10 }): void => {
  fireEvent.pointerDown(card, event(kind, at));
};
const move = (card: HTMLElement, kind: Kind, at: { x: number; y: number }): void => {
  fireEvent.pointerMove(card, event(kind, at));
};
const release = (card: HTMLElement, kind: Kind, at: { x: number; y: number }): void => {
  fireEvent.pointerUp(card, event(kind, at));
};

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-dragging");
});

describe("R3734001 a pool card dragged to the deck by pointer", () => {
  it("R3734001 a mouse past the threshold, released over the deck, calls onDrop with the card", () => {
    const { onDrop, card, deck } = setup();
    press(card, "mouse");
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    move(card, "mouse", INSIDE);
    expect(screen.getByTestId(DECK_DRAG_GHOST)).toHaveTextContent("name of c1");
    expect(screen.getByTestId(DECK_DRAG_GHOST)).toHaveAttribute("data-pointer", "mouse");
    expect(document.documentElement).toHaveAttribute("data-dragging", "deck");
    expect(deck).toHaveAttribute("data-drop", "over");
    release(card, "mouse", INSIDE);
    expect(onDrop).toHaveBeenCalledExactlyOnceWith("c1");
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    expect(deck).not.toHaveAttribute("data-drop");
  });

  it("R3734001 a touch that moves across lifts the card and drops it", () => {
    const { onDrop, card } = setup();
    press(card, "touch", { x: 10, y: 150 });
    move(card, "touch", { x: 10 + DRAG_THRESHOLD_PX + 4, y: 152 });
    expect(screen.getByTestId(DECK_DRAG_GHOST)).toHaveAttribute("data-pointer", "touch");
    move(card, "touch", INSIDE);
    release(card, "touch", INSIDE);
    expect(onDrop).toHaveBeenCalledExactlyOnceWith("c1");
  });

  it("R3734001 a touch that moves down first is the pool's scroll: nothing lifts, and nothing is added", () => {
    const { onDrop, card } = setup();
    press(card, "touch", { x: 150, y: 10 });
    move(card, "touch", { x: 152, y: 10 + DRAG_THRESHOLD_PX + 4 });
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    // Even a later move across and into the deck does not pick it up again.
    move(card, "touch", INSIDE);
    release(card, "touch", INSIDE);
    expect(onDrop).not.toHaveBeenCalled();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
  });

  it("R3734001 a release anywhere but over the deck adds nothing", () => {
    const { onDrop, card, deck } = setup();
    press(card, "mouse");
    move(card, "mouse", INSIDE);
    move(card, "mouse", { x: 400, y: 400 });
    expect(deck).not.toHaveAttribute("data-drop");
    release(card, "mouse", { x: 400, y: 400 });
    expect(onDrop).not.toHaveBeenCalled();
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
  });

  it("R3734001 a press below the threshold stays a click, and a drop with no deck on screen adds nothing", () => {
    const { onDrop, onClick, card } = setup();
    press(card, "mouse");
    move(card, "mouse", { x: 10 + DRAG_THRESHOLD_PX - 2, y: 10 });
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    release(card, "mouse", { x: 12, y: 10 });
    fireEvent.click(card);
    expect(onClick).toHaveBeenCalledOnce();
    expect(onDrop).not.toHaveBeenCalled();
    // A deck of no size (hidden) is no target.
    screen.getByTestId(DECK_DROP).getBoundingClientRect = () => ({ left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0, x: 0, y: 0, toJSON: () => ({}) });
    press(card, "mouse", { x: 0, y: 0 });
    move(card, "mouse", { x: 0, y: 30 });
    release(card, "mouse", { x: 0, y: 0 });
    expect(onDrop).not.toHaveBeenCalled();
  });

  it("R3734001 the click that follows a drag is swallowed, and the next one is not", () => {
    const { onClick, card } = setup();
    press(card, "mouse");
    move(card, "mouse", INSIDE);
    release(card, "mouse", INSIDE);
    fireEvent.click(card);
    expect(onClick).not.toHaveBeenCalled();
    press(card, "mouse");
    release(card, "mouse", { x: 10, y: 10 });
    fireEvent.click(card);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("R3734001 Escape, a canceled pointer and a blur cancel the drag, and nothing is added", () => {
    const { onDrop, card, deck } = setup();
    press(card, "mouse");
    move(card, "mouse", INSIDE);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    expect(deck).not.toHaveAttribute("data-drop");
    release(card, "mouse", INSIDE);

    press(card, "mouse");
    move(card, "mouse", INSIDE);
    fireEvent.pointerCancel(card, event("mouse", INSIDE));
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();

    press(card, "mouse");
    move(card, "mouse", INSIDE);
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    release(card, "mouse", INSIDE);
    expect(onDrop).not.toHaveBeenCalled();
  });

  it("R3734001 a card the deck would not take is lifted too, so the editor refuses it as it does a tap; a card outside the deck editor's pool, or a press that is not the primary button, is never lifted", () => {
    const { onDrop, deck } = setup();
    const held = screen.getByTestId("c2");
    press(held, "mouse");
    move(held, "mouse", INSIDE);
    expect(screen.getByTestId(DECK_DRAG_GHOST)).toHaveAttribute("data-card", "c2");
    release(held, "mouse", INSIDE);
    expect(onDrop).toHaveBeenCalledExactlyOnceWith("c2");
    onDrop.mockClear();
    // The read-only almanac's pool card has no `data-legal`.
    const almanac = screen.getByTestId("c3");
    press(almanac, "mouse");
    move(almanac, "mouse", INSIDE);
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    release(almanac, "mouse", INSIDE);
    const card = screen.getByTestId("c1");
    fireEvent.pointerDown(card, { ...event("mouse", { x: 10, y: 10 }), button: 2 });
    move(card, "mouse", INSIDE);
    expect(screen.queryByTestId(DECK_DRAG_GHOST)).toBeNull();
    release(card, "mouse", INSIDE);
    expect(onDrop).not.toHaveBeenCalled();
    expect(deck).not.toHaveAttribute("data-drop");
  });

  it("R3734001 the browser's own drag of a pool card's art is refused", () => {
    const { card } = setup();
    const allowed = fireEvent.dragStart(card);
    expect(allowed).toBe(false);
  });
});
