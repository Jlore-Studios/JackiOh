// Where the hover preview goes (docs/polish/6-cards.md, B27). The spec fixes the side order (left,
// right, above, below; #912 put the left first) and the PREVIEW_MARGIN_PX clamp, not the offset from
// the anchor or the alignment beside it, so each case has room to spare and asserts only what a side means.

import { describe, expect, it } from "vitest";

import { PREVIEW_MARGIN_PX } from "./constants.ts";
import { estimatedPreviewSize, isPhoneViewport, placePreview, type Rect } from "./index.ts";
import { rectOf } from "./placement.ts";

type Size = { width: number; height: number };
type Side = "right" | "left" | "above" | "below";

const DESKTOP: Size = { width: 1280, height: 720 };
const PHONE: Size = { width: 390, height: 844 };

/** Roughly the hover card: PREVIEW_HEIGHT_PX tall at 5:7. */
const PREVIEW: Size = { width: 272, height: 380 };
const SMALL: Size = { width: 200, height: 200 };

function rightOf(anchor: Rect): number {
  return anchor.left + anchor.width;
}

function bottomOf(anchor: Rect): number {
  return anchor.top + anchor.height;
}

/** The clamp: the whole box stays PREVIEW_MARGIN_PX inside the viewport. */
function expectInside(placed: { left: number; top: number }, size: Size, viewport: Size): void {
  expect(placed.left, "left edge").toBeGreaterThanOrEqual(PREVIEW_MARGIN_PX);
  expect(placed.top, "top edge").toBeGreaterThanOrEqual(PREVIEW_MARGIN_PX);
  expect(placed.left + size.width, "right edge").toBeLessThanOrEqual(viewport.width - PREVIEW_MARGIN_PX);
  expect(placed.top + size.height, "bottom edge").toBeLessThanOrEqual(viewport.height - PREVIEW_MARGIN_PX);
}

/** What the chosen side means geometrically: the preview does not cover the anchor from that side. */
function expectOnSide(placed: { left: number; top: number }, side: Side, anchor: Rect, size: Size): void {
  if (side === "right") expect(placed.left).toBeGreaterThanOrEqual(rightOf(anchor));
  if (side === "left") expect(placed.left + size.width).toBeLessThanOrEqual(anchor.left);
  if (side === "above") expect(placed.top + size.height).toBeLessThanOrEqual(anchor.top);
  if (side === "below") expect(placed.top).toBeGreaterThanOrEqual(bottomOf(anchor));
}

type Case = {
  name: string;
  anchor: Rect;
  viewport: Size;
  size: Size;
  side: Side;
};

const CASES: Case[] = [
  {
    name: "B27 goes right when the left edge is too close",
    anchor: { left: 100, top: 200, width: 100, height: 140 },
    viewport: DESKTOP,
    size: PREVIEW,
    side: "right",
  },
  {
    name: "B27 prefers left over right when both have room (#912)",
    anchor: { left: 600, top: 200, width: 100, height: 140 },
    viewport: DESKTOP,
    size: PREVIEW,
    side: "left",
  },
  {
    name: "B27 goes left when there is room on the left, even though above also fits",
    anchor: { left: 1150, top: 500, width: 100, height: 140 },
    viewport: DESKTOP,
    size: PREVIEW,
    side: "left",
  },
  {
    name: "B27 falls back to above when neither side has room, even though below also fits",
    anchor: { left: 20, top: 300, width: 350, height: 100 },
    viewport: PHONE,
    size: SMALL,
    side: "above",
  },
  {
    name: "B27 falls back to below when neither side nor above has room",
    anchor: { left: 20, top: 40, width: 350, height: 100 },
    viewport: PHONE,
    size: SMALL,
    side: "below",
  },
];

describe("rectOf", () => {
  it("is an element's box on screen, as a plain rect (#2160)", () => {
    const element = document.createElement("div");
    element.getBoundingClientRect = () => ({ left: 10, top: 20, width: 30, height: 40, right: 40, bottom: 60, x: 10, y: 20, toJSON: () => ({}) });
    expect(rectOf(element)).toEqual({ left: 10, top: 20, width: 30, height: 40 });
  });
});

describe("placePreview: the side (B27)", () => {
  for (const c of CASES) {
    it(c.name, () => {
      const placed = placePreview(c.anchor, c.viewport, c.size);
      expect(placed.side).toBe(c.side);
      expectOnSide(placed, c.side, c.anchor, c.size);
      expectInside(placed, c.size, c.viewport);
    });
  }

  it("B27 still answers below when the preview fits nowhere", () => {
    // An anchor as big as the phone: no side, and neither above nor below, has any room at all.
    const anchor: Rect = { left: 0, top: 0, width: PHONE.width, height: PHONE.height };
    expect(placePreview(anchor, PHONE, SMALL).side).toBe("below");
  });

  it("B27 returns finite numbers for left and top", () => {
    const placed = placePreview({ left: 1150, top: 500, width: 100, height: 140 }, DESKTOP, PREVIEW);
    expect(Number.isFinite(placed.left)).toBe(true);
    expect(Number.isFinite(placed.top)).toBe(true);
  });
});

