// R3546004: a GIGA permanent drawn across its four zones (issue #3546). The engine marks only
// the anchor's view with `giga: true` and leaves the three covered zones empty, so the board
// stamps each zone `data-giga` (`game/giga.ts`) and the stylesheet spans the anchor's card.
// R5053000 (#5053): in Defense Position it lies on its side across the same four zones. jsdom lays
// nothing out, so those tests read the sheets as text and work the boxes out for a few zone sizes.

import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { testid } from "./contract.ts";
import { keyframeBodies, parseDecls, parseSheet, splitTop, type Sheet } from "../test/cssSheet.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "../test/fixtures.ts";
import { renderBoard } from "../test/board.tsx";
import { readSheet } from "../test/setup.ts";

afterEach(() => {
  cleanup();
});

function gigaOf(board: HTMLElement, side: "you" | "opponent", row: "units" | "backrow", lane: number): string | null {
  const zone = board.querySelector(`[data-testid="${testid.zone(side, row, lane)}"]`);
  if (zone === null) throw new Error(`no zone ${side} ${row} ${lane}`);
  return zone.getAttribute("data-giga");
}

describe("R3546004 a GIGA permanent drawn across its four zones", () => {
  it("R3546004 your GIGA unit's zone is the anchor and its three others are covered", () => {
    const board = renderBoard(
      baseView({
        you: {
          ...emptySide("p1"),
          units: [unit("p1", { giga: true }), null, null, null, null],
        },
      }),
    );
    expect(gigaOf(board, "you", "units", 1)).toBe("anchor");
    expect(gigaOf(board, "you", "units", 2)).toBe("covered");
    expect(gigaOf(board, "you", "backrow", 1)).toBe("covered");
    expect(gigaOf(board, "you", "backrow", 2)).toBe("covered");
    expect(gigaOf(board, "you", "units", 3)).toBeNull();
    expect(gigaOf(board, "you", "backrow", 3)).toBeNull();
  });

  it("R3546004 an enemy GIGA field spell covers its unit zones", () => {
    const board = renderBoard(
      baseView({
        opponent: {
          ...emptySide("p2"),
          backrow: [faceUpBackrow("p2", { giga: true }), null, null, null, null],
        },
      }),
    );
    expect(gigaOf(board, "opponent", "backrow", 1)).toBe("anchor");
    expect(gigaOf(board, "opponent", "backrow", 2)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 1)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 2)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 3)).toBeNull();
  });

  // R3572000 (#3572) since gave GIGA a glyph and a treatment, drawn on the field where it acts,
  // so the unit now wears its chip where this test once pinned none.
  it("R3546004 a unit holding the GIGA keyword renders, wearing GIGA's chip and treatment", () => {
    const board = renderBoard(
      baseView({
        you: {
          ...emptySide("p1"),
          units: [unit("p1", { keywords: [{ kind: "GIGA" }] }), null, null, null, null],
        },
      }),
    );
    // No throw above, and the kind draws as R3572000's keyword visuals say: a visible chip and its treatment.
    expect(board.querySelector('.keywords > [data-keyword="GIGA"]:not([data-chip="hidden"])')).not.toBeNull();
    expect(board.querySelector('[data-keyword-fx="GIGA"]')).not.toBeNull();
  });
});

const boardCss = parseSheet(readSheet("src/game/board.css"));
const animCss = parseSheet(readSheet("src/game/animations.css"));

/** A unit zone's inside, which both `%` and the anchor zone's `cqw`/`cqh` read, and the lanes' gap. */
type ZoneSize = { w: number; h: number; gap: number };

/** A desktop's wide zone, an upright phone's, a portrait tablet's (each measured in a browser, rounded). */
const ZONES: readonly ZoneSize[] = [
  { w: 190, h: 93, gap: 6 },
  { w: 66, h: 75, gap: 2 },
  { w: 137, h: 127, gap: 6 },
];

/** A box in a zone: `x` from its left edge, `y` from the edge the GIGA grows away from. */
type Box = { x: number; y: number; w: number; h: number };

