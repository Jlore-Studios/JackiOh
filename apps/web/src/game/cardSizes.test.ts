// #3341: the board's and the hand's cards are larger than they were at the sizes a player holds the game at, and
// the numbers behind it are named once (game/config.ts) and held to board.css here. jsdom has no layout engine:
// e2e/cypress/component/card-sizes.cy.tsx measures a full board in a browser at the same five sizes.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { BOARD_CARD_BUDGET, HAND_LIFT_RISE, HAND_LIFT_SCALE } from "./config.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** board.css without its comments, its whitespace collapsed. */
const css = readFileSync(join(HERE, "board.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/\s+/g, " ");

type Layout = keyof typeof BOARD_CARD_BUDGET;

/** `--card-h`'s height term, the budget's `(100cqh - fixed) / rows`. */
function heightTerm(layout: Layout, extra = ""): string {
  const { fixedPx, rows } = BOARD_CARD_BUDGET[layout];
  return `calc((100cqh - ${fixedPx}px${extra}) / ${rows})`;
}

function clampOf(layout: Layout, term: string): string {
  const { minPx, maxPx } = BOARD_CARD_BUDGET[layout];
  return `--card-h: clamp(${minPx}px, ${term}, ${maxPx}px);`;
}

describe("#3341 the board's card height is the budget in game/config.ts", () => {
  it("#3341 board.css declares each layout's --card-h from BOARD_CARD_BUDGET", () => {
    const { desktop } = BOARD_CARD_BUDGET;
    const widthTerm = `calc((100cqw - ${desktop.widthFixedPx}px) / ${desktop.widthRows})`;
    expect(css).toContain(clampOf("desktop", heightTerm("desktop")));
    expect(css).toContain(clampOf("desktop", `min(${heightTerm("desktop")}, ${widthTerm})`));
    expect(css).toContain(clampOf("tablet", heightTerm("tablet")));
    expect(css).toContain(clampOf("base", heightTerm("base")));
    expect(css).toContain(clampOf("phone", heightTerm("phone")));
    expect(css).toContain(clampOf("phone", heightTerm("phone", " - var(--board-rail-phone-h)")));
    expect(css).toContain(clampOf("landscape", heightTerm("landscape")));
  });

  it("#3341 no layout still declares a budget the constants do not name", () => {
    // Every `100cqh - Npx` in board.css is one of the budget's fixed parts.
    const fixed = new Set([...css.matchAll(/\(100cqh - (\d+)px/g)].map((match) => Number(match[1])));
    const named = new Set(Object.values(BOARD_CARD_BUDGET).map((budget) => budget.fixedPx));
    expect([...fixed].sort((a, b) => a - b)).toEqual([...named].sort((a, b) => a - b));
  });

  /** The budget's card height at a board's content box, `clamp(min, min(height term, width term), max)`. */
  function cardHeight(budget: { minPx: number; fixedPx: number; rows: number; maxPx: number; widthFixedPx?: number; widthRows?: number }, box: { w: number; h: number }): number {
    const byHeight = (box.h - budget.fixedPx) / budget.rows;
    const byWidth = budget.widthFixedPx === undefined || budget.widthRows === undefined ? Infinity : (box.w - budget.widthFixedPx) / budget.widthRows;
    return Math.min(budget.maxPx, Math.max(budget.minPx, Math.min(byHeight, byWidth)));
  }

  /** What each layout's `--card-h` was before #3341: `{ fixed, rows, cap }`, and the desktop's width term. */
  const BEFORE = {
    desktop: { minPx: 56, fixedPx: 215, rows: 5.48, maxPx: 112, widthFixedPx: 334, widthRows: 10.66 },
    phone: { minPx: 44, fixedPx: 348, rows: 5.24, maxPx: 90 },
    landscape: { minPx: 40, fixedPx: 40, rows: 4.73, maxPx: 62 },
  } as const;

  /** The hand's ratio to `--card-h` in each layout, unchanged. */
  const HAND_RATIO = { desktop: 1.48, phone: 1.24, landscape: 1.4 } as const;

  /**
   * The five sizes, with the size of the board's content box (the `cqh` and `cqw` of its container) measured in a
   * browser by card-sizes.cy.tsx's mount: nothing above the board (#2917) and its padding off.
   */
  const SIZES = [
    { label: "1280x720", layout: "desktop", box: { w: 1240, h: 680 }, least: 1.5 },
    { label: "1920x1080", layout: "desktop", box: { w: 1880, h: 1040 }, least: 6 },
    { label: "390x844", layout: "phone", box: { w: 358, h: 820 }, least: 1.5 },
    { label: "360x780", layout: "phone", box: { w: 328, h: 756 }, least: 1.5 },
    { label: "844x390", layout: "landscape", box: { w: 812, h: 366 }, least: 6 },
  ] as const;

  it("#3341 at the five sizes the board's card and the hand's are larger than before", () => {
    for (const size of SIZES) {
      const now = cardHeight(BOARD_CARD_BUDGET[size.layout], size.box);
      const before = cardHeight(BEFORE[size.layout], size.box);
      expect(now - before, `${size.label}: the board's card`).toBeGreaterThanOrEqual(size.least);
      expect((now - before) * HAND_RATIO[size.layout], `${size.label}: the hand's card`).toBeGreaterThanOrEqual(size.least);
    }
  });

  it("#3341 the cap is the largest that keeps a 1920x1080 field row as tall as it was", () => {
    // The field's four rows share what the hand leaves (`1fr`), so a hand that grows past this takes it from them: at
    // 1920x1080 a row was 169.1px and the board's other parts, 198px before and 186px now, were measured in a browser.
    const ROW_BEFORE = 169.1;
    const OTHER_AFTER = 186.1;
    const { maxPx } = BOARD_CARD_BUDGET.desktop;
    const row = (1040 - OTHER_AFTER - HAND_RATIO.desktop * maxPx) / 4;
    expect(row, "a row now").toBeGreaterThanOrEqual(ROW_BEFORE);
    const bigger = (1040 - OTHER_AFTER - HAND_RATIO.desktop * (maxPx + 1)) / 4;
    expect(bigger, "a row at a cap one px larger").toBeLessThan(ROW_BEFORE);
  });

  it("#3341 the tablet's and the one column's budgets are as they were: their height already goes to the field", () => {
    expect(BOARD_CARD_BUDGET.tablet).toEqual({ minPx: 60, fixedPx: 355, rows: 5.3, maxPx: 113 });
    expect(BOARD_CARD_BUDGET.base).toEqual({ minPx: 60, fixedPx: 330, rows: 5.88, maxPx: 112 });
  });

  it("#3341 a layout's fixedPx plus the spacing it gave back is the fixed part it had", () => {
    // Each saving is a spacing step cut in board.css, counted over the gaps it sits in (CLAUDE.md rule 9: named).
    const lanes = 3 * (4 - 2); // a lane's four rows, 3 gaps, `--gap` 4px to 2px on a phone
    expect(BOARD_CARD_BUDGET.desktop.fixedPx + 3 * (6 - 4) + (10 - 4), "desktop: three lane gaps 6 to 4px, the room over your fan 10 to 4px").toBe(BEFORE.desktop.fixedPx);
    expect(BOARD_CARD_BUDGET.desktop.widthFixedPx + 9 * (6 - 4), "desktop: the fan's nine gaps 6 to 4px").toBe(BEFORE.desktop.widthFixedPx);
    expect(BOARD_CARD_BUDGET.phone.fixedPx + 4 * (6 - 4) + lanes + 2 * (4 - 2), "phone: four row gaps, the lanes' three, the mat's two sides").toBe(BEFORE.phone.fixedPx);
    // The phone on its side gives back 14px of its 40 and keeps 2 of them against a hand peek it cannot spare.
    expect(BOARD_CARD_BUDGET.landscape.fixedPx + 2 * (6 - 4) + lanes + 2 * (4 - 2) - 2, "landscape: two row gaps, the lanes' three, the mat's two sides, less the 2px kept").toBe(BEFORE.landscape.fixedPx);
  });
});

describe("#3341 a lifted hand card rises and grows as game/config.ts says", () => {
  /** Every declaration of a lift token in board.css, in file order: the base, the tablet's, the phone's, the landscape's. */
  function declared(token: string): number[] {
    return [...css.matchAll(new RegExp(`${token}: ([\\d.]+);`, "g"))].map((match) => Number(match[1]));
  }

  it("#3341 the lift tokens in board.css are HAND_LIFT_SCALE and HAND_LIFT_RISE", () => {
    expect(declared("--hand-lift-scale")).toEqual([HAND_LIFT_SCALE.desktop, HAND_LIFT_SCALE.tablet, HAND_LIFT_SCALE.phone, HAND_LIFT_SCALE.landscape]);
    expect(declared("--hand-lift-rise")).toEqual([HAND_LIFT_RISE.desktop, HAND_LIFT_RISE.tablet, HAND_LIFT_RISE.phone]);
  });

  it("#3341 each lift is larger than it was, the phone on its side as it was", () => {
    // Before: the desktop's 8px step with no growth, the tablet's 1.08, the phone's 1.6 and the landscape's 1.7.
    expect(HAND_LIFT_SCALE.desktop).toBeGreaterThan(1);
    expect(HAND_LIFT_SCALE.tablet).toBeGreaterThan(1.08);
    expect(HAND_LIFT_SCALE.phone).toBeGreaterThan(1.6);
    expect(HAND_LIFT_SCALE.landscape).toBe(1.7);
    expect(HAND_LIFT_RISE.tablet).toBe(0.12);
    expect(HAND_LIFT_RISE.phone).toBeGreaterThan(0.3);
  });

  it("#3341 every lift reads the tokens, and none states a rise of its own but the playing card's step", () => {
    const lifts = [...css.matchAll(/transform: (translateY\([^;]*\)(?: scale\([^;]*\))?);/g)].map((match) => match[1] ?? "");
    const reading = lifts.filter((lift) => lift.includes("--hand-lift-"));
    expect(reading, "the desktop's rest and hover, the tablet's, the phone's and the landscape's").toHaveLength(5);
    for (const lift of reading) expect(lift, "rise and growth both").toMatch(/var\(--hand-lift-scale\)/);
    // The card being played only steps up, so what it is aimed at stays in reach (B45).
    expect(lifts.filter((lift) => !lift.includes("--hand-lift-")).some((lift) => lift.includes("-10px"))).toBe(true);
    expect(css).not.toContain("translateY(-8px)");
  });
});
