// The AI cards' verbs (effects/datacenter.ts): T-AI-4 Chain of Thought's draw repeated while the card it
// brought is cheap (§2.4, R58, R65, R596, CHAIN_OF_THOUGHT_REPEATS) and T-AI-6 Datacenter Fire's sweep
// of Field Spells that hits the heroes once per Field Spell it dooms (§4.4, §4.5, R46, R59, R408).
// Through fixture definitions (fixtures/datacenter.ts); the real cards' tests cover the same cases again.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { animateCard } from "../src/animated";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { CHAIN_OF_THOUGHT_REPEATS, HAND_CAP } from "../src/config";
import { destroyFieldSpellsAndHit, drawWhileCheap, fieldSpellsDoomed } from "../src/effects/datacenter";
import { addModifier } from "../src/modifiers";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { stateCheck } from "../src/stateCheck";
import { vanillaCatalog } from "./fixtures/catalog";
import { COMBAT_SCRIPTS, combatCatalog } from "./fixtures/combat";
import {
  DATACENTER_SCRIPTS,
  DYING_FIELD_DAMAGE,
  castOnDraw,
  datacenterCatalog,
  dyingField,
  field,
  fieldTrap,
  free,
  hardField,
  one,
  runner,
  trap,
  two,
  xCost,
} from "./fixtures/datacenter";
import { actResult, flush, playing, tower } from "./fixtures/field";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { FIXTURE_SCRIPTS, fixtureCatalog } from "./fixtures/scripts";

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog(datacenterCatalog(combatCatalog(fixtureCatalog(vanillaCatalog()))));
  registerScripts({ ...registeredScripts(), ...FIXTURE_SCRIPTS, ...COMBAT_SCRIPTS, ...DATACENTER_SCRIPTS });
  state.players.p1.hand = [];
  state.players.p2.hand = [];
  state.active = "p1";
  return state;
}

/** The Spell running the verb, resolving as a Spell does (§10.5 step 4). */
function resolving(state: GameState, player: PlayerId = "p1"): CardInstance {
  return newInstance(state, runner.id, player, { z: "resolving", player });
}

/** Apply one effect as the resolving Spell, then the state check §10.5 runs after its list (§4.5). */
function run(state: GameState, effect: Effect, controller: PlayerId = "p1"): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, resolving(state, controller), { controller }));
  stateCheck(sink);
  state.rngCursor = sink.rng.cursor;
  return events;
}

function heroHits(events: readonly GameEvent[], player: PlayerId): number[] {
  return eventsOfType(events, "damage").flatMap((event) => (event.targetId === `hero-${player}` ? [event.amount] : []));
}

