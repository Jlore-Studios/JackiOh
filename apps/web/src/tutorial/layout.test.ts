// Where the floating coach bubble goes (tutorial/layout.ts, R2434003). On a phone the coach does not
// float (it is a panel in the page, Coach.test.tsx), so nothing here places a bubble on a phone's screen.

import { describe, expect, it } from "vitest";

import { placeBubble, overlapArea, padRect, unionRect, type PlaceInput, type Rect } from "./layout.ts";

const VIEWPORT = { width: 1280, height: 720 };
const BUBBLE = { width: 340, height: 150 };
/** The board at 1280x720 (practice.css): the five lanes, between the HUD and your hand. */
const BOARD: Rect = { left: 20, top: 88, width: 978, height: 484 };
/** The middle of the board, a third of it each way: no bubble ever covers it. */
const MIDDLE: Rect = {
  left: BOARD.left + BOARD.width / 3,
  top: BOARD.top + BOARD.height / 3,
  width: BOARD.width / 3,
  height: BOARD.height / 3,
};

function input(over: Partial<PlaceInput>): PlaceInput {
  return {
    anchor: null,
    bubble: BUBBLE,
    viewport: VIEWPORT,
    insetTop: 60,
    margin: 8,
    minHeight: 96,
    ...over,
  };
}

function bubbleRect(place: ReturnType<typeof placeBubble>, size = BUBBLE): Rect {
  return {
    left: place.left,
    top: place.top,
    width: size.width,
    height: place.maxHeight === null ? size.height : Math.min(size.height, place.maxHeight),
  };
}

function inside(rect: Rect, viewport: { width: number; height: number }, margin: number, insetTop: number): boolean {
  return (
    rect.left >= margin &&
    rect.top >= insetTop + margin &&
    rect.left + rect.width <= viewport.width - margin &&
    rect.top + rect.height <= viewport.height - margin
  );
}

describe("R2434003 the coach bubble's placement on a desktop", () => {
  it("R2434003 docks to a corner, never over the middle of the board, inside the viewport under the HUD, wherever its anchor is", () => {
    const anchors: (Rect | null)[] = [null];
    for (let left = 0; left <= 1200; left += 150) {
      for (let top = 70; top <= 640; top += 95) anchors.push({ left, top, width: 80, height: 70 });
    }
    // The whole hand along the bottom, End turn in the sidebar, a prompt in the middle.
    anchors.push({ left: 200, top: 590, width: 700, height: 125 });
    anchors.push({ left: 1030, top: 330, width: 240, height: 60 });
    anchors.push({ left: 360, top: 200, width: 560, height: 320 });
    for (const anchor of anchors) {
      const place = placeBubble(input({ anchor }));
      const rect = bubbleRect(place);
      const where = JSON.stringify({ anchor, place });
      expect(place.side, where).toMatch(/^dock-(top|bottom)-(left|right)$/);
      expect(overlapArea(rect, MIDDLE), where).toBe(0);
      expect(inside(rect, VIEWPORT, 8, 60), where).toBe(true);
      if (anchor !== null) expect(overlapArea(rect, anchor), where).toBe(0);
    }
  });

  it("R2434003 with nothing to keep clear, sits bottom right, over the log beside your hero's row", () => {
    const place = placeBubble(input({}));
    expect(place.side).toBe("dock-bottom-right");
    expect(place.left + BUBBLE.width).toBe(1280 - 8);
    expect(place.top + BUBBLE.height).toBe(720 - 8);
    const slim = { width: 260, height: 44 };
    const waiting = placeBubble(input({ bubble: slim }));
    expect(waiting.left + slim.width).toBe(1280 - 8);
    expect(waiting.top + slim.height).toBe(720 - 8);
  });

  it("R2434003 moves to the next corner rather than cover its anchor, then what a play asks for, then a soft obstacle", () => {
    const endTurn: Rect = { left: 1030, top: 600, width: 240, height: 60 };
    expect(placeBubble(input({ anchor: endTurn })).side).toBe("dock-bottom-left");

    const lowLeft: Rect = { left: 20, top: 600, width: 200, height: 100 };
    expect(placeBubble(input({ anchor: endTurn, keepClear: [lowLeft] })).side).toBe("dock-top-right");

    const opponentHand: Rect = { left: 1100, top: 80, width: 170, height: 60 };
    expect(placeBubble(input({ anchor: endTurn, keepClear: [lowLeft], avoid: [opponentHand] })).side).toBe("dock-top-left");
  });

  it("R2434003 with every corner covering something, covers the anchor least of all", () => {
    const anchor: Rect = { left: 0, top: 60, width: 1280, height: 100 };
    const place = placeBubble(input({ anchor }));
    expect(place.side.startsWith("dock-bottom")).toBe(true);
    expect(overlapArea(bubbleRect(place), anchor)).toBe(0);
  });

  it("R2434003 on a short screen, never taller than the room under the HUD, and its text scrolls", () => {
    const short = { width: 1280, height: 300 };
    const tall = { width: 340, height: 400 };
    const place = placeBubble(input({ viewport: short, bubble: tall }));
    expect(place.maxHeight).toBe(300 - 8 - 68);
    expect(place.top).toBe(68);
  });

  it("unions and pads the anchor's rectangles", () => {
    expect(unionRect([])).toBeNull();
    expect(
      unionRect([
        { left: 10, top: 20, width: 30, height: 40 },
        { left: 100, top: 5, width: 10, height: 10 },
      ]),
    ).toEqual({ left: 10, top: 5, width: 100, height: 55 });
    expect(padRect({ left: 10, top: 10, width: 20, height: 20 }, 6)).toEqual({ left: 4, top: 4, width: 32, height: 32 });
  });
});
