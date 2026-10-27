// R2095000: the Both boards view's styles, read off the stylesheet as `board-ux.test.tsx` reads
// `board.css`. No JavaScript width number (CLAUDE.md rule 9): the narrow fallback is a media query
// at the Duos phone layout's own breakpoint.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

function css(): string {
  return readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", "duos", "duos.css"), "utf8");
}

describe("R2095000 both boards styles", () => {
  it("R2095000 duos.css hides the Both toggle and falls back to one board below the phone breakpoint", () => {
    const text = css();

    // Wide: the two boards lay out as a row, the teammate's narrower so it draws smaller.
    expect(text).toMatch(/\.duos-both\s*\{\s*display:\s*flex;/);
    expect(text).toMatch(/\.duos-board-teammate\s*\{[^}]*flex:\s*0 0 40%;/);

    // Narrow: at the Duos phone layout's breakpoint the toggle hides and only the own board shows.
    const narrow = text.slice(text.indexOf("@media (max-width: 600px)"));
    expect(narrow).toMatch(/\.duos-both-toggle\s*\{\s*display:\s*none;/);
    expect(narrow).toMatch(/\.duos-board\[data-both="true"\] \.duos-board-teammate\s*\{\s*display:\s*none;/);
    expect(narrow).toMatch(/\.duos-both\s*\{\s*display:\s*block;/);
  });
});
