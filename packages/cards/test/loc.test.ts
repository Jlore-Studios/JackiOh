// E36: every card with a script carries `loc`, the non-blank, non-comment lines of its script file
// with the import declarations left out, written into the catalog by `scripts/gen-loc.ts` (part of
// `pnpm --filter @jackioh/cards gen`) and held current here, as `_generated.ts` is by the barrel.
// It is card data (Classic #48 Hired Shrimp and Classic+ #44/#45 read it) and so part of a card's
// patch history (B4.2, R631): a moved `loc` is a catalog change for a pending fragment to claim,
// and only the promotion snapshots it — shipped snapshots are never amended.

import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_IDS, CATALOG_VERSION } from "../src/catalog-data";
import { countLoc, expectedLocs } from "../scripts/gen-loc";
import { readFragments, readSnapshot, revertPending, type Catalog } from "../scripts/patches-io";

const REGENERATE =
  "run `pnpm --filter @jackioh/cards gen`, commit packages/cards/catalog.json, and claim the moved cards with a pending fragment (`pnpm --filter @jackioh/cards patches <version> \"<title>\"`)";

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

  it("keeps the newest snapshot's loc equal to the catalog's, pending fragments aside (B4.2, R631)", () => {
    // A pending fragment may hold the catalog ahead of the newest snapshot on its claimed cards,
    // `loc` included: reverted to the snapshot, the two agree entry for entry.
    const claimed = new Set(readFragments().flatMap(({ fragment }) => fragment.cards));
    const catalog = CATALOG as unknown as Catalog;
    const snapshot = readSnapshot(CATALOG_VERSION);
    const reverted = revertPending(catalog, snapshot, claimed);
    const drift = CATALOG_IDS.filter((id) => JSON.stringify(reverted[id]?.["loc"]) !== JSON.stringify(snapshot[id]?.["loc"]));
    expect(drift, REGENERATE).toEqual([]);
  });
});
