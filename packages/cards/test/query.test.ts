// BUILD M4-T2 acceptance for SPEC §5.1's one catalog query, and the reference every card agent
// copies when its card needs a random pool or a Discover.
//
// The pools below are written as explicit id lists, so the test is the diff: if the filter or the
// catalog drifts, the failure names the exact card that appeared or vanished. An index repeats across
// sets since patch v0.2.0 ("43" is a Core, a Classic and a Classic+ card, B2.2), so pools are named by
// id; and a pool that names no set draws from every set (R380, B2.6), in catalog order: Core, then
// Classic, then Classic+, each by §5 index. Each pool test also states, in words and in code, the
// argument object the card script must pass — that is the part the card files are going to copy.
//
// `query` reads the *registered* catalog (`packages/engine/src/catalog.ts`), which in a real game is
// registered by `registerAll()` in `src/index.ts`; here we register the catalog data directly so
// this file does not pull in the script registry.

import { registerCatalog } from "@jackioh/engine";
import type { CardDef } from "@jackioh/shared";
import { beforeAll, describe, expect, it } from "vitest";

import { CATALOG, CATALOG_VERSION, cardDefByIndex } from "../src/catalog-data";
import { TRAP_TYPES, catalog, pool, query, queryCost } from "../src/query";

/** Results are compared by catalog id: an index names a card only within its set (B2.2). */
function ids(defs: readonly CardDef[]): string[] {
  return defs.map((def) => def.id);
}

/** Core's own pools, by §5 index, for the checks §8 writes by number. */
function coreIndices(defs: readonly CardDef[]): string[] {
  return defs.filter((def) => def.set === "Core").map((def) => def.index);
}

/** Every token of every set (B2.1's census itself is catalog.test.ts's to prove). */
const TOKEN_IDS = Object.values(CATALOG)
  .filter((def) => def.token)
  .map((def) => def.id);

/** Every non-token def, in catalog order — the pool a plain query answers (R380). */
const NON_TOKEN = Object.values(CATALOG).filter((def) => !def.token);

beforeAll(() => {
  registerCatalog(CATALOG, CATALOG_VERSION);
});

describe("the cards-layer surface (§5.1: one query function)", () => {
  it("exposes query, pool, cost and trapTypes on `catalog`, the object card scripts call", () => {
    // src/index.ts re-exports `catalog`, `query` and the `CardQuery` type; everything a card script
    // needs therefore has to be reachable through `catalog`.
    expect(catalog.query).toBe(query);
    expect(catalog.pool).toBe(pool);
    expect(catalog.cost).toBe(queryCost);
    expect(catalog.trapTypes).toBe(TRAP_TYPES);
  });

  it("R35, R61 'Field Trap counts as Trap', so TRAP_TYPES names both types", () => {
    expect(TRAP_TYPES).toEqual(["Trap", "Field Trap"]);
  });
});

describe("§5.1 tokens are out of every pool unless the card names the token pool", () => {
  it("R380 a plain query({}) is every set's non-token cards: no def.token, no Token tag, no Token rarity", () => {
    const all = query({});

    // Every non-token def of every set, in catalog order — the census is catalog.test.ts's to prove.
    expect(ids(all)).toEqual(ids(NON_TOKEN));
    expect(all.filter((def) => def.token)).toEqual([]);
    expect(all.filter((def) => def.tags.includes("Token"))).toEqual([]);
    expect(all.filter((def) => def.rarity === "Token")).toEqual([]);
    // And named outright, because "never a token" is the rule cards depend on (BUILD M4-T4 row 51.1
    // wants #51.1 "absent from every random pool"):
    expect(TOKEN_IDS.length).toBeGreaterThan(0);
    for (const id of TOKEN_IDS) {
      expect(ids(all)).not.toContain(id);
    }
  });

  it("§5.1 query({ tags: ['Token'] }) does return them — the card named the pool itself", () => {
    expect(ids(query({ tags: ["Token"] })).sort()).toEqual([...TOKEN_IDS].sort());
  });

  it("§5.1 token: true and a pool named by id reach tokens too; token: false forbids them", () => {
    expect(ids(query({ token: true })).sort()).toEqual([...TOKEN_IDS].sort());
    // A card that names its token by id (Rush Token, Sheep Token, …) names the pool itself.
    expect(ids(query({ defId: ["core-t-rush", "core-t-sheep"] })).sort()).toEqual(["core-t-rush", "core-t-sheep"]);
    expect(ids(query({ token: false, tags: ["Token"] }))).toEqual([]);
  });

  it("R380 a pool that names a set keeps to it (Core #82 KY's Trial, #97 Zephyrs)", () => {
    expect(ids(query({ set: "Core" }))).toEqual(ids(NON_TOKEN.filter((def) => def.set === "Core")));
    expect(ids(query({ set: "Classic", tags: ["Book"] }))).toEqual([
      "classic-003",
      "classic-012",
      "classic-016",
      "classic-024",
      "classic-029",
      "classic-055",
      "classic-070",
    ]);
  });
});

