// #3341: the hover preview and the phone sheet draw a card larger, so its art can be studied: on a desktop the preview's
// art is over twice as wide as a board unit's whole face, whose art now runs up to the frame. jsdom has no layout engine;
// e2e/cypress/component/card-sizes.cy.tsx measures the two in a browser.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { FACE_ASPECT } from "../constants.ts";
import {
  PREVIEW_EXTENDED_STACKED_MAX_PX,
  PREVIEW_GLOSSARY_GAP_PX,
  PREVIEW_HEIGHT_PX,
  PREVIEW_HEIGHT_SHARE,
  PREVIEW_MARGIN_PX,
  PREVIEW_MAX_VIEWPORT_SHARE,
  PREVIEW_PHONE_WIDTH_PX,
  PREVIEW_SHEET_HEIGHT_PX,
  PREVIEW_SHEET_SHARE,
} from "./constants.ts";
import { previewCardHeight, previewHeight } from "./placement.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const read = (path: string): string => readFileSync(join(HERE, path), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/\s+/g, " ");

/** A Unit's art window in a full face: it leaves the sides `13cqw` each (cards.css). */
function unitArtShare(): number {
  const rule = /\.cf\[data-card-type="Unit"\] \.cf-art-frame \{([^}]*)\}/.exec(read("../cards.css"));
  expect(rule, "a Unit's art window rule").not.toBeNull();
  const side = (name: string): number => Number(new RegExp(`${name}: ([\\d.]+)cqw;`).exec(rule?.[1] ?? "")?.[1]);
  return 1 - (side("left") + side("right")) / 100;
}

describe("#3341 the hover preview is larger", () => {
  it("#3341 PREVIEW_HEIGHT_PX is more than the 380 it was, and the preview's height grows with the screen to a share of it", () => {
    expect(PREVIEW_HEIGHT_PX).toBeGreaterThan(380);
    expect(previewHeight(720)).toBe(PREVIEW_HEIGHT_PX);
    expect(previewHeight(1080)).toBeCloseTo(1080 * PREVIEW_HEIGHT_SHARE, 6);
    // Never past the viewport's share inspect.css's `80vh` caps it at, however short the screen.
    expect(previewHeight(480)).toBeCloseTo(480 * PREVIEW_MAX_VIEWPORT_SHARE, 6);
    expect(read("inspect.css")).toContain("max-height: 80vh");
    expect(PREVIEW_MAX_VIEWPORT_SHARE).toBe(0.8);
  });

  it("#3341 on a desktop the preview's art is at least twice as wide as a board unit's whole face", () => {
    const art = unitArtShare();
    expect(art).toBeCloseTo(0.74, 6);
    // The unit's face on each screen: its row's height, as measured in a browser with a full board (card-sizes.cy.tsx), at
    // 0.86 across (board.css's `aspect-ratio`), and its art all of that width now.
    const ROWS = [
      { screen: "1280x720", height: 720, row: 91.4 },
      { screen: "1366x768", height: 768, row: 100.5 },
      { screen: "1920x1080", height: 1080, row: 170.1 },
      { screen: "2560x1440", height: 1440, row: 246 },
    ] as const;
    for (const { screen, height, row } of ROWS) {
      const preview = previewHeight(height) * FACE_ASPECT * art;
      expect(preview, `${screen}: the preview's art against twice a unit's face`).toBeGreaterThanOrEqual(2 * 0.86 * row);
    }
  });
});

describe("#3602 the phone preview stacks the full text across the card's width", () => {
  it("#3602 the phone breakpoint and the stacked panel's height are named, and the CSS mirrors them", () => {
    expect(PREVIEW_PHONE_WIDTH_PX).toBe(600);
    expect(PREVIEW_EXTENDED_STACKED_MAX_PX).toBe(200);
    const css = read("inspect.css");
    expect(css).toContain("@media (max-width: 600px)");
    expect(css).toContain(".inspect-facegroup { flex-direction: column; max-width: 100%; }");
    expect(css).toContain(
      ".inspect-hover .inspect-extended { align-self: stretch; width: auto; max-height: 200px; overflow-y: auto; }",
    );
  });

  it("#3602 the card keeps its desktop height where it fits, and reserves the stacked panel where it does not", () => {
    // 390x844 with the text: previewHeight(844) fits the width and leaves card, gap and panel inside.
    expect(previewCardHeight({ width: 390, height: 844 }, true)).toBe(PREVIEW_HEIGHT_PX);
    // 430x932: the taller screen's share, as on a desktop.
    expect(previewCardHeight({ width: 430, height: 932 }, true)).toBe(932 * PREVIEW_HEIGHT_SHARE);
    // 360x640 with the text: card, one gap and the stacked panel exactly fill the height's margins.
    expect(previewCardHeight({ width: 360, height: 640 }, true)).toBe(
      640 - 2 * PREVIEW_MARGIN_PX - PREVIEW_GLOSSARY_GAP_PX - PREVIEW_EXTENDED_STACKED_MAX_PX,
    );
    // Off a phone the card is previewHeight, with or without the text.
    expect(previewCardHeight({ width: 1280, height: 720 }, true)).toBe(previewHeight(720));
    expect(previewCardHeight({ width: 1280, height: 720 }, false)).toBe(previewHeight(720));
  });
});

describe("#3341 the phone sheet's card is larger", () => {
  it("#3341 inspect.css holds PREVIEW_SHEET_HEIGHT_PX and PREVIEW_SHEET_SHARE, and keeps a narrow phone's card on its screen", () => {
    const css = read("inspect.css");
    const share = `${PREVIEW_SHEET_SHARE * 100}dvh`;
    expect(css).toContain(`.inspect-face--sheet { height: min(${PREVIEW_SHEET_HEIGHT_PX}px, ${PREVIEW_SHEET_SHARE * 100}vh, calc((100vw - 2 * var(--space-9)) * 7 / 5)); height: min(${PREVIEW_SHEET_HEIGHT_PX}px, ${share}, calc((100vw - 2 * var(--space-9)) * 7 / 5)); }`);
    expect(css, "the phone on its side").toContain(`height: min(${PREVIEW_SHEET_HEIGHT_PX}px, calc(92dvh - 44px));`);
    expect(PREVIEW_SHEET_HEIGHT_PX).toBeGreaterThan(380);
    expect(PREVIEW_SHEET_SHARE).toBeGreaterThan(0.56);
  });
});
