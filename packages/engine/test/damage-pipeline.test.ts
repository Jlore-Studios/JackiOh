// B5 E6's additions to the damage pipeline (SPEC §4.4; docs/classic-sets.md B5 E6; R463): Spell
// Damage at step 0, the hero's divisors after Armor and the lowest hit cap at step 3, Trample on a
// Spell — and that every projection of a hit (R44's lethal projection, the Zephyrs scorer) reads the
// same steps as the hit itself.

import { describe, expect, it } from "vitest";
import { damage } from "../src/effects";
import { dealDamage, heroDamageCap, heroDamageDivisor, heroHitAmount, loseHealth, spellDamageOf } from "../src/damage";
import { drawOne } from "../src/draw";
import { applyEffects, makeContext } from "../src/resolve";
import type { CardInstance, GameState } from "../src/state";
import { projectedHeroDamage } from "../src/subsystems/lethal";
import { antiOneshot } from "./fixtures/scripts";
import {
  animeArmor,
  argus,
  bolt,
  grunt,
  hiddenArgus,
  hiddenLens,
  lance,
  lens,
  playing,
  recorder,
  replaysTo,
  solar,
  sweep,
  wall,
} from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

function spellIn(state: GameState, defId: string): CardInstance {
  const card = inHand(state, defId, "p1")[0];
  if (card === undefined) throw new Error("no card");
  return card;
}

