// BUILD M4-T2 acceptance: the registry test. It proves the contract 110 card files and the test
// harness are written against — that `CARDS` is keyed by catalog id, that every `def` is the
// catalog's own object, that `registerAll()` hands the engine the whole catalog and the scripts,
// and that a card with no script file yet degrades to `EMPTY_SCRIPT` instead of crashing.
//
// It also owns the unit tests for `scripts/naming.ts`, the id <-> filename convention that
// `gen-registry.ts` (which files exist) and `missing-tests.ts` (which tests are missing) share.

import { describe, expect, it } from "vitest";
import {
  EMPTY_SCRIPT,
  catalogVersion,
  createGame,
  newInstance,
  registerCatalog,
  registerScripts,
  registeredCatalog,
  scriptOf,
  scriptsFor,
} from "@jackioh/engine";
import {
  CARDS,
  CATALOG,
  CATALOG_IDS,
  CATALOG_VERSION,
  buildRegistry,
  cardDef,
  registerAll,
  scriptsOf,
  type CardModule,
} from "../src/index";
import { SCRIPT_MODULES } from "../src/scripts/_generated";
import {
  compareSortKeys,
  expectedBasename,
  expectedRelPath,
  folderOf,
  matchesCard,
  moduleAliasOf,
  prefixRank,
  resolveBasename,
  resolveRelPath,
  setSegmentOf,
  slugPrefixOf,
  slugify,
  sortKey,
} from "../scripts/naming";

/**
 * SPEC §8 + §7 and B2.1's census: Core's 100 cards + 11 tokens, Classic's 90 cards, Classic+'s 78
 * cards + 38 tokens. `catalog.test.ts` proves the values.
 */
const CATALOG_SIZE = 317;

/**
 * BUILD M4-T2's gate is "every catalog id has a script and every script has a catalog entry".
 *
 * The second half is strict today. The first half cannot be: BUILD M4-T4 is the task that writes
 * the 110 script files, and this test has to stay green while they land one at a time. So the ids
 * with no script are compared against an allowlist, and this flag is the whole gate:
 *
 *   M4-T4 finished -> set EXPECT_ALL_SCRIPTS = true (this one line). The allowlist becomes empty
 *   and the test then fails, by id, for every card still missing `src/scripts/NNN-slug.ts`.
 *
 * The allowlist is computed instead of transcribed on purpose: with 30 agents adding card files in
 * parallel, a hand-listed one would funnel all of them through the same line of this file.
 */
const EXPECT_ALL_SCRIPTS = false;
/**
 * Core's 111 files have all landed (M4-T4), so a Core card without a script is a regression; the
 * Classic and Classic+ scripts land in patch v0.2.0's card waves (B10.2, M9-T6 … M9-T8).
 */
const IDS_ALLOWED_WITHOUT_SCRIPT: readonly string[] = EXPECT_ALL_SCRIPTS
  ? []
  : CATALOG_IDS.filter((id) => setSegmentOf(id) !== "core");

const scriptedIds = Object.keys(CARDS);
const idsWithoutScript = CATALOG_IDS.filter((id) => CARDS[id] === undefined);
/** Any real card, for the error-path tests. #1 Big D-fender is index 1 (SPEC §8). */
const SOME_ID = CATALOG_IDS[0] ?? "core-001";
const HEROIC_POWER_ID = "core-098";
const HISTORIC_DECK = [
  HEROIC_POWER_ID,
  ...CATALOG_IDS.filter((id) => id !== HEROIC_POWER_ID && CATALOG[id]?.token !== true).slice(0, 19),
];

/** `CARDS[id]` under `noUncheckedIndexedAccess`, for an id the test already knows is present. */
function moduleFor(id: string): CardModule {
  const mod = CARDS[id];
  if (mod === undefined) throw new Error(`no script registered for "${id}"`);
  return mod;
}

