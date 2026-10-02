// #37, Global Cosmetic: "make it more evident when a zone is locked". A Locked zone is drawn with a
// padlock (LockIcon.tsx), and a card dragged onto one it cannot go into is refused with a blocked X
// where it landed (BlockedMark.tsx) rather than silently snapping back.
//
// Driven through `<Game/>` the way drag-layer.test.tsx drives it: jsdom has no
// `document.elementsFromPoint`, so each test stubs it with the element the pointer is "over".

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import Game from "../../game/Game.tsx";
import { BLOCKED_MARK_MS } from "../../game/drag/BlockedMark.tsx";
import { DRAG_THRESHOLD_PX } from "../../game/drag/model.ts";
import { __resetSettingsForTests } from "../../settings/index.ts";
import { baseView, card, emptySide, unit } from "../fixtures.ts";

/**
 * h1 is a unit that may go into lane 4 or 5: lane 3 is Locked, so the engine does not offer it. It has
 * two candidates on purpose, because a play with one left is committed by a drop anywhere on the board.
 */
function lockedView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "h1", defId: "core-008", cost: 3 }),
        card({ instanceId: "s1", defId: "core-005", cost: 1 }),
      ],
      units: [unit("p1", { instanceId: "u1" }), null, null, null, null],
      locks: { units: [false, false, true, false, false], backrow: [false, false, false, false, false] },
    }),
    opponent: emptySide("p2", { hand: { count: 3 }, units: [null, null, null, null, null] }),
  });
}

const H1_LANE4: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 4 } };
const H1_LANE5: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 5 } };
const S1_PLAY: ActionBody = { type: "play", instanceId: "s1" };
const LEGAL: readonly ActionBody[] = [H1_LANE4, H1_LANE5, S1_PLAY, { type: "endTurn" }];

const POINTER = 1;
const START = { x: 200, y: 400 };

let under: Element[] = [];

function over(...stack: Element[]): void {
  under = stack;
}

const el = (testid: string): HTMLElement => screen.getByTestId(testid);

/** Lifts `source`, moves over `stack`, and releases there. */
function dragTo(source: string, ...stack: Element[]): void {
  over(el(source));
  fireEvent.pointerDown(el(source), { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y });
  fireEvent.pointerMove(window, { pointerId: POINTER, clientX: START.x, clientY: START.y - DRAG_THRESHOLD_PX });
  over(...stack);
  fireEvent.pointerUp(window, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y - 40 });
}

beforeEach(() => {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
  __resetSettingsForTests();
});

describe("#37 a Locked zone reads as locked", () => {
  it("draws a padlock in a locked zone and in no other", () => {
    render(<Game view={lockedView()} legal={LEGAL} onAction={vi.fn()} />);

    expect(within(el("zone-you-units-3")).getByLabelText("Locked zone")).toBeInTheDocument();
    expect(within(el("zone-you-units-3")).getByLabelText("Locked zone").tagName.toLowerCase()).toBe("svg");
    expect(within(el("zone-you-units-4")).queryByLabelText("Locked zone")).toBeNull();
    expect(within(el("zone-you-units-1")).queryByLabelText("Locked zone")).toBeNull();
    expect(el("zone-you-units-3")).toHaveAttribute("data-locked", "true");
  });
});

describe("#37 a card dropped on a Locked zone is refused with a blocked X", () => {
  it("shows the mark on the locked zone and sends nothing", () => {
    const onAction = vi.fn();
    render(<Game view={lockedView()} legal={LEGAL} onAction={onAction} />);

    dragTo("hand-card-h1", el("zone-you-units-3"));

    expect(onAction).not.toHaveBeenCalled();
    expect(el("drag-blocked")).toHaveAttribute("data-zone", "zone-you-units-3");
    // The drag itself is over: no ghost, no arrow.
    expect(screen.queryByTestId("drag-ghost")).toBeNull();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
  });

  it("takes the mark down after BLOCKED_MARK_MS", () => {
    vi.useFakeTimers();
    render(<Game view={lockedView()} legal={LEGAL} onAction={vi.fn()} />);

    dragTo("hand-card-h1", el("zone-you-units-3"));
    expect(screen.queryByTestId("drag-blocked")).not.toBeNull();

    act(() => {
      vi.advanceTimersByTime(BLOCKED_MARK_MS - 1);
    });
    expect(screen.queryByTestId("drag-blocked")).not.toBeNull();
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });

  it("refuses a card standing in the locked zone the same way (the zone is what was dropped on)", () => {
    const view = lockedView();
    view.you.units[2] = unit("p1", { instanceId: "u3" });
    render(<Game view={view} legal={LEGAL} onAction={vi.fn()} />);

    dragTo("hand-card-h1", el("card-u3"), el("zone-you-units-3"));

    expect(el("drag-blocked")).toHaveAttribute("data-zone", "zone-you-units-3");
  });

  it("does not mark a drop on an offered zone: the play goes through", () => {
    const onAction = vi.fn();
    render(<Game view={lockedView()} legal={LEGAL} onAction={onAction} />);

    dragTo("hand-card-h1", el("zone-you-units-4"));

    expect(onAction).toHaveBeenCalledWith(H1_LANE4);
    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });

  it("does not mark a drop on an open zone the card is merely not offered", () => {
    const onAction = vi.fn();
    render(<Game view={lockedView()} legal={LEGAL} onAction={onAction} />);

    dragTo("hand-card-h1", el("zone-you-units-2"));

    expect(onAction).not.toHaveBeenCalled();
    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });

  it("does not mark a drop on the other row's locked zone: this card places nothing there", () => {
    const view = lockedView();
    view.you.locks.backrow[1] = true;
    render(<Game view={view} legal={LEGAL} onAction={vi.fn()} />);

    dragTo("hand-card-h1", el("zone-you-backrow-2"));

    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });

  it("does not mark an unplaced drop (outside the board)", () => {
    render(<Game view={lockedView()} legal={LEGAL} onAction={vi.fn()} />);

    dragTo("hand-card-h1", document.body);

    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });
});
