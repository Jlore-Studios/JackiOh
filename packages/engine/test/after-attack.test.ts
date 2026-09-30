// "After this attacks" (`Script.afterAttack`): the attacker's hook, run once the state check that closes
// each of its combats has run — a declared attack's (§4.2 step 5) and a forced one's (R53) — also when
// it died there, then on the snapshot it fought with (R78, R89); never for an attack called off before
// it fought (R44). Classic #13 Boots on the Ground, Classic+ #73.1 Classic Golem and Core #32 Prem
// Panther (R426) are its users.

import { describe, expect, it } from "vitest";
import { forcedAttacksOn } from "../src/effects";
import { AFTER_ATTACK_WORK } from "../src/combat";
import { applyEffects, makeContext } from "../src/resolve";
import { owedWork } from "../src/work";
import {
  answer,
  deathAsker,
  grunt,
  notes,
  pawn,
  playing,
  recorder,
  replaysTo,
  roundTrip,
  veteran,
  veteranAsker,
  wall,
} from "./fixtures/damage-combat";
import { put, sinkFor, slot } from "./fixtures/harness";

describe("Script.afterAttack", () => {
  it("a declared attack: the hook runs once the check has closed, with the units it destroyed", () => {
    const state = playing("dc-after-declared");
    const striker = put(state, veteran.id, slot("p1", "units", 1));
    striker.buffs = { attack: 1, health: 3 };
    const victim = put(state, grunt.id, slot("p2", "units", 1));
    const game = recorder(state);
    game.play({ type: "attack", attackerId: striker.id, targetId: victim.id, playerId: "p1" });
    expect(notes(game.state())).toEqual([`after:${victim.id}:${victim.id}:true:false:field`]);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("an attacker that died in its combat runs the hook on the snapshot it fought with", () => {
    const state = playing("dc-after-died");
    const striker = put(state, veteran.id, slot("p1", "units", 1));
    const blocker = put(state, wall.id, slot("p2", "units", 1));
    blocker.buffs = { attack: 4, health: 0 };
    const game = recorder(state);
    game.play({ type: "attack", attackerId: striker.id, targetId: blocker.id, playerId: "p1" });
    expect(notes(game.state())).toEqual([`after:${blocker.id}::false:false:field`]);
    expect(game.state().players.p1.graveyard.map((card) => card.id)).toContain(striker.id);
  });

  it("a forced attack runs it too, flagged forced; an attack called off before it fought does not", () => {
    const state = playing("dc-after-forced");
    const striker = put(state, veteran.id, slot("p1", "units", 1));
    const sink = sinkFor(state);
    applyEffects([forcedAttacksOn({ target: { of: "enemyHero" }, attackers: "self" })], makeContext(sink, null, { controller: "p1" }));
    expect(notes(state)).toEqual(["after:hero-p2::true:true:field"]);

    const cancelled = playing("dc-after-cancelled");
    const idle = put(cancelled, veteran.id, slot("p1", "units", 1));
    put(cancelled, pawn.id, slot("p2", "backrow", 1));
    const game = recorder(cancelled);
    game.play({ type: "attack", attackerId: idle.id, targetId: "hero-p2", playerId: "p1" });
    expect(notes(game.state())).toEqual([]);
    expect(game.state().players.p2.hero.health).toBe(30);
  });

  it("R113 a question inside the hook pauses it; the rest and the check after it survive a round trip", () => {
    const state = playing("dc-after-pause");
    const striker = put(state, veteranAsker.id, slot("p1", "units", 1));
    const game = recorder(state);
    game.play({ type: "attack", attackerId: striker.id, targetId: "hero-p2", playerId: "p1" });
    const paused = game.state();
    expect(notes(paused)).toEqual(["after:before"]);
    expect(paused.pending?.playerId).toBe("p1");
    expect(owedWork(paused, AFTER_ATTACK_WORK)).toHaveLength(1);
    const resumed = answer(roundTrip(paused)).state;
    expect(notes(resumed)).toEqual(["after:before", "after:answered:true", "after:tail"]);
    expect(resumed.work).toEqual([]);
    const pending = paused.pending;
    if (pending === null) throw new Error("expected a prompt");
    game.play({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: "p1" });
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });

  it("R113 a Death's question in the check before it owes the hook behind the Death", () => {
    const state = playing("dc-after-death-asks");
    const striker = put(state, veteran.id, slot("p1", "units", 1));
    const doomed = put(state, deathAsker.id, slot("p2", "units", 1));
    const game = recorder(state);
    game.play({ type: "attack", attackerId: striker.id, targetId: doomed.id, playerId: "p1" });
    const paused = game.state();
    expect(notes(paused)).toEqual(["death:ask"]);
    expect(owedWork(paused, AFTER_ATTACK_WORK)).toHaveLength(1);
    const resumed = answer(roundTrip(paused)).state;
    // The Death finishes, then the hook — with the kill the check made, and its attacker alive.
    expect(notes(resumed)).toEqual(["death:ask", "death:answered", `after:${doomed.id}:${doomed.id}:true:false:field`]);
    expect(resumed.work).toEqual([]);
  });
});
