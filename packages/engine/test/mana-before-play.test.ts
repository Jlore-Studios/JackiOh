// `EffectContext.manaBeforePlay` (Classic #22 Mid Runner: "If you had 4 or more mana when you played
// this"): the player's current mana as the play began — at §10.5 step 1, before step 2 pays — or as a
// cast began, handed to the played card's own Cry and every continuation of it, across a pause and a
// JSON round trip.

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { chooseMode, damage } from "../src/effects";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { castCard } from "../src/resolve";
import type { CardScripts, EffectContext } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type GameState } from "../src/state";
import { inHand, sinkFor } from "./fixtures/harness";
import { only, pbAct, pbPlaying, roundTrip } from "./fixtures/playPipelineB";

function unit(name: string, index: number): CardDef {
  return {
    id: `pbm-${name}`,
    index: String(index),
    name: `${name} (mana before play)`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 2,
    base: { attack: 2, health: 2, keywords: [], text: name },
    radiant: { attack: 4, health: 4, keywords: [], text: name },
  };
}
/** Its Cry deals 4 when its player had 4 or more mana as they played it, else 1. */
const runner = unit("runner", 4621);
/** The same, read in the step a prompt re-enters. */
const askingRunner = unit("asking-runner", 4622);

function hit(ctx: EffectContext): ReturnType<typeof damage>[] {
  return [damage({ to: { of: "enemyHero" }, amount: (ctx.manaBeforePlay ?? 0) >= 4 ? 4 : 1 })];
}

const SCRIPTS: Record<string, CardScripts> = {
  [runner.id]: { base: { cry: hit }, radiant: { cry: hit } },
  [askingRunner.id]: {
    base: { cry: () => [chooseMode({ options: ["go"], step: "go" })], resume: { go: hit } },
    radiant: {},
  },
};

function playing(seed: string, mana: number): GameState {
  const state = pbPlaying(seed);
  registerCatalog({ ...registeredCatalog(), [runner.id]: runner, [askingRunner.id]: askingRunner });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.players.p1.mana.current = mana;
  return state;
}

describe("the player's mana as the play began (Classic #22)", () => {
  it("reads the mana before step 2 paid, not after", () => {
    const rich = playing("mana-before-4", 4);
    const card = only(inHand(rich, runner.id, "p1"));
    const after = pbAct(rich, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    expect(after.players.p1.mana.current).toBe(2);
    expect(after.players.p2.hero.health).toBe(rich.players.p2.hero.health - 4);

    const poor = playing("mana-before-3", 3);
    const other = only(inHand(poor, runner.id, "p1"));
    const then = pbAct(poor, { type: "play", instanceId: other.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    expect(then.players.p2.hero.health).toBe(poor.players.p2.hero.health - 1);
  });

  it("is carried into the step a prompt re-enters, across a JSON round trip", () => {
    const state = playing("mana-before-pause", 4);
    const card = only(inHand(state, askingRunner.id, "p1"));
    const paused = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    expect(paused.pending?.kind).toBe("mode");
    const round = roundTrip(paused);
    const answer = only(legalActions(paused, "p1").filter((action) => action.type === "answer"));
    const live = pbAct(paused, { ...answer, playerId: "p1" });
    const again = pbAct(round, { ...answer, playerId: "p1" });
    expect(hashState(again)).toBe(hashState(live));
    expect(live.players.p2.hero.health).toBe(state.players.p2.hero.health - 4);
  });

  it("a cast records its caster's mana as the cast begins", () => {
    const state = playing("mana-before-cast", 5);
    const card = newInstance(state, runner.id, "p1", { z: "hand", player: "p1" });
    state.players.p1.hand.push(card);
    const before = state.players.p2.hero.health;
    castCard(sinkFor(state), card);
    expect(state.players.p2.hero.health).toBe(before - 4);
    expect(state.players.p1.mana.current).toBe(5);
  });
});
