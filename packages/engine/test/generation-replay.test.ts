// Whole games on decks of the generation fixtures (docs/classic-sets.md B5 E19, E23–E25), played by
// the random policy of §10.7 and folded back from their logs (§9.2, §9.3, `replay.ts`): every
// placement prompt, fusion, digest id, transform and recruit a game reaches replays to the same
// state, and every state a prompt paused on survives a JSON round trip.

import type { Action, ActionBody } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { AI_END_TURN_PROBABILITY, DECK_SIZE } from "../src/config";
import { beginGame, legalActions, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createRng } from "../src/rng";
import { createGame, type GameState } from "../src/state";
import {
  bigBody,
  body,
  charger,
  crawler,
  deckFusion,
  dusting,
  fuseA,
  fuseB,
  fuser,
  juhan,
  lab,
  mutate,
  outbreak,
  pileOn,
  plagueBook,
  quietTrap,
  registerGeneration,
  scatter,
  slime,
  slop,
  toxins,
} from "./fixtures/generation";
import { setupCatalog } from "./fixtures/harness";

const DECK = [
  plagueBook,
  slime,
  crawler,
  toxins,
  outbreak,
  dusting,
  scatter,
  charger,
  quietTrap,
  body,
  bigBody,
  fuseA,
  fuseB,
  lab,
  slop,
  deckFusion,
  mutate,
  fuser,
  juhan,
  pileOn,
].map((entry) => entry.id);

/** What a game reached, so the run can assert it exercised the systems it claims to replay. */
type Reached = { prompts: number; placements: number; fusions: number; transforms: number; summons: number };

function play(seed: string): { state: GameState; log: Action[]; reached: Reached } {
  setupCatalog();
  registerGeneration();
  let state = beginGame(createGame({ seed, decks: [DECK, DECK] })).state;
  const policy = createRng(`policy-${seed}`);
  const log: Action[] = [];
  const reached: Reached = { prompts: 0, placements: 0, fusions: 0, transforms: 0, summons: 0 };

  for (let step = 0; state.result === null; step += 1) {
    if (step > 4000) throw new Error(`game ${seed} did not finish`);
    if (state.pending !== null) {
      reached.prompts += 1;
      // §9.3: a paused state is plain data.
      expect(JSON.parse(JSON.stringify(state))).toEqual(state);
    }
    const player = seatToAct(state);
    const actions = legalActions(state, player).filter(
      (action) => action.type !== "concede" && action.type !== "offerDraw" && action.type !== "answerDraw",
    );
    if (actions.length === 0) throw new Error(`no legal action for ${player} in game ${seed}`);
    const others = actions.filter((action) => action.type !== "endTurn");
    const endTurn = actions.find((action) => action.type === "endTurn");
    const chosen =
      others.length === 0 || (endTurn !== undefined && policy.chance(AI_END_TURN_PROBABILITY))
        ? (endTurn ?? (others[policy.int(others.length)] as ActionBody))
        : (others[policy.int(others.length)] as ActionBody);
    const action = { ...chosen, playerId: player, nonce: `g${log.length}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`${action.type} rejected in ${seed}: ${result.error}`);
    for (const event of result.events) {
      if (event.type === "counterChanged" && event.placed !== undefined) reached.placements += 1;
      if (event.type === "fused") reached.fusions += 1;
      if (event.type === "transformed") reached.transforms += 1;
      if (event.type === "summoned") reached.summons += 1;
    }
    log.push(action);
    state = result.state;
  }
  return { state, log, reached };
}

describe("E19, E23–E25 whole games replay (§9.3)", () => {
  it("R113 random games on the generation decks fold back to the same state, through every pause", { timeout: 120_000 }, () => {
    const total: Reached = { prompts: 0, placements: 0, fusions: 0, transforms: 0, summons: 0 };
    for (const seed of ["gen-a", "gen-b", "gen-c", "gen-d", "gen-e", "gen-f"]) {
      const game = play(seed);
      const folded = fold({ seed, decks: [DECK, DECK], log: game.log });
      expect(folded.errors).toEqual([]);
      expect(hashState(folded.state)).toBe(hashState(game.state));
      for (const key of Object.keys(total) as (keyof Reached)[]) total[key] += game.reached[key];
    }
    expect(DECK).toHaveLength(DECK_SIZE);
    // The run is seeded end to end, so what it reaches is fixed; it must reach every system.
    expect(total.prompts).toBeGreaterThan(0);
    expect(total.placements).toBeGreaterThan(0);
    expect(total.fusions).toBeGreaterThan(0);
    expect(total.transforms + total.summons).toBeGreaterThan(0);
  });
});