describe("§5.1, R387 excludeDefId: a random pool never offers the card that generated it", () => {
  it("§5.1 excludeDefId removes a single card", () => {
    // #57 Conjure KY carries the KY tag itself, so without excludeDefId it is in its own pool.
    expect(coreIndices(query({ tags: ["KY"] }))).toEqual(["31", "51", "57", "82"]);
    expect(coreIndices(query({ tags: ["KY"], excludeDefId: "core-057" }))).toEqual(["31", "51", "82"]);
    // By id, never by index: Classic+ #57 Book of Stats is not Core #57, and stays in its pools.
    expect(ids(query({ tags: ["Book"], excludeDefId: "core-057" }))).toContain("classicplus-057");
  });

  it("§5.1 excludeDefId removes every card in a list", () => {
    expect(coreIndices(query({ tags: ["KY"], excludeDefId: ["core-057", "core-082"] }))).toEqual(["31", "51"]);
  });

  it("§5.1 excludeDefId composes with the type, rarity and cost filters", () => {
    expect(coreIndices(query({ type: TRAP_TYPES, excludeDefId: "core-018" }))).toEqual(["41", "60", "71", "85", "96"]);
    expect(ids(query({ rarity: "Mythic", excludeDefId: "core-096" }))).not.toContain("core-096");
    expect(ids(query({ cost: 1, tags: ["KY"], excludeDefId: "core-031" }))).toEqual([
      "core-051",
      "core-082",
      "classicplus-041",
      "classicplus-042",
      "classicplus-062",
    ]);
  });

  it("§5.1 pool(ownId, args) adds the caller's id to excludeDefId instead of replacing it", () => {
    expect(coreIndices(pool("core-057", { tags: ["KY"] }))).toEqual(["31", "51", "82"]);
    expect(coreIndices(pool("core-082", { tags: ["KY"], excludeDefId: "core-057" }))).toEqual(["31", "51"]);
  });
});

describe("the KY pool (#57 Conjure KY): Core #31, #51, #82 and Classic+ #41, #42, #62 (BUILD M4-T4 row 57, B2.6)", () => {
  // THE ARGUMENTS A CARD SCRIPT PASSES:
  //     catalog.query({ tags: ["KY"], excludeDefId: "core-057" })
  //   or, equivalently and harder to get wrong,
  //     catalog.pool("core-057", { tags: ["KY"] })
  // Nothing else is needed: tokens are excluded by default (§5.1), which is what keeps #51.1 KY's
  // Empty Notebook and Classic+ #42.1 KY's Gift — tokens that carry the KY tag — out of the pool;
  // and a pool that names no set reaches every set (R380), so the Classic+ KY cards join it.
  const KY_POOL = ["core-031", "core-051", "core-082", "classicplus-041", "classicplus-042", "classicplus-062"];

  it("BUILD row 57 the query a card script writes returns exactly those six defs", () => {
    expect(ids(catalog.query({ tags: ["KY"], excludeDefId: "core-057" }))).toEqual(KY_POOL);
    expect(ids(catalog.pool("core-057", { tags: ["KY"] }))).toEqual(KY_POOL);
  });

  it("BUILD row 51.1 the KY pool excludes the KY-tagged tokens #51.1 and Classic+ #42.1, and the generator #57", () => {
    const names = catalog.pool("core-057", { tags: ["KY"] }).map((def) => def.name);

    expect(names).toEqual(["KY's Math Equation", "KY's Private Tutor", "KY's Trial", "KY's Constant", "KY's Test", "KY's Papaya"]);
    expect(cardDefByIndex("Core", "51.1").tags).toContain("KY"); // it really is in the tag …
    expect(cardDefByIndex("Core", "51.1").token).toBe(true); // … and it really is a token
    expect(cardDefByIndex("Classic+", "42.1").tags).toContain("KY");
    expect(ids(catalog.pool("core-057", { tags: ["KY"] }))).not.toContain("core-051-1");
    expect(ids(catalog.pool("core-057", { tags: ["KY"] }))).not.toContain("classicplus-042-1");
    expect(ids(catalog.pool("core-057", { tags: ["KY"] }))).not.toContain("core-057");
  });
});