describe("registry (BUILD M4-T2)", () => {
  it("keys CARDS by catalog id and takes every def from the catalog", () => {
    for (const [id, mod] of Object.entries(CARDS)) {
      expect(CATALOG_IDS, `"${id}" is not a catalog id`).toContain(id);
      // Identity, not equality: a script does `export const def = cardDef("core-043")`, so its def
      // is the object the engine and the client see. A hand-written def would pass a deep compare
      // and then drift from catalog.json.
      expect(mod.def, `${id}: def must come from cardDef("${id}")`).toBe(cardDef(id));
      expect(mod.def.id).toBe(id);
    }
  });

  it("has no duplicate def.id", () => {
    // `buildRegistry` throws on a duplicate, so reaching this line already proves it; the count
    // check is what catches two files that both compile to the same card.
    expect(scriptedIds).toHaveLength(SCRIPT_MODULES.length);
    expect(new Set(SCRIPT_MODULES.map((mod) => mod.def.id)).size).toBe(SCRIPT_MODULES.length);
  });

  it("throws when two scripts claim one card", () => {
    const mod: CardModule = { def: cardDef(SOME_ID), base: {}, radiant: {} };
    expect(() => buildRegistry([mod, mod])).toThrow(/two card scripts claim/);
  });

  it("throws when a script's def is not a catalog card", () => {
    const def = { ...cardDef(SOME_ID), id: "core-999" };
    expect(() => buildRegistry([{ def, base: {}, radiant: {} }])).toThrow(/not in/);
  });

  it("registerAll registers all 317 defs and the catalog version", () => {
    registerAll();
    const registered = registeredCatalog();
    expect(Object.keys(registered)).toHaveLength(CATALOG_SIZE);
    expect(CATALOG_IDS).toHaveLength(CATALOG_SIZE);
    for (const id of CATALOG_IDS) {
      expect(registered[id], `"${id}" reached the engine`).toBe(cardDef(id));
    }
    expect(catalogVersion()).toBe(CATALOG_VERSION);
  });

  it("registerAll is idempotent and cheap to repeat", () => {
    registerAll();
    const catalogAfterFirst = registeredCatalog();
    const scriptsAfterFirst = scriptsOf();
    registerAll();
    registerAll();
    expect(registeredCatalog()).toBe(catalogAfterFirst);
    expect(registeredCatalog()).toBe(CATALOG);
    expect(scriptsOf()).toBe(scriptsAfterFirst);
    expect(catalogVersion()).toBe(CATALOG_VERSION);
  });

  it("registerAll puts the real registries back after a fixture catalog", () => {
    registerAll();
    // An engine test registers its own fixtures (BUILD §0); the next registerAll must undo that,
    // which is why the guard compares identities instead of latching a boolean.
    registerCatalog({}, "fixture");
    registerScripts({});
    registerAll();
    expect(registeredCatalog()).toBe(CATALOG);
    expect(catalogVersion()).toBe(CATALOG_VERSION);
  });

  it("R388 selects Heroic Power's v0.2.0 script for a match pinned to that catalog", () => {
    registerAll();
    const historicGame = createGame({
      seed: "historic-hero-power",
      decks: [HISTORIC_DECK, HISTORIC_DECK],
      catalogVersion: "v0.2.0",
    });
    const historic = historicGame.players.p1.library.find((card) => card.defId === HEROIC_POWER_ID);
    if (historic === undefined) throw new Error("historic deck must contain Heroic Power");
    historic.memory.power = "draw";
    const script = scriptOf(historic);
    expect(script.cost).toBeDefined();
    expect(script.cry).toBeDefined();
    expect(historic.catalogVersion).toBe("v0.2.0");

    // A current match starting after the historic one must not change its script selection.
    const current = newInstance({ nextId: 1, catalogVersion: CATALOG_VERSION }, HEROIC_POWER_ID, "p1", {
      z: "hand",
      player: "p1",
    });
    current.memory.power = "draw";
    expect(scriptOf(current).cost).toBeUndefined();
    expect(scriptOf(current).cry).toBeUndefined();
  });

  it("gives the engine each module's own base and radiant script", () => {
    registerAll();
    for (const id of scriptedIds) {
      const mod = moduleFor(id);
      const entry = scriptsFor(id);
      expect(entry.base, `${id}: base script`).toBe(mod.base);
      expect(entry.radiant, `${id}: radiant script`).toBe(mod.radiant);
    }
  });

  it("falls back to EMPTY_SCRIPT for a catalog id with no script, so nothing crashes", () => {
    registerAll();
    // The first uncovered card while M4-T4 is in flight; once every card has a script there is no
    // such id, and an unregistered defId exercises the same engine fallback.
    const unscripted = idsWithoutScript[0] ?? "core-not-a-card";
    const entry = scriptsFor(unscripted);
    expect(entry.base).toBe(EMPTY_SCRIPT);
    expect(entry.radiant).toBe(EMPTY_SCRIPT);
  });

  it("every catalog id has a script and every script has a catalog entry", () => {
    // Strict: a script file can never name a card the catalog does not ship.
    expect(scriptedIds.filter((id) => !CATALOG_IDS.includes(id))).toEqual([]);
    // Strict once EXPECT_ALL_SCRIPTS flips with M4-T4 (see the comment on the flag).
    expect(idsWithoutScript.filter((id) => !IDS_ALLOWED_WITHOUT_SCRIPT.includes(id))).toEqual([]);
    if (EXPECT_ALL_SCRIPTS) expect(scriptedIds).toHaveLength(CATALOG_SIZE);
  });
});

