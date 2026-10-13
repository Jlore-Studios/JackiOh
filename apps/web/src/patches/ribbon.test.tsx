// R1383: the newest set's "New" ribbon. The window is counted in patches.json's own order, never from
// a version string or a date, so a fixture log of five patches proves it without the real history;
// the real history proves it against what shipped (Classic+ in v0.2.0).

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import PoolGrid from "../game/deckbuilder/PoolGrid.tsx";
import { poolCardId } from "../game/deckbuilder/testids.ts";
import { NEW_RIBBON_PATCHES, NEW_RIBBON_SET, PATCH_LOG, ribbonSet, type RibbonPatch } from "./ribbon.ts";

afterEach(cleanup);

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

const patch = (version: string, ...added: string[]): RibbonPatch => ({
  version,
  changes: added.map((id) => ({ id, kind: "added" })),
});

/** Five patches whose versions are in no sorted order: only the file's order counts. */
const LOG: readonly RibbonPatch[] = [patch("a", "core-001"), patch("b"), patch("c", "meditative-027"), patch("d"), patch("e")];

describe("R1383 the window of the New ribbon", () => {
  it("R1383 the window is the patch that ships the set and the one after", () => {
    expect(NEW_RIBBON_PATCHES).toBe(2);
    const wears = ["a", "b", "c", "d", "e", "zzz"].map((version) => ribbonSet(LOG, version, CATALOG, "Meditative"));
    expect(wears).toEqual([null, null, "Meditative", "Meditative", null, null]);
    // A change that is not an addition does not ship a set, and the newest set is the one asked for.
    expect(ribbonSet([{ version: "c", changes: [{ id: "meditative-027", kind: "changed" }] }], "c", CATALOG, "Meditative")).toBeNull();
    expect(ribbonSet(LOG, "c", CATALOG, "Classic")).toBeNull();
  });

  it("R1383 on the real history Classic+ wore it in v0.2.0 and the next patch only", () => {
    const at = PATCH_LOG.findIndex((entry) => entry.version === "v0.2.0");
    expect(at).toBeGreaterThan(0);
    const wear = (offset: number): string | null => ribbonSet(PATCH_LOG, PATCH_LOG[at + offset]?.version ?? "", CATALOG, "Classic+");
    expect([wear(-1), wear(0), wear(1), wear(2)]).toEqual([null, "Classic+", "Classic+", null]);
    // No set is in its window in this build, and the constant is the same answer.
    expect(ribbonSet(PATCH_LOG, CATALOG_VERSION, CATALOG, "Classic+")).toBeNull();
    expect(NEW_RIBBON_SET).toBe(ribbonSet(PATCH_LOG, CATALOG_VERSION, CATALOG));
  });
});

describe("R1383 the ribbon on a pool card", () => {
  const catalog: CatalogSnapshot = { version: "ribbon-test", cards: { "meditative-027": def("meditative-027"), "core-001": def("core-001") } };
  const ids = ["meditative-027", "core-001"];

  it("R1383 a card of the ribbon's set wears New and says so", () => {
    render(<PoolGrid ids={ids} catalog={catalog} onInspect={() => {}} ribbon="Meditative" />);
    const fresh = screen.getByTestId(poolCardId("meditative-027"));
    expect(fresh.querySelector(".db-new")).toHaveTextContent("New");
    expect(fresh.getAttribute("aria-label")).toMatch(/, new\. Show details$/u);
    const other = screen.getByTestId(poolCardId("core-001"));
    expect(other.querySelector(".db-new")).toBeNull();
    expect(other.getAttribute("aria-label")).not.toMatch(/new/iu);
  });

  it("R1383 with no ribbon set no card wears it", () => {
    render(<PoolGrid ids={ids} catalog={catalog} onInspect={() => {}} ribbon={null} />);
    for (const id of ids) {
      expect(screen.getByTestId(poolCardId(id)).querySelector(".db-new"), id).toBeNull();
      expect(screen.getByTestId(poolCardId(id)).getAttribute("aria-label"), id).not.toMatch(/, new/u);
    }
  });
});