describe("the trap pool (#67 Zoomerbin Oomen's Radiant face): every set's Traps and Field Traps", () => {
  // THE ARGUMENTS A CARD SCRIPT PASSES:
  //     catalog.query({ type: catalog.trapTypes })      // TRAP_TYPES = ["Trap", "Field Trap"]
  //   or catalog.pool("core-067", { type: TRAP_TYPES })       // #67 is not itself a trap, but §5.1 anyway
  // Both types, because SPEC says "Field Trap counts as Trap" (§8 #51, #85, R35, R61) while the
  // filter matches `def.type` exactly.
  const TRAP_POOL = [
    "core-018",
    "core-041",
    "core-060",
    "core-071",
    "core-085",
    "core-096",
    "classic-005",
    "classic-009",
    "classic-010",
    "classic-014",
    "classic-017",
    "classic-038",
    "classic-052",
    "classic-063",
    "classic-065",
    "classic-072",
    "classic-088",
    "classicplus-001",
    "classicplus-002",
    "classicplus-022",
    "classicplus-074",
  ];

  it("BUILD row 67 the query a card script writes returns exactly those defs", () => {
    expect(ids(catalog.query({ type: TRAP_TYPES }))).toEqual(TRAP_POOL);
    expect(ids(catalog.pool("core-067", { type: TRAP_TYPES }))).toEqual(TRAP_POOL);
  });

  it("§8 #67 base asks for a (1) Cost Trap: every Core trap but #85, and Classic+ #22 Blood Moon, the one new one (B2.6)", () => {
    expect(ids(catalog.query({ type: TRAP_TYPES, cost: 1 }))).toEqual([
      "core-018",
      "core-041",
      "core-060",
      "core-071",
      "core-096",
      "classicplus-022",
    ]);
  });

  it("§5.1 the pool mixes both types: Core #18, #71, Classic #5, #38, #88 and Classic+ #74 are Field Traps", () => {
    const byType = (type: string): string[] => ids(catalog.query({ type: TRAP_TYPES }).filter((def) => def.type === type));

    expect(byType("Field Trap")).toEqual(["core-018", "core-071", "classic-005", "classic-038", "classic-088", "classicplus-074"]);
    expect(coreIndices(catalog.query({ type: TRAP_TYPES }).filter((def) => def.type === "Trap"))).toEqual(["41", "60", "85", "96"]);
  });

  it("§5.1 asking for type 'Trap' alone drops the Field Traps — why TRAP_TYPES exists", () => {
    expect(ids(catalog.query({ type: "Trap" }))).toEqual(TRAP_POOL.filter((id) => CATALOG[id]?.type === "Trap"));
    expect(coreIndices(catalog.query({ type: "Trap" }))).toEqual(["41", "60", "85", "96"]);
  });
});

