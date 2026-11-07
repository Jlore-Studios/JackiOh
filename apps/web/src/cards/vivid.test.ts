// #3341: a card's art holds its colours at the sizes the board draws it: a unit's picture runs up to the frame with nothing
// over it but a rarity ring and a black scrim at its foot, a small face's picture is lifted in saturation and contrast
// (and never on Radiant's gold window, R2725004), and a hand card's art window grows where its name band and rules box give
// room. The numbers are named once (constants.ts) and held to the stylesheet here; jsdom has no layout engine, so
// e2e/cypress/component/card-sizes.cy.tsx measures the sizes in a browser.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { ART_SMALL_FACE_LIFT, HAND_ART_GROW_CQH, HAND_TEXT_BOX_MIN_CQH, MINION_ART_FOOT_SHARE } from "./constants.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** A sheet without its comments, its whitespace collapsed. */
function sheet(path: string): string {
  return readFileSync(join(HERE, path), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/\s+/g, " ");
}

const cards = sheet("cards.css");
const art = sheet("art/art.css");

/** The balanced `{ … }` block that opens at `open`, its inner text without the braces. */
function blockFrom(text: string, open: number): string {
  let depth = 0;
  for (let at = open; at < text.length; at += 1) {
    if (text[at] === "{") depth += 1;
    if (text[at] === "}") {
      depth -= 1;
      if (depth === 0) return text.slice(open + 1, at);
    }
  }
  throw new Error("unbalanced braces");
}

/** The body of the first rule whose selector is exactly `selector`, outside any block when `inside` is not given. */
function ruleOf(text: string, selector: string): string {
  const at = text.indexOf(`${selector} {`);
  expect(at, `a rule for ${selector}`).toBeGreaterThanOrEqual(0);
  return blockFrom(text, at + selector.length + 1);
}

/** The inner text of the first at-rule that starts with `prelude`. */
function atRule(text: string, prelude: string): string {
  const at = text.indexOf(`${prelude} {`);
  expect(at, `an at-rule ${prelude}`).toBeGreaterThanOrEqual(0);
  return blockFrom(text, at + prelude.length + 1);
}

describe("#3341 a small face's art is lifted, and never Radiant's gold", () => {
  const SMALL = atRule(cards, "@container cardface (max-height: 149px)");
  const LIFT = `filter: saturate(${ART_SMALL_FACE_LIFT.saturate}) contrast(${ART_SMALL_FACE_LIFT.contrast});`;

  it("#3341 ART_SMALL_FACE_LIFT is the filter inside the small-face container rule", () => {
    expect(SMALL).toContain(LIFT);
    expect(cards.split(LIFT)).toHaveLength(2);
    expect(ART_SMALL_FACE_LIFT.saturate).toBeGreaterThan(1);
    expect(ART_SMALL_FACE_LIFT.contrast).toBeGreaterThan(1);
  });

  it("#3341 the lift's selector leaves out a Radiant face and the art that wears Radiant's paint (R2725004)", () => {
    const at = SMALL.indexOf(LIFT);
    const selector = SMALL.slice(SMALL.lastIndexOf("}", at) + 1, SMALL.lastIndexOf("{", at)).trim();
    expect(selector).toContain(':not([data-radiant-face="true"])');
    expect(selector).toContain(':not([data-art-variant="radiant"])');
    expect(selector).toContain(".cf-art");
  });

  it("#3341 nothing is laid over the art to tint it: the minion's picture has a black scrim and a rarity ring only", () => {
    const portrait = ".cf[data-layout=\"minion\"]:not([data-full-art=\"true\"]) .cf-portrait::after";
    const body = ruleOf(cards, portrait);
    // Black or clear only: no colour of its own to wash the picture with.
    expect(body).toMatch(/background: linear-gradient\(180deg, transparent 62%, rgb\(0 0 0 \/ 0\.35\)\);/);
    expect(body).toContain("box-shadow: inset 0 0 0 1.4cqmin var(--cf-rarity);");
    expect(body).not.toMatch(/mix-blend-mode|backdrop-filter/);
    expect(art).not.toMatch(/\.cf-art--frame[^}]*(background|mix-blend-mode)/);
  });
});