describe("drawWhileCheap (T-AI-4 Chain of Thought)", () => {
  it("draws 1, and again while the card it brought costs maxCost or less, at most repeats + 1 draws", () => {
    const state = game("chain-five");
    const library = setLibrary(state, "p1", [one.id, one.id, one.id, one.id, one.id, one.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(state.players.p1.hand).toHaveLength(CHAIN_OF_THOUGHT_REPEATS + 1);
    expect(library.slice(CHAIN_OF_THOUGHT_REPEATS + 1).every((card) => card.zone.z === "library")).toBe(true);
  });

  it("stops after the first card that costs more, which is still drawn", () => {
    const state = game("chain-stop");
    const [a, b, c] = setLibrary(state, "p1", [free.id, two.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect([a?.zone.z, b?.zone.z, c?.zone.z]).toEqual(["hand", "hand", "library"]);
  });

  it("a higher threshold goes on through it", () => {
    const state = game("chain-two");
    const [, , c] = setLibrary(state, "p1", [free.id, two.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 2, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(c?.zone.z).toBe("hand");
  });

  it("R65 an X-cost card reads 0 as it arrives", () => {
    const state = game("chain-x");
    const [, b] = setLibrary(state, "p1", [xCost.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 0, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(b?.zone.z).toBe("hand");
  });

  it("R65 the card's current cost: a costMod and a player's price rule both count", () => {
    const state = game("chain-mod");
    const [a, b, c] = setLibrary(state, "p1", [two.id, one.id, one.id]);
    if (a === undefined) throw new Error("library");
    a.costMod = -1;
    run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect([a.zone.z, b?.zone.z, c?.zone.z]).toEqual(["hand", "hand", "hand"]);

    const taxed = game("chain-tax");
    const [d, e] = setLibrary(taxed, "p1", [one.id, one.id]);
    addModifier({ state: taxed, events: [] }, "p1", { kind: "costRule", rule: { amount: 1 }, expiry: { until: "never" } });
    run(taxed, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect([d?.zone.z, e?.zone.z]).toEqual(["hand", "library"]);
  });

  it("R596 a card cast on draw ends the chain, even though its cast's own repeat brings a card", () => {
    const state = game("chain-cod");
    const [cast, next, after] = setLibrary(state, "p1", [castOnDraw.id, one.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(cast?.zone.z).toBe("graveyard");
    expect(next?.zone.z).toBe("hand");
    expect(after?.zone.z).toBe("library");
  });

  it("§2.4 R4 a burned card ends the chain", () => {
    const state = game("chain-burn");
    inHand(state, "fx-1", "p1", HAND_CAP);
    const [a, b] = setLibrary(state, "p1", [one.id, one.id]);
    const events = run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(a?.zone.z).toBe("graveyard");
    expect(b?.zone.z).toBe("library");
    expect(eventsOfType(events, "burned")).toHaveLength(1);
  });

  it("§2.4 a fatigue draw ends the chain after one hit", () => {
    const state = game("chain-fatigue");
    setLibrary(state, "p1", []);
    const events = run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(eventsOfType(events, "fatigue")).toHaveLength(1);
  });

  it("R129 it takes no rng draw", () => {
    const state = game("chain-rng");
    setLibrary(state, "p1", [one.id, one.id]);
    run(state, drawWhileCheap({ maxCost: 1, repeats: CHAIN_OF_THOUGHT_REPEATS }));
    expect(state.rngCursor).toBe(0);
  });
});

describe("destroyFieldSpellsAndHit (T-AI-6 Datacenter Fire)", () => {
  it("R59 destroys every Field Spell on both sides together, and no Trap or Field Trap", () => {
    const state = game("fire-all");
    const mine = put(state, field.id, slot("p1", "backrow", 1));
    const theirs = put(state, field.id, slot("p2", "backrow", 2));
    const keptTrap = put(state, trap.id, slot("p2", "backrow", 3));
    const keptFieldTrap = put(state, fieldTrap.id, slot("p1", "backrow", 4));
    run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    expect([mine.zone.z, theirs.zone.z]).toEqual(["graveyard", "graveyard"]);
    expect([keptTrap.zone.z, keptFieldTrap.zone.z]).toEqual(["field", "field"]);
  });

  it("§4.4 one hit per hero of damagePer times the count, the active player's hero first (R68)", () => {
    const state = game("fire-hits");
    put(state, field.id, slot("p1", "backrow", 1));
    put(state, field.id, slot("p2", "backrow", 1));
    put(state, field.id, slot("p2", "backrow", 2));
    const events = run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    expect(heroHits(events, "p1")).toEqual([3]);
    expect(heroHits(events, "p2")).toEqual([3]);
    expect(eventsOfType(events, "damage").map((event) => event.targetId)).toEqual(["hero-p1", "hero-p2"]);
    expect(state.players.p1.hero.health).toBe(27);
  });

  it("R46 an Indestructible Field Spell survives and doesn't count", () => {
    const state = game("fire-hard");
    const hard = put(state, hardField.id, slot("p1", "backrow", 1));
    put(state, field.id, slot("p2", "backrow", 1));
    const events = run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    expect(hard.zone.z).toBe("field");
    expect(heroHits(events, "p1")).toEqual([1]);
  });

  it("R129 none doomed, no damage at all", () => {
    const state = game("fire-none");
    put(state, hardField.id, slot("p1", "backrow", 1));
    put(state, trap.id, slot("p2", "backrow", 1));
    const events = run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 2 }));
    expect(eventsOfType(events, "damage")).toEqual([]);
  });

  it("the enemy side only: your Field Spells stay and only the enemy hero is hit", () => {
    const state = game("fire-enemy");
    const mine = put(state, field.id, slot("p1", "backrow", 1));
    const theirs = put(state, field.id, slot("p2", "backrow", 1));
    put(state, field.id, slot("p2", "backrow", 2));
    const events = run(state, destroyFieldSpellsAndHit({ side: "enemy", damagePer: 2 }));
    expect(mine.zone.z).toBe("field");
    expect(theirs.zone.z).toBe("graveyard");
    expect(heroHits(events, "p2")).toEqual([4]);
    expect(heroHits(events, "p1")).toEqual([]);
  });

  it("§4.5 a destroyed Field Spell that prints Death fires it", () => {
    const state = game("fire-death");
    put(state, dyingField.id, slot("p2", "backrow", 1));
    run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    // 1 from the fire, and the dying Field Spell's Death hits its enemy, p1, for 3.
    expect(state.players.p1.hero.health).toBe(30 - 1 - DYING_FIELD_DAMAGE);
    expect(state.players.p2.hero.health).toBe(29);
  });

  it("R588 R383 an animated Field Spell standing in a unit zone is a Unit there: neither destroyed nor counted", () => {
    const state = game("fire-animated");
    const animated = put(state, field.id, slot("p2", "backrow", 1));
    expect(animateCard({ state, events: [] }, animated)).toBe(true);
    const backrow = put(state, field.id, slot("p2", "backrow", 2));
    const events = run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    expect(animated.zone).toMatchObject({ z: "field", row: "units" });
    expect(backrow.zone.z).toBe("graveyard");
    expect(heroHits(events, "p2")).toEqual([1]);
  });

  it("R418 R446 a carrier Field Spell holding a Unit is destroyed and counted, and the Unit it holds is not", () => {
    const begun = playing("fire-tower");
    put(begun, tower.id, slot("p1", "backrow", 2));
    const [rider] = inHand(begun, plain.id, "p1");
    flush(begun, "p1");
    const played = actResult(begun, { type: "play", instanceId: rider?.id ?? "", zone: { row: "backrow", lane: 2 }, playerId: "p1" });
    expect(played.error).toBeUndefined();
    const state = played.state;
    registerCatalog({ ...registeredCatalog(), [runner.id]: runner });
    const events = run(state, destroyFieldSpellsAndHit({ side: "any", damagePer: 1 }));
    expect(eventsOfType(events, "destroyed").map((event) => event.defId)).toEqual([tower.id]);
    expect(heroHits(events, "p1")).toEqual([1]);
    expect(state.players.p1.units.flat().some((card) => card?.id === rider?.id)).toBe(true);
  });

  it("R280 fieldSpellsDoomed is the count the sweep hits with, read without writing", () => {
    const state = game("fire-read");
    put(state, field.id, slot("p1", "backrow", 1));
    put(state, hardField.id, slot("p2", "backrow", 1));
    put(state, field.id, slot("p2", "backrow", 2));
    const reader = { state, self: resolving(state), radiant: false, controller: "p1" as const };
    const before = JSON.stringify(state);
    expect(fieldSpellsDoomed(reader, "any")).toHaveLength(2);
    expect(fieldSpellsDoomed(reader, "enemy")).toHaveLength(1);
    expect(JSON.stringify(state)).toBe(before);
  });
});