describe("naming (BUILD M4-T2, M4-T3)", () => {
  it("turns a catalog id into its filename prefix", () => {
    expect(slugPrefixOf("core-001")).toBe("001");
    expect(slugPrefixOf("core-043")).toBe("043");
    expect(slugPrefixOf("core-051-1")).toBe("051-1");
    expect(slugPrefixOf("core-t-rush")).toBe("t-rush");
    expect(slugPrefixOf("classicplus-012-1")).toBe("012-1");
    expect(slugPrefixOf("classicplus-t-ai-01")).toBe("t-ai-01");
    expect(() => slugPrefixOf("core")).toThrow(/not a catalog id/);
  });

  it("files each set in its own folder (B2.2): Core at the top, classic/ and classic-plus/", () => {
    expect(setSegmentOf("core-043")).toBe("core");
    expect(setSegmentOf("classic-043")).toBe("classic");
    expect(setSegmentOf("classicplus-043")).toBe("classicplus");
    expect(setSegmentOf("boss-001")).toBeUndefined();
    expect(folderOf("core-043")).toBe("");
    expect(folderOf("classic-043")).toBe("classic");
    expect(folderOf("classicplus-012-1")).toBe("classic-plus");
    expect(expectedRelPath("core-043", cardDef("core-043").name)).toBe("043-big-felinor");
    expect(expectedRelPath("classic-043", cardDef("classic-043").name)).toBe("classic/043-plague-nuke");
    expect(expectedRelPath("classicplus-012-1", cardDef("classicplus-012-1").name)).toBe("classic-plus/012-1-devour");
    // The AI generated cards are shared tokens of Classic+, but carry their slug (their prefix alone
    // names no card), unlike Core's §7 tokens.
    expect(expectedRelPath("classicplus-t-ai-01", cardDef("classicplus-t-ai-01").name)).toBe(
      "classic-plus/t-ai-01-helpful-assistant",
    );
    expect(expectedRelPath("core-t-rush", cardDef("core-t-rush").name)).toBe("t-rush");
  });

  it("resolves a file by its folder's set, since an index repeats across sets", () => {
    expect(resolveRelPath("043-big-felinor", CATALOG_IDS)).toBe("core-043");
    expect(resolveRelPath("classic/043-plague-nuke", CATALOG_IDS)).toBe("classic-043");
    expect(resolveRelPath("classic-plus/043-ai-slop", CATALOG_IDS)).toBe("classicplus-043");
    // A misspelled slug still lands on its card, in its own set only.
    expect(resolveRelPath("classic/043-plague-nuk", CATALOG_IDS)).toBe("classic-043");
    expect(resolveRelPath("classic-plus/012-1-devour", CATALOG_IDS)).toBe("classicplus-012-1");
    expect(resolveRelPath("classic-plus/012-the-mother-pancake", CATALOG_IDS)).toBe("classicplus-012");
    expect(resolveRelPath("classic-plus/t-ai-10-fine-tuning", CATALOG_IDS)).toBe("classicplus-t-ai-10");
    expect(resolveRelPath("classic/091-no-such-card", CATALOG_IDS)).toBeUndefined();
    expect(resolveRelPath("legacy/043-big-felinor", CATALOG_IDS)).toBeUndefined();
    expect(resolveBasename("043-plague-nuke", CATALOG_IDS, "classic")).toBe("classic-043");
  });

  it("slugifies a card name the way the shipped filenames spell it", () => {
    expect(slugify("Big D-fender")).toBe("big-d-fender");
    expect(slugify("KY's Empty Notebook")).toBe("kys-empty-notebook"); // apostrophe drops out
    expect(slugify("CN-Virus")).toBe("cn-virus");
    expect(slugify("/fullsend")).toBe("fullsend");
    expect(slugify("Call to Chaos (Core Edition)")).toBe("call-to-chaos-core-edition");
    expect(slugify('"Miss" Mrow')).toBe("miss-mrow");
    expect(slugify("4-mana 7/7")).toBe("4-mana-7-7");
  });

  it("names the file each card expects", () => {
    expect(expectedBasename("core-001", cardDef("core-001").name)).toBe("001-big-d-fender");
    expect(expectedBasename("core-043", cardDef("core-043").name)).toBe("043-big-felinor");
    expect(expectedBasename("core-051-1", cardDef("core-051-1").name)).toBe("051-1-kys-empty-notebook");
    expect(expectedBasename("core-090-1", cardDef("core-090-1").name)).toBe("090-1-cn-virus");
    // SPEC §7's shared tokens are filed under the bare prefix.
    expect(expectedBasename("core-t-rush", cardDef("core-t-rush").name)).toBe("t-rush");
    expect(expectedBasename("core-t-bread", cardDef("core-t-bread").name)).toBe("t-bread");
  });

  it("a token sub-index after a card prefix is not a slug (051 vs 051-1)", () => {
    // The hard case: KY's Empty Notebook (#51.1) lives next to KY's Private Tutor (#51).
    expect(matchesCard("051-1-kys-empty-notebook", "core-051-1")).toBe(true);
    expect(matchesCard("051-1-kys-empty-notebook", "core-051")).toBe(false);
    expect(matchesCard("051-kys-private-tutor", "core-051")).toBe(true);
    expect(matchesCard("051-kys-private-tutor", "core-051-1")).toBe(false);
    // Same shape, same answers, with the exact catalog-aware resolution.
    expect(matchesCard("051-1-kys-empty-notebook", "core-051-1", CATALOG_IDS)).toBe(true);
    expect(matchesCard("051-1-kys-empty-notebook", "core-051", CATALOG_IDS)).toBe(false);
  });

  it("matches the slugless shared-token form and rejects a bare prefix mismatch", () => {
    expect(matchesCard("t-rush", "core-t-rush")).toBe(true);
    expect(matchesCard("t-rush-token", "core-t-rush")).toBe(true);
    expect(matchesCard("t-rush", "core-t-sheep")).toBe(false);
    expect(matchesCard("1-big-d-fender", "core-001")).toBe(false); // unpadded prefix
    expect(matchesCard("0011-big-d-fender", "core-001")).toBe(false);
  });

  it("needs allIds for #25, whose own slug starts with a digit segment", () => {
    // "4-mana 7/7" slugifies to `4-mana-7-7`, so the heuristic reads the leading `4-` as a token
    // sub-index; the catalog says otherwise, because `core-025-4` does not exist. This is why every
    // script in this package resolves against the id list.
    expect(matchesCard("025-4-mana-7-7", "core-025")).toBe(false);
    expect(matchesCard("025-4-mana-7-7", "core-025", CATALOG_IDS)).toBe(true);
    expect(resolveBasename("025-4-mana-7-7", CATALOG_IDS)).toBe("core-025");
  });

  it("round-trips every catalog id through its expected filename", () => {
    // The whole catalog at once: no two cards may claim one path, and every expected path must
    // resolve back to the card that expects it — which is what makes `missing-tests.ts` a
    // trustworthy gate.
    const seen = new Map<string, string>();
    for (const id of CATALOG_IDS) {
      const path = expectedRelPath(id, cardDef(id).name);
      const owner = seen.get(path);
      expect(owner, `${path}.ts is claimed by both ${owner} and ${id}`).toBeUndefined();
      seen.set(path, id);
      expect(resolveRelPath(path, CATALOG_IDS), `${path} -> ${id}`).toBe(id);
      expect(matchesCard(expectedBasename(id, cardDef(id).name), id, CATALOG_IDS)).toBe(true);
    }
    expect(seen.size).toBe(CATALOG_SIZE);
  });

  it("resolves nothing for a filename that names no card", () => {
    expect(resolveBasename("harness", CATALOG_IDS)).toBeUndefined();
    expect(resolveBasename("15-hit-job", CATALOG_IDS)).toBeUndefined(); // unpadded
    expect(resolveBasename("core-001-big-d-fender", CATALOG_IDS)).toBeUndefined(); // id, not prefix
  });

  it("orders files by SPEC §5 index, card tokens after their card, shared tokens last", () => {
    expect(prefixRank("001")).toBe(1);
    expect(prefixRank("025")).toBe(25);
    expect(prefixRank("051-1")).toBeCloseTo(51.1);
    expect(prefixRank("t-rush")).toBe(Number.POSITIVE_INFINITY);

    const ordered = CATALOG_IDS.map((id) => ({ id, path: expectedRelPath(id, cardDef(id).name) }))
      .sort((a, b) => compareSortKeys(sortKey(a.path, a.id), sortKey(b.path, b.id)))
      .map((entry) => entry.path);

    // Catalog order: Core, then Classic, then Classic+ (B2.2), each by §5 index.
    expect(ordered[0]).toBe("001-big-d-fender");
    expect(ordered.indexOf("051-1-kys-empty-notebook")).toBe(ordered.indexOf("051-kys-private-tutor") + 1);
    expect(ordered.indexOf("100-ceaseless-void")).toBe(111 - 7);
    expect(ordered.slice(105, 111)).toEqual(["t-bread", "t-coin", "t-felinor", "t-ghoul", "t-rush", "t-sheep"]);
    expect(ordered[111]).toBe("classic/001-curse-of-the-forgotten-classic");
    expect(ordered[201]).toBe("classic-plus/001-doom-shroom");
    expect(ordered.indexOf("classic-plus/012-8-frostspatula")).toBe(ordered.indexOf("classic-plus/012-the-mother-pancake") + 8);
    expect(ordered.slice(-10)[0]).toBe("classic-plus/t-ai-01-helpful-assistant");
    expect(ordered.at(-1)).toBe("classic-plus/t-ai-10-fine-tuning");
    // Catalog.json's own order, but for the shared tokens, which the files sort by name.
    const shared = (path: string): boolean => /(^|\/)t-/.test(path);
    expect(CATALOG_IDS.map((id) => expectedRelPath(id, cardDef(id).name)).filter((path) => !shared(path))).toEqual(
      ordered.filter((path) => !shared(path)),
    );
    // Unrecognised filenames sort after everything, so the barrel stays deterministic.
    expect(compareSortKeys(sortKey("t-rush", "core-t-rush"), sortKey("mystery", undefined))).toBeLessThan(0);
  });

  it("makes a legal, unique module alias for each filename", () => {
    expect(moduleAliasOf("001-big-d-fender")).toBe("m001_big_d_fender");
    expect(moduleAliasOf("051-1-kys-empty-notebook")).toBe("m051_1_kys_empty_notebook");
    expect(moduleAliasOf("t-rush")).toBe("mt_rush");
    expect(moduleAliasOf("classic/043-plague-nuke")).toBe("mclassic_043_plague_nuke");
    expect(moduleAliasOf("classic-plus/043-ai-slop")).toBe("mclassic_plus_043_ai_slop");
    const aliases = CATALOG_IDS.map((id) => moduleAliasOf(expectedRelPath(id, cardDef(id).name)));
    expect(new Set(aliases).size).toBe(CATALOG_SIZE);
  });
});
