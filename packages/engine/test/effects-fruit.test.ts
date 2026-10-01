// The Classic+ Fruit verbs (effects/fruit.ts): the Grapes C+ #65 Two Grapes and #66 Vine of Grapes roll
// by `GRAPE_ODDS` with Lucky (R382, §6.1), C+ #65.2/#65.3's hit-or-heal and priced draw (§2.4, R58,
// R65), and C+ #65.5 Mythic Grape's hand replaced card for card (R11, R60, R129, R387). Through fixture
// scripts (fixtures/fruit.ts); the real cards' tests cover the same cases again.

import type { GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog } from "../src/catalog";
import { GRAPE_ODDS, HAND_CAP } from "../src/config";
import {
  addRolledGrapes,
  damageEnemyOrHealFriend,
  drawPriced,
  replaceHandWithRandom,
  rollGrape,
} from "../src/effects/fruit";
import { makeContext } from "../src/resolve";
import { createRng } from "../src/rng";
import type { Effect } from "../src/script";
import { registerScripts } from "../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { vanillaCatalog } from "./fixtures/catalog";
import { COMBAT_SCRIPTS, combatCatalog } from "./fixtures/combat";
import { FRUIT_SCRIPTS, castOnDraw, fruitCatalog, grapeRoller, mythic, pricedDraw, replacer } from "./fixtures/fruit";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { FIXTURE_SCRIPTS, fixtureCatalog } from "./fixtures/scripts";

const GRAPE_IDS = GRAPE_ODDS.map((grape) => grape.defId);

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog(fruitCatalog(combatCatalog(fixtureCatalog(vanillaCatalog()))));
  registerScripts({ ...FIXTURE_SCRIPTS, ...COMBAT_SCRIPTS, ...FRUIT_SCRIPTS });
  state.players.p1.hand = [];
  state.players.p2.hand = [];
  return state;
}

/** The card running the script, resolving as a Spell does (§10.5 step 4). */
function resolving(state: GameState, defId: string, radiant = false, player: PlayerId = "p1"): CardInstance {
  const card = newInstance(state, defId, player, { z: "resolving", player });
  card.radiant = radiant;
  return card;
}

function run(
  state: GameState,
  effect: Effect,
  self: CardInstance | null = null,
  targets: Selection[] = [],
  controller: PlayerId = "p1",
): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, self, { controller, targets }));
  state.rngCursor = sink.rng.cursor;
  return events;
}

