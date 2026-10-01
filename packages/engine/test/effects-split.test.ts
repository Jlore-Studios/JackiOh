// B5 E37 random split damage (`effects/split.ts`): "deal N damage split among enemies" — N hits of 1,
// each a damage instance of its own on a random enemy still standing (Classic+ #3's Death).

import { describe, expect, it } from "vitest";
import { damageSplit } from "../src/effects";
import { applyEffects, makeContext } from "../src/resolve";
import { stateCheck } from "../src/stateCheck";
import { cloneState, type GameState } from "../src/state";
import { bolt, grunt, rattle, snake, wall, warded, playing } from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

function snakeDies(state: GameState): ReturnType<typeof sinkFor> {
  const sink = sinkFor(state);
  stateCheck(sink);
  return sink;
}

describe("E37 damageSplit", () => {
  it("Classic+ #3's Death: one 1-damage hit per token on a random enemy, deterministic from the seed", () => {
    const build = (): { state: GameState; snakeId: string; ids: string[] } => {
      const state = playing("dc-split");
      const dying = put(state, snake.id, slot("p1", "units", 1));
      dying.counters.plague = 3;
      dying.markedDestroyed = true;
      const a = put(state, wall.id, slot("p2", "units", 1));
      const b = put(state, wall.id, slot("p2", "units", 2));
      return { state, snakeId: dying.id, ids: [a.id, b.id, "hero-p2"] };
    };
    const one = build();
    const sink = snakeDies(one.state);
    const hits = eventsOfType(sink.events, "damage");
    expect(hits).toHaveLength(3);
    for (const hit of hits) {
      expect(hit.sourceId).toBe(one.snakeId);
      expect(hit.amount).toBe(1);
      expect(one.ids).toContain(hit.targetId);
    }
    // The same seed and board draw the same enemies (R60: the match rng).
    const two = build();
    const again = eventsOfType(snakeDies(two.state).events, "damage");
    expect(again.map((hit) => hit.targetId)).toEqual(hits.map((hit) => hit.targetId));
  });

  it("an enemy an earlier hit killed is no longer standing, so no later hit lands on it", () => {
    for (const seed of ["dc-split-a", "dc-split-b", "dc-split-c", "dc-split-d"]) {
      const state = playing(seed);
      const source = put(state, grunt.id, slot("p1", "units", 1));
      const victim = put(state, rattle.id, slot("p2", "units", 1));
      const sink = sinkFor(state);
      applyEffects([damageSplit({ amount: 6, among: "enemies" })], makeContext(sink, source));
      const hits = eventsOfType(sink.events, "damage");
      expect(hits).toHaveLength(6);
      expect(hits.filter((hit) => hit.targetId === victim.id).length).toBeLessThanOrEqual(1);
      const onHero = hits.filter((hit) => hit.targetId === "hero-p2").length;
      expect(state.players.p2.hero.health).toBe(30 - onHero);
    }
  });

  it("hits of more than 1, the last taking what is left; an enemy Unit pool leaves the hero out", () => {
    const state = playing("dc-split-per-hit");
    const source = put(state, grunt.id, slot("p1", "units", 1));
    const target = put(state, wall.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    applyEffects([damageSplit({ amount: 5, perHit: 2, among: "enemyUnits" })], makeContext(sink, source));
    expect(eventsOfType(sink.events, "damage").map((hit) => [hit.targetId, hit.amount])).toEqual([
      [target.id, 2],
      [target.id, 2],
      [target.id, 1],
    ]);
    // With no enemy unit standing there is nothing to hit.
    const empty = cloneState(state);
    empty.players.p2.units = empty.players.p2.units.map(() => null);
    const quiet = sinkFor(empty);
    applyEffects([damageSplit({ amount: 3, among: "enemyUnits" })], makeContext(quiet, null, { controller: "p1" }));
    expect(quiet.events).toEqual([]);
  });

  it("E35 a Spell's split passes a unit immune to Spells by", () => {
    const state = playing("dc-split-immune");
    put(state, warded.id, slot("p2", "units", 1));
    const spell = inHand(state, bolt.id, "p1")[0];
    if (spell === undefined) throw new Error("no spell");
    const sink = sinkFor(state);
    applyEffects([damageSplit({ amount: 4, among: "enemies" })], makeContext(sink, spell));
    expect(eventsOfType(sink.events, "damage").map((hit) => hit.targetId)).toEqual(["hero-p2", "hero-p2", "hero-p2", "hero-p2"]);
  });
});
