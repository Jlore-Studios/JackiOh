// Replay with play pipeline B's cards (SPEC §9.2, §9.3; docs/classic-sets.md B5 E11, E12, E15, B4.5):
// whole games on decks of this workstream's fixtures — graveyard permissions, random casts, casts from
// the graveyard, price rules, Forever&'s return, a Tribute card — played by §10.7's random policy,
// then folded from `(seed, decks, log)` by `replay.fold` and compared state for state. The run
// asserts that it reached what it exists to replay: plays from the graveyard, random casts, prompts
// answered mid-cast, and a state at rest that never carries a cast's mode.

import type { Action, ActionBody, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { AI_END_TURN_PROBABILITY } from "../src/config";
import { beginGame, legalActions, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createRng } from "../src/rng";
import { createGame, type GameState } from "../src/state";
import { setupCatalog } from "./fixtures/harness";
import {
  RANDOM_POOL,
  castTrap,
  discoverSpell,
  forever,
  graveSpell,
  graveUnit,
  joggBox,
  modeSpell,
  monkey,
  namedCaster,
  plantation,
  registerPipelineB,
  secondWind,
  solarius,
  targetSpell,
  tax,
  theirChoice,
  titan,
  toeCracker,
  trickster,
  tyrant,
  zeroSpell,
} from "./fixtures/playPipelineB";

const DECK = [
  secondWind,
  plantation,
  joggBox,
  solarius,
  forever,
  trickster,
  toeCracker,
  monkey,
  tax,
  graveSpell,
  zeroSpell,
  targetSpell,
  modeSpell,
  discoverSpell,
  theirChoice,
  titan,
  namedCaster,
  tyrant,
  castTrap,
  graveUnit,
].map((card) => card.id);

const SEEDS = Array.from({ length: 8 }, (_, at) => `pb-replay-${at + 1}`);
/** Each seed is played once and folded twice; long random-policy games on these decks take seconds. */
const TEST_TIMEOUT_MS = 120_000;
const STEP_CAP = 4000;

type Played = { state: GameState; log: Action[]; events: GameEvent[]; hashes: string[] };

/** §10.7's random policy over `legalActions`, never conceding or offering a draw. */
function play(seed: string): Played {
  setupCatalog();
  registerPipelineB();
  let state = beginGame(createGame({ seed, decks: [DECK, DECK] })).state;
  const policy = createRng(`policy-${seed}`);
  const log: Action[] = [];
  const events: GameEvent[] = [];
  const hashes: string[] = [];
  for (let step = 0; state.result === null; step += 1) {
    if (step > STEP_CAP) throw new Error(`game ${seed} did not finish`);
    const player = seatToAct(state);
    const actions = legalActions(state, player).filter(
      (action) => action.type !== "concede" && action.type !== "offerDraw" && action.type !== "answerDraw",
    );
    const others = actions.filter((action) => action.type !== "endTurn");
    const endTurn = actions.find((action) => action.type === "endTurn");
    const chosen =
      others.length === 0 || (endTurn !== undefined && policy.chance(AI_END_TURN_PROBABILITY))
        ? (endTurn ?? (others[policy.int(others.length)] as ActionBody))
        : (others[policy.int(others.length)] as ActionBody);
    const action = { ...chosen, playerId: player, nonce: `r${log.length}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`${action.type} rejected in ${seed}: ${result.error}`);
    log.push(action);
    events.push(...result.events);
    state = result.state;
    hashes.push(hashState(state));
    // R452: a cast's mode is in force only while its steps run; a state at rest never carries it.
    if (state.castsResolving !== undefined) throw new Error(`a cast mode leaked into a state at rest in ${seed}`);
  }
  return { state, log, events, hashes };
}

describe("play pipeline B folds exactly (§9.2, §9.3)", () => {
  it("R452 R453 R454 R455 games on graveyard plays, random casts and price rules replay to the same state, step by step", () => {
    let graveyardPlays = 0;
    let randomCasts = 0;
    let prompts = 0;
    let returns = 0;
    for (const seed of SEEDS) {
      const live = play(seed);
      const folded = fold({ seed, decks: [DECK, DECK], log: live.log });
      expect(folded.errors).toEqual([]);
      expect(hashState(folded.state)).toBe(hashState(live.state));

      // Step by step, through JSON at every step: each action from the round-tripped state lands on
      // the state the live game had.
      let state = beginGame(createGame({ seed, decks: [DECK, DECK] })).state;
      live.log.forEach((action, at) => {
        state = reduce(JSON.parse(JSON.stringify(state)) as GameState, action).state;
        expect(hashState(state)).toBe(live.hashes[at]);
      });

      graveyardPlays += live.events.filter((event) => event.type === "cardPlayed" && event.from === "graveyard").length;
      randomCasts += live.events.filter(
        (event) => event.type === "cardPlayed" && event.costPaid === 0 && RANDOM_POOL.includes(event.defId),
      ).length;
      prompts += live.events.filter((event) => event.type === "promptOpened").length;
      returns += live.events.filter((event) => event.type === "addedToHand" && event.defId !== joggBox.id).length;
    }
    // The run reached what it exists to replay.
    expect(graveyardPlays).toBeGreaterThan(0);
    expect(randomCasts).toBeGreaterThan(0);
    expect(prompts).toBeGreaterThan(0);
    expect(returns).toBeGreaterThan(0);
  }, TEST_TIMEOUT_MS);
});
