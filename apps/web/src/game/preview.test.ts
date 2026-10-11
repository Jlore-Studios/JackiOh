// R1400 (#552): R1420's preview reaches the browser's development module. The module the unit tests
// load is the one scripts/build-wasm.sh builds by default, with crates/wasm's `preview` feature, so it
// can preview a set; the dev hotseat's E2E injection is its only caller, and its readable deck
// pre-check (decks.ts) accepts the previewed set's cards as the engine then does. Vitest gives each
// test file its own module, so the preview here reaches no other file.

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import { describe, expect, it } from "vitest";

import { createGame, previewSets } from "../wasm/index.ts";
import { resolveDecks } from "./decks.ts";

const DEFS = Object.values(CATALOG);
const deckable = (set: string): string[] =>
  DEFS.filter((def) => def.set === set && !def.token && !def.tags.includes("Token")).map((def) => def.id);

const MEDITATIVE = deckable("Meditative").slice(0, DECK_SIZE);
const CORE = deckable("Core").slice(0, DECK_SIZE);
const SIZES: readonly [number, number] = [DECK_SIZE, DECK_SIZE];

describe("R1400 R1420's preview in the browser's development module", () => {
  it("R1400 the premise: the catalog holds a deck's worth of a set that has not shipped", () => {
    expect(MEDITATIVE).toHaveLength(DECK_SIZE);
    expect(CORE).toHaveLength(DECK_SIZE);
  });

  it("R1400 the hotseat's readable pre-check refuses an unshipped set's card until the set is previewed, and a token always", () => {
    const decks = { a: MEDITATIVE, b: CORE };
    expect(resolveDecks("a", "b", CATALOG, decks)).toEqual({ error: expect.stringContaining("has not shipped yet") });
    expect(resolveDecks("a", "b", CATALOG, decks, SIZES, ["Meditative"])).toEqual({ decks: [MEDITATIVE, CORE] });
    const token = DEFS.find((def) => def.set === "Meditative" && def.token)?.id ?? "";
    expect(token).not.toBe("");
    expect(resolveDecks("a", "b", CATALOG, { a: [...MEDITATIVE.slice(1), token], b: CORE }, SIZES, ["Meditative"])).toEqual({
      error: expect.stringContaining("Token"),
    });
  });

  it("R1400 the engine refuses such a deck until the module previews the set, and creates the game after", () => {
    const args = { seed: "r1400", decks: [MEDITATIVE, CORE] as const };
    expect(() => createGame(args)).toThrow();
    previewSets([]);
    expect(() => createGame(args)).toThrow();
    previewSets(["Meditative"]);
    expect(createGame(args)).toEqual(expect.objectContaining({ seed: "r1400" }));
  });
});
