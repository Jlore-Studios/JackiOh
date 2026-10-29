// R768, issue #508: what a replay shows, through the WebAssembly module. A game the random policy
// plays is opened with `replayOpen` and read with `replayPage`; every step of both seats is
// `viewFor` of the state the client held after that many actions.

import { describe, expect, it } from "vitest";

import {
  REPLAY_CHECKPOINT_EVERY,
  REPLAY_PAGE_STEPS,
  createRng,
  hashState,
  registeredCatalog,
  replayOpen,
  replayPage,
  seatToAct,
  subsystems,
  viewFor,
} from "@jackioh/engine";
import type { GameState, ReplayCheckpoints, ReplayStep } from "@jackioh/engine";
import type { Action, PlayerId } from "@jackioh/shared";

import { beginGame, createGame, reduce, type FoldInput, catalogVersion } from "./index.ts";
import { resolveDecks } from "../game/decks.ts";

const SEED = "replay-wasm";
/** Three checkpoints' worth of actions and a few over: the game's end need not be reached. */
const GAME_ACTIONS = 3 * REPLAY_CHECKPOINT_EVERY + 5;
const SEATS: readonly PlayerId[] = ["p1", "p2"];

type Played = { input: FoldInput; states: GameState[] };

/** The seeded game: the random policy's actions, and the state after each (index 0 is `beginGame`'s). */
function play(): Played {
  const resolved = resolveDecks("cheap20", "cheap20", registeredCatalog());
  if ("error" in resolved) throw new Error(resolved.error);
  const { decks } = resolved;
  let state = beginGame(createGame({ seed: SEED, decks })).state;
  const states = [state];
  const log: Action[] = [];
  const policy = createRng(`policy-${SEED}`);
  while (log.length < GAME_ACTIONS && state.result === null) {
    const seat = seatToAct(state);
    if (seat === null) break;
    const body = subsystems.chooseAction(state, seat, policy);
    if (body === null) break;
    const action = { ...body, playerId: seat, nonce: `r${String(log.length)}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`${seat} ${body.type}: ${result.error}`);
    state = result.state;
    states.push(state);
    log.push(action);
  }
  return { input: { seed: SEED, decks, log }, states };
}

function pages(checkpoints: ReplayCheckpoints, seat: PlayerId, total: number): ReplayStep[] {
  const steps: ReplayStep[] = [];
  while (steps.length < total) {
    const page = replayPage(checkpoints, seat, steps.length, REPLAY_PAGE_STEPS);
    expect(page.reduces).toBeLessThanOrEqual(REPLAY_CHECKPOINT_EVERY);
    expect(page.steps.length).toBeGreaterThan(0);
    steps.push(...page.steps);
  }
  return steps;
}

describe("replay through the module", () => {
  const { input, states } = play();
  const finalHash = hashState(states[states.length - 1] as GameState);

  it("R768 the module's steps are viewFor after each accepted action, for both seats, a page within the reduce budget", () => {
    // Long enough for two checkpoints, so a page reaches past one.
    expect(input.log.length).toBeGreaterThan(2 * REPLAY_CHECKPOINT_EVERY);
    const opened = replayOpen(input, { catalogVersion: catalogVersion(), finalHash });
    if (opened.kind !== "ready") throw new Error(`refused: ${opened.reason}`);
    expect(opened.steps).toBe(input.log.length + 1);
    for (const seat of SEATS) {
      const steps = pages(opened.checkpoints, seat, opened.steps);
      expect(steps).toHaveLength(opened.steps);
      steps.forEach((step, k) => {
        expect(step.step).toBe(k);
        expect(step.view).toEqual(viewFor(states[k] as GameState, seat));
        expect(step.turn).toBe(step.view.turn);
      });
    }
  });

  it("R768 a wrong final hash is refused as rules_changed and another catalog version as earlier_patch", () => {
    expect(replayOpen(input, { catalogVersion: catalogVersion(), finalHash: "00000000" })).toEqual({
      kind: "refused",
      reason: "rules_changed",
    });
    expect(replayOpen(input, { catalogVersion: "v0.0.0-earlier", finalHash })).toEqual({
      kind: "refused",
      reason: "earlier_patch",
    });
  });
});
