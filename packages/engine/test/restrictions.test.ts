// B5 E35 unit restrictions and statuses (SPEC §4.2, §6.1; docs/classic-sets.md B5 E35): can't be
// attacked, attacked only from its own lane, can't attack or be attacked, Immune to Spells' "doesn't
// affect it" half, a keyword that holds only while a condition does, and the forced attacks on a
// random enemy and on the unit's own hero — which skip §4.2 steps 1 to 3 (R53) but obey these.

import { describe, expect, it } from "vitest";
import { damage, damageAll, destroy, forcedAttackOwnHero, forcedAttackRandom, forcedAttacksOn, plague, vanilla } from "../src/effects";
import { attackTargets, canAttack, forceAttacksRandom, FORCED_RANDOM_WORK, randomAttackTargets, whyCannotAttack } from "../src/combat";
import { unitView } from "../src/layers";
import { reduce } from "../src/reduce";
import { effectIsFromSpell, registerAttackBar, unaffectedBy } from "../src/restrictions";
import { applyEffects, makeContext } from "../src/resolve";
import type { CardDef } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { findInstance, type CardInstance } from "../src/state";
import { owedWork } from "../src/work";
import {
  askController,
  bolt,
  charger,
  fighter,
  grunt,
  note,
  notes,
  playing,
  recorder,
  replaysTo,
  roundTrip,
  answer,
  statue,
  storm,
  topLoser,
  wall,
  warded,
} from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

const unitOf = (instance: CardInstance) => ({ kind: "unit" as const, instance });
const heroOf = (player: "p1" | "p2") => ({ kind: "hero" as const, player });

/** A 1/1 whose Death asks its controller something, so a run of forced attacks pauses between two. */
const asker: CardDef = {
  id: "dc-asker",
  index: "4799",
  name: "asker (damage and combat)",
  set: "Core",
  type: "Unit",
  tags: [],
  rarity: "Common",
  token: false,
  cost: 0,
  base: { attack: 1, health: 1, keywords: [], text: "asker" },
  radiant: { attack: 2, health: 2, keywords: [], text: "asker" },
};
const brute: CardDef = { ...asker, id: "dc-brute", index: "4798", name: "brute", base: { attack: 9, health: 9, keywords: [], text: "brute" }, radiant: { attack: 18, health: 18, keywords: [], text: "brute" } };
const askerScripts: CardScripts = {
  base: { death: () => [note("asker:death"), askController("answered")], resume: { answered: () => [note("asker:answered")] } },
  radiant: {},
};

function withAsker(): void {
  registerCatalog({ ...registeredCatalog(), [asker.id]: asker, [brute.id]: brute });
  registerScripts({ ...registeredScripts(), [asker.id]: askerScripts });
}

