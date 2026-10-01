// B5 E7 set health and E8 heal into damage, the effect verbs (`effects/health.ts`). The replacement
// that turns a heal into damage as it happens is `replacements.test.ts`'s; this file proves the verbs
// a card writes: `setHealth` (Classic #29) and `convertHealing`.

import { describe, expect, it } from "vitest";
import { convertHealing, heal, setHealth } from "../src/effects";
import { dealDamage, healHero } from "../src/damage";
import { applyEffects, makeContext } from "../src/resolve";
import { viewFor } from "../src/viewFor";
import { bloodMoon, gambit, grunt, playing, recorder, replaysTo, vitalKill } from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

describe("E7 setHealth", () => {
  it("sets either hero's health, up or down, with no pipeline: no damage, no heal, no replacement", () => {
    const state = playing("dc-set-health");
    const self = put(state, grunt.id, slot("p1", "units", 1));
    put(state, gambit.id, slot("p1", "backrow", 1));
    state.players.p1.hero.armor = 5;
    state.players.p2.hero.health = 40;
    const sink = sinkFor(state);
    const ctx = makeContext(sink, self);
    applyEffects([setHealth({ to: { of: "enemyHero" }, value: 13 }), setHealth({ to: { of: "selfHero" }, value: 13 })], ctx);
    expect(state.players.p2.hero.health).toBe(13);
    expect(state.players.p1.hero.health).toBe(13);
    // Down to 0 is no hit either: a Final Gambit has nothing to answer.
    applyEffects([setHealth({ to: { of: "selfHero" }, value: 0 })], ctx);
    expect(state.players.p1.hero.health).toBe(0);
    expect(eventsOfType(sink.events, "healthSet")).toEqual([
      { type: "healthSet", player: "p2", health: 13, sourceId: self.id },
      { type: "healthSet", player: "p1", health: 13, sourceId: self.id },
      { type: "healthSet", player: "p1", health: 0, sourceId: self.id },
    ]);
    expect(eventsOfType(sink.events, "damage")).toEqual([]);
    expect(eventsOfType(sink.events, "healed")).toEqual([]);
    expect(eventsOfType(sink.events, "trapFired")).toEqual([]);
  });

  it("a target that is not a hero fizzles", () => {
    const state = playing("dc-set-health-unit");
    const self = put(state, grunt.id, slot("p1", "units", 1));
    const sink = sinkFor(state);
    applyEffects([setHealth({ to: { of: "self" }, value: 13 })], makeContext(sink, self));
    expect(sink.events).toEqual([]);
  });

  it("R97 the event is public to both seats, and the game replays", () => {
    const state = playing("dc-set-health-view");
    const spell = inHand(state, vitalKill.id, "p1")[0];
    if (spell === undefined) throw new Error("no spell");
    const game = recorder(state);
    game.play({ type: "play", instanceId: spell.id, targets: [{ pick: "hero", player: "p2" }], playerId: "p1" });
    for (const viewer of ["p1", "p2"] as const) {
      const seen = viewFor(game.state(), viewer).events.filter((event) => event.type === "healthSet");
      expect(seen).toEqual([{ type: "healthSet", player: "p2", health: 13, sourceId: spell.id }]);
    }
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);
  });
});

describe("E8 convertHealing", () => {
  it("installs a this-turn modifier: heals on its controller's enemies become Pierce damage from the card", () => {
    const state = playing("dc-convert");
    const moon = put(state, bloodMoon.id, slot("p2", "backrow", 2));
    moon.faceUp = true;
    const sink = sinkFor(state);
    applyEffects([convertHealing()], makeContext(sink, moon));
    expect(state.players.p2.mods).toEqual([
      { kind: "healToDamage", converterId: moon.id, expiry: { until: "thisTurn", turn: state.turn }, id: expect.any(String) as string },
    ]);
    state.players.p1.hero.armor = 2;
    healHero(sink, "p1", 7);
    expect(state.players.p1.hero.health).toBe(23);
    // A unit of the enemy's: a heal of 3 on it deals 3.
    const body = put(state, grunt.id, slot("p1", "units", 1));
    applyEffects([heal({ target: { of: "instance", instanceId: body.id }, amount: 3 })], makeContext(sink, body));
    expect(body.damage).toBe(3);
    // The badge both seats read says what it does (R169).
    const label = "Healing on your enemies deals Pierce damage instead";
    expect(viewFor(state, "p2").you.modifiers.map((mod) => mod.label)).toEqual([label]);
    expect(viewFor(state, "p1").opponent.modifiers.map((mod) => mod.label)).toEqual([label]);
  });

  it("a converted heal is a new damage instance: Divine Shield and the lethal window meet it", () => {
    const state = playing("dc-convert-lethal");
    const moon = put(state, bloodMoon.id, slot("p2", "backrow", 2));
    const sink = sinkFor(state);
    applyEffects([convertHealing()], makeContext(sink, moon));
    put(state, gambit.id, slot("p1", "backrow", 1));
    state.players.p1.hero.health = 3;
    healHero(sink, "p1", 5);
    // p1's Final Gambit sent the 5 to p2's hero.
    expect(state.players.p1.hero.health).toBe(3);
    expect(state.players.p2.hero.health).toBe(25);
    expect(eventsOfType(sink.events, "redirected")).toHaveLength(1);
  });

  it("with no card to come from, it does nothing", () => {
    const state = playing("dc-convert-none");
    const sink = sinkFor(state);
    applyEffects([convertHealing()], makeContext(sink, null));
    expect(state.players.p1.mods).toEqual([]);
    dealDamage(sink, { source: null, target: { kind: "hero", player: "p1" }, amount: 1 });
    expect(state.players.p1.hero.health).toBe(29);
  });
});
