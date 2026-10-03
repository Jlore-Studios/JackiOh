// effects/perks.ts: C+ #46 Felinor Flagbearer's Armor for the rest of the game and C+ #49 Jay Fungus's
// discount on a random hand card. Applied straight to fixture cards; the real cards' tests cover the
// same cases again (packages/cards/test/classic-plus/046-felinor-flagbearer.test.ts, 049-jay-fungus.test.ts).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog } from "../src/catalog";
import { heroHitAmount } from "../src/damage";
import { discountRandomInHand, gainHeroArmor } from "../src/effects/perks";
import { effectiveCost } from "../src/mana";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import type { GameState } from "../src/state";
import { spellDef, unitDef, vanillaCatalog } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, sinkFor } from "./fixtures/harness";

const free = unitDef(801, { cost: 0 });
const xSpell = spellDef(802, { cost: "X" });
const pricey = unitDef(803, { cost: 3 });

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...vanillaCatalog(), [free.id]: free, [xSpell.id]: xSpell, [pricey.id]: pricey });
  state.players.p1.hand = [];
  return state;
}

function run(state: GameState, effect: Effect): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, null, { controller: "p1" }));
  state.rngCursor = sink.rng.cursor;
  return events;
}

describe("gainHeroArmor (C+ #46)", () => {
  it("R124 adds to the hero's own Armor, stacks, and §4.4 step 2 takes it off each hit but a Pierce one", () => {
    const state = game("perks-armor");
    run(state, gainHeroArmor({ amount: 1 }));
    run(state, gainHeroArmor({ amount: 2 }));
    expect(state.players.p1.hero.armor).toBe(3);
    expect(state.players.p2.hero.armor).toBe(0);
    expect(heroHitAmount(state, "p1", 5)).toBe(2);
    expect(heroHitAmount(state, "p1", 5, true)).toBe(5);
  });

  it("names the other hero with `player: \"enemy\"` and ignores a non-positive amount", () => {
    const state = game("perks-armor-enemy");
    run(state, gainHeroArmor({ amount: 2, player: "enemy" }));
    run(state, gainHeroArmor({ amount: 0 }));
    run(state, gainHeroArmor({ amount: -3 }));
    expect(state.players.p2.hero.armor).toBe(2);
    expect(state.players.p1.hero.armor).toBe(0);
  });
});

describe("discountRandomInHand (C+ #49)", () => {
  it("R65 only a card above (0) that is not X-cost is discounted, by `costMod`", () => {
    const state = game("perks-discount");
    const [zero] = inHand(state, free.id, "p1");
    const [x] = inHand(state, xSpell.id, "p1");
    const [three] = inHand(state, pricey.id, "p1");
    const events = run(state, discountRandomInHand({ amount: 2 }));
    expect(three?.costMod).toBe(-2);
    expect(zero?.costMod).toBe(0);
    expect(x?.costMod).toBe(0);
    expect(eventsOfType(events, "costChanged")).toEqual([{ type: "costChanged", instanceId: three?.id, cost: 1 }]);
  });

  it("§2.3 a discount past the price floors the cost at (0)", () => {
    const state = game("perks-floor");
    const [three] = inHand(state, pricey.id, "p1");
    run(state, discountRandomInHand({ amount: 20 }));
    expect(three?.costMod).toBe(-20);
    expect(three === undefined ? -1 : effectiveCost(state, three)).toBe(0);
  });

  it("R129 a hand with nothing to discount draws no random number", () => {
    const state = game("perks-none");
    inHand(state, free.id, "p1");
    inHand(state, xSpell.id, "p1");
    const before = state.rngCursor;
    expect(run(state, discountRandomInHand({ amount: 2 }))).toEqual([]);
    expect(state.rngCursor).toBe(before);
  });

  it("R60 the pick is uniform over the cards it can make cheaper", () => {
    const hits = new Map<string, number>();
    for (let i = 0; i < 300; i += 1) {
      const state = game(`perks-uniform-${i}`);
      const cards = inHand(state, pricey.id, "p1", 3);
      run(state, discountRandomInHand({ amount: 1 }));
      const hit = cards.findIndex((card) => card.costMod === -1);
      hits.set(String(hit), (hits.get(String(hit)) ?? 0) + 1);
    }
    expect([...hits.keys()].sort()).toEqual(["0", "1", "2"]);
    for (const count of hits.values()) expect(count).toBeGreaterThan(60);
  });
});
