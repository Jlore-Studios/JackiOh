// B5 E5 replacement windows, with E8's heal conversion and E9's redirects (docs/classic-sets.md B5;
// SPEC §4.2, §4.4, §4.5, §3.2; R460, R461, R462, R463). Each moment is proved through a fixture card
// shaped like the Classic or Classic+ card that uses it (`fixtures/damage-combat.ts`): a Final
// Gambit, a Shadowstep, a Blood Moon, a Voidwalker, a Second Wind, a Pile On and a Joro.
//
// Every scenario that runs through `reduce` is also replayed from its start state (§9.3); the ones
// that pause survive `JSON.parse(JSON.stringify(state))` and finish from the round-tripped copy; and
// the moments that read a hidden card (a face-down Trap, a hand) are checked from the other seat's
// view (R97, R177): a replacement that declines leaves that view exactly as a card without one would.

import { describe, expect, it } from "vitest";
import { discard, heal } from "../src/effects";
import { dealDamage, healHero, loseHealth } from "../src/damage";
import { drawOne } from "../src/draw";
import { answerTargeting, replacementOf } from "../src/replacements";
import { makeContext, applyEffects } from "../src/resolve";
import { sacrificeNow, stateCheck } from "../src/stateCheck";
import { findInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { antiOneshot } from "./fixtures/scripts";
import {
  animeArmor,
  answer,
  argus,
  bloodMoon,
  eventTypes,
  gambit,
  gambitAsker,
  grunt,
  inPile,
  joro,
  leech,
  mend,
  notes,
  phoenix,
  pileOn,
  playing,
  rattle,
  recorder,
  replaysTo,
  roundTrip,
  secondWind,
  shadowstep,
  storm,
  vitalKill,
  voidwalker,
  wall,
  watcher,
} from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

function hero(state: GameState, player: "p1" | "p2"): number {
  return state.players[player].hero.health;
}

function backrowAt(state: GameState, player: "p1" | "p2", lane: number): CardInstance | null {
  return state.players[player].backrow[lane - 1] ?? null;
}

function unitAt(state: GameState, player: "p1" | "p2", lane: number): CardInstance | null {
  return state.players[player].units[lane - 1]?.[0] ?? null;
}

// ---------------------------------------------------------------------------
// §4.4: would take lethal damage (Classic #52 Final Gambit)
// ---------------------------------------------------------------------------

describe("E5 would take lethal damage, E9 damage redirect", () => {
  it("R460 a lethal hit is redirected once: the first Final Gambit fires and the second stays set", () => {
    const state = playing("dc-gambit-two");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const first = put(state, gambit.id, slot("p2", "backrow", 1));
    const second = put(state, gambit.id, slot("p2", "backrow", 2));
    state.players.p2.hero.health = 2;
    const handBefore = state.players.p2.hand.length;
    const game = recorder(state);

    const result = game.play({ type: "attack", attackerId: attacker.id, targetId: "hero-p2", playerId: "p1" });
    const after = game.state();

    // The hit moved to p1's hero, as a new instance from the same source (E9).
    expect(hero(after, "p1")).toBe(30 - 2);
    // p2 took nothing, then its follow-up healed 10 and drew 3 once the combat was done.
    expect(hero(after, "p2")).toBe(2 + 10);
    expect(after.players.p2.hand.length).toBe(handBefore + 3);
    expect(notes(after)).toEqual(["gambit:after:p1"]);
    const redirects = eventsOfType(result.events, "redirected");
    expect(redirects).toEqual([{ type: "redirected", what: "damage", fromId: "hero-p2", toId: "hero-p1", byInstanceId: first.id }]);
    expect(eventsOfType(result.events, "trapFired").map((e) => e.instanceId)).toEqual([first.id]);
    // The first is spent to its owner's graveyard; the second re-checked the changed hit and stays set.
    expect(inPile(after, "p2", "graveyard", first.id)).toBe(true);
    expect(backrowAt(after, "p2", 2)?.id).toBe(second.id);
    expect(backrowAt(after, "p2", 2)?.faceUp).toBeUndefined();
    expect(after.work).toEqual([]);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("R460 a redirected hit meets the other hero's replacements as a new instance, each card once", () => {
    const state = playing("dc-gambit-both");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const theirs = put(state, gambit.id, slot("p2", "backrow", 1));
    const mine = put(state, gambit.id, slot("p1", "backrow", 1));
    state.players.p1.hero.health = 1;
    state.players.p2.hero.health = 2;
    const game = recorder(state);

    const result = game.play({ type: "attack", attackerId: attacker.id, targetId: "hero-p2", playerId: "p1" });
    const after = game.state();

    // p2's Gambit sent it to p1, p1's sent it back, and p2 has no second one: p2 falls, p1 wins.
    expect(eventsOfType(result.events, "redirected").map((e) => [e.fromId, e.toId, e.byInstanceId])).toEqual([
      ["hero-p2", "hero-p1", theirs.id],
      ["hero-p1", "hero-p2", mine.id],
    ]);
    expect(hero(after, "p2")).toBe(0);
    expect(hero(after, "p1")).toBe(1);
    expect(after.result).toEqual({ winner: "p1", reason: "hero-death" });
    // The game ended at the check after the combat, so neither follow-up resolved.
    expect(notes(after)).toEqual([]);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("a hit that is not lethal, and losing health, leave a Final Gambit set", () => {
    const state = playing("dc-gambit-quiet");
    const trap = put(state, gambit.id, slot("p2", "backrow", 1));
    state.players.p2.hero.health = 5;
    const sink = sinkFor(state);
    dealDamage(sink, { source: null, target: { kind: "hero", player: "p2" }, amount: 4 });
    expect(hero(state, "p2")).toBe(1);
    // R18: lose health is not damage, so no replacement answers it, lethal or not.
    loseHealth(sink, "p2", 3);
    expect(hero(state, "p2")).toBe(-2);
    expect(backrowAt(state, "p2", 1)?.id).toBe(trap.id);
    expect(eventTypes(sink.events)).not.toContain("trapFired");
  });

  it("fatigue is damage: a lethal fatigue draw is redirected, and the follow-up waits for the draw to end", () => {
    const state = playing("dc-gambit-fatigue");
    put(state, gambit.id, slot("p2", "backrow", 1));
    state.players.p2.library = [];
    state.players.p2.hero.health = 1;
    const sink = sinkFor(state);
    drawOne(sink, "p2");
    expect(hero(state, "p2")).toBe(1);
    expect(hero(state, "p1")).toBe(29);
    // The follow-up is owed on `state.work` (R113, R462) and the resolution loop resolves it.
    expect(state.work.map((item) => item.resume.step)).toEqual(["after"]);
    settle(sink);
    // Heal 10, then three more fatigue draws of 2, 3 and 4.
    expect(hero(state, "p2")).toBe(1 + 10 - 2 - 3 - 4);
    expect(state.players.p2.fatigueCount).toBe(4);
    expect(state.work).toEqual([]);
  });

  it("E9 the redirected hit goes through the other hero's Armor, divisor and cap", () => {
    const state = playing("dc-gambit-guarded");
    put(state, gambit.id, slot("p2", "backrow", 1));
    put(state, argus.id, slot("p1", "backrow", 1));
    put(state, antiOneshot.id, slot("p1", "backrow", 2));
    state.players.p1.hero.armor = 1;
    state.players.p2.hero.health = 9;
    const sink = sinkFor(state);
    const dealt = dealDamage(sink, { source: null, target: { kind: "hero", player: "p2" }, amount: 12 });
    // 12 − 1 Armor = 11, halved and rounded up = 6, capped at Anti-oneshot's 5.
    expect(dealt).toBe(5);
    expect(hero(state, "p1")).toBe(25);
    expect(hero(state, "p2")).toBe(9);
    // And a cap alone keeps a hit from being lethal in the first place.
    put(state, animeArmor.id, slot("p2", "units", 1));
    state.players.p2.hero.health = 2;
    const second = put(state, gambit.id, slot("p2", "backrow", 3));
    dealDamage(sink, { source: null, target: { kind: "hero", player: "p2" }, amount: 12 });
    expect(hero(state, "p2")).toBe(1);
    expect(backrowAt(state, "p2", 3)?.id).toBe(second.id);
  });

  it("R113 a follow-up that asks pauses after the replaced event, survives a round trip and replays", () => {
    const state = playing("dc-gambit-pause");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    put(state, gambitAsker.id, slot("p2", "backrow", 1));
    state.players.p2.hero.health = 2;
    const game = recorder(state);

    game.play({ type: "attack", attackerId: attacker.id, targetId: "hero-p2", playerId: "p1" });
    const paused = game.state();
    expect(hero(paused, "p1")).toBe(28);
    expect(notes(paused)).toEqual(["asker:before"]);
    expect(paused.pending?.playerId).toBe("p2");
    // The rest of the follow-up is owed as plain data.
    expect(JSON.parse(JSON.stringify(paused.work))).toEqual(paused.work);

    const resumed = answer(roundTrip(paused)).state;
    expect(notes(resumed)).toEqual(["asker:before", "asker:answered", "asker:tail"]);
    expect(hero(resumed, "p2")).toBe(12);
    expect(resumed.work).toEqual([]);
    expect(resumed.pending).toBeNull();

    // The live game answered the same prompt: its log replays to the same state.
    const pending = paused.pending;
    if (pending === null) throw new Error("expected a prompt");
    game.play({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: "p2" });
    expect(notes(game.state())).toEqual(notes(resumed));
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("R177 a face-down Final Gambit that declines a hit tells the other seat nothing", () => {
    const withGambit = playing("dc-gambit-hidden");
    const withOther = playing("dc-gambit-hidden");
    const a = put(withGambit, grunt.id, slot("p1", "units", 1));
    put(withGambit, gambit.id, slot("p2", "backrow", 1));
    const b = put(withOther, grunt.id, slot("p1", "units", 1));
    put(withOther, shadowstep.id, slot("p2", "backrow", 1));
    const one = recorder(withGambit);
    const two = recorder(withOther);
    one.play({ type: "attack", attackerId: a.id, targetId: "hero-p2", playerId: "p1" });
    two.play({ type: "attack", attackerId: b.id, targetId: "hero-p2", playerId: "p1" });
    expect(viewFor(one.state(), "p1")).toEqual(viewFor(two.state(), "p1"));
  });
});

// ---------------------------------------------------------------------------
// Heals: would be healed, and heal into damage (Classic+ #22 Blood Moon)
// ---------------------------------------------------------------------------

describe("E5 would be healed, E8 heal becomes damage", () => {
  it("R462 an enemy's heal sets Blood Moon off and is converted, and the rest of the turn converts too", () => {
    const state = playing("dc-moon");
    const moon = put(state, bloodMoon.id, slot("p2", "backrow", 1));
    const firstMend = inHand(state, mend.id, "p1")[0] as CardInstance;
    const secondMend = inHand(state, mend.id, "p1")[0] as CardInstance;
    const thirdMend = inHand(state, mend.id, "p1")[0] as CardInstance;
    state.players.p1.hero.armor = 3;
    const game = recorder(state);

    const first = game.play({ type: "play", instanceId: firstMend.id, targets: [{ pick: "hero", player: "p1" }], playerId: "p1" });
    let now = game.state();
    // Pierce: the hero's Armor 3 stops none of the 5, and the damage is Blood Moon's.
    expect(hero(now, "p1")).toBe(25);
    expect(eventsOfType(first.events, "damage")).toEqual([
      { type: "damage", sourceId: moon.id, targetId: "hero-p1", amount: 5, combat: false },
    ]);
    expect(eventsOfType(first.events, "healed")).toEqual([]);
    expect(inPile(now, "p2", "graveyard", moon.id)).toBe(true);
    expect(now.players.p2.mods.map((mod) => mod.kind)).toEqual(["healToDamage"]);

    // The modifier converts the next heal on p1 with no trap left to fire.
    const second = game.play({ type: "play", instanceId: secondMend.id, targets: [{ pick: "hero", player: "p1" }], playerId: "p1" });
    now = game.state();
    expect(hero(now, "p1")).toBe(20);
    expect(eventTypes(second.events)).not.toContain("trapFired");

    // A heal on Blood Moon's own side is no enemy's: it heals.
    game.play({ type: "play", instanceId: thirdMend.id, targets: [{ pick: "hero", player: "p2" }], playerId: "p1" });
    now = game.state();
    expect(hero(now, "p2")).toBe(35);
    expect(replaysTo(game.start, game.log, now)).toBe(true);

    // "For the rest of this turn": gone at the turn's cleanup.
    game.play({ type: "endTurn", playerId: "p1" });
    now = game.state();
    expect(now.players.p2.mods).toEqual([]);
    const sink = sinkFor(now);
    healHero(sink, "p1", 4);
    expect(hero(now, "p1")).toBe(24);
  });

  it("R462 Lifesteal, heal up to and heal to full are heals; a heal on an undamaged unit converts its stated amount", () => {
    const state = playing("dc-moon-kinds");
    put(state, bloodMoon.id, slot("p2", "backrow", 1));
    const sucker = put(state, leech.id, slot("p1", "units", 1));
    const target = put(state, wall.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    // Lifesteal: 3 dealt heals p1 3, which Blood Moon turns into 3 Pierce damage on p1.
    dealDamage(sink, { source: sucker, target: { kind: "unit", instance: target }, amount: 3 });
    expect(hero(state, "p1")).toBe(27);
    // Heal up to 30: the 3 it would restore.
    const ctx = makeContext(sink, sucker);
    applyEffects([heal({ target: { of: "selfHero" }, upTo: 30 })], ctx);
    expect(hero(state, "p1")).toBe(24);
    // Heal to full on the leech's 2 damage: 2.
    sucker.damage = 2;
    applyEffects([heal({ target: { of: "self" }, toFull: true })], ctx);
    expect(sucker.damage).toBe(4);
    // Heal 5 on an undamaged unit: 5, though it would restore nothing.
    const fresh = put(state, grunt.id, slot("p1", "units", 2));
    applyEffects([heal({ target: { of: "instance", instanceId: fresh.id }, amount: 5 })], ctx);
    expect(fresh.damage).toBe(5);
    // A heal of nothing (heal to full on an undamaged unit) is no heal at all: nothing converts.
    const whole = put(state, grunt.id, slot("p1", "units", 3));
    const before = sink.events.length;
    applyEffects([heal({ target: { of: "instance", instanceId: whole.id }, toFull: true })], ctx);
    expect(sink.events.length).toBe(before);
    expect(whole.damage).toBe(0);
  });

  it("E7 set health is no heal: Blood Moon stays set", () => {
    const state = playing("dc-moon-set");
    const moon = put(state, bloodMoon.id, slot("p2", "backrow", 1));
    state.players.p1.hero.health = 5;
    const spell = inHand(state, vitalKill.id, "p1")[0] as CardInstance;
    const game = recorder(state);
    const result = game.play({ type: "play", instanceId: spell.id, targets: [{ pick: "hero", player: "p1" }], playerId: "p1" });
    expect(hero(game.state(), "p1")).toBe(13);
    expect(eventsOfType(result.events, "healthSet")).toEqual([{ type: "healthSet", player: "p1", health: 13, sourceId: spell.id }]);
    expect(backrowAt(game.state(), "p2", 1)?.id).toBe(moon.id);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("E8 the Radiant Field Trap fires once, stays face-up and converts from then on", () => {
    const state = playing("dc-moon-radiant");
    const moon = put(state, bloodMoon.id, slot("p2", "backrow", 1), { radiant: true });
    const sink = sinkFor(state);
    healHero(sink, "p1", 4);
    expect(hero(state, "p1")).toBe(26);
    expect(eventsOfType(sink.events, "trapFired").map((e) => e.instanceId)).toEqual([moon.id]);
    expect(backrowAt(state, "p2", 1)?.faceUp).toBe(true);
    // From now on: the face-up Field Trap converts by its text, with no firing and no modifier.
    healHero(sink, "p1", 6);
    expect(hero(state, "p1")).toBe(20);
    expect(eventsOfType(sink.events, "trapFired")).toHaveLength(1);
    expect(state.players.p2.mods).toEqual([]);
    // Its own side's heals go through.
    healHero(sink, "p2", 6);
    expect(hero(state, "p2")).toBe(36);
  });
});

// ---------------------------------------------------------------------------
// §4.5 step 1: would die (Classic #14's Radiant face)
// ---------------------------------------------------------------------------

describe("E5 would die", () => {
  it("R460 its controller's dying units flicker instead, one firing for them all; a second Shadowstep stays set", () => {
    const state = playing("dc-shadowstep");
    const mine = [put(state, rattle.id, slot("p1", "units", 1)), put(state, grunt.id, slot("p1", "units", 2))];
    const theirs = put(state, rattle.id, slot("p2", "units", 1));
    const trap = put(state, shadowstep.id, slot("p1", "backrow", 1));
    const spare = put(state, shadowstep.id, slot("p1", "backrow", 2));
    for (const unit of mine) unit.buffs = { attack: 1, health: 0 };
    const spell = inHand(state, storm.id, "p1")[0] as CardInstance;
    const game = recorder(state);

    const result = game.play({ type: "play", instanceId: spell.id, playerId: "p1" });
    const after = game.state();

    // Back in their zones at once, reset (no buff, no damage) and summoning sick.
    for (const [lane, unit] of mine.entries()) {
      const back = unitAt(after, "p1", lane + 1);
      expect(back?.id).toBe(unit.id);
      expect(back?.damage).toBe(0);
      expect(back?.buffs).toEqual({ attack: 0, health: 0 });
      expect(back?.summonedTurn).toBe(after.turn);
    }
    expect(eventsOfType(result.events, "flickered").map((e) => e.instanceId)).toEqual(mine.map((u) => u.id));
    // No death for them: no `destroyed`, no Death hook; the enemy's rattle dies as ever.
    expect(eventsOfType(result.events, "destroyed").map((e) => e.instanceId)).toEqual([theirs.id]);
    expect(notes(after)).toEqual(["rattle:death:p2", `shadowstep:${mine.map((u) => u.id).join(",")}`]);
    expect(eventsOfType(result.events, "trapFired").map((e) => e.instanceId)).toEqual([trap.id]);
    expect(backrowAt(after, "p1", 2)?.id).toBe(spare.id);
    expect(after.counters.destroyed).toBe(1);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("R462 a Sacrifice is not offered the would-die window", () => {
    const state = playing("dc-shadowstep-sacrifice");
    const unit = put(state, rattle.id, slot("p1", "units", 1));
    const trap = put(state, shadowstep.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    sacrificeNow(sink, unit);
    expect(inPile(state, "p1", "graveyard", unit.id)).toBe(true);
    expect(notes(state)).toEqual(["rattle:death:p1"]);
    expect(backrowAt(state, "p1", 1)?.id).toBe(trap.id);
  });

  it("the follow-up reads what it flickered from its record", () => {
    const state = playing("dc-shadowstep-record");
    const unit = put(state, grunt.id, slot("p1", "units", 3));
    put(state, shadowstep.id, slot("p1", "backrow", 1));
    unit.markedDestroyed = true;
    const sink = sinkFor(state);
    stateCheck(sink);
    const owed = state.work[0];
    const record = owed === undefined ? null : replacementOf({ data: owed.resume.data });
    expect(record?.flickered).toEqual([{ instanceId: unit.id, defId: grunt.id, radiant: false }]);
    expect(record?.event).toEqual({ moment: "wouldDie", units: [{ instanceId: unit.id, controller: "p1" }] });
  });
});

// ---------------------------------------------------------------------------
// Every move into a graveyard (Classic #50, #28, #60)
// ---------------------------------------------------------------------------

describe("E5 would go to a graveyard", () => {
  it("R461 a unit exiled instead of dying has not died: no Death, no Reborn, no destroyed count", () => {
    const state = playing("dc-void");
    put(state, voidwalker.id, slot("p1", "units", 1));
    const bird = put(state, phoenix.id, slot("p2", "units", 1));
    const bell = put(state, rattle.id, slot("p1", "units", 2));
    const spell = inHand(state, storm.id, "p1")[0] as CardInstance;
    const exiledBefore = state.counters.exiled;
    const game = recorder(state);

    const result = game.play({ type: "play", instanceId: spell.id, playerId: "p1" });
    const after = game.state();

    expect(inPile(after, "p2", "exile", bird.id)).toBe(true);
    expect(inPile(after, "p1", "exile", bell.id)).toBe(true);
    expect(unitAt(after, "p2", 1)).toBeNull();
    expect(after.reserved).toEqual([]);
    expect(eventsOfType(result.events, "destroyed")).toEqual([]);
    expect(notes(after)).toEqual([]);
    expect(after.counters.destroyed).toBe(0);
    // The Spell itself, on its way to its graveyard, is exiled too; three exiles in all.
    expect(inPile(after, "p1", "exile", spell.id)).toBe(true);
    expect(after.counters.exiled).toBe(exiledBefore + 3);
    expect(eventsOfType(result.events, "exiled").map((e) => e.instanceId).sort()).toEqual([bird.id, bell.id, spell.id].sort());
    expect(eventsOfType(result.events, "enteredGraveyard")).toEqual([]);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("R463 a Voidwalker's own card reaches its graveyard, and so does every card that dies with it", () => {
    const state = playing("dc-void-together");
    const walker = put(state, voidwalker.id, slot("p1", "units", 1));
    const bell = put(state, rattle.id, slot("p2", "units", 1));
    walker.markedDestroyed = true;
    bell.markedDestroyed = true;
    const sink = sinkFor(state);
    stateCheck(sink);
    expect(inPile(state, "p1", "graveyard", walker.id)).toBe(true);
    expect(inPile(state, "p2", "graveyard", bell.id)).toBe(true);
    expect(notes(state)).toEqual(["rattle:death:p2"]);
  });

  it("the Radiant Voidwalker exiles only its opponent's cards; a discard is replaced like any move", () => {
    const state = playing("dc-void-radiant");
    put(state, voidwalker.id, slot("p1", "units", 1), { radiant: true });
    const own = put(state, rattle.id, slot("p1", "units", 2));
    const foe = put(state, rattle.id, slot("p2", "units", 2));
    own.markedDestroyed = true;
    foe.markedDestroyed = true;
    const sink = sinkFor(state);
    stateCheck(sink);
    expect(inPile(state, "p1", "graveyard", own.id)).toBe(true);
    expect(inPile(state, "p2", "exile", foe.id)).toBe(true);
    expect(notes(state)).toEqual(["rattle:death:p1"]);
  });

  it("a fired Trap on its way to its graveyard is exiled under a Voidwalker", () => {
    const state = playing("dc-void-trap");
    put(state, voidwalker.id, slot("p1", "units", 1));
    const trap = put(state, gambit.id, slot("p2", "backrow", 1));
    state.players.p2.hero.health = 3;
    const sink = sinkFor(state);
    dealDamage(sink, { source: null, target: { kind: "hero", player: "p2" }, amount: 5 });
    expect(inPile(state, "p2", "exile", trap.id)).toBe(true);
    expect(eventsOfType(sink.events, "exiled").map((e) => e.instanceId)).toEqual([trap.id]);
  });

  it("Second Wind exiles its controller's own cards and no one else's", () => {
    const state = playing("dc-wind");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const own = put(state, grunt.id, slot("p1", "units", 1));
    const foe = put(state, grunt.id, slot("p2", "units", 1));
    own.markedDestroyed = true;
    foe.markedDestroyed = true;
    stateCheck(sinkFor(state));
    expect(inPile(state, "p1", "exile", own.id)).toBe(true);
    expect(inPile(state, "p2", "graveyard", foe.id)).toBe(true);
  });

  it("Pile On goes to the bottom of its owner's library when it resolves, and when it is discarded", () => {
    const state = playing("dc-pile");
    const spell = inHand(state, pileOn.id, "p1")[0] as CardInstance;
    const game = recorder(state);
    const result = game.play({ type: "play", instanceId: spell.id, playerId: "p1" });
    const after = game.state();
    const library = after.players.p1.library;
    expect(library[library.length - 1]?.id).toBe(spell.id);
    expect(notes(after)).toEqual(["pile-on:cry"]);
    const shuffled = eventsOfType(result.events, "shuffledIn");
    expect(shuffled.map((e) => [e.instanceId, e.position])).toEqual([[spell.id, library.length - 1]]);
    // R311: it went in openly, so its owner's library list knows it.
    expect(library[library.length - 1]?.knownAs).toEqual({ defId: pileOn.id, radiant: false });
    expect(replaysTo(game.start, game.log, after)).toBe(true);
    // The other seat is told a card went into p1's library, and not where (R97).
    const seen = viewFor(after, "p2").events.find((event) => event.type === "shuffledIn");
    expect(seen).toMatchObject({ type: "shuffledIn", player: "p1" });
    expect(seen?.type === "shuffledIn" ? seen.position : null).not.toBe(library.length - 1);

    // Discarded from a hand: the same replacement, whatever sends it.
    const second = playing("dc-pile-discard");
    const held = inHand(second, pileOn.id, "p1")[0] as CardInstance;
    const sink = sinkFor(second);
    applyEffects([discard({ target: { of: "instance", instanceId: held.id } })], makeContext(sink, null, { controller: "p1" }));
    const lib = second.players.p1.library;
    expect(lib[lib.length - 1]?.id).toBe(held.id);
    expect(eventTypes(sink.events).filter((type) => type === "discarded" || type === "shuffledIn")).toEqual(["discarded", "shuffledIn"]);
  });

  it("R460 replacements of one move apply in R68's order: the active side's first, each once", () => {
    // p1 active: p1's Voidwalker (a unit) comes before p1's Second Wind (backrow) and before the card
    // itself — exiled, and nothing after it re-checks a card no longer on its way to a graveyard.
    const one = playing("dc-order-one");
    put(one, voidwalker.id, slot("p1", "units", 1));
    put(one, secondWind.id, slot("p1", "backrow", 1));
    const first = inHand(one, pileOn.id, "p1")[0] as CardInstance;
    const a = recorder(one);
    a.play({ type: "play", instanceId: first.id, playerId: "p1" });
    expect(inPile(a.state(), "p1", "exile", first.id)).toBe(true);

    // The opponent's Voidwalker comes after the active side, Pile On's own clause included.
    const two = playing("dc-order-two");
    put(two, voidwalker.id, slot("p2", "units", 1));
    const second = inHand(two, pileOn.id, "p1")[0] as CardInstance;
    const b = recorder(two);
    b.play({ type: "play", instanceId: second.id, playerId: "p1" });
    const library = b.state().players.p1.library;
    expect(library[library.length - 1]?.id).toBe(second.id);

    // Second Wind (backrow) comes before the card itself on the same side.
    const three = playing("dc-order-three");
    put(three, secondWind.id, slot("p1", "backrow", 1));
    const third = inHand(three, pileOn.id, "p1")[0] as CardInstance;
    const c = recorder(three);
    c.play({ type: "play", instanceId: third.id, playerId: "p1" });
    expect(inPile(c.state(), "p1", "exile", third.id)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// "A friendly unit is targeted" (Classic #33 Joro)
// ---------------------------------------------------------------------------

describe("E5 a friendly unit is targeted, E9 attack redirect", () => {
  it("an attack on a unit is moved to the Joro its controller summons from hand", () => {
    const state = playing("dc-joro");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const chosen = put(state, wall.id, slot("p2", "units", 1));
    const decoy = inHand(state, joro.id, "p2")[0] as CardInstance;
    put(state, watcher.id, slot("p2", "backrow", 1));
    const game = recorder(state);

    const result = game.play({ type: "attack", attackerId: attacker.id, targetId: chosen.id, playerId: "p1" });
    const after = game.state();

    expect(eventTypes(result.events).slice(0, 3)).toEqual(["summoned", "redirected", "attackDeclared"]);
    expect(eventsOfType(result.events, "summoned")[0]).toMatchObject({ instanceId: decoy.id, player: "p2", row: "units", lane: 2 });
    expect(eventsOfType(result.events, "redirected")).toEqual([
      { type: "redirected", what: "attack", fromId: chosen.id, toId: decoy.id, byInstanceId: decoy.id },
    ]);
    expect(eventsOfType(result.events, "attackDeclared")[0]?.targetId).toBe(decoy.id);
    // The 2/2 killed the 1/1 Joro and took 1 back; the wall it chose was never hit.
    expect(inPile(after, "p2", "graveyard", decoy.id)).toBe(true);
    expect(findInstance(after, chosen.id)?.damage).toBe(0);
    expect(findInstance(after, attacker.id)?.damage).toBe(1);
    // The declaration reached the traps once, in its window, not again from the frontier (R100); the
    // interposer's summon reached them in that window too, before the combat.
    expect(notes(after)).toEqual([`watch:attack:${decoy.id}`, `watch:summon:${decoy.id}`]);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("no open zone, an attack on the hero, or a forced attack: Joro stays in hand", () => {
    const full = playing("dc-joro-full");
    const attacker = put(full, grunt.id, slot("p1", "units", 1));
    const chosen = put(full, wall.id, slot("p2", "units", 1));
    for (const lane of [2, 3, 4, 5]) put(full, wall.id, slot("p2", "units", lane));
    const decoy = inHand(full, joro.id, "p2")[0] as CardInstance;
    const one = recorder(full);
    one.play({ type: "attack", attackerId: attacker.id, targetId: chosen.id, playerId: "p1" });
    expect(inPile(one.state(), "p2", "hand", decoy.id)).toBe(true);
    expect(findInstance(one.state(), chosen.id)?.damage).toBe(2);

    const open = playing("dc-joro-hero");
    const striker = put(open, grunt.id, slot("p1", "units", 1));
    put(open, wall.id, slot("p2", "units", 1));
    const held = inHand(open, joro.id, "p2")[0] as CardInstance;
    const two = recorder(open);
    two.play({ type: "attack", attackerId: striker.id, targetId: "hero-p2", playerId: "p1" });
    expect(inPile(two.state(), "p2", "hand", held.id)).toBe(true);
  });

  it("the play pipeline's half: a pick of a friendly unit is answered through the same hook", () => {
    const state = playing("dc-joro-pick");
    const chosen = put(state, wall.id, slot("p2", "units", 1));
    const decoy = inHand(state, joro.id, "p2")[0] as CardInstance;
    const sink = sinkFor(state);
    // Its own controller's pick is no opponent's targeting.
    expect(answerTargeting(sink, { target: chosen, by: "p2", what: "target" })).toBeNull();
    const moved = answerTargeting(sink, { target: chosen, by: "p1", what: "target" });
    expect(moved?.id).toBe(decoy.id);
    expect(eventsOfType(sink.events, "redirected")).toEqual([
      { type: "redirected", what: "target", fromId: chosen.id, toId: decoy.id, byInstanceId: decoy.id },
    ]);
    expect(unitAt(state, "p2", 2)?.summonedTurn).toBe(state.turn);
  });

  it("R177 a Joro that cannot answer leaves the attacker's view as any other hand card would", () => {
    const build = (card: string): ReturnType<typeof recorder> => {
      const state = playing("dc-joro-hidden");
      const attacker = put(state, grunt.id, slot("p1", "units", 1));
      put(state, wall.id, slot("p2", "units", 1));
      inHand(state, card, "p2");
      const game = recorder(state);
      game.play({ type: "attack", attackerId: attacker.id, targetId: "hero-p2", playerId: "p1" });
      return game;
    };
    expect(viewFor(build(joro.id).state(), "p1")).toEqual(viewFor(build(grunt.id).state(), "p1"));
  });
});

// ---------------------------------------------------------------------------
// R97, R177: what each seat reads of the events these moments emit
// ---------------------------------------------------------------------------

describe("R97, R177 the replacement events in both views", () => {
  it("a redirect, a flicker and the trap that did them are public to both seats once fired", () => {
    const state = playing("dc-views");
    const attacker = put(state, grunt.id, slot("p1", "units", 1));
    const trap = put(state, gambit.id, slot("p2", "backrow", 1));
    state.players.p2.hero.health = 2;
    const game = recorder(state);
    game.play({ type: "attack", attackerId: attacker.id, targetId: "hero-p2", playerId: "p1" });
    for (const viewer of ["p1", "p2"] as const) {
      const events = viewFor(game.state(), viewer).events;
      expect(events.filter((event) => event.type === "redirected")).toEqual([
        { type: "redirected", what: "damage", fromId: "hero-p2", toId: "hero-p1", byInstanceId: trap.id },
      ]);
      // `trapFired` names the trap to its controller only, as every firing does; the other seat reads
      // the card where it went, its owner's graveyard.
      expect(events.filter((event) => event.type === "trapFired").map((event) => event.type === "trapFired" && event.defId)).toEqual([
        viewer === "p2" ? gambit.id : "hidden",
      ]);
      const graveyard = viewFor(game.state(), viewer)[viewer === "p2" ? "you" : "opponent"].graveyard;
      expect(graveyard.map((card) => card.instanceId)).toContain(trap.id);
    }

    const flick = playing("dc-views-flicker");
    const unit = put(flick, grunt.id, slot("p1", "units", 1));
    put(flick, shadowstep.id, slot("p1", "backrow", 1));
    const spell = inHand(flick, storm.id, "p1")[0] as CardInstance;
    const other = recorder(flick);
    other.play({ type: "play", instanceId: spell.id, playerId: "p1" });
    for (const viewer of ["p1", "p2"] as const) {
      const flickered = viewFor(other.state(), viewer).events.filter((event) => event.type === "flickered");
      expect(flickered).toEqual([{ type: "flickered", player: "p1", instanceId: unit.id, defId: grunt.id, row: "units", lane: 1 }]);
    }
  });

  it("R177 a face-down Blood Moon or Shadowstep that declines leaves the other seat's view as another trap would", () => {
    const build = (trapId: string): GameState => {
      const state = playing("dc-views-decline");
      put(state, trapId, slot("p1", "backrow", 1));
      const foe = put(state, rattle.id, slot("p2", "units", 1));
      foe.markedDestroyed = true;
      const game = recorder(state);
      // p1 heals its own hero (no enemy of p1's trap) and p2's unit dies (none of p1's units).
      const cure = inHand(game.state(), mend.id, "p1")[0] as CardInstance;
      game.play({ type: "play", instanceId: cure.id, targets: [{ pick: "hero", player: "p1" }], playerId: "p1" });
      return game.state();
    };
    const moon = build(bloodMoon.id);
    const step = build(shadowstep.id);
    const plain = build(gambit.id);
    expect(viewFor(moon, "p2")).toEqual(viewFor(plain, "p2"));
    expect(viewFor(step, "p2")).toEqual(viewFor(plain, "p2"));
  });
});
