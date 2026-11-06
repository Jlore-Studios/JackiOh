// #37, Global Cosmetic: "fix laggy switching-to-defense-position animation". The positionSwitched
// entry (animations.ts) plays over the board as it was BEFORE the switch (Game.tsx plans entries
// against the view still on screen), so the card's own `data-position` says which way it is turning.
//
// The sheet is read as text (vitest stubs CSS imports, as fx/css.test.ts does), and the one fact it
// must agree with Card.tsx on, the resting DEF transform, is read off a rendered card.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Game from "./Game.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { readSheet } from "../test/setup.ts";

const css = readSheet("src/game/animations.css");
const board = readSheet("src/game/board.css");

/** The first `.lane { … }` rule at or after `from` in board.css, up to its closing brace. */
function laneRule(from: number): string {
  const start = board.indexOf(".lane {", from);
  if (start < 0) throw new Error(".lane rule not found in board.css");
  return board.slice(start, board.indexOf("}", start));
}

/** The transform at one keyframe stop of `@keyframes <name>`. */
function stop(name: string, at: "0%" | "100%"): string {
  const block = new RegExp(`@keyframes ${name}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(css)?.[1];
  if (block === undefined) throw new Error(`@keyframes ${name} not found`);
  const found = new RegExp(`${at}\\s*\\{[^}]*?transform:\\s*([^;]+);`).exec(block)?.[1];
  if (found === undefined) throw new Error(`${name} has no transform at ${at}`);
  return found.trim();
}

function animationNameFor(selector: string): string | undefined {
  const escaped = selector.replace(/[[\]"=.]/g, "\\$&");
  return new RegExp(`${escaped}\\s*\\{[^}]*?animation-name:\\s*([\\w-]+);`).exec(css)?.[1];
}

afterEach(cleanup);

function renderUnit(position: "ATK" | "DEF"): HTMLElement {
  const view = baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId: "u1", position }), null, null, null, null] }),
    opponent: emptySide("p2", { hand: { count: 0 } }),
  });
  render(<Game view={view} legal={[]} onAction={() => {}} />);
  return screen.getByTestId("card-u1");
}

describe("#37 positionSwitched turns the card between its two resting transforms", () => {
  it("a unit at rest in ATK has no transform, and in DEF is turned and scaled", () => {
    expect(renderUnit("ATK").style.transform).toBe("");
    cleanup();
    expect(renderUnit("DEF").style.transform).toBe("rotate(90deg) scale(var(--def-scale, 1))");
  });

  it("jk-rotate-def (ATK to DEF) starts upright and ends on the DEF transform the new view rests on", () => {
    expect(stop("jk-rotate-def", "0%")).toBe("rotate(0deg) scale(1)");
    expect(stop("jk-rotate-def", "100%")).toBe(renderUnit("DEF").style.transform);
  });

  it("jk-rotate-atk (DEF to ATK) starts on the DEF transform and ends upright", () => {
    expect(stop("jk-rotate-atk", "0%")).toBe(renderUnit("DEF").style.transform);
    expect(stop("jk-rotate-atk", "100%")).toBe("rotate(0deg) scale(1)");
  });

  it("the card still showing DEF turns back to ATK; any other turns into DEF", () => {
    expect(animationNameFor('[data-animating="positionSwitched"]')).toBe("jk-rotate-def");
    expect(animationNameFor('[data-animating="positionSwitched"][data-position="DEF"]')).toBe("jk-rotate-atk");
  });
});

/** The phone portrait block of board.css, which ends where the narrow phone's begins. */
function phoneBlock(): string {
  const start = board.indexOf("@media (max-width: 600px) {");
  expect(start, "board.css has a phone portrait block").toBeGreaterThan(-1);
  const end = board.indexOf("@media (max-width: 380px) {");
  expect(end, "the narrow-phone block follows the phone portrait block").toBeGreaterThan(start);
  return board.slice(start, end);
}

describe("#2932 a phone held upright draws field cards in a card shape that grows by height", () => {
  it("the phone block names the shape bounds, drops #2900's tile, and sizes the card off its zone", () => {
    const phone = phoneBlock();
    expect(phone).toContain("--field-ratio-min: 0.72;");
    expect(phone).toContain("--field-ratio-max: 0.86;");
    expect(phone).not.toContain("field-tile-max");
    expect(phone).not.toContain("--tile-w");
    expect(phone).toMatch(/\.zone\s*\{[^}]*container-type:\s*size;/);
    const card = /\.zone\s*>\s*\.card\s*\{([\s\S]*?)\}/.exec(phone)?.[1];
    expect(card, "board.css has a phone `.zone > .card` rule").toBeDefined();
    if (card === undefined) return;
    expect(card).toContain("--field-card-w: min(100cqw, calc(100cqh * var(--field-ratio-max)));");
    expect(card).toContain("--field-card-h: min(100cqh, calc(100cqw / var(--field-ratio-min)));");
    expect(card).toContain("width: var(--field-card-w);");
    expect(card).toContain("height: var(--field-card-h);");
    expect(card).toContain("max-width: 100%");
    expect(card).toMatch(/--def-scale:\s*min\(1,\s*tan\(atan2\(100cqw, var\(--field-card-h\)\)\),\s*tan\(atan2\(100cqh, var\(--field-card-w\)\)\)\)/);
  });

  it("the four rows still share the field's height, none shrunk for the others", () => {
    const phone = phoneBlock();
    expect(phone).toMatch(/\.field\s*\{[^}]*align-content:\s*stretch;/);
    const lane = /\.lane\s*\{([\s\S]*?)\}/.exec(phone)?.[1];
    expect(lane, "the phone block sizes the lane").toBeDefined();
    expect(lane).toContain("height: 100%;");
    expect(lane).toContain("grid-template-rows: repeat(4, minmax(var(--card-h), 1fr));");
  });

  it("#3257 an empty zone's placeholder is sized to the box a field card takes there, less an inset", () => {
    const placeholder = /\.zone::before\s*\{([\s\S]*?)\}/.exec(phoneBlock())?.[1];
    expect(placeholder, "the phone block sizes `.zone::before`").toBeDefined();
    if (placeholder === undefined) return;
    // The card's own box (the `.zone > .card` rule's two lines), read off the zone as the card's are.
    expect(placeholder).toContain("--field-card-w: min(100cqw, calc(100cqh * var(--field-ratio-max)));");
    expect(placeholder).toContain("--field-card-h: min(100cqh, calc(100cqw / var(--field-ratio-min)));");
    expect(placeholder).toContain("--zone-pad-inset: 2px;");
    // A pseudo-element is content-box: the border is in the size.
    expect(placeholder).toContain("box-sizing: border-box;");
    expect(placeholder).toContain("width: calc(var(--field-card-w) - 2 * var(--zone-pad-inset));");
    expect(placeholder).toContain("height: calc(var(--field-card-h) - 2 * var(--zone-pad-inset));");
    // What the base keeps: hidden once a card stands in the zone, and no hit.
    expect(board).toMatch(/\.zone:has\(>\s*\.card\)::before\s*\{[^}]*display:\s*none;/);
    expect(board).toMatch(/\.zone::before\s*\{[^}]*pointer-events:\s*none;/);
  });

  it("#3257 the badges stand in their own zone's top-left inner corner, above the name plate", () => {
    const phone = phoneBlock();
    const badges = /\.board \.zone\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phone)?.[1];
    expect(badges, "the phone block places `.field-badges`").toBeDefined();
    if (badges === undefined) return;
    expect(badges).toContain("position: absolute;");
    expect(badges).toContain("top: var(--badges-inset);");
    expect(badges).toContain("left: var(--badges-inset);");
    expect(badges).toContain("--badges-inset: 2px;");
    // The card's own size, repeated so `cqw`/`cqh` read the stack's container, its zone.
    expect(badges).toContain("--field-card-w: min(100cqw, calc(100cqh * var(--field-ratio-max)));");
    expect(badges).toContain("--field-card-h: min(100cqh, calc(100cqw / var(--field-ratio-min)));");
    // It stops above the name plate, so FieldBadges.tsx folds what does not fit into the "+n".
    expect(badges).toContain(
      "max-height: calc((100cqh + var(--field-card-h)) / 2 - var(--badges-stat) - var(--badges-plate) - var(--badges-inset));",
    );
    // Only the Activate control takes a press (activate.css).
    expect(badges).toContain("pointer-events: none;");
    // Nothing hangs out into the lane beside it, so nothing clears that lane's switch either.
    expect(badges).not.toContain("right:");
    expect(badges).not.toContain("wrap-reverse");
    expect(badges).not.toContain("--badges-top");
    expect(phone).not.toContain("--badges-top");
    expect(phone).not.toContain("--switch-target");
    expect(phone).not.toContain("--switch-clear");
    expect(phone).not.toMatch(/\.board \.zone\[data-lane="1"\]\s*>\s*\.field-badges/);
    // The marks are flex items that keep their size, and the cost steps aside under them.
    const items = /\.board \.field-badges :is\(\.activate-control, \.keyword, \.cf-kw-more\)\s*\{([\s\S]*?)\}/.exec(phone)?.[1] ?? "";
    expect(items).toContain("flex: none;");
    const cost = /\.board \.zone:has\(>\s*\.field-badges :is\(\.activate-control, \.keyword, \.cf-kw-more\)\)\s*>\s*\.card \.cost-gem\s*\{([\s\S]*?)\}/.exec(phone)?.[1] ?? "";
    expect(cost).toContain("visibility: hidden;");
  });

  it("#3257 over a DEF card the stack stops above the turned card, so it covers no turned name, attack or health", () => {
    const phone = phoneBlock();
    const def = /\.board \.zone:has\(>\s*\.card\[data-position="DEF"\]\)\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phone)?.[1];
    expect(def, "the phone block caps the stack over a DEF card").toBeDefined();
    if (def === undefined) return;
    // The card's own `--def-scale` (the `.zone > .card` rule's), repeated: the stack is no child of the card.
    const card = /\.zone\s*>\s*\.card\s*\{([\s\S]*?)\}/.exec(phone)?.[1] ?? "";
    const scale = /--def-scale:\s*([^;]+);/.exec(card)?.[1];
    expect(scale, "the phone's card has a `--def-scale`").toBeDefined();
    expect(def).toContain(`--def-scale: ${scale};`);
    // The turned card's box is as tall as the card is wide, scaled, and centred in the zone: the room is the band above it.
    expect(def).toContain("max-height: calc((100cqh - var(--field-card-w) * var(--def-scale)) / 2 - var(--badges-inset));");
    // Only the room changes: where the stack stands, and that it is never turned, stays the ATK rule's.
    expect(def).not.toMatch(/top:|left:|right:|transform:|position:/);
  });

  it("#3257 the badges' plate numbers are the face's", () => {
    // The stack is no child of the card, so it works the gem and the plate out from the card's size:
    // the same numbers as cards.css's, which this test holds it to.
    const cards = readSheet("src/cards/cards.css");
    expect(cards).toContain("--mf-stat: min(30cqmin, 26cqh);");
    expect(cards).toContain("--mf-plate: max(11cqh, 10px);");
    const badges = /\.board \.zone\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phoneBlock())?.[1] ?? "";
    const stat = /--badges-stat:\s*([^;]+);/.exec(badges)?.[1] ?? "";
    const plate = /--badges-plate:\s*([^;]+);/.exec(badges)?.[1] ?? "";
    expect(stat).toContain("* 0.3");
    expect(stat).toContain("* 0.26");
    expect(plate).toContain("* 0.11");
    expect(plate).toContain("10px");
  });

  it("#3257 the Activate press band starts at the control and stops at the zone", () => {
    const activate = readSheet("src/game/activate.css");
    const band = /\.board \.field-badges \.activate-control::before\s*\{([\s\S]*?)\}/.exec(activate)?.[1] ?? "";
    expect(band).toContain("top: -1px;");
    expect(band).toContain("bottom: auto;");
    expect(band).toContain("height: min(var(--hit-min), calc(100cqh - var(--badges-inset)));");
    expect(activate).not.toContain("--badges-top");
  });

  it("the match bar rides the top strip and the seats are thinner", () => {
    const phone = phoneBlock();
    expect(phone).toMatch(/\.app-shell--wide\s*>\s*\.game\[data-rail-top="true"\]\s*\{[^}]*display:\s*grid;/);
    expect(phone).toMatch(/\.game\[data-rail-top="true"\]\s*>\s*\.turn-banner\s*\{[^}]*grid-row:\s*1;/);
    // The finished-game chip is in-flow, so it takes a row of its own under the board instead of
    // auto-placing beside the rail and crushing the one-line ranks.
    expect(phone).toMatch(/\.game\[data-rail-top="true"\]\s*>\s*:is\(\[data-testid="result-overlay"\], \.trade-bar\)\s*\{[^}]*grid-row:\s*3;/);
    expect(phone).toMatch(/\.hero\s*\{[^}]*--hero-h:\s*44px;/);
    // The thinner seat leaves a pile's 44px band over your hand's top strip: the band stays above it (B29).
    expect(phone).toMatch(/\.seat\s+\.pile--browsable\s*\{[^}]*z-index:\s*var\(--z-content\);/);
  });
});

describe("#2330 a DEF card is scaled only as far as its zone needs", () => {
  it("the inline scale is the lane's --def-scale: on a phone worked out from its zone (#2556), 1 on a desktop or a tablet", () => {
    expect(renderUnit("DEF").style.transform).toContain("scale(var(--def-scale, 1))");
    const phone = laneRule(0);
    expect(phone).toContain("container-type: inline-size;");
    expect(phone).toMatch(/--def-scale: min\(1, tan\(atan2\(/);
    expect(phone).not.toMatch(/--def-scale: [\d.]+;/);
    const roomy = board.indexOf("@media (min-width: 761px) and (min-height: 501px) {");
    expect(roomy).toBeGreaterThan(-1);
    expect(laneRule(roomy)).toContain("--def-scale: 1;");
    expect(laneRule(roomy)).toContain("container-type: normal;");
  });
});
