// Patch v0.2.0's pools (docs/classic-sets.md B2.2, B2.6, B4.1): one format across Core, Classic
// and Classic+ (R380), the Fruit pool's Grapes and Dropshipping's "including tokens" (R382), and a
// card that never generates itself, named by its id because an index repeats across sets (R387).
// Fixture definitions only (CLAUDE.md: the engine does not depend on packages/cards).

import type { CardDef, CardDefs, CardFace, SetName, Tag } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";
import {
  defByIndex,
  excludingDefId,
  fusedIdParts,
  query,
  registerCatalog,
  selfDefIds,
} from "../src/catalog";

const FACE: CardFace = { keywords: [], text: "fixture" };

function card(id: string, set: SetName, index: string, overrides: Partial<CardDef> = {}): CardDef {
  return {
    id,
    index,
    name: `Pool fixture ${id}`,
    set,
    type: "Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: FACE,
    radiant: FACE,
    ...overrides,
  };
}

function token(id: string, set: SetName, index: string, tags: Tag[]): CardDef {
  return card(id, set, index, { tags: [...tags, "Token"], rarity: "Token", token: true });
}

/** Three sets that reuse the index "43", a Fruit card in each, and tokens of both kinds. */
const DEFS: CardDef[] = [
  card("core-043", "Core", "43"),
  card("core-047", "Core", "47", { tags: ["Fruit"] }),
  card("classic-043", "Classic", "43"),
  card("classic-010", "Classic", "10"),
  card("classicplus-043", "Classic+", "43"),
  card("classicplus-058", "Classic+", "58", { tags: ["Fruit"] }),
  card("classicplus-065", "Classic+", "65", { tags: ["Fruit"] }),
  token("classicplus-065-1", "Classic+", "65.1", ["Fruit"]),
  token("classicplus-065-2", "Classic+", "65.2", ["Fruit"]),
  token("core-065-1", "Core", "65.1", []),
  token("core-t-rush", "Core", "T-rush", []),
];

function catalogOf(defs: readonly CardDef[]): CardDefs {
  return Object.fromEntries(defs.map((def) => [def.id, def]));
}

const ids = (defs: readonly CardDef[]): string[] => defs.map((def) => def.id);

beforeEach(() => {
  // Registered in reverse, so the order `query` returns cannot be the registry's insertion order.
  registerCatalog(catalogOf([...DEFS].reverse()));
});

describe("R380: one format, and a pool that names no set draws from every set", () => {
  it("R380 a plain query reaches Core, Classic and Classic+, set by set in SET_ORDER, then by index", () => {
    expect(ids(query({}))).toEqual([
      "core-043",
      "core-047",
      "classic-010",
      "classic-043",
      "classicplus-043",
      "classicplus-058",
      "classicplus-065",
    ]);
  });

  it("R380 a pool that names its set keeps to it, and a list of sets reads as 'or'", () => {
    expect(ids(query({ set: "Core" }))).toEqual(["core-043", "core-047"]);
    expect(ids(query({ set: ["Classic", "Classic+"] }))).toEqual([
      "classic-010",
      "classic-043",
      "classicplus-043",
      "classicplus-058",
      "classicplus-065",
    ]);
  });

  it("R380 B2.2: an index names a card only within its set, so '43' never finds two cards", () => {
    expect(defByIndex("Core", "43")?.id).toBe("core-043");
    expect(defByIndex("Classic", "43")?.id).toBe("classic-043");
    expect(defByIndex("Classic+", "43")?.id).toBe("classicplus-043");
    // Core's #65.1 and Classic+'s #65.1 share an index and are two different tokens.
    expect(defByIndex("Core", "65.1")?.id).toBe("core-065-1");
    expect(defByIndex("Classic+", "65.1")?.id).toBe("classicplus-065-1");
    expect(defByIndex("Classic", "65.1")).toBeUndefined();
  });
});

describe("R382: the Fruit pool holds the Grapes, and only a pool that takes every token reaches others", () => {
  it("R382 a Fruit pool is the non-token Fruit cards of every set plus the Fruit tokens (the Grapes)", () => {
    expect(ids(query({ tags: ["Fruit"] }))).toEqual([
      "core-047",
      "classicplus-058",
      "classicplus-065",
      "classicplus-065-1",
      "classicplus-065-2",
    ]);
  });

  it("R382 the Grapes are in no other pool: a plain query, a type pool and a set pool leave them out", () => {
    expect(ids(query({}))).not.toContain("classicplus-065-1");
    expect(ids(query({ type: "Spell" }))).not.toContain("classicplus-065-1");
    expect(ids(query({ set: "Classic+" }))).not.toContain("classicplus-065-2");
    // A token that is no Fruit stays out of the Fruit pool.
    expect(ids(query({ tags: ["Fruit"] }))).not.toContain("core-065-1");
  });

  it("R382 withTokens takes every token of every set beside the cards (Dropshipping), itself excluded", () => {
    const pool = ids(query(excludingDefId({ withTokens: true }, "classicplus-058")));
    expect(pool).toEqual([
      "core-043",
      "core-047",
      "core-065-1",
      "core-t-rush",
      "classic-010",
      "classic-043",
      "classicplus-043",
      "classicplus-065",
      "classicplus-065-1",
      "classicplus-065-2",
    ]);
    expect(pool).not.toContain("classicplus-058");
  });
});

describe("R387: a card never generates itself, named by its id", () => {
  it("R387 excludeDefId removes exactly that card, not every card that shares its index", () => {
    const pool = ids(query({ excludeDefId: "classic-043" }));
    expect(pool).not.toContain("classic-043");
    expect(pool).toContain("core-043");
    expect(pool).toContain("classicplus-043");
  });

  it("R387 excludingDefId adds the running card's id to what the caller excluded, never replacing it", () => {
    expect(excludingDefId({ excludeDefId: "core-043" }, "classic-010")).toEqual({
      excludeDefId: ["core-043", "classic-010"],
    });
    // Already excluded: the query is handed back as it was.
    const asked = { excludeDefId: ["classic-010"] };
    expect(excludingDefId(asked, "classic-010")).toBe(asked);
    expect(excludingDefId(asked, undefined)).toBe(asked);
  });

  it("R387 a fused card excludes every ingredient's definition, a fused ingredient's included", () => {
    expect(fusedIdParts("t-2:(t-1:classic-043+core-047)+classicplus-058")).toEqual([
      "t-1:classic-043+core-047",
      "classicplus-058",
    ]);
    expect(selfDefIds("t-2:(t-1:classic-043+core-047)+classicplus-058")).toEqual([
      "classic-043",
      "core-047",
      "classicplus-058",
    ]);
    const pool = ids(query(excludingDefId({}, "t-2:(t-1:classic-043+core-047)+classicplus-058")));
    expect(pool).toEqual(["core-043", "classic-010", "classicplus-043", "classicplus-065"]);
    // A catalog card stands for itself alone; a bare crafted id has no ingredients.
    expect(selfDefIds("classic-043")).toEqual(["classic-043"]);
    expect(fusedIdParts("t-3")).toBeNull();
  });
});