/** The declarations of the top-level rules naming `selector`, the later winning. */
function declsOf(sheet: Sheet, selector: string): Map<string, string> {
  const merged = new Map<string, string>();
  for (const rule of sheet.rules) if (rule.media === null && rule.selectors.includes(selector)) for (const [k, v] of rule.decls) merged.set(k, v);
  return merged;
}

function declared(decls: Map<string, string>, name: string): string {
  const value = decls.get(name);
  if (value === undefined) throw new Error(`no ${name}`);
  return value;
}

/** One of the GIGA rules' lengths in px: `0`, or a `calc()` adding and taking `Ncqw`, `Ncqh`, `N%` (along `axis`) and `var(--gap)`. */
function px(value: string, zone: ZoneSize, axis: "x" | "y"): number {
  let total = 0;
  let sign = 1;
  for (const token of value.replace(/^calc\((.*)\)$/, "$1").split(" ")) {
    if (token === "+" || token === "-") {
      sign = token === "+" ? 1 : -1;
      continue;
    }
    const length = /^(\d+(?:\.\d+)?)(cqw|cqh|%)$/.exec(token);
    let part: number;
    if (token === "0") part = 0;
    else if (token === "var(--gap)") part = zone.gap;
    else if (length !== null) part = (Number(length[1]) / 100) * (length[2] === "cqw" || (length[2] === "%" && axis === "x") ? zone.w : zone.h);
    else throw new Error(`unread length ${token} in ${value}`);
    total += sign * part;
  }
  return total;
}

/** A number the turn's rules state: `var(--x)`, `calc(1 / …)` or `tan(atan2(a, b))`, which is `a / b`. */
function ratio(value: string, decls: Map<string, string>, zone: ZoneSize): number {
  const ref = /^var\((--[\w-]+)\)$/.exec(value);
  if (ref !== null) return ratio(declared(decls, ref[1]!), decls, zone);
  const inverse = /^calc\(1 \/ (.+)\)$/.exec(value);
  if (inverse !== null) return 1 / ratio(inverse[1]!, decls, zone);
  const quotient = /^tan\(atan2\((.+)\)\)$/.exec(value);
  if (quotient === null) throw new Error(`unread ratio ${value}`);
  const [a, b] = splitTop(quotient[1]!, ",");
  return px(a!, zone, "x") / px(b!, zone, "x");
}

function boxOf(decls: Map<string, string>, zone: ZoneSize, edge: "top" | "bottom"): Box {
  return {
    x: px(declared(decls, "left"), zone, "x"),
    y: px(declared(decls, edge), zone, "y"),
    w: px(declared(decls, "width"), zone, "x"),
    h: px(declared(decls, "height"), zone, "y"),
  };
}

/** Where `box` shows under `scale(sx, sy)` and then a turn of `quarter` (0 or 90deg), about its centre. */
function shown(box: Box, quarter: boolean, sx = 1, sy = 1): Box {
  const w = box.w * sx;
  const h = box.h * sy;
  const [across, down] = quarter ? [h, w] : [w, h];
  return { x: box.x + box.w / 2 - across / 2, y: box.y + box.h / 2 - down / 2, w: across, h: down };
}

function expectSameBox(actual: Box, expected: Box): void {
  for (const key of ["x", "y", "w", "h"] as const) expect(actual[key], key).toBeCloseTo(expected[key], 6);
}

/** Each side's anchor card selector and the edge its span grows away from (board.css, R3546004). */
const SIDES = [
  { cell: '.zone[data-side="you"][data-row="units"][data-giga="anchor"] > .card', edge: "top" },
  { cell: '.zone[data-side="opponent"][data-row="units"][data-giga="anchor"] > .card', edge: "bottom" },
] as const;

function transformAt(name: string, at: "0%" | "100%"): string | undefined {
  const body = keyframeBodies(animCss).get(name) ?? "";
  for (const chunk of body.split("}")) {
    const [stop, decls] = chunk.split("{");
    if (stop?.trim() === at) return parseDecls(decls ?? "").get("transform");
  }
  return undefined;
}

