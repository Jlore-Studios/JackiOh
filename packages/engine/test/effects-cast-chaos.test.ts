// R28 meets R452: a Call to Chaos that a random cast makes (Classic+ #47 Jogg's Box's `castRandom`) is a
// cast of the Call to Chaos chain, the first, so its own recursion stops at CALL_TO_CHAOS_CHAIN_CAP casts
// in all. Through fixture cards; the real card's test covers the same case again
// (packages/cards/test/classic-plus/047-joggs-box.test.ts).

import type { CardDef, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { CALL_TO_CHAOS_CHAIN_CAP } from "../src/config";
import { castRandom } from "../src/effects/cast";
import { applyEffects, makeContext } from "../src/resolve";
import type { Hook } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type GameState } from "../src/state";
import { castRandomCallToChaos, chaosChainOf } from "../src/subsystems/callToChaos";
import { eventsOfType, newGame, setLibrary, sinkFor } from "./fixtures/harness";

function spell(id: string, extra: Partial<CardDef> = {}): CardDef {
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
    ...extra,
  };
}

const chaos = spell("cc-chaos-fixture", { tags: ["Call to Chaos"], rarity: "Legendary", cost: 4 });
const box = spell("cc-box-fixture", { rarity: "Legendary", cost: 4 });

/** A game whose only Call to Chaos runs `cry`, and where the box's pool is that card alone. */
function game(seed: string, cry: Hook): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), [chaos.id]: chaos, [box.id]: box });
  registerScripts({ ...registeredScripts(), [chaos.id]: { base: { cry }, radiant: { cry } } });
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  setLibrary(state, "p1", []);
  return state;
}

function castFromBox(state: GameState): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  const self = newInstance(state, box.id, "p1", { z: "resolving", player: "p1" });
  applyEffects([castRandom({ query: { tags: ["Call to Chaos"] }, count: 1 })], makeContext(sink, self, { controller: "p1" }));
  state.rngCursor = sink.rng.cursor;
  return events;
}

describe("castRandom and R28's Call to Chaos chain (Classic+ #47)", () => {
  it("R28 a Call to Chaos a random cast makes is the chain's first cast", () => {
    const depths: number[] = [];
    const state = game("box-chaos-first", (ctx) => {
      depths.push(chaosChainOf(ctx.self));
      return [];
    });
    castFromBox(state);
    expect(depths).toEqual([1]);
  });

  it("R28 a chain begun by a random cast stops at CALL_TO_CHAOS_CHAIN_CAP casts in all", () => {
    const depths: number[] = [];
    const state = game("box-chaos-chain", (ctx) => {
      depths.push(chaosChainOf(ctx.self));
      return [castRandomCallToChaos()];
    });
    const events = castFromBox(state);
    expect(eventsOfType(events, "cardPlayed")).toHaveLength(CALL_TO_CHAOS_CHAIN_CAP);
    expect(depths).toEqual(Array.from({ length: CALL_TO_CHAOS_CHAIN_CAP }, (_, i) => i + 1));
  });
});
