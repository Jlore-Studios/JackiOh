// R3045000: how a fused definition is read for how its card looks (fused.ts, issue #3045).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, CardDefs } from "@jackioh/shared";

import { fusedDef } from "../test/fixtures.ts";
import { DORMANT_STARSHIP_ID, DORMANT_STARSHIP_NAME, fusedParts, holdsDormantStarship } from "./fused.ts";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

const BEAM = "stinkyscape-050";
const MINER = "stinkyscape-063";
const HOST = "core-020";

/** A resolver that knows the catalog and the fusions given. */
function resolver(...fused: readonly CardDef[]): (id: string) => CardDef | undefined {
  const defs: CardDefs = { ...CATALOG, ...Object.fromEntries(fused.map((fusion) => [fusion.id, fusion])) };
  return (id) => defs[id];
}

describe("R3045000: a fused definition's parts", () => {
  it("R3045000 the Dormant Starship's id and name are the catalog's", () => {
    expect(def(DORMANT_STARSHIP_ID).name).toBe(DORMANT_STARSHIP_NAME);
  });

  it("R3045000 a card that is no fusion is its own part, known or not", () => {
    expect(fusedParts(HOST, resolver())).toEqual([HOST]);
    expect(fusedParts("core-999", resolver())).toEqual(["core-999"]);
  });

  it("R3045000 a fusion lists its ingredients in order, as its definition does", () => {
    const equipped = fusedDef([def(BEAM), def(HOST)]);
    expect(fusedParts(equipped.id, resolver(equipped))).toEqual([BEAM, HOST]);
  });

  it("R3045000 a fusion of fusions is walked to its leaves, from the definitions or from the ids they spell", () => {
    const inner = fusedDef([def(BEAM), def(HOST)], 1);
    const outer = fusedDef([def(MINER), inner], 2);
    expect(outer.id).toBe(`t-2:${MINER}+(${inner.id})`);
    expect(fusedParts(outer.id, resolver(inner, outer))).toEqual([MINER, BEAM, HOST]);
    // The view carries the definition a card names, not the ones inside it (R243): the id spells the rest.
    expect(fusedParts(outer.id, resolver(outer))).toEqual([MINER, BEAM, HOST]);
    const twice = fusedDef([def(BEAM), outer], 3);
    expect(fusedParts(twice.id, resolver(twice))).toEqual([BEAM, MINER, BEAM, HOST]);
  });

  it("R3045000 a Radiant mark on a spelled ingredient is not part of its id", () => {
    const bare: CardDef = { ...fusedDef([def(BEAM), def(HOST)], 4) };
    delete bare.ingredients;
    expect(fusedParts(`t-4:${BEAM}*+${HOST}`, resolver())).toEqual([BEAM, HOST]);
    expect(fusedParts(`t-5:${MINER}+(t-4:${BEAM}*+${HOST})*`, resolver())).toEqual([MINER, BEAM, HOST]);
    expect(fusedParts(bare.id, resolver(bare))).toEqual([BEAM, HOST]);
  });

  it("R3045000 a digest id (R468) hides what it fuses, unless its definition is held", () => {
    const digest = "t-6:#0123456789abcdef";
    expect(fusedParts(digest, resolver())).toBeNull();
    const held: CardDef = { ...fusedDef([def(BEAM), def(HOST)], 6), id: digest };
    expect(fusedParts(digest, resolver(held))).toEqual([BEAM, HOST]);
    // And one hidden inside a fusion hides the whole list, since its place in it is unknown.
    const outer: CardDef = { ...fusedDef([def(MINER), def(HOST)], 7), ingredients: [{ defId: MINER }, { defId: digest }] };
    expect(fusedParts(outer.id, resolver(outer))).toBeNull();
  });

  it("R3045000 a definition that lists itself ends the walk", () => {
    const loop: CardDef = { ...fusedDef([def(BEAM), def(HOST)], 8), id: "t-8:loop", ingredients: [{ defId: "t-8:loop" }, { defId: HOST }] };
    expect(fusedParts(loop.id, resolver(loop))).toBeNull();
  });
});

describe("R3045000: a fusion holding the Dormant Starship", () => {
  const hull = def(DORMANT_STARSHIP_ID);

  it("R3045000 a piece fused into the hull holds it, at any depth, and the hull alone does not", () => {
    const once = fusedDef([hull, def(MINER)], 1);
    const twice = fusedDef([once, def(BEAM)], 2);
    expect(holdsDormantStarship(hull)).toBe(false);
    expect(holdsDormantStarship(once)).toBe(true);
    // The inner definition is not held, but the id spells it.
    expect(holdsDormantStarship(twice)).toBe(true);
    expect(holdsDormantStarship(fusedDef([def(BEAM), def(HOST)]))).toBe(false);
    expect(holdsDormantStarship(def(HOST))).toBe(false);
  });
});
