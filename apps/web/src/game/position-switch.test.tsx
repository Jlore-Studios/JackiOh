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

  it("the badges hang off the card's corner, the match bar rides the top strip, and the seats are thinner", () => {
    const phone = phoneBlock();
    const badges = /\.board \.zone\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phone)?.[1];
    expect(badges, "the phone block places `.field-badges`").toBeDefined();
    expect(badges).toContain("position: absolute;");
    // The stack's corner is the card's own top-left, worked out from the card's size (the zone
    // centres the card): never the zone's bare corner, so it tracks the card on a tall phone too.
    expect(badges).toContain("--field-card-w: min(100cqw, calc(100cqh * var(--field-ratio-max)));");
    expect(badges).toContain("--field-card-h: min(100cqh, calc(100cqw / var(--field-ratio-min)));");
    expect(badges).toContain("right: calc(100% - ((100cqw - var(--field-card-w)) / 2) - 2px);");
    expect(badges).not.toContain("top: 0;");
    // It hangs over the zone beside it, whose switch sits at that corner: only the Activate control takes a press.
    expect(badges).toContain("pointer-events: none;");
  });

  it("the badges start below the switch target of the zone beside them, wrap inside their own zone's height, and keep their press band under the control", () => {
    const phone = phoneBlock();
    const badges = /\.board \.zone\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phone)?.[1] ?? "";
    // The stack's top is the card's top or the switch target's height, whichever is lower down: the
    // zone left of this one holds its switch (⟳) at its top-right, where this stack hangs.
    expect(badges).toContain(
      "--badges-top: max(calc((100cqh - var(--field-card-h)) / 2), min(var(--switch-target), calc(50cqh - var(--switch-clear))));",
    );
    expect(badges).toContain("top: var(--badges-top);");
    // ... and the target it clears is the switch's own: the same two numbers as the switch's rule.
    const switchRule = /\.board \.zone\s*>\s*\.switch-button\s*\{([\s\S]*?)\}/.exec(board)?.[1] ?? "";
    for (const name of ["--switch-target", "--switch-clear"]) {
      const of = (rule: string): string | undefined => new RegExp(`${name}:\\s*([^;]+);`).exec(rule)?.[1];
      expect(of(switchRule), `the switch declares ${name}`).toBeDefined();
      expect(of(badges), `the stack's ${name} is the switch's`).toBe(of(switchRule));
    }
    expect(switchRule).toContain("width: min(var(--switch-target), calc(50% - var(--switch-clear)));");
    expect(switchRule).toContain("height: min(var(--switch-target), calc(50% - var(--switch-clear)));");
    // What does not fit under it wraps to a column on its left: it never reaches the row below.
    expect(badges).toContain("flex-wrap: wrap-reverse;");
    expect(badges, "wrap-reverse swaps the cross axis: flex-start is the card's edge").toContain("align-items: flex-start;");
    expect(badges).toContain("max-height: calc(100cqh - var(--badges-top));");
    // The Activate control's press band reaches down from the control, never up over the switch or
    // the row above, and no further than its zone.
    const activate = readSheet("src/game/activate.css");
    const band = /\.board \.field-badges \.activate-control::before\s*\{([\s\S]*?)\}/.exec(activate)?.[1] ?? "";
    expect(band).toContain("top: -1px;");
    expect(band).toContain("bottom: auto;");
    expect(band).toContain("height: min(var(--hit-min), calc(100cqh - var(--badges-top)));");
  });

  it("lane 1's stack has no neighbour to clear and no column to wrap into, so it starts at the card's top and never wraps", () => {
    const lane1 = /\.board \.zone\[data-lane="1"\]\s*>\s*\.field-badges\s*\{([\s\S]*?)\}/.exec(phoneBlock())?.[1] ?? "";
    expect(lane1).toContain("--badges-top: calc((100cqh - var(--field-card-h)) / 2);");
    expect(lane1).toContain("flex-wrap: nowrap;");
    expect(lane1).toContain("align-items: flex-end;");
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
