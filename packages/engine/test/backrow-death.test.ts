// §4.5 step 3's Death for a backrow card (docs/classic-sets.md A5, Classic+ #61 Bauble Bubble, Classic+
// #12.8 Frostspatula destroyed in the backrow): a Field Spell, Trap or Field Trap that prints Death
// fires it when it goes from its backrow zone to a graveyard — destroyed or sacrificed, as a Unit's
// does — in R68's order beside the units of its side, and never on a bounce or an exile (§6.2).

import { describe, expect, it } from "vitest";
import { bounce, exile } from "../src/effects";
import { applyEffects, makeContext } from "../src/resolve";
import { sacrificeNow, stateCheck } from "../src/stateCheck";
import type { CardInstance } from "../src/state";
import { bauble, grunt, notes, playing, rattle, recorder, replaysTo, storm, voidwalker } from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

describe("§4.5 step 3: Death for a collected backrow card", () => {
  it("a destroyed Field Spell fires its Death, in R68's order: the active side's units, its backrow, then the other side", () => {
    const state = playing("dc-backrow-death");
    const foeBack = put(state, bauble.id, slot("p2", "backrow", 1));
    const foe = put(state, rattle.id, slot("p2", "units", 3));
    const pop = put(state, bauble.id, slot("p1", "backrow", 2));
    const own = put(state, rattle.id, slot("p1", "units", 4));
    for (const card of [foeBack, foe, pop, own]) card.markedDestroyed = true;
    const sink = sinkFor(state);
    stateCheck(sink);
    expect(eventsOfType(sink.events, "destroyed").map((event) => event.instanceId)).toEqual([own.id, pop.id, foe.id, foeBack.id]);
    expect(notes(state)).toEqual(["rattle:death:p1", "bauble:death", "rattle:death:p2", "bauble:death"]);
  });

  it("the Death hooks run in that order, and a bounce or an exile is no Death", () => {
    const state = playing("dc-backrow-order");
    const own = put(state, rattle.id, slot("p1", "units", 1));
    const pop = put(state, bauble.id, slot("p1", "backrow", 1));
    const foe = put(state, rattle.id, slot("p2", "units", 1));
    const spell = inHand(state, storm.id, "p1")[0] as CardInstance;
    pop.markedDestroyed = true;
    const game = recorder(state);
    game.play({ type: "play", instanceId: spell.id, playerId: "p1" });
    expect(notes(game.state())).toEqual(["rattle:death:p1", "bauble:death", "rattle:death:p2"]);
    expect([own, pop, foe].map((card) => game.state().players[card.owner].graveyard.some((c) => c.id === card.id))).toEqual([
      true,
      true,
      true,
    ]);
    expect(replaysTo(game.start, game.log, game.state())).toBe(true);

    const quiet = playing("dc-backrow-no-death");
    const one = put(quiet, bauble.id, slot("p1", "backrow", 1));
    const two = put(quiet, bauble.id, slot("p1", "backrow", 2));
    const sink = sinkFor(quiet);
    const ctx = makeContext(sink, null, { controller: "p1" });
    applyEffects([bounce({ target: { of: "instance", instanceId: one.id } }), exile({ target: { of: "instance", instanceId: two.id } })], ctx);
    stateCheck(sink);
    expect(notes(quiet)).toEqual([]);
  });

  it("a sacrificed backrow card fires its Death; one exiled instead of reaching its graveyard does not (R461)", () => {
    const state = playing("dc-backrow-sacrifice");
    const pop = put(state, bauble.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    sacrificeNow(sink, pop);
    expect(notes(state)).toEqual(["bauble:death"]);

    const walled = playing("dc-backrow-void");
    put(walled, voidwalker.id, slot("p2", "units", 1));
    const gone = put(walled, bauble.id, slot("p1", "backrow", 1));
    put(walled, grunt.id, slot("p1", "units", 1));
    gone.markedDestroyed = true;
    const voidSink = sinkFor(walled);
    stateCheck(voidSink);
    expect(notes(walled)).toEqual([]);
    expect(walled.players.p1.exile.map((card) => card.id)).toEqual([gone.id]);
    expect(eventsOfType(voidSink.events, "destroyed")).toEqual([]);
  });
});
