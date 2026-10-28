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

    // Wide: while both boards show the own board, which carries the rail, the portal is hidden
    // (it stays mounted as the narrow fallback's flip control).
    const wideHide = /\.duos-board\[data-both="true"\]\[data-showing="own"\] \.duos-portal\s*\{\s*display:\s*none;/;
    expect(text).toMatch(wideHide);

    // Narrow: at the Duos phone layout's breakpoint the toggle hides, the portal comes back
    // under the same selector (so it wins by source order), and only the showing board, which
    // carries the rail (DuosBoard.test.tsx), draws.
    const narrow = text.slice(text.indexOf("@media (max-width: 600px)"));
    expect(text.search(wideHide)).toBeLessThan(text.indexOf("@media (max-width: 600px)"));
    expect(narrow).toMatch(/\.duos-both-toggle\s*\{\s*display:\s*none;/);
    expect(narrow).toMatch(
      /\.duos-board\[data-both="true"\]\[data-showing="own"\] \.duos-portal\s*\{\s*display:\s*revert;/,
    );
    expect(narrow).toMatch(
      /\.duos-board\[data-both="true"\]\[data-showing="own"\] \.duos-board-teammate\s*\{\s*display:\s*none;/,
    );
    expect(narrow).toMatch(
      /\.duos-board\[data-both="true"\]\[data-showing="teammate"\] \.duos-board-own\s*\{\s*display:\s*none;/,
    );
    // `.duos-both` stays a flex row (a block parent has no definite height, so the size-container
    // board would collapse) and the teammate wrapper, shown alone, takes the row's room.
    expect(narrow).not.toMatch(/\.duos-both\s*\{\s*display:\s*block;/);
    expect(narrow).toMatch(
      /\.duos-board\[data-both="true"\]\[data-showing="teammate"\] \.duos-board-teammate\s*\{\s*flex:\s*1 1 0;/,
    );
  });
});
