// R2095000: the Both boards view's styles, read off the stylesheet. No JavaScript width number
// (CLAUDE.md rule 9): the narrow fallback is a media query at the Duos phone layout's breakpoint.

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

    expect(text).toMatch(/\.duos-both\s*\{\s*display:\s*flex;/);
    expect(text).toMatch(/\.duos-board-teammate\s*\{[^}]*flex:\s*0 0 40%;/);

    // Wide: the own board carries the rail, so the portal is hidden (still mounted, as the narrow
    // fallback's flip control).
    const wideHide = /\.duos-board\[data-both="true"\]\[data-showing="own"\] \.duos-portal\s*\{\s*display:\s*none;/;
    expect(text).toMatch(wideHide);

    // Narrow: the portal returns under the same selector (winning by source order), and only the
    // showing board, which carries the rail (DuosBoard.test.tsx), draws.
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
