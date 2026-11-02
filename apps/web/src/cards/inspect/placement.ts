// Where the hover preview goes (B27). Pure but for `rectOf`, which measures an anchor for it; the
// caller measures the viewport.

import { PREVIEW_GAP_PX, PREVIEW_MARGIN_PX } from "./constants.ts";

export type Rect = { left: number; top: number; width: number; height: number };

/** The anchor a preview or a tip is placed against: `element`'s box on screen now. */
export function rectOf(element: Element): Rect {
  const box = element.getBoundingClientRect();
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}
type PreviewSide = "right" | "left" | "above" | "below";

/**
 * "beside" is B27's order (#912 put the left first, where #814 asked for a card's full text), for a
 * card with neighbours all round. "above" goes first for a card in a row (the hand, the resolving
 * strip), as Hearthstone lifts a hand card: beside it, it would cover the card the pointer heads for.
 */
export type PreviewPrefer = "beside" | "above";

const ORDER: Readonly<Record<PreviewPrefer, readonly PreviewSide[]>> = {
  beside: ["left", "right", "above", "below"],
  above: ["above", "left", "right", "below"],
};

/** Keeps `start .. start + size` at least PREVIEW_MARGIN_PX inside `0 .. extent`, top/left first. */
function clamp(start: number, size: number, extent: number): number {
  const max = extent - size - PREVIEW_MARGIN_PX;
  return Math.max(PREVIEW_MARGIN_PX, Math.min(start, max));
}

/** The first side in `prefer`'s order the preview fits on, and its top-left corner there; if none, below. */
export function placePreview(
  anchor: Rect,
  viewport: { width: number; height: number },
  size: { width: number; height: number },
  prefer: PreviewPrefer = "beside",
): { left: number; top: number; side: PreviewSide } {
  const centeredTop = clamp(anchor.top + anchor.height / 2 - size.height / 2, size.height, viewport.height);
  const centeredLeft = clamp(anchor.left + anchor.width / 2 - size.width / 2, size.width, viewport.width);

  const right = anchor.left + anchor.width + PREVIEW_GAP_PX;
  const left = anchor.left - PREVIEW_GAP_PX - size.width;
  const above = anchor.top - PREVIEW_GAP_PX - size.height;
  const below = anchor.top + anchor.height + PREVIEW_GAP_PX;

  for (const side of ORDER[prefer]) {
    if (side === "right" && right + size.width <= viewport.width - PREVIEW_MARGIN_PX) {
      return { left: right, top: centeredTop, side };
    }
    if (side === "left" && left >= PREVIEW_MARGIN_PX) {
      return { left, top: centeredTop, side };
    }
    if (side === "above" && above >= PREVIEW_MARGIN_PX) {
      return { left: centeredLeft, top: above, side };
    }
    if (side === "below" && below + size.height <= viewport.height - PREVIEW_MARGIN_PX) {
      return { left: centeredLeft, top: below, side };
    }
  }

  return { left: centeredLeft, top: clamp(below, size.height, viewport.height), side: "below" };
}