describe("R5053000 a GIGA unit in Defense Position lies on its side across its four zones", () => {
  it("R5053000 turned a quarter at full size, its DEF box covers exactly the four zones its ATK box does", () => {
    for (const { cell, edge } of SIDES) {
      const atk = declsOf(boardCss, cell);
      const def = new Map([...atk, ...declsOf(boardCss, `${cell}[data-position="DEF"]`)]);
      // Card.tsx turns a DEF card `rotate(90deg) scale(var(--def-scale, 1))`: a pure quarter turn here.
      expect(def.get("--def-scale"), cell).toBe("1");
      for (const zone of ZONES) {
        const footprint = boxOf(atk, zone, edge);
        expect(footprint.w).toBeCloseTo(2 * zone.w + zone.gap, 6);
        expect(footprint.h).toBeCloseTo(2 * zone.h + zone.gap, 6);
        expectSameBox(shown(boxOf(def, zone, edge), true), footprint);
      }
    }
  });

  it("R5053000 the anchor zone answers its size to its card while the card is on its side or turning, in every layout", () => {
    const container = boardCss.rules.find((rule) =>
      rule.selectors.some((s) => s.startsWith('.zone[data-giga="anchor"]:has(') && s.includes('[data-position="DEF"]') && s.includes('[data-animating="positionSwitched"]')),
    );
    expect(container?.media).toBeNull();
    expect(container?.decls.get("container-type")).toBe("size");
  });

  it("R5053000 the switch turns a GIGA from one resting box onto the other, ending inside its four zones", () => {
    const turning = declsOf(animCss, '.zone[data-giga="anchor"] > [data-animating="positionSwitched"]');
    const back = new Map([...turning, ...declsOf(animCss, '.zone[data-giga="anchor"] > [data-animating="positionSwitched"][data-position="DEF"]')]);
    expect(turning.get("animation-name")).toBe("jk-rotate-def-giga");
    expect(back.get("animation-name")).toBe("jk-rotate-atk-giga");
    // Each starts on its card's resting transform: ATK has none, DEF is a quarter turn at `--def-scale: 1`.
    expect(transformAt("jk-rotate-def-giga", "0%")).toBe("rotate(0deg) scale(1)");
    expect(transformAt("jk-rotate-atk-giga", "0%")).toBe("rotate(90deg) scale(1)");
    expect(transformAt("jk-rotate-def-giga", "100%")).toBe("rotate(90deg) scale(var(--giga-turn-x), var(--giga-turn-y))");
    expect(transformAt("jk-rotate-atk-giga", "100%")).toBe("rotate(0deg) scale(var(--giga-turn-x), var(--giga-turn-y))");
    for (const { cell, edge } of SIDES) {
      const atk = declsOf(boardCss, cell);
      const def = new Map([...atk, ...declsOf(boardCss, `${cell}[data-position="DEF"]`)]);
      for (const zone of ZONES) {
        const footprint = boxOf(atk, zone, edge);
        const toDef = shown(footprint, true, ratio(declared(turning, "--giga-turn-x"), turning, zone), ratio(declared(turning, "--giga-turn-y"), turning, zone));
        expectSameBox(toDef, footprint);
        const toAtk = shown(boxOf(def, zone, edge), false, ratio(declared(back, "--giga-turn-x"), back, zone), ratio(declared(back, "--giga-turn-y"), back, zone));
        expectSameBox(toAtk, footprint);
      }
    }
  });

  it("R5053000 on a landscape phone the one-tile gem cap passes a GIGA's anchor by, so its gems keep the face's own size", () => {
    const caps = boardCss.rules.filter(
      (rule) => rule.media?.includes("(orientation: landscape) and (max-height: 500px)") === true && rule.decls.get("min-width") === "13px" && rule.selectors.some((s) => s.endsWith(" .stat")),
    );
    expect(caps.length).toBeGreaterThan(0);
    for (const cap of caps) expect(cap.selectors).toEqual(['.zone:where(:not([data-giga="anchor"])) .stat']);
  });
});