describe("rollGrape (R382, BUILD §2 GRAPE_ODDS)", () => {
  it("R382 the table is the five Grapes, worst to best, and its percents sum to 100", () => {
    expect(GRAPE_ODDS.map((grape) => grape.percent)).toEqual([12, 60, 20, 7, 1]);
    expect(GRAPE_ODDS.reduce((sum, grape) => sum + grape.percent, 0)).toBe(100);
  });

  it("R382 many seeded rolls match the odds (12 / 60 / 20 / 7 / 1 percent)", () => {
    const rng = createRng("grape-odds");
    const counts = new Map<string, number>();
    const rolls = 20_000;
    for (let i = 0; i < rolls; i += 1) {
      const id = rollGrape(rng);
      counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    GRAPE_ODDS.forEach((grape) => {
      const share = ((counts.get(grape.defId) ?? 0) / rolls) * 100;
      expect(Math.abs(share - grape.percent)).toBeLessThan(1.5);
    });
  });

  it("§6.1 Lucky 1 rolls twice and keeps the better: Rotten falls to ~1.4%, Mythic rises to ~2%", () => {
    const rng = createRng("grape-lucky");
    const counts = new Map<string, number>();
    const rolls = 20_000;
    for (let i = 0; i < rolls; i += 1) {
      const id = rollGrape(rng, 1);
      counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    const share = (id: string): number => ((counts.get(id) ?? 0) / rolls) * 100;
    expect(share(GRAPE_IDS[0] ?? "")).toBeLessThan(2.5);
    expect(share(GRAPE_IDS[4] ?? "")).toBeGreaterThan(1.4);
    expect(share(GRAPE_IDS[4] ?? "")).toBeLessThan(2.8);
  });

  it("§6.1 a Lucky roll is two draws of the rng, an ordinary one draw", () => {
    const plain = createRng("grape-draws");
    rollGrape(plain);
    expect(plain.cursor).toBe(1);
    const lucky = createRng("grape-draws");
    rollGrape(lucky, 1);
    expect(lucky.cursor).toBe(2);
  });

  it("§6.1 Lucky keeps the later entry: with a fixed seed the lucky pick is the best of the two plain picks", () => {
    for (let i = 0; i < 50; i += 1) {
      const a = createRng(`grape-best-${i}`);
      const first = GRAPE_IDS.indexOf(rollGrape(a));
      const second = GRAPE_IDS.indexOf(rollGrape(a));
      const b = createRng(`grape-best-${i}`);
      expect(GRAPE_IDS.indexOf(rollGrape(b, 1))).toBe(Math.max(first, second));
    }
  });
});

describe("addRolledGrapes (C+ #65, #66)", () => {

  it("R60 adds N Grapes, each its own roll, to the controller's hand", () => {
    const state = game("grapes-three");
    const events = run(state, addRolledGrapes({ count: 3 }), resolving(state, grapeRoller.id));

    expect(state.players.p1.hand).toHaveLength(3);
    expect(state.players.p1.hand.every((card) => GRAPE_IDS.includes(card.defId))).toBe(true);
    expect(state.players.p1.hand.every((card) => !card.radiant)).toBe(true);
    expect(eventsOfType(events, "addedToHand")).toHaveLength(3);
    expect(state.rngCursor).toBe(3);
  });

  it("R74 radiant: every Grape is made Radiant", () => {
    const state = game("grapes-radiant");
    run(state, addRolledGrapes({ count: 3, radiant: true, lucky: 0 }), resolving(state, grapeRoller.id));
    expect(state.players.p1.hand.every((card) => card.radiant)).toBe(true);
  });

  it("§6.1 the Lucky is the running card's own: the Radiant face's printed Lucky 1 is two draws a Grape", () => {
    const state = game("grapes-lucky");
    run(state, addRolledGrapes({ count: 3, radiant: true }), resolving(state, grapeRoller.id, true));
    expect(state.rngCursor).toBe(6);

    const plain = game("grapes-lucky");
    run(plain, addRolledGrapes({ count: 3 }), resolving(plain, grapeRoller.id, false));
    expect(plain.rngCursor).toBe(3);
  });

  it("§2.4 R4 a full hand burns each Grape that doesn't fit", () => {
    const state = game("grapes-burn");
    inHand(state, "fx-1", "p1", HAND_CAP - 1);
    const events = run(state, addRolledGrapes({ count: 3 }), resolving(state, grapeRoller.id));
    expect(state.players.p1.hand).toHaveLength(HAND_CAP);
    expect(eventsOfType(events, "burned")).toHaveLength(2);
    expect(state.players.p1.graveyard.filter((card) => GRAPE_IDS.includes(card.defId))).toHaveLength(2);
  });

  it("§10.7 a fixed seed and cursor give fixed Grapes in a fixed order", () => {
    const roll = (): string[] => {
      const state = game("grapes-fixed");
      run(state, addRolledGrapes({ count: 5 }), resolving(state, grapeRoller.id));
      return state.players.p1.hand.map((card) => card.defId);
    };
    expect(roll()).toEqual(roll());
  });

  it("a count of 0 adds nothing and draws nothing", () => {
    const state = game("grapes-none");
    run(state, addRolledGrapes({ count: 0 }), resolving(state, grapeRoller.id));
    expect(state.players.p1.hand).toEqual([]);
    expect(state.rngCursor).toBe(0);
  });
});

describe("damageEnemyOrHealFriend (C+ #65.2, #65.3)", () => {
  it("an enemy Unit takes the hit from the card running the script", () => {
    const state = game("hit-enemy");
    const enemy = put(state, "fx-5", slot("p2", "units", 1));
    const self = resolving(state, pricedDraw.id);
    const events = run(state, damageEnemyOrHealFriend({ amount: 2 }), self, [{ pick: "instance", instanceId: enemy.id }]);
    expect(enemy.damage).toBe(2);
    expect(eventsOfType(events, "damage")[0]?.sourceId).toBe(self.id);
  });

  it("the enemy hero takes the hit", () => {
    const state = game("hit-hero");
    run(state, damageEnemyOrHealFriend({ amount: 2 }), resolving(state, pricedDraw.id), [{ pick: "hero", player: "p2" }]);
    expect(state.players.p2.hero.health).toBe(28);
  });

  it("R19 a friendly Unit is healed, never hit", () => {
    const state = game("heal-friend");
    const friend = put(state, "fx-5", slot("p1", "units", 1));
    friend.damage = 1;
    const events = run(state, damageEnemyOrHealFriend({ amount: 2 }), resolving(state, pricedDraw.id), [
      { pick: "instance", instanceId: friend.id },
    ]);
    expect(friend.damage).toBe(0);
    expect(eventsOfType(events, "damage")).toEqual([]);
  });

  it("R19 your own hero is healed past 30 (a hero has no maximum)", () => {
    const state = game("heal-hero");
    run(state, damageEnemyOrHealFriend({ amount: 4 }), resolving(state, pricedDraw.id), [{ pick: "hero", player: "p1" }]);
    expect(state.players.p1.hero.health).toBe(34);
  });

  it("a target gone by resolution is nothing", () => {
    const state = game("hit-gone");
    const events = run(state, damageEnemyOrHealFriend({ amount: 2 }), resolving(state, pricedDraw.id), [
      { pick: "instance", instanceId: "c-missing" },
    ]);
    expect(events).toEqual([]);
  });
});

describe("drawPriced (C+ #65.2, #65.3)", () => {
  it("R65 the drawn card takes the discount, which stacks on its costMod", () => {
    const state = game("priced-mod");
    const [top] = setLibrary(state, "p1", ["fx-3", "fx-4"]);
    if (top === undefined) throw new Error("library");
    top.costMod = 1;
    const events = run(state, drawPriced({ costMod: -1 }), resolving(state, pricedDraw.id));
    expect(top.zone.z).toBe("hand");
    expect(top.costMod).toBe(0);
    expect(eventsOfType(events, "costChanged").map((event) => event.instanceId)).toEqual([top.id]);
  });

  it("R65 a set price is a costOverride", () => {
    const state = game("priced-override");
    const [top] = setLibrary(state, "p1", ["fx-3"]);
    run(state, drawPriced({ costOverride: 0 }), resolving(state, pricedDraw.id));
    expect(top?.costOverride).toBe(0);
  });

  it("§2.4 R4 a burned card takes no price", () => {
    const state = game("priced-burn");
    inHand(state, "fx-1", "p1", HAND_CAP);
    const [top] = setLibrary(state, "p1", ["fx-3"]);
    run(state, drawPriced({ costMod: -1 }), resolving(state, pricedDraw.id));
    expect(top?.zone.z).toBe("graveyard");
    expect(top?.costMod).toBe(0);
  });

  it("§2.4 a fatigue draw brings no card and prices nothing", () => {
    const state = game("priced-fatigue");
    state.players.p1.library = [];
    const events = run(state, drawPriced({ costMod: -1 }), resolving(state, pricedDraw.id));
    expect(eventsOfType(events, "fatigue")).toHaveLength(1);
    expect(eventsOfType(events, "costChanged")).toEqual([]);
  });

  it("R58 a card cast on draw never reaches the hand, so neither it nor the card the draw then brings is priced", () => {
    const state = game("priced-cod");
    const [cast, next] = setLibrary(state, "p1", [castOnDraw.id, "fx-3"]);
    run(state, drawPriced({ costMod: -1 }), resolving(state, pricedDraw.id));
    expect(cast?.zone.z).toBe("graveyard");
    expect(next?.zone.z).toBe("hand");
    expect(next?.costMod).toBe(0);
  });
});

describe("replaceHandWithRandom (C+ #65.5)", () => {
  it("moves every hand card to the graveyard — not a discard — and adds as many cards of the pool", () => {
    const state = game("replace-hand");
    const old = inHand(state, "fx-1", "p1", 3);
    const events = run(state, replaceHandWithRandom({ query: { rarity: "Mythic" }, costOverride: 0 }), resolving(state, replacer.id));

    expect(old.every((card) => card.zone.z === "graveyard")).toBe(true);
    expect(eventsOfType(events, "discarded")).toEqual([]);
    expect(eventsOfType(events, "enteredGraveyard").map((event) => event.instanceId)).toEqual(old.map((card) => card.id));
    expect(state.players.p1.hand).toHaveLength(3);
    expect(state.players.p1.hand.every((card) => card.defId === mythic.id && card.costOverride === 0)).toBe(true);
  });

  it("R387 the card running it is never in its own pool", () => {
    const state = game("replace-self");
    inHand(state, "fx-1", "p1", 4);
    run(state, replaceHandWithRandom({ query: { rarity: "Mythic" } }), resolving(state, replacer.id));
    expect(state.players.p1.hand.some((card) => card.defId === replacer.id)).toBe(false);
  });

  it("R74 radiant: the new cards are Radiant", () => {
    const state = game("replace-radiant");
    inHand(state, "fx-1", "p1", 2);
    run(state, replaceHandWithRandom({ query: { rarity: "Mythic" }, radiant: true }), resolving(state, replacer.id));
    expect(state.players.p1.hand.every((card) => card.radiant)).toBe(true);
  });

  it("R11 a unit-token card in the hand ceases to exist, and still counts as a card replaced", () => {
    const state = game("replace-token");
    const [token] = inHand(state, "fx-token-rush", "p1", 1);
    inHand(state, "fx-1", "p1", 1);
    run(state, replaceHandWithRandom({ query: { rarity: "Mythic" } }), resolving(state, replacer.id));
    expect(token?.zone.z).not.toBe("graveyard");
    expect(state.players.p1.graveyard.some((card) => card.id === token?.id)).toBe(false);
    expect(state.players.p1.hand).toHaveLength(2);
  });

  it("R129 an empty hand moves nothing, adds nothing and draws nothing from the rng", () => {
    const state = game("replace-empty");
    const events = run(state, replaceHandWithRandom({ query: { rarity: "Mythic" } }), resolving(state, replacer.id));
    expect(events).toEqual([]);
    expect(state.rngCursor).toBe(0);
  });
});