describe("E35 attack restrictions (§4.2 step 2)", () => {
  it("can't be attacked: never a target of a declared or a forced attack, still one of effects", () => {
    const state = playing("dc-cant-be-attacked");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const jet = put(state, fighter.id, slot("p2", "units", 1));
    expect(attackTargets(state, attacker).map((t) => (t.kind === "unit" ? t.instance.id : t.player))).toEqual(["p2"]);
    expect(whyCannotAttack(state, attacker, unitOf(jet))).toBe("that unit cannot be attacked");
    const refused = reduce(state, { type: "attack", attackerId: attacker.id, targetId: jet.id, playerId: "p1", nonce: "dc-x" });
    expect(refused.error).toBe("that unit cannot be attacked");
    // A forced attack on it does not happen, in silence.
    const sink = sinkFor(state);
    applyEffects([forcedAttacksOn({ target: { of: "instance", instanceId: jet.id }, attackers: "self" })], makeContext(sink, attacker));
    expect(eventsOfType(sink.events, "attackDeclared")).toEqual([]);
    // Effects still reach it.
    applyEffects([damage({ to: { of: "instance", instanceId: jet.id }, amount: 1 })], makeContext(sink, attacker));
    expect(jet.damage).toBe(1);
    // A Taunt it gains binds nobody, since nobody may attack it.
    jet.grantedKeywords = [{ kind: "Taunt" }];
    expect(canAttack(state, attacker, heroOf("p2"))).toBe(true);
  });

  it("attacked only from its lane; its Taunt binds only the attackers that may reach it", () => {
    const state = playing("dc-lane-only");
    const far = put(state, grunt.id, slot("p1", "units", 1));
    const near = put(state, grunt.id, slot("p1", "units", 3));
    const top = put(state, topLoser.id, slot("p2", "units", 3));
    expect(whyCannotAttack(state, far, unitOf(top))).toBe("only a unit in its lane may attack that unit");
    // The Taunt it prints binds only the lane-3 attacker, which must attack it.
    expect(canAttack(state, far, heroOf("p2"))).toBe(true);
    expect(canAttack(state, near, heroOf("p2"))).toBe(false);
    expect(canAttack(state, near, unitOf(top))).toBe(true);
    const game = recorder(state);
    game.play({ type: "attack", attackerId: near.id, targetId: top.id, playerId: "p1" });
    game.play({ type: "attack", attackerId: far.id, targetId: "hero-p2", playerId: "p1" });
    expect(game.state().players.p2.hero.health).toBe(28);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("can't attack or be attacked; a restriction from where a card stands is registered by the module that knows it", () => {
    const state = playing("dc-statue");
    const stone = put(state, statue.id, slot("p1", "units", 1));
    const attacker = put(state, grunt.id, slot("p2", "units", 2));
    const other = put(state, grunt.id, slot("p1", "units", 2));
    state.active = "p2";
    expect(whyCannotAttack(state, stone, heroOf("p2"))).toBe("that unit cannot attack");
    expect(attackTargets(state, attacker).map((t) => (t.kind === "unit" ? t.instance.id : t.player))).toEqual([other.id, "p1"]);
    const previous = registerAttackBar("test-carried", (_, unit) => (unit.id === other.id ? { cantBeAttacked: true, cantAttack: true } : {}));
    try {
      expect(attackTargets(state, attacker).map((t) => (t.kind === "unit" ? t.instance.id : t.player))).toEqual(["p1"]);
      expect(randomAttackTargets(state, other, "enemies")).toEqual([]);
    } finally {
      registerAttackBar("test-carried", previous);
    }
    expect(attackTargets(state, attacker)).toHaveLength(2);
  });
});

describe("E35 Immune to Spells: a Spell's effects pass it by", () => {
  it("a Spell's single target and its sweep skip it; a Unit's reaches it", () => {
    const state = playing("dc-immune");
    const shielded = put(state, warded.id, slot("p2", "units", 1));
    const open = put(state, wall.id, slot("p2", "units", 2));
    const sweep = inHand(state, storm.id, "p1")[0] as CardInstance;
    const game = recorder(state);
    game.play({ type: "play", instanceId: sweep.id, playerId: "p1" });
    expect(findInstance(game.state(), shielded.id)?.damage).toBe(0);
    expect(findInstance(game.state(), open.id)?.damage).toBe(2);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);

    const sink = sinkFor(state);
    const spell = inHand(state, bolt.id, "p1")[0] as CardInstance;
    const spellCtx = makeContext(sink, spell, { controller: "p1", targets: [{ pick: "instance", instanceId: shielded.id }] });
    applyEffects([damage({ to: { of: "chosen" }, amount: 3 }), destroy({ target: { of: "chosen" } })], spellCtx);
    expect(shielded.damage).toBe(0);
    expect(shielded.markedDestroyed).toBeUndefined();
    const body = put(state, grunt.id, slot("p1", "units", 1));
    applyEffects([damageAll({ amount: 1, side: "enemy" })], makeContext(sink, body));
    expect(shielded.damage).toBe(1);
  });

  it("a continuation whose Spell has gone is still a Spell's (its definition), and the Radiant Top Loser is immune", () => {
    const state = playing("dc-immune-ctx");
    const radiantTop = put(state, topLoser.id, slot("p2", "units", 1), { radiant: true });
    const sink = sinkFor(state);
    const gone = { ...makeContext(sink, null, { controller: "p1" }), defId: bolt.id, radiant: false };
    expect(effectIsFromSpell(gone)).toBe(true);
    expect(unaffectedBy(gone, radiantTop)).toBe(true);
    expect(unaffectedBy({ ...gone, defId: grunt.id }, radiantTop)).toBe(false);
    expect(unaffectedBy(gone, put(state, topLoser.id, slot("p2", "units", 2)))).toBe(false);
  });
});

describe("E35 a keyword that holds only while a condition does", () => {
  it("First Strike while it has a Plague Token, gone when the token is, and with the text under a Vanilla", () => {
    const state = playing("dc-conditional");
    const bull = put(state, charger.id, slot("p1", "units", 1));
    const keywords = (): string[] => unitView(state, bull).keywords.map((k) => k.kind);
    expect(keywords()).not.toContain("First Strike");
    const sink = sinkFor(state);
    applyEffects([plague({ target: { of: "self" }, amount: 1 })], makeContext(sink, bull));
    expect(keywords()).toContain("First Strike");
    // It strikes first: the 2/2 it attacks dies before hitting back.
    const victim = put(state, grunt.id, slot("p2", "units", 1));
    const game = recorder(state);
    game.play({ type: "attack", attackerId: bull.id, targetId: victim.id, playerId: "p1" });
    expect(findInstance(game.state(), bull.id)?.damage).toBe(0);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
    applyEffects([vanilla({ target: { of: "self" } })], makeContext(sink, bull));
    expect(keywords()).not.toContain("First Strike");
  });
});

describe("E35 forced attacks on a random enemy and on the unit's own hero", () => {
  it("a random enemy is drawn from the targets the attacker may attack; an enemy Unit leaves the hero out", () => {
    const state = playing("dc-random");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    put(state, fighter.id, slot("p2", "units", 1));
    const top = put(state, topLoser.id, slot("p2", "units", 3));
    const open = put(state, wall.id, slot("p2", "units", 2));
    const ids = (among: "enemies" | "enemyUnits"): string[] =>
      randomAttackTargets(state, attacker, among).map((t) => (t.kind === "unit" ? t.instance.id : t.player));
    // Not the unattackable fighter, not the lane-3 Top Loser from lane 1; Taunt and sickness waived.
    expect(ids("enemyUnits")).toEqual([open.id]);
    expect(ids("enemies")).toEqual([open.id, "p2"]);
    attacker.summonedTurn = state.turn;
    attacker.position = "DEF";
    const sink = sinkFor(state);
    applyEffects([forcedAttackRandom({ attacker: { of: "self" }, among: "enemyUnits" })], makeContext(sink, attacker));
    expect(eventsOfType(sink.events, "attackDeclared")).toEqual([
      { type: "attackDeclared", attackerId: attacker.id, targetId: open.id, forced: true },
    ]);
    expect(top.damage).toBe(0);
    // With nothing it may attack, nothing happens.
    const lonely = playing("dc-random-none");
    const striker = put(lonely, grunt.id, slot("p1", "units", 1));
    put(lonely, fighter.id, slot("p2", "units", 1));
    const quiet = sinkFor(lonely);
    applyEffects([forcedAttackRandom({ attacker: { of: "self" }, among: "enemyUnits", times: 3 })], makeContext(quiet, striker));
    expect(quiet.events).toEqual([]);
  });

  it("R113 a Death that asks between two random attacks owes the rest, which survives a round trip", () => {
    const state = playing("dc-random-pause");
    withAsker();
    const attacker = put(state, brute.id, slot("p1", "units", 1));
    // The first draw picks the asker: it stands where the match rng's first pick lands.
    const firstPick = sinkFor(state).rng.int(2);
    const askerCard = put(state, asker.id, slot("p2", "units", firstPick === 0 ? 1 : 2));
    const plain = put(state, wall.id, slot("p2", "units", firstPick === 0 ? 2 : 1));
    const sink = sinkFor(state);
    forceAttacksRandom(sink, attacker, "enemyUnits", 2);
    expect(notes(state)).toEqual(["asker:death"]);
    expect(state.pending?.playerId).toBe("p2");
    const owed = owedWork(state, FORCED_RANDOM_WORK);
    expect(owed.map((item) => item.resume.data.run)).toEqual([
      { attacker: attacker.id, among: "enemyUnits", left: 1, since: expect.any(Number) as number },
    ]);
    state.rngCursor = sink.rng.cursor;
    const resumed = answer(roundTrip(state)).state;
    expect(notes(resumed)).toEqual(["asker:death", "asker:answered"]);
    // The second attack went to the one enemy Unit left.
    expect(findInstance(resumed, plain.id)?.zone.z).toBe("graveyard");
    expect(findInstance(resumed, askerCard.id)?.zone.z).toBe("graveyard");
    expect(resumed.work).toEqual([]);
  });

  it("the Radiant's second attack happens only while the unit is still on the field (R96)", () => {
    const state = playing("dc-random-gone");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const spiky = put(state, grunt.id, slot("p2", "units", 2));
    spiky.buffs = { attack: 5, health: 20 };
    const sink = sinkFor(state);
    forceAttacksRandom(sink, attacker, "enemyUnits", 2);
    // The first strike back killed it, so there is no second attack.
    expect(eventsOfType(sink.events, "attackDeclared").map((event) => event.targetId)).toEqual([spiky.id]);
    expect(findInstance(state, attacker.id)?.zone.z).toBe("graveyard");
  });

  it("a forced attack on its own hero: the hero takes its attack and strikes nothing back", () => {
    const state = playing("dc-own-hero");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    attacker.buffs = { attack: 3, health: 0 };
    const sink = sinkFor(state);
    applyEffects([forcedAttackOwnHero({ attacker: { of: "self" } })], makeContext(sink, attacker));
    expect(state.players.p1.hero.health).toBe(25);
    expect(attacker.damage).toBe(0);
    expect(eventsOfType(sink.events, "attackDeclared")).toEqual([
      { type: "attackDeclared", attackerId: attacker.id, targetId: "hero-p1", forced: true },
    ]);
    // A unit that cannot attack does not.
    const stone = put(state, statue.id, slot("p1", "units", 2));
    applyEffects([forcedAttackOwnHero({ attacker: { of: "self" } })], makeContext(sink, stone));
    expect(state.players.p1.hero.health).toBe(25);
  });
});