describe("R35 the Transmogulate pool (#83): every non-token Legendary but #83 (B2.6)", () => {
  // THE ARGUMENTS A CARD SCRIPT PASSES:
  //     catalog.query({ rarity: "Legendary", excludeDefId: "core-083" })
  //   or catalog.pool("core-083", { rarity: "Legendary" })
  // R35: "Pool: the Legendary-rarity cards except #83", which since patch v0.2.0 reaches every set;
  // patch v0.2.2's rarity pass (R651) added Classic #9 Income Tax and #28 Second Wind to it.
  // For replacing a card on the board, the script narrows the same pool by type and asks for
  // TRAP_TYPES when the board card is a trap ("Field Trap counts as Trap").
  const R35_POOL = [
    "core-052",
    "core-085",
    "core-087",
    "core-092",
    "core-093",
    "core-095",
    "classic-004",
    "classic-007",
    "classic-009",
    "classic-028",
    "classic-033",
    "classic-044",
    "classic-045",
    "classic-056",
    "classic-080",
    "classic-085",
    "classicplus-012",
    "classicplus-013",
    "classicplus-019",
    "classicplus-035",
    "classicplus-037",
    "classicplus-042",
    "classicplus-043",
    "classicplus-046",
    "classicplus-047",
    "classicplus-048",
    "classicplus-073",
    "classicplus-075",
    "classicplus-078",
  ];

  it("R35 the filter a card script writes returns exactly that list", () => {
    expect(ids(catalog.query({ rarity: "Legendary", excludeDefId: "core-083" }))).toEqual(R35_POOL);
    expect(ids(catalog.pool("core-083", { rarity: "Legendary" }))).toEqual(R35_POOL);
  });

  it("R35 the pool is the Legendary rarity set minus #83, and nothing else", () => {
    expect(coreIndices(catalog.query({ rarity: "Legendary" }))).toEqual(["52", "83", "85", "87", "92", "93", "95"]);
    expect(ids(catalog.pool("core-083", { rarity: "Legendary" }))).not.toContain("core-083");
    expect(catalog.pool("core-083", { rarity: "Legendary" }).every((def) => def.rarity === "Legendary")).toBe(true);
    expect(catalog.pool("core-083", { rarity: "Legendary" }).every((def) => !def.token)).toBe(true);
  });

  it("R35 narrowed by type for a board replacement, with Field Trap counting as Trap", () => {
    // The Legendary traps are Core #85 and, since patch v0.2.2, Classic #9 Income Tax, so a board
    // trap — Trap or Field Trap — is replaced by one of them.
    expect(ids(catalog.pool("core-083", { rarity: "Legendary", type: TRAP_TYPES }))).toEqual(["core-085", "classic-009"]);
    expect(coreIndices(catalog.pool("core-083", { rarity: "Legendary", type: "Unit" }))).toEqual(["52", "92"]);
  });
});

describe("R65 pools and filters read a definition's cost out of play", () => {
  it("R65 an X-cost card's queryCost is 0 and it answers a cost-0 query", () => {
    // Core #24 Efficiency Dividend, #74 Adaptive UI, #98 Heroic Power, Classic #87 Plague Chalice,
    // Classic+ #40 Appropriations and #69 Buff Billy are the catalog's X-cost cards.
    const xCards = ["core-024", "core-074", "core-098", "classic-087", "classicplus-040", "classicplus-069"];
    expect(ids(query({}).filter((def) => def.cost === "X"))).toEqual(xCards);
    const cost0 = ids(query({ cost: 0 }));
    for (const id of xCards) {
      expect(queryCost(CATALOG[id] as CardDef)).toBe(0);
      expect(cost0).toContain(id);
    }
  });

  it("R65 an embiggen card's queryCost is its base price, and it answers that cost's query", () => {
    // Core #46 Suppressive Aura, #59 Unbiased Immigration, #84 Going Long and Classic+ #36 Conjure
    // Bones are "2 embiggen 4".
    for (const id of ["core-046", "core-059", "core-084", "classicplus-036"]) {
      const def = CATALOG[id] as CardDef;
      expect(def.cost).toEqual({ base: 2, embiggen: 4 });
      expect(queryCost(def)).toBe(2);
      expect(ids(query({ cost: 2 }))).toContain(id);
      expect(ids(query({ cost: 4 }))).not.toContain(id);
    }
  });

  it("R65 costRange reads the same number, so a bracket agrees with queryCost (#7, #51)", () => {
    // #7 Jewelosco Scarab's "2-cost" Discover and #51's 0-1 / 2 / 3 / 4+ brackets share this read.
    const bracket0to1 = query({ costRange: { min: 0, max: 1 } });
    expect(ids(bracket0to1)).toEqual(ids(query({}).filter((def) => queryCost(def) <= 1)));
    expect(ids(bracket0to1)).toContain("core-024"); // an X card reads as 0
    expect(ids(query({ costRange: { min: 4 } }))).not.toContain("core-046"); // an embiggen card reads as 2
  });
});

