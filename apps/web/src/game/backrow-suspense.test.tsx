// #2111, R2111000: a trap's three moments, set, wait and flip, each have their own motion, and a
// face-down card's motion says nothing of it. The motion is CSS, so the sheets are read as text (the
// way fx/css.test.ts reads them); the DOM tests pin what the one breathing rule is drawn on.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import { PAIR_OFFSET_MS } from "../audio/constants.ts";
import { FX_TRAP_BURST_AT } from "../fx/constants.ts";
import { renderBoard } from "../test/board.tsx";
import { baseView, emptySide, faceUpBackrow } from "../test/fixtures.ts";
import { ANIMATIONS, durationFor } from "./animations.ts";

afterEach(() => {
  cleanup();
});

const SRC = dirname(dirname(fileURLToPath(import.meta.url)));
const code = (sheet: string): string => readFileSync(join(SRC, sheet), "utf8").replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\s+/g, " ");
const ANIM = code("game/animations.css");
const FACEDOWN = code("game/facedown.css");
const FX = code("fx/fx.css");

const keyframes = (css: string, name: string): string =>
  new RegExp(String.raw`@keyframes ${name} \{((?: ?[^{}]*\{[^{}]*\})*) ?\}`).exec(css)?.[1] ?? "";
const stop = (body: string, pct: number): string => new RegExp(String.raw`(?:^| )${String(pct)}% \{([^{}]*)\}`).exec(body)?.[1]?.trim() ?? "";

/** The one rule that breathes: every face-down card, and nothing about a card in its selector. */
const BREATHES = '.card.card-facedown, .card[data-unrevealed="true"]:not([data-animating]):not([data-glow])';

/** Two of your own face-down traps under their veils, and four of the opponent's backs, each different. */
function waitingView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      backrow: [
        faceUpBackrow("p1", { instanceId: "own-trap", defId: "core-041", type: "Trap", cost: 1, unrevealed: true }),
        faceUpBackrow("p1", { instanceId: "own-field-trap", defId: "core-061", type: "Field Trap", cost: 3, unrevealed: true }),
        null,
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", {
      backrow: [{ faceDown: true, cost: 2, plague: 3 }, { faceDown: true, cost: 0 }, { faceDown: true }, { faceDown: true, cost: 5, buried: 1 }, null],
    }),
  });
}

describe("R2111000 the set", () => {
  it("R2111000 a card landing in a backrow zone drops on jk-trap-set, flat and heavy, no flash", () => {
    expect(ANIM).toContain('.zone[data-row="backrow"][data-animating="summoned"]::after { animation-name: jk-trap-set; }');
    const set = keyframes(ANIM, "jk-trap-set");
    expect(stop(set, 0)).toContain("opacity: 0");
    expect(stop(set, 44)).toContain("scale(1.03, 0.94)");
    expect(set).not.toContain("brightness");
    expect(set).not.toBe(keyframes(ANIM, "jk-summon-scale"));
  });

  it("R2111000 the set lands where its thud sounds", () => {
    const set = keyframes(ANIM, "jk-trap-set");
    const delay = /\.game:has\(\[data-animating="cardPlayed"\]\) \.zone\[data-animating="summoned"\]::after \{ animation-delay: calc\((\d+)ms/.exec(ANIM)?.[1];
    const landing = /(\d+)% \{ transform: translateY\(0\) scale\(1\.03, 0\.94\);/.exec(set)?.[1];
    expect(delay).toBeDefined();
    expect(landing).toBeDefined();
    expect(Number(delay) + (Number(landing) * ANIMATIONS.summoned.durationMs) / 100).toBe(PAIR_OFFSET_MS);
  });

  it("R2111000 the effects layer's stand-in is laid down too", () => {
    expect(FX).toContain(
      '.fx-dom > .fx-hold[data-land="set"] { animation: fx-hold-fly var(--fx-land-ms) cubic-bezier(0.4, 0, 0.85, 0.45) both, fx-hold-set 360ms cubic-bezier(0.2, 0.7, 0.3, 1) var(--fx-land-ms) forwards; }',
    );
    expect(stop(keyframes(FX, "fx-hold-set"), 30)).toContain("brightness(0.85)");
    expect(keyframes(FX, "fx-hold-set")).not.toBe(keyframes(FX, "fx-hold-slam"));
  });
});

describe("R2111000 the wait", () => {
  it("R2111000 every face-down card breathes one breath from one rule", () => {
    expect(FACEDOWN).toContain(`${BREATHES} { animation: jk-facedown-breathe calc(4.8s * var(--anim-scale, 1)) var(--ease-in-out) infinite; }`);
    expect(FACEDOWN.split("jk-facedown-breathe")).toHaveLength(3);
    expect(ANIM + FX).not.toContain("jk-facedown-breathe");
    const breath = keyframes(FACEDOWN, "jk-facedown-breathe");
    expect(breath).toContain("0%, 100% { scale: 1; filter: brightness(1); }");
    expect(breath).toContain("50% { scale: 0.985; filter: brightness(1.08); }");
  });

  it("R2111000 on the board every back and your own face-down trap is drawn alike for it", () => {
    const board = renderBoard(waitingView());
    const backs = [...board.querySelectorAll<HTMLElement>(".card-facedown")];
    expect(backs).toHaveLength(4);
    expect(new Set(backs.map((back) => back.className)).size).toBe(1);
    for (const back of backs) {
      expect(back.hasAttribute("style")).toBe(false);
      expect(back.matches(BREATHES)).toBe(true);
    }
    const veiled = [...board.querySelectorAll<HTMLElement>('[data-unrevealed="true"]')];
    expect(veiled).toHaveLength(2);
    for (const own of veiled) {
      expect(own.hasAttribute("style")).toBe(false);
      expect(own.matches(BREATHES)).toBe(true);
    }
  });
});

describe("R2111000 the flip", () => {
  it("R2111000 a firing trap gathers itself before it turns at FX_TRAP_BURST_AT", () => {
    expect(Math.round(FX_TRAP_BURST_AT * 100)).toBe(30);
    for (const name of ["jk-trap-flip", "jk-trap-flip-hold"]) {
      const flip = keyframes(ANIM, name);
      expect(stop(flip, 0), name).toContain("rotateY(180deg) scale(1);");
      expect(stop(flip, 15), name).toContain("rotateY(180deg) scale(0.94);");
      expect(stop(flip, 15), name).toContain("brightness(0.8);");
      expect(stop(flip, 30), name).toContain("rotateY(0deg) scale(1.12);");
      expect(flip.indexOf(" 15% {"), name).toBeGreaterThanOrEqual(0);
      expect(flip.indexOf(" 15% {"), name).toBeLessThan(flip.indexOf(" 30% {"));
    }
    expect(ANIMATIONS.trapFired.durationMs).toBe(700);
  });
});

describe("R2111000 reduced motion", () => {
  it("R2111000 reduced motion turns all three off", () => {
    expect(FACEDOWN).toContain(`@media (prefers-reduced-motion: reduce) { ${BREATHES} { animation: none; } }`);
    expect(FACEDOWN).toContain(
      ':root[data-reduce-motion="true"] .card.card-facedown, :root[data-reduce-motion="true"] .card[data-unrevealed="true"]:not([data-animating]):not([data-glow]) { animation: none; }',
    );
    for (const type of ["cardPlayed", "summoned", "trapFired"] as const) expect(durationFor(type, true), type).toBe(0);
    expect(ANIM).toContain(
      '[data-animating="trapFired"] { animation-name: jk-trap-flip; animation-duration: calc(700ms * var(--anim-scale) * var(--anim-squeeze, 1));',
    );
  });
});
