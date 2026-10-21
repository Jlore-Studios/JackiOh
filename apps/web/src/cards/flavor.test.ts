// R1437000: a definition with no flavor draws Physical, and a fused one its first ingredient's.

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { CATALOG } from "../wire/cards.ts";
import { FLAVOR_FALLBACK, flavorOf, flavorOfId } from "./flavor.ts";

/** A fused definition: it carries no flavor of its own, only the ingredients it was made of. */
function fused(...ingredients: string[]): Pick<CardDef, "flavor" | "ingredients"> {
  return { ingredients: ingredients.map((defId) => ({ defId })) };
}

describe("R1437000 which flavor a definition draws", () => {
  it("R1437000 a def with no flavor draws Physical", () => {
    expect(FLAVOR_FALLBACK).toBe("Physical");
    expect(flavorOf(undefined)).toBe("Physical");
    expect(flavorOf({})).toBe("Physical");
    // A hidden card, and an id no catalog has (a crafted or custom card the view brought no definition for).
    expect(flavorOfId("hidden")).toBe("Physical");
    expect(flavorOfId("t-404")).toBe("Physical");
  });

  it("R1437000 a Fused def draws its first ingredient's catalog flavor", () => {
    expect(CATALOG["core-019"]?.flavor).toBe("Brute");
    expect(CATALOG["core-022"]?.flavor).toBe("Toxic");
    expect(flavorOf(fused("core-019", "core-022"))).toBe("Brute");
    expect(flavorOf(fused("core-022", "core-019"))).toBe("Toxic");
    // An ingredient the catalog does not know gives nothing, so the fallback.
    expect(flavorOf(fused("t-9", "core-019"))).toBe("Physical");
    expect(flavorOf(fused())).toBe("Physical");
  });

  it("R1437000 a def's own flavor wins over its ingredients", () => {
    expect(flavorOf({ flavor: "Holy", ...fused("core-019") })).toBe("Holy");
  });

  it("R1437000 flavorOfId reads the catalog, then the match's own definitions", () => {
    expect(flavorOfId("core-019")).toBe("Brute");
    const made: CardDef = { ...CATALOG["core-001"]!, id: "t-1", flavor: undefined, ingredients: [{ defId: "core-019" }] };
    expect(flavorOfId("t-1")).toBe("Physical");
    expect(flavorOfId("t-1", { "t-1": made })).toBe("Brute");
    // The catalog is asked first, so a match's definition never shadows a catalog id.
    expect(flavorOfId("core-019", { "core-019": made })).toBe("Brute");
  });
});
