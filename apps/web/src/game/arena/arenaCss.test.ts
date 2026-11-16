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

/** Whether `body` declares an animation: `animation`, whitespace, `:` (#4009). */
function hasAnimationDecl(body: string): boolean {
  const keyword = "animation";
  let at = body.indexOf(keyword);
  while (at !== -1) {
    let i = at + keyword.length;
    while (i < body.length && /\s/.test(body.charAt(i))) i++;
    if (body.charAt(i) === ":") return true;
    at = body.indexOf(keyword, at + 1);
  }
  return false;
}

/**
 * The first `@media` guard holding `.field .zone:hover`'s rule (#4009: the nested
 * `/@media\s*([^{]+)\{\s*(\.field \.zone:hover[^{]*)\{([^}]*)\}/` rescans brace-free runs).
 */
function hoverRule(code: string): { query: string; selector: string; body: string } | null {
  const head = ".field .zone:hover";
  let at = 0;
  for (;;) {
    const media = code.indexOf("@media", at);
    if (media === -1) return null;
    let i = media + "@media".length;
    while (/\s/.test(code.charAt(i))) i++;
    const open = code.indexOf("{", i);
    if (open === -1) return null;
    const query = code.slice(i, open);
    let inner = -1;
    let extra = "";
    if (query !== "") {
      let j = open + 1;
      while (/\s/.test(code.charAt(j))) j++;
      if (code.startsWith(head, j)) {
        inner = code.indexOf("{", j + head.length);
        if (inner !== -1) extra = code.slice(j + head.length, inner);
      }
    }
    if (inner === -1) {
      at = media + 1;
      continue;
    }
    const close = code.indexOf("}", inner + 1);
    if (close === -1) return null;
    return { query, selector: head + extra, body: code.slice(inner + 1, close) };
  }
}

describe("R3348001 arena.css and config.ts agree", () => {
  it("R3348000 R3348001 each animation time the sheet states is a number config.ts names, and each is used", () => {
    const named = new Map<string, string>([
      [`${String(TILE_FEEL.rippleMs)}ms`, "TILE_FEEL.rippleMs"],
      [`${String(CORNER_TAP_MS)}ms`, "CORNER_TAP_MS"],
      [`${String(CORNER_BIG_MS)}ms`, "CORNER_BIG_MS"],
      [`${String(CORNER_RARE_MS)}ms`, "CORNER_RARE_MS"],
      [`${String(CORNER_SEQUENCE_MS)}ms`, "CORNER_SEQUENCE_MS"],
    ]);
    // No `\d*\.?\d+`: its runs trade characters super-linearly; the alternation below matches the
    // same numbers (#4009).
    const stated = animations(CODE).flatMap((value) => value.match(/(?<![\w.-])(\d+(?:\.\d+)?|\.\d+)ms(?![\w-])/g) ?? []);
    expect(stated.length).toBeGreaterThan(0);
    for (const time of stated) expect(named.has(time), `${time} is a named constant`).toBe(true);
    for (const [time, name] of named) expect(stated, name).toContain(time);
  });

  it("R3348001 a still look is for no animation: the still prop and tile rules start none", () => {
    expect(CODE).toMatch(/\[data-play="still"\]/);
    // Rule scans as character loops (#4009: the class-heavy patterns rescan a brace-free run).
    const chunks = CODE.split("}");
    if (!CODE.endsWith("}")) chunks.pop();
    for (const chunk of chunks) {
      let start = 0;
      let open = chunk.indexOf("{", start);
      while (open !== -1) {
        const selector = chunk.slice(start, open);
        if (selector.includes('[data-play="still"]')) {
          expect(chunk.slice(open + 1), selector.trim()).not.toMatch(/animation/);
          break;
        }
        start = open + 1;
        open = chunk.indexOf("{", start);
      }
    }
    // Every moving rule asks for the move, never the still or the unset.
    for (const chunk of chunks) {
      const open = chunk.lastIndexOf("{");
      if (open === -1) continue;
      const before = chunk.slice(0, open);
      const selector = before.slice(before.lastIndexOf("{") + 1);
      const body = chunk.slice(open + 1);
      if (selector === "" || body.includes("{") || !hasAnimationDecl(body)) continue;
      expect(selector).toMatch(/\[data-play="move"\]/);
    }
    expect(CORNER_STILL_MS).toBeGreaterThan(0);
  });

  it("R3348000 hovering a tile lights its edge only where a mouse hovers, and never a tile with a state of its own", () => {
    // The `@media` guard holding `.field .zone:hover`'s rule (#4009: the nested pattern rescans).
    const hover = hoverRule(CODE);
    expect(hover?.query.trim()).toBe("(hover: hover) and (pointer: fine)");
    for (const own of [".zone-locked", ".zone-reserved", "[data-glow]", '[data-selected="true"]']) expect(hover?.selector, own).toContain(own);
    expect(hover?.body).toMatch(/box-shadow/);
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
    // A prop and the one inset it keeps from the field's edge (--corner-inset): it stands against the first row.
    expect(strip?.[1]).toMatch(/padding-block:\s*calc\(var\(--hit-min\) \+ var\(--space-2\)\);/);
    expect(CODE).toMatch(/--corner-inset:\s*var\(--space-2\)/);
  });
});
