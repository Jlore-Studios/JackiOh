// Where the hover preview goes (B27). Pure but for `rectOf`, which measures an anchor for it; the
// caller measures the viewport.

import {
  PREVIEW_EXTENDED_STACKED_MAX_PX,
  PREVIEW_EXTENDED_WIDTH_PX,
  PREVIEW_GAP_PX,
  PREVIEW_GLOSSARY_GAP_PX,
  PREVIEW_GLOSSARY_WIDTH_PX,
  PREVIEW_HEIGHT_PX,
  PREVIEW_HEIGHT_SHARE,
  PREVIEW_MARGIN_PX,
  PREVIEW_MAX_VIEWPORT_SHARE,
  PREVIEW_PHONE_WIDTH_PX,
  PREVIEW_REFS_WIDTH_PX,
} from "./constants.ts";
import { FACE_ASPECT } from "../constants.ts";

export type Rect = { left: number; top: number; width: number; height: number };

/** The anchor a preview or a tip is placed against: `element`'s box on screen now. */
export function rectOf(element: Element): Rect {
  const box = element.getBoundingClientRect();
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}
/**
 * #3341: the hover preview card's height on a viewport this tall: PREVIEW_HEIGHT_PX, or PREVIEW_HEIGHT_SHARE of the
 * viewport where that is more, and never more than PREVIEW_MAX_VIEWPORT_SHARE of it (inspect.css's `80vh` too).
 */
export function previewHeight(viewportHeight: number): number {
  return Math.min(Math.max(PREVIEW_HEIGHT_PX, viewportHeight * PREVIEW_HEIGHT_SHARE), viewportHeight * PREVIEW_MAX_VIEWPORT_SHARE);
}

/** #3602: viewports this wide or narrower draw the phone hover-preview layout (PREVIEW_PHONE_WIDTH_PX). */
export function isPhoneViewport(viewportWidth: number): boolean {
  return viewportWidth <= PREVIEW_PHONE_WIDTH_PX;
}

/**
 * #3602: the hover preview card's height on this viewport, with or without the full-text panel. Off
 * a phone it is `previewHeight`, unchanged; on a phone it also fits the width, and with the text it
 * leaves room for one gap and the stacked panel, so card and text fit the height together.
 */
export function previewCardHeight(viewport: { width: number; height: number }, withExtended: boolean): number {
  const height = previewHeight(viewport.height);
  if (!isPhoneViewport(viewport.width)) return height;
  const fitWidth = (viewport.width - 2 * PREVIEW_MARGIN_PX) / FACE_ASPECT;
  if (!withExtended) return Math.max(0, Math.min(height, fitWidth));
  const fitStacked = viewport.height - 2 * PREVIEW_MARGIN_PX - PREVIEW_GLOSSARY_GAP_PX - PREVIEW_EXTENDED_STACKED_MAX_PX;
  return Math.max(0, Math.min(height, fitWidth, fitStacked));
}

/** Which panels a hover preview draws beside or around its card. */
export type PreviewPanels = { withGlossary: boolean; withRefs: boolean; withExtended: boolean };

/**
 * #3602: the preview's size before it has been laid out, from the same numbers inspect.css uses. Off
 * a phone the card stands with its panels beside it; on a phone the side panels hide and the full
 * text stacks across the card's width, so the estimate counts the card alone, or the card with one
 * gap and the stacked panel.
 */
export function estimatedPreviewSize(
  viewport: { width: number; height: number },
  panels: PreviewPanels,
): { width: number; height: number } {
  const cardHeight = previewCardHeight(viewport, panels.withExtended);
  const cardWidth = cardHeight * FACE_ASPECT;
  if (isPhoneViewport(viewport.width)) {
    return {
      width: cardWidth,
      height: cardHeight + (panels.withExtended ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_EXTENDED_STACKED_MAX_PX : 0),
    };
  }
  const glossary = panels.withGlossary ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_GLOSSARY_WIDTH_PX : 0;
  const refs = panels.withRefs ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_REFS_WIDTH_PX : 0;
  const extended = panels.withExtended ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_EXTENDED_WIDTH_PX : 0;
  return { width: cardWidth + glossary + refs + extended, height: cardHeight };
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
