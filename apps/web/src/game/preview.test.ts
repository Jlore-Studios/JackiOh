// R1400 (#552): R1420's preview reaches the browser's development module. The module the unit tests
// load is the one scripts/build-wasm.sh builds by default, with crates/wasm's `preview` feature, so it
// can preview a set; the dev hotseat's E2E injection is its only caller, and its readable deck
// pre-check (decks.ts) accepts the previewed set's cards as the engine then does. Vitest gives each
// test file its own module, so the preview here reaches no other file.

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import type { CardDefs } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { createGame, previewSets } from "../wasm/index.ts";
import { resolveDecks } from "./decks.ts";

const DEFS = Object.values(CATALOG);
const deckable = (set: string): string[] =>
  DEFS.filter((def) => def.set === set && !def.token && !def.tags.includes("Token")).map((def) => def.id);

const MEDITATIVE = deckable("Meditative").slice(0, DECK_SIZE);
const CORE = deckable("Core").slice(0, DECK_SIZE);
const SIZES: readonly [number, number] = [DECK_SIZE, DECK_SIZE];

/**
 * Since v0.3.5 the catalog holds no set that has not shipped (the Meditative set shipped, R1420), so
 * the readable pre-check's preview is proven on a fixture: Core's first cards copied into the reserved
 * Boss set, which `SHIPPED_SETS` does not list, and a Boss token beside them.
 */
const BOSS_CATALOG: CardDefs = {
  ...CATALOG,
  ...Object.fromEntries(
    CORE.map((id, at) => {
      const boss = `boss-${String(at + 1).padStart(3, "0")}`;
      return [boss, { ...CATALOG[id], id: boss, index: String(at + 1), set: "Boss" }];
    }),
  ),
  "boss-t-1": { ...CATALOG[CORE[0] ?? ""], id: "boss-t-1", index: "1.1", set: "Boss", token: true },
} as CardDefs;
const BOSS = CORE.map((_, at) => `boss-${String(at + 1).padStart(3, "0")}`);

describe("R1400 R1420's preview in the browser's development module", () => {
  it("R1400 the premise: the Meditative set ships since v0.3.5, so its deck needs no preview", () => {
    expect(MEDITATIVE).toHaveLength(DECK_SIZE);
    expect(CORE).toHaveLength(DECK_SIZE);
    expect(resolveDecks("a", "b", CATALOG, { a: MEDITATIVE, b: CORE })).toEqual({ decks: [MEDITATIVE, CORE] });
  });

  it("R1400 the hotseat's readable pre-check refuses an unshipped set's card until the set is previewed, and a token always", () => {
    const decks = { a: BOSS, b: CORE };
    expect(resolveDecks("a", "b", BOSS_CATALOG, decks)).toEqual({ error: expect.stringContaining("has not shipped yet") });
    expect(resolveDecks("a", "b", BOSS_CATALOG, decks, SIZES, ["Boss"])).toEqual({ decks: [BOSS, CORE] });
    expect(resolveDecks("a", "b", BOSS_CATALOG, { a: [...BOSS.slice(1), "boss-t-1"], b: CORE }, SIZES, ["Boss"])).toEqual({
      error: expect.stringContaining("Token"),
    });
  });

  it("R1400 the engine creates a game of the shipped Meditative set with or without a preview", () => {
    const args = { seed: "r1400", decks: [MEDITATIVE, CORE] as const };
    expect(createGame(args)).toEqual(expect.objectContaining({ seed: "r1400" }));
    previewSets([]);
    expect(createGame(args)).toEqual(expect.objectContaining({ seed: "r1400" }));
    previewSets(["Meditative"]);
    expect(createGame(args)).toEqual(expect.objectContaining({ seed: "r1400" }));
  });
});