describe("#3341 a board unit's art runs up to the frame", () => {
  it("#3341 .cf-art--frame is square and has no shadow of its own", () => {
    const body = ruleOf(art, ".cf-art--frame");
    expect(body).toContain("border-radius: 0;");
    expect(body).toContain("box-shadow: none;");
  });

  it("#3341 the portrait is a rectangle from the panel's top and sides down to MINION_ART_FOOT_SHARE of a gem above the foot", () => {
    const portrait = ruleOf(cards, ".cf .cf-portrait");
    expect(portrait).toContain("top: 0;");
    expect(portrait).toContain("right: 0;");
    expect(portrait).toContain("left: 0;");
    expect(portrait).toContain("bottom: var(--mf-art-foot);");
    expect(portrait, "no oval, no ring round the box").not.toMatch(/border-radius|box-shadow/);
    expect(cards).toContain(`--mf-art-foot: calc(var(--mf-stat) * ${MINION_ART_FOOT_SHARE});`);
    expect(MINION_ART_FOOT_SHARE).toBeGreaterThan(0);
    expect(MINION_ART_FOOT_SHARE).toBeLessThan(1);
  });

  it("#3341 the ring a Radiant, a foil Mythic and a Taunt wear is drawn over the art's rim, for a face that is not full art", () => {
    const base = '.cf[data-layout="minion"]:not([data-full-art="true"]) .cf-portrait::after';
    expect(ruleOf(cards, `.cf[data-layout="minion"][data-radiant-face="true"]:not([data-full-art="true"]) .cf-portrait::after`)).toContain("var(--foil-gold)");
    expect(
      ruleOf(
        cards,
        '.cf[data-layout="minion"][data-rarity="Mythic"]:where([data-foil="static"], [data-foil="animated"]):not([data-full-art="true"]) .cf-portrait::after',
      ),
    ).toContain("#94d0ff");
    expect(ruleOf(cards, '.cf[data-layout="minion"][data-taunt="true"]:not([data-full-art="true"]) .cf-portrait::after')).toContain("#c3cad8");
    expect(cards).toContain(base);
  });
});

describe("#3341 a hand card's art grows", () => {
  const SMALL = atRule(cards, "@container cardface (max-height: 149px)");

  it("#3341 below 150px the art window is HAND_ART_GROW_CQH taller and the name band that much lower", () => {
    expect(SMALL).toContain(`.card-hand .cf:not([data-layout="minion"]) .cf-art-frame { height: calc(43cqh + ${HAND_ART_GROW_CQH}cqh); }`);
    expect(SMALL).toContain(`.card-hand .cf:not([data-layout="minion"]):not([data-card-type="Unit"]) .cf-art-frame { height: calc(50cqh + ${HAND_ART_GROW_CQH}cqh); }`);
    // Before, a Unit's band began at 52cqh and any other card's at 59cqh with a 31cqh height, ending at 90cqh.
    expect(SMALL).toContain(`.card-hand .cf:not([data-layout="minion"]) .card-name { top: ${52 + HAND_ART_GROW_CQH}cqh; }`);
    expect(SMALL).toContain(`.card-hand .cf:not([data-layout="minion"]):not([data-card-type="Unit"]) .card-name { top: ${59 + HAND_ART_GROW_CQH}cqh; height: ${31 - HAND_ART_GROW_CQH}cqh; }`);
  });

  it("#3341 from 150px the box takes only the height its text needs, and the art gets the rest", () => {
    const FROM = atRule(cards, "@container cardface (not (max-height: 149px))");
    for (const part of [
      "height: calc(37cqh + var(--cf-grow));",
      "height: calc(40cqh + var(--cf-grow));",
      "top: calc(42cqh + var(--cf-grow));",
      "top: calc(50.2cqh + var(--cf-grow));",
      "top: calc(55.5cqh + var(--cf-grow)); height: calc(var(--cf-text-h) - var(--cf-grow));",
    ]) {
      expect(FROM, part).toContain(part);
    }
    // The rules box starts 31.5cqh tall, and the least it shrinks to leaves the rest to the art.
    expect(cards).toContain("--cf-grow: max(0cqh, calc(31.5cqh - var(--cf-text-need, 31.5) * 1cqh));");
    expect(HAND_TEXT_BOX_MIN_CQH).toBeGreaterThan(0);
    expect(HAND_TEXT_BOX_MIN_CQH).toBeLessThan(31.5);
    // A face whose art already runs across it, or whose box clamps, keeps both as they were.
    expect(FROM).toContain('[data-full-art="true"]');
    expect(FROM).toContain('[data-version="version-extended-art"]');
    expect(FROM).toContain('.card-text:not([data-clamped="true"])');
  });
});