describe("E6 Spell Damage (§4.4 step 0)", () => {
  it("a Spell's hit is raised by the sum of the Spell Damage on its controller's units, once per hit", () => {
    const state = playing("dc-spell-damage");
    put(state, solar.id, slot("p1", "units", 1));
    put(state, solar.id, slot("p1", "units", 2), { radiant: true });
    // The opponent's Spell Damage is theirs; a Field Spell and a face-down Trap printing it are no units.
    put(state, solar.id, slot("p2", "units", 1));
    put(state, lens.id, slot("p1", "backrow", 1));
    put(state, hiddenLens.id, slot("p1", "backrow", 2));
    expect(spellDamageOf(state, "p1")).toBe(2 + 5);
    const target = put(state, wall.id, slot("p2", "units", 2));
    target.buffs = { attack: 0, health: 20 };
    const spell = spellIn(state, bolt.id);
    const game = recorder(state);

    const result = game.play({ type: "play", instanceId: spell.id, targets: [{ pick: "instance", instanceId: target.id }], playerId: "p1" });
    expect(eventsOfType(result.events, "damage")).toEqual([
      { type: "damage", sourceId: spell.id, targetId: target.id, amount: 3 + 7, combat: false },
    ]);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("not a Unit's, a Field Spell's or a Trap's hit; and a hit of 0 is nothing to raise", () => {
    const state = playing("dc-spell-damage-other");
    const mage = put(state, solar.id, slot("p1", "units", 1));
    const field = put(state, sweep.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    applyEffects([damage({ to: { of: "enemyHero" }, amount: 2 })], makeContext(sink, mage));
    applyEffects([damage({ to: { of: "enemyHero" }, amount: 2 })], makeContext(sink, field));
    expect(state.players.p2.hero.health).toBe(26);
    const spell = spellIn(state, bolt.id);
    expect(dealDamage(sink, { source: spell, target: { kind: "hero", player: "p2" }, amount: 0 })).toBe(0);
    expect(state.players.p2.hero.health).toBe(26);
    // A Spell that is resolving, or in a hand, is the Spell all the same.
    expect(dealDamage(sink, { source: spell, target: { kind: "hero", player: "p2" }, amount: 1 })).toBe(3);
  });
});

describe("E6 Trample on a Spell", () => {
  it("the excess over the target Unit's health hits its controller's hero as a new instance, raised once", () => {
    const state = playing("dc-lance");
    put(state, solar.id, slot("p1", "units", 1));
    const target = put(state, grunt.id, slot("p2", "units", 1));
    state.players.p2.hero.armor = 3;
    const spell = spellIn(state, lance.id);
    const game = recorder(state);
    const result = game.play({ type: "play", instanceId: spell.id, targets: [{ pick: "instance", instanceId: target.id }], playerId: "p1" });
    // 11 + 2 Spell Damage = 13: 2 on the 2/2, 11 on to the hero, less its Armor 3.
    expect(eventsOfType(result.events, "damage").map((hit) => [hit.targetId, hit.amount])).toEqual([
      [target.id, 2],
      ["hero-p2", 8],
    ]);
    expect(game.state().players.p2.hero.health).toBe(22);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("a printed Trample is read off the Spell, and an effect may state it where no source is left", () => {
    const state = playing("dc-lance-stated");
    const target = put(state, grunt.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    dealDamage(sink, { source: spellIn(state, lance.id), target: { kind: "unit", instance: target }, amount: 5 });
    expect(state.players.p2.hero.health).toBe(27);
    const other = put(state, grunt.id, slot("p2", "units", 2));
    dealDamage(sink, { source: null, target: { kind: "unit", instance: other }, amount: 6, flags: { trample: true } });
    expect(state.players.p2.hero.health).toBe(23);
    // Without it, nothing tramples.
    const third = put(state, grunt.id, slot("p2", "units", 3));
    dealDamage(sink, { source: null, target: { kind: "unit", instance: third }, amount: 6 });
    expect(state.players.p2.hero.health).toBe(23);
  });
});

describe("E6 hero divisors and caps (§4.4 steps 2 and 3)", () => {
  it("R463 divides after Armor, rounded up once; several multiply; then the lowest cap", () => {
    const state = playing("dc-divisors");
    put(state, argus.id, slot("p2", "backrow", 1));
    expect(heroDamageDivisor(state, "p2")).toBe(2);
    expect(heroHitAmount(state, "p2", 5)).toBe(3);
    state.players.p2.hero.armor = 1;
    expect(heroHitAmount(state, "p2", 5)).toBe(2);
    // Pierce skips the Armor and nothing else.
    expect(heroHitAmount(state, "p2", 5, true)).toBe(3);
    state.players.p2.hero.armor = 0;
    // Radiant (4) beside base (2): 8, and 5 / 8 rounds up to 1 — once, not at each divisor.
    put(state, argus.id, slot("p2", "backrow", 2), { radiant: true });
    expect(heroDamageDivisor(state, "p2")).toBe(8);
    expect(heroHitAmount(state, "p2", 5)).toBe(1);
    expect(heroHitAmount(state, "p2", 17)).toBe(3);
    // Caps: Anti-oneshot's 5 and Anime Armor's 1 — the lowest wins.
    const capped = playing("dc-caps");
    put(capped, antiOneshot.id, slot("p2", "backrow", 1));
    expect(heroDamageCap(capped, "p2")).toBe(5);
    put(capped, animeArmor.id, slot("p2", "units", 1));
    expect(heroDamageCap(capped, "p2")).toBe(1);
    expect(heroHitAmount(capped, "p2", 30)).toBe(1);
    // A divisor comes before the cap: 30 / 2 = 15, capped at 1 either way; with only Anti-oneshot, 5.
    const both = playing("dc-caps-order");
    put(both, argus.id, slot("p2", "backrow", 1));
    put(both, antiOneshot.id, slot("p2", "backrow", 2));
    expect(heroHitAmount(both, "p2", 8)).toBe(4);
    expect(heroHitAmount(both, "p2", 30)).toBe(5);
  });

  it("R463 a face-down Trap guards no hero until it fires", () => {
    const state = playing("dc-hidden-guard");
    const trap = put(state, hiddenArgus.id, slot("p2", "backrow", 1));
    expect(heroHitAmount(state, "p2", 6)).toBe(6);
    trap.faceUp = true;
    expect(heroHitAmount(state, "p2", 6)).toBe(3);
  });

  it("applies to every hit on the hero — fatigue included — and not to a unit; lose health is not damage", () => {
    const state = playing("dc-divisor-hits");
    put(state, argus.id, slot("p1", "backrow", 1));
    const body = put(state, grunt.id, slot("p1", "units", 1));
    body.buffs = { attack: 0, health: 10 };
    const sink = sinkFor(state);
    dealDamage(sink, { source: null, target: { kind: "hero", player: "p1" }, amount: 7 });
    expect(state.players.p1.hero.health).toBe(26);
    dealDamage(sink, { source: null, target: { kind: "unit", instance: body }, amount: 7 });
    expect(body.damage).toBe(7);
    state.players.p1.library = [];
    state.players.p1.fatigueCount = 2;
    drawOne(sink, "p1");
    // The 3rd fatigue draw deals 3, halved and rounded up to 2 (R125).
    expect(state.players.p1.hero.health).toBe(24);
    // R18: losing health is no hit, so nothing divides it.
    loseHealth(sink, "p1", 5);
    expect(state.players.p1.hero.health).toBe(19);
  });

  it("R44 the lethal projection reads the same steps as the hit", () => {
    const state = playing("dc-projection");
    put(state, argus.id, slot("p2", "backrow", 1));
    put(state, animeArmor.id, slot("p2", "units", 1));
    for (const amount of [1, 2, 5, 9]) {
      expect(projectedHeroDamage(state, "p2", amount)).toBe(heroHitAmount(state, "p2", amount));
    }
    const unguarded = playing("dc-projection-plain");
    put(unguarded, argus.id, slot("p2", "backrow", 1));
    expect(projectedHeroDamage(unguarded, "p2", 9)).toBe(5);
  });
});
