// E36: every card with a script carries `loc`, the non-blank, non-comment lines of its script file
// with the import declarations left out, written into the catalog by `scripts/gen-loc.ts` (part of
// `pnpm --filter @jackioh/cards gen`) and held current here, as `_generated.ts` is by the barrel.
// It is card data (Classic #48 Hired Shrimp and Classic+ #44/#45 read it) and so part of a card's
// patch history (B4.2): the current patch's snapshot carries the same numbers.

import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_IDS, CATALOG_VERSION } from "../src/catalog-data";
import { countLoc, expectedLocs } from "../scripts/gen-loc";
import { readSnapshot } from "../scripts/patches-io";

const REGENERATE = "run `pnpm --filter @jackioh/cards gen` and commit packages/cards/catalog.json and its patch snapshot";

describe("E36 lines of code (docs/classic-sets.md B5, B4.2)", () => {
  it("counts code lines only: blank lines, comments and imports are not code", () => {
    const source = [
      "// A header comment.",
      "/**",
      " * A doc comment, // with slashes inside.",
      " */",
      'import type { Script } from "@jackioh/engine";',
      "import {",
      "  destroy,",
      "  heal,",
      '} from "@jackioh/engine/effects";',
      "",
      'export const def = cardDef("core-043"); // a trailing comment',
      "export const base: Script = {",
      '  cry: () => [heal({ amount: 2, url: "https://example.invalid/" })],',
      "  /* an inline block */ death: () => [],",
      "};",
      "export const radiant = base;",
    ].join("\n");
    expect(countLoc(source)).toBe(6);
  });

  it("reads a template literal that runs over lines as code, and a string's // as part of it", () => {
    expect(countLoc("const a = `one\n// still the string\ntwo`;\n")).toBe(3);
    expect(countLoc('const url = "http://x";\n')).toBe(1);
  });

  it("gives every scripted card its current loc, and no loc to a card without a script", () => {
    const expected = expectedLocs(CATALOG_IDS);
    const stale = CATALOG_IDS.filter((id) => CATALOG[id]?.loc !== expected.get(id)).map(
      (id) => `${id}: catalog ${String(CATALOG[id]?.loc)}, script ${String(expected.get(id))}`,
    );
    expect(stale, REGENERATE).toEqual([]);
    // Every Core card has had its script since M4-T4.
    expect(CATALOG_IDS.filter((id) => id.startsWith("core-") && expected.get(id) === undefined)).toEqual([]);
  });

  it("keeps the current patch's snapshot's loc equal to the catalog's (B4.2)", () => {
    const snapshot = readSnapshot(CATALOG_VERSION);
    const drift = CATALOG_IDS.filter((id) => snapshot[id]?.["loc"] !== CATALOG[id]?.loc);
    expect(drift, REGENERATE).toEqual([]);
  });
});
