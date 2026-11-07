// R3348000, R3348001 (#3348): what arena.css states that arena/config.ts also does: the animation times, the
// hover's guard and the still looks.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { CORNER_BIG_MS, CORNER_RARE_MS, CORNER_SEQUENCE_MS, CORNER_STILL_MS, CORNER_TAP_MS, TILE_FEEL } from "./config.ts";

const CSS = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "arena.css"), "utf8");
const CODE = CSS.replace(/\/\*[\s\S]*?\*\//g, " ");
const animations = (code: string): string[] => [...code.matchAll(/(?<![\w-])animation\s*:\s*([^;]+)/g)].map((match) => (match[1] ?? "").trim());

describe("R3348001 arena.css and config.ts agree", () => {
  it("R3348000 R3348001 each animation time the sheet states is a number config.ts names, and each is used", () => {
    const named = new Map<string, string>([
      [`${String(TILE_FEEL.rippleMs)}ms`, "TILE_FEEL.rippleMs"],
      [`${String(CORNER_TAP_MS)}ms`, "CORNER_TAP_MS"],
      [`${String(CORNER_BIG_MS)}ms`, "CORNER_BIG_MS"],
      [`${String(CORNER_RARE_MS)}ms`, "CORNER_RARE_MS"],
      [`${String(CORNER_SEQUENCE_MS)}ms`, "CORNER_SEQUENCE_MS"],
    ]);
    const stated = animations(CODE).flatMap((value) => value.match(/(?<![\w.-])\d*\.?\d+ms(?![\w-])/g) ?? []);
    expect(stated.length).toBeGreaterThan(0);
    for (const time of stated) expect(named.has(time), `${time} is a named constant`).toBe(true);
    for (const [time, name] of named) expect(stated, name).toContain(time);
  });

  it("R3348001 a still look is for no animation: the still prop and tile rules start none", () => {
    expect(CODE).toMatch(/\[data-play="still"\]/);
    for (const rule of CODE.matchAll(/([^{}]*\[data-play="still"\][^{}]*)\{([^{}]*)\}/g)) {
      expect(rule[2] ?? "", rule[1]?.trim()).not.toMatch(/animation/);
    }
    // Every moving rule asks for the move, never the still or the unset.
    for (const rule of CODE.matchAll(/([^{}]+)\{([^{}]*animation\s*:[^{}]*)\}/g)) {
      expect(rule[1] ?? "").toMatch(/\[data-play="move"\]/);
    }
    expect(CORNER_STILL_MS).toBeGreaterThan(0);
  });

  it("R3348000 hovering a tile lights its edge only where a mouse hovers, and never a tile with a state of its own", () => {
    const hover = CODE.match(/@media\s*([^{]+)\{\s*(\.field \.zone:hover[^{]*)\{([^}]*)\}/);
    expect(hover?.[1]?.trim()).toBe("(hover: hover) and (pointer: fine)");
    for (const own of [".zone-locked", ".zone-reserved", "[data-glow]", '[data-selected="true"]']) expect(hover?.[2], own).toContain(own);
    expect(hover?.[3]).toMatch(/box-shadow/);
    expect(CODE.match(/\.zone:hover/g)).toHaveLength(1);
  });

  it("R3348001 a prop that does not fit, or has not been measured, is neither drawn nor hit", () => {
    const rule = CODE.match(/\.corner-prop\[data-fit="false"\],\s*\.corner-props \.corner-prop\[data-fit="pending"\]\s*\{([^}]*)\}/);
    expect(rule?.[1]).toMatch(/visibility:\s*hidden/);
    expect(rule?.[1]).toMatch(/pointer-events:\s*none/);
  });

  it("R3348001 a prop is at least --hit-min each way, and the opponent's takes no press", () => {
    expect(CODE).toMatch(/\.corner-prop\s*\{[^}]*width:\s*var\(--hit-min\);[^}]*height:\s*var\(--hit-min\);/);
    expect(CODE).toMatch(/\[data-half="theirs"\] \.corner-prop\s*\{[^}]*pointer-events:\s*none/);
  });

  it("R3348001 a tall phone's field keeps a --hit-min strip above and below its lanes for the props", () => {
    const strip = CODE.match(/@media\s*\(max-width: 600px\) and \(min-height: 840px\)\s*\{\s*\.board \.field\s*\{([^}]*)\}/);
    expect(strip?.[1]).toMatch(/padding-block:\s*calc\(var\(--hit-min\) \+ var\(--space-2\) \* 2\)/);
  });
});
