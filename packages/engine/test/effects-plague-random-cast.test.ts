// R452 meets E19 (R471, R668): "Place N Plague Tokens" inside a random cast (Classic+ #47 Jogg's Box
// casting Classic #70 Book of Plague) asks its caster nothing — every placement goes on one random
// permanent — as a random cast makes every choice at random. Through fixture cards; the real cards'
// tests cover the same case again (packages/cards/test/classic-plus/047-joggs-box.test.ts).

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { castRandom } from "../src/effects/cast";
import { placePlagueTokens } from "../src/effects/plague";
import { applyEffects, makeContext } from "../src/resolve";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type GameState } from "../src/state";
import { newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

function spell(id: string): CardDef {
  return {
    id,
    index: id,
    name: id,
    set: "Core",
    type: "Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
  };
}

const plagueSpell = spell("pr-plague-fixture");
const box = spell("pr-box-fixture");
const TOKENS = 3;

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), [plagueSpell.id]: plagueSpell, [box.id]: box });
  const placing = { cry: () => [placePlagueTokens({ count: TOKENS })] };
  registerScripts({ ...registeredScripts(), [plagueSpell.id]: { base: placing, radiant: placing } });
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  setLibrary(state, "p1", []);
  return state;
}

function castFromBox(state: GameState): void {
  const sink = sinkFor(state, []);
  const self = newInstance(state, box.id, "p1", { z: "resolving", player: "p1" });
  applyEffects([castRandom({ query: { defId: plagueSpell.id }, count: 1 })], makeContext(sink, self, { controller: "p1" }));
  state.rngCursor = sink.rng.cursor;
}

function tokensOnField(state: GameState): number {
  return [state.players.p1, state.players.p2]
    .flatMap((side) => side.units.flat())
    .reduce((sum, card) => sum + (card?.counters.plague ?? 0), 0);
}

describe("R452 R471 placements inside a random cast", () => {
  it("R452 R668 its caster is never asked: every placement lands on one random permanent at once", () => {
    const state = game("plague-random-cast");
    put(state, "fx-1", slot("p1", "units", 1));
    put(state, "fx-2", slot("p2", "units", 1));
    castFromBox(state);
    expect(state.pending).toBeNull();
    expect(tokensOnField(state)).toBe(TOKENS);
  });

  it("R129 with no permanent on the field the placements fizzle, and nothing is asked", () => {
    const state = game("plague-random-empty");
    castFromBox(state);
    expect(state.pending).toBeNull();
    expect(tokensOnField(state)).toBe(0);
  });

  it("R471 outside a random cast the same spell still asks its caster", () => {
    const state = game("plague-asked");
    put(state, "fx-1", slot("p1", "units", 1));
    const sink = sinkFor(state, []);
    const self = newInstance(state, plagueSpell.id, "p1", { z: "resolving", player: "p1" });
    applyEffects([placePlagueTokens({ count: TOKENS })], makeContext(sink, self, { controller: "p1" }));
    expect(state.pending?.playerId).toBe("p1");
  });
});