describe("§9.3, R60 pool order is deterministic, so a seeded pick replays", () => {
  it("§9.3 two identical calls return the same defs in the same order", () => {
    const first = query({ type: "Unit", costRange: { min: 2, max: 3 } });
    const second = query({ type: "Unit", costRange: { min: 2, max: 3 } });

    expect(first.map((def) => def.id)).toEqual(second.map((def) => def.id));
    expect(first.length).toBeGreaterThan(1);
  });

  it("R60 a multi-result pool comes back in catalog order: set (Core, Classic, Classic+), then §5 index, ascending", () => {
    const units = query({ type: "Unit" });
    const SET_RANK: Record<string, number> = { Core: 0, Classic: 1, "Classic+": 2 };
    const keys = units.map((def) => [SET_RANK[def.set] ?? 3, Number.parseFloat(def.index)] as const);

    expect(keys.length).toBeGreaterThan(1);
    expect(keys).toEqual([...keys].sort((a, b) => a[0] - b[0] || a[1] - b[1]));
    // Spot-checked against the catalog so "ascending" cannot be satisfied by an empty result:
    expect(ids(query({ rarity: "Legendary" }))[0]).toBe("core-052");
    expect(ids(units).indexOf("classic-002")).toBeGreaterThan(ids(units).indexOf("core-100"));
  });

  it("§9.3 the order does not depend on the order the registry handed the defs over", () => {
    const forward = query({}).map((def) => def.id);
    const reversedCatalog = Object.fromEntries(Object.entries(CATALOG).reverse());

    try {
      registerCatalog(reversedCatalog, CATALOG_VERSION);
      expect(query({}).map((def) => def.id)).toEqual(forward);
      expect(coreIndices(query({ tags: ["KY"], excludeDefId: "core-057" }))).toEqual(["31", "51", "82"]);
    } finally {
      registerCatalog(CATALOG, CATALOG_VERSION);
    }
  });

  // ENGINE DEFECT, reported to the lead and NOT worked around in src/query.ts.
  // §9.3, R60: the pool order is "a strict total order … never depends on the order the registry
  // happened to hand the defs over", which is what makes a seeded `rng.pick`/`shuffle` over a pool
  // replay identically. This was pinned as `it.fails` while the engine's comparator subtracted two
  // `indexRank`s: both are +Infinity for a non-numeric index, `Infinity - Infinity` is NaN, and
  // `NaN !== 0` returned NaN, which `sort` reads as 0 — so both tie-breaks were skipped and the
  // four shared tokens came back in insertion order. The comparator now compares instead of
  // subtracting, so this is a plain `it`; registering the catalog reversed is the regression guard.
  it("§9.3 the token pool's order is insertion-independent", () => {
    const forward = ids(query({ tags: ["Token"] }));
    const reversedCatalog = Object.fromEntries(Object.entries(CATALOG).reverse());

    try {
      registerCatalog(reversedCatalog, CATALOG_VERSION);
      expect(ids(query({ tags: ["Token"] }))).toEqual(forward);
    } finally {
      registerCatalog(CATALOG, CATALOG_VERSION);
    }
  });
});

describe("R382 the Fruit pool holds the five Grapes; a pool that takes every token takes them all (B2.6)", () => {
  const GRAPES = ["classicplus-065-1", "classicplus-065-2", "classicplus-065-3", "classicplus-065-4", "classicplus-065-5"];

  it("R382 a Fruit pool is every non-token Fruit card and the five Grapes, the generating card excluded", () => {
    // Core #47 Fig of Life, Classic+'s Fruit cards and the Grapes (each after the card that defines
    // it, in catalog order); Classic+ #58 Fruit Basket asks for its pool.
    const expected = Object.values(CATALOG)
      .filter((def) => def.tags.includes("Fruit") && (!def.token || GRAPES.includes(def.id)))
      .map((def) => def.id)
      .filter((id) => id !== "classicplus-058");
    expect(ids(pool("classicplus-058", { tags: ["Fruit"] }))).toEqual(expected);
    expect(expected.filter((id) => CATALOG[id]?.token)).toEqual(GRAPES);
    expect(ids(query({ tags: ["Fruit"] }))).toContain("core-047");
  });

  it("R382 no other pool reaches a Grape: not a plain pool, not a Spell pool, not a rarity pool", () => {
    for (const args of [{}, { type: "Spell" as const }, { rarity: "Common" as const }, { cost: 1 }, { tags: ["KY" as const] }]) {
      const got = ids(query(args));
      expect(GRAPES.filter((id) => got.includes(id)), JSON.stringify(args)).toEqual([]);
    }
  });

  it("R382 Classic+ #23 Dropshipping's pool takes every card and every token of every set but itself", () => {
    const every = ids(pool("classicplus-023", { withTokens: true }));
    expect(every).toHaveLength(Object.keys(CATALOG).length - 1);
    expect(every).not.toContain("classicplus-023");
    for (const id of [...GRAPES, "classicplus-019-3", "classicplus-t-ai-01", "core-t-coin", "core-051-1"]) {
      expect(every).toContain(id);
    }
  });
});
