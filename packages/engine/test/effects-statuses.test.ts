// B5 E35 unit statuses (`effects/statuses.ts`): Berserk (Classic+ #19.2 sends Classic+ #19.5 there)
// and "may attack again" (Classic+ #73.1's Classic Golem after a kill).

import { describe, expect, it } from "vitest";
import { bounce, goBerserk, mayAttackAgain, vanilla } from "../src/effects";
import { canAttack } from "../src/combat";
import { legalActions } from "../src/reduce";
import { isBerserk } from "../src/restrictions";
import { applyEffects, makeContext } from "../src/resolve";
import { findInstance } from "../src/state";
import { viewFor } from "../src/viewFor";
import { botLoser, grunt, playing, recorder, replaysTo } from "./fixtures/damage-combat";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";

describe("E35 goBerserk", () => {
  it("sets the status, marks it in both views, and a Berserk Bot Loser attacks its own hero at its start of turn", () => {
    const state = playing("dc-berserk");
    const bot = put(state, botLoser.id, slot("p1", "units", 2));
    const sink = sinkFor(state);
    applyEffects([goBerserk({ target: { of: "instance", instanceId: bot.id } })], makeContext(sink, null, { controller: "p2" }));
    expect(isBerserk(bot)).toBe(true);
    expect(eventsOfType(sink.events, "marked")).toEqual([
      { type: "marked", instanceId: bot.id, mark: "berserk", color: "red", added: true },
    ]);
    // A second call changes nothing and says nothing.
    applyEffects([goBerserk({ target: { of: "instance", instanceId: bot.id } })], makeContext(sink, null));
    expect(eventsOfType(sink.events, "marked")).toHaveLength(1);
    for (const viewer of ["p1", "p2"] as const) {
      const view = viewFor(state, viewer);
      const side = viewer === "p1" ? view.you : view.opponent;
      expect(side.units[1]?.berserk).toBe(true);
    }

    // Its card's own text makes the attacks: at p1's next start of turn, 5 to p1's own hero.
    const game = recorder(state);
    game.play({ type: "endTurn", playerId: "p1" });
    expect(game.state().players.p1.hero.health).toBe(30);
    const back = game.play({ type: "endTurn", playerId: "p2" });
    const after = game.state();
    expect(after.players.p1.hero.health).toBe(25);
    expect(eventsOfType(back.events, "attackDeclared")).toEqual([
      { type: "attackDeclared", attackerId: bot.id, targetId: "hero-p1", forced: true },
    ]);
    expect(replaysTo(game.start, game.log, after)).toBe(true);
  });

  it("a unit that can't go Berserk does not, and the status goes when the unit leaves the field, not with its text", () => {
    const state = playing("dc-berserk-never");
    const calm = put(state, botLoser.id, slot("p1", "units", 1), { radiant: true });
    const wild = put(state, grunt.id, slot("p1", "units", 2));
    const sink = sinkFor(state);
    const ctx = makeContext(sink, null, { controller: "p1" });
    applyEffects([goBerserk({ target: { of: "instance", instanceId: calm.id } })], ctx);
    expect(isBerserk(calm)).toBe(false);
    applyEffects([goBerserk({ target: { of: "instance", instanceId: wild.id } })], ctx);
    // A Vanilla takes text; Berserk is not text.
    applyEffects([vanilla({ target: { of: "instance", instanceId: wild.id } })], ctx);
    expect(isBerserk(wild)).toBe(true);
    // Leaving the field resets it (R78).
    applyEffects([bounce({ target: { of: "instance", instanceId: wild.id } })], ctx);
    expect(findInstance(state, wild.id)?.berserk).toBeUndefined();
  });
});

describe("E35 mayAttackAgain", () => {
  it("a unit that has attacked gets a fresh exertion; one that entered this turn is no longer sick", () => {
    const state = playing("dc-again");
    const striker = put(state, grunt.id, slot("p1", "units", 1));
    const fresh = put(state, grunt.id, slot("p1", "units", 2));
    fresh.summonedTurn = state.turn;
    const game = recorder(state);
    game.play({ type: "attack", attackerId: striker.id, targetId: "hero-p2", playerId: "p1" });
    let now = game.state();
    const hasAttack = (id: string): boolean =>
      legalActions(now, "p1").some((action) => action.type === "attack" && action.attackerId === id);
    expect(hasAttack(striker.id)).toBe(false);
    expect(hasAttack(fresh.id)).toBe(false);

    const sink = sinkFor(now);
    const ctx = makeContext(sink, null, { controller: "p1" });
    const liveStriker = findInstance(now, striker.id);
    const liveFresh = findInstance(now, fresh.id);
    if (liveStriker === undefined || liveFresh === undefined) throw new Error("units gone");
    applyEffects(
      [mayAttackAgain({ target: { of: "instance", instanceId: striker.id } }), mayAttackAgain({ target: { of: "instance", instanceId: fresh.id } })],
      ctx,
    );
    now = sink.state;
    expect(hasAttack(striker.id)).toBe(true);
    expect(canAttack(now, liveFresh, { kind: "hero", player: "p2" })).toBe(true);
  });

  it("it lends no attack a unit could not otherwise make", () => {
    const state = playing("dc-again-def");
    const guard = put(state, grunt.id, slot("p1", "units", 1));
    guard.position = "DEF";
    const sink = sinkFor(state);
    applyEffects([mayAttackAgain({ target: { of: "self" } })], makeContext(sink, guard));
    expect(canAttack(state, guard, { kind: "hero", player: "p2" })).toBe(false);
  });
});