describe("placePreview: the clamp (B27)", () => {
  it("B27 clamps top so a right-hand preview beside a card at the bottom stays inside", () => {
    const anchor: Rect = { left: 100, top: 650, width: 100, height: 60 };
    const placed = placePreview(anchor, DESKTOP, PREVIEW);
    expect(placed.side).toBe("right");
    expectOnSide(placed, "right", anchor, PREVIEW);
    expectInside(placed, PREVIEW, DESKTOP);
  });

  it("B27 clamps top so a right-hand preview beside a card at the very top stays inside", () => {
    const anchor: Rect = { left: 100, top: 0, width: 100, height: 60 };
    const placed = placePreview(anchor, DESKTOP, PREVIEW);
    expect(placed.side).toBe("right");
    expect(placed.top).toBeGreaterThanOrEqual(PREVIEW_MARGIN_PX);
    expectInside(placed, PREVIEW, DESKTOP);
  });

  it("B27 clamps left so a preview above a card hugging the right edge stays inside", () => {
    const anchor: Rect = { left: 180, top: 500, width: 205, height: 110 };
    const placed = placePreview(anchor, PHONE, SMALL);
    expect(placed.side).toBe("above");
    expectOnSide(placed, "above", anchor, SMALL);
    expectInside(placed, SMALL, PHONE);
  });

  it("B27 clamps left so a preview above a card hugging the left edge stays inside", () => {
    const size: Size = { width: 250, height: 200 };
    const anchor: Rect = { left: 0, top: 500, width: 150, height: 110 };
    const placed = placePreview(anchor, PHONE, size);
    expect(placed.side).toBe("above");
    expectOnSide(placed, "above", anchor, size);
    expectInside(placed, size, PHONE);
  });

  it("B27 clamps left so a preview below a card at the top-right corner stays inside", () => {
    const anchor: Rect = { left: 190, top: 10, width: 195, height: 100 };
    const placed = placePreview(anchor, PHONE, SMALL);
    expect(placed.side).toBe("below");
    expectOnSide(placed, "below", anchor, SMALL);
    expectInside(placed, SMALL, PHONE);
  });

  it("B27 keeps a left-hand preview inside when its card sits low on the screen", () => {
    const anchor: Rect = { left: 1150, top: 600, width: 100, height: 110 };
    const placed = placePreview(anchor, DESKTOP, PREVIEW);
    expect(placed.side).toBe("left");
    expectOnSide(placed, "left", anchor, PREVIEW);
    expectInside(placed, PREVIEW, DESKTOP);
  });
});

describe("placePreview: a card in a row prefers above", () => {
  it("rises over a hand card at the bottom of a desktop screen instead of covering its neighbour", () => {
    const anchor: Rect = { left: 92, top: 477, width: 55, height: 76 };
    const placed = placePreview(anchor, DESKTOP, PREVIEW, "above");
    expect(placed.side).toBe("above");
    expectOnSide(placed, "above", anchor, PREVIEW);
    expectInside(placed, PREVIEW, DESKTOP);
  });

  it("falls back to B27's order, left then right, when there is no room above", () => {
    const anchor: Rect = { left: 100, top: 40, width: 55, height: 76 };
    const placed = placePreview(anchor, DESKTOP, PREVIEW, "above");
    expect(placed.side).toBe("right");
    expectOnSide(placed, "right", anchor, PREVIEW);
    expectInside(placed, PREVIEW, DESKTOP);
  });

  it("keeps B27's order when nothing is asked for", () => {
    const anchor: Rect = { left: 92, top: 477, width: 55, height: 76 };
    expect(placePreview(anchor, DESKTOP, PREVIEW).side).toBe("right");
    expect(placePreview(anchor, DESKTOP, PREVIEW, "beside").side).toBe("right");
  });
});

describe("#3602 the phone preview's stacked estimate fits and its placed box stays inside", () => {
  const VIEWPORTS: Size[] = [
    { width: 390, height: 844 },
    { width: 430, height: 932 },
  ];
  const WITH_TEXT = { withGlossary: false, withRefs: false, withExtended: true };
  const ANCHORS = [
    { anchor: { left: 145, top: 120, width: 100, height: 140 }, prefer: "above" as const },
    { anchor: { left: 145, top: 350, width: 100, height: 140 }, prefer: "beside" as const },
  ];

  it("#3602 viewports 600px wide or narrower draw the phone layout, wider ones do not", () => {
    expect(isPhoneViewport(390)).toBe(true);
    expect(isPhoneViewport(600)).toBe(true);
    expect(isPhoneViewport(601)).toBe(false);
    expect(isPhoneViewport(1280)).toBe(false);
  });

  for (const viewport of VIEWPORTS) {
    it(`#3602 at ${String(viewport.width)}x${String(viewport.height)} the stacked estimate fits the width`, () => {
      const size = estimatedPreviewSize(viewport, WITH_TEXT);
      expect(size.width, "estimate against the viewport less its margins").toBeLessThanOrEqual(
        viewport.width - 2 * PREVIEW_MARGIN_PX,
      );
    });

    for (const c of ANCHORS) {
      it(`#3602 at ${String(viewport.width)}x${String(viewport.height)} a ${c.prefer} preview with the full text lands inside`, () => {
        const size = estimatedPreviewSize(viewport, WITH_TEXT);
        expectInside(placePreview(c.anchor, viewport, size, c.prefer), size, viewport);
      });
    }
  }

  it("#3602 at 390x844 a preview without the full text fits and lands inside", () => {
    const viewport: Size = { width: 390, height: 844 };
    const size = estimatedPreviewSize(viewport, { withGlossary: false, withRefs: false, withExtended: false });
    expect(size.width, "estimate against the viewport less its margins").toBeLessThanOrEqual(
      viewport.width - 2 * PREVIEW_MARGIN_PX,
    );
    expectInside(
      placePreview({ left: 145, top: 120, width: 100, height: 140 }, viewport, size, "above"),
      size,
      viewport,
    );
  });
});
