// Replay fixtures (#511): a seeded game folded in WebAssembly and read through `replayPage`, which is
// what a practice or online source hands the player, and a fake source over those steps. A test
// helper, so the one place in the replay's tests that reaches the module.

import { REPLAY_PAGE_STEPS, createRng, hashState, registeredCatalog, replayOpen, replayPage, seatToAct, subsystems } from "@jackioh/engine";
import type { GameState, ReplayStep } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";

import { resolveDecks } from "../game/decks.ts";
import { ReplayFailure } from "../replay/source.ts";
import type { ReplaySource } from "../replay/source.ts";
import { beginGame, catalogVersion, createGame, reduce } from "../wasm/index.ts";

// Re-picked for patch v0.3.11 (#1149): Zephyrs and CN in a Bottle left the (0) Cost cards, so `cheap20`
// changed, and "replay-player" now ends with the viewer's hand empty.
const SEED = "replay-player-5";
/** Two pages' worth of actions and a few over, then a concede: the game ends, in a few turns. */
const GAME_ACTIONS = 2 * REPLAY_PAGE_STEPS + 8;
/** The turns the folded game must span for turn moves to have somewhere to go. */
const MIN_TURNS = 3;

let folded: ReplayStep[] | null = null;

/** Every step of one seeded game as seat p1 saw it: the random policy's actions, then p1 concedes. */
export function foldedReplaySteps(): readonly ReplayStep[] {
  if (folded !== null) return folded;
  const resolved = resolveDecks("cheap20", "cheap20", registeredCatalog());
  if ("error" in resolved) throw new Error(resolved.error);
  const { decks } = resolved;
  let state = beginGame(createGame({ seed: SEED, decks })).state;
  const log: Action[] = [];
  const policy = createRng(`policy-${SEED}`);
  const play = (seat: Action["playerId"], body: ReturnType<typeof subsystems.chooseAction>): void => {
    if (body === null) throw new Error("the policy had no action");
    const action = { ...body, playerId: seat, nonce: `r${String(log.length)}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`${seat} ${body.type}: ${result.error}`);
    state = result.state;
    log.push(action);
  };
  while (log.length < GAME_ACTIONS && state.result === null) {
    const seat = seatToAct(state);
    if (seat === null) break;
    play(seat, subsystems.chooseAction(state, seat, policy));
  }
  if (state.result === null) play(seatToAct(state) ?? "p1", { type: "concede" });
  if (state.result === null) throw new Error("the folded game did not end");

  const opened = replayOpen({ seed: SEED, decks, log }, { catalogVersion: catalogVersion(), finalHash: hashState(state as GameState) });
  if (opened.kind !== "ready") throw new Error(`the folded game was refused: ${opened.reason}`);
  const steps: ReplayStep[] = [];
  while (steps.length < opened.steps) {
    const page = replayPage(opened.checkpoints, "p1", steps.length, REPLAY_PAGE_STEPS);
    if (page.steps.length === 0) throw new Error(`an empty page at step ${String(steps.length)}`);
    steps.push(...page.steps);
  }
  const turns = new Set(steps.map((step) => step.turn));
  if (turns.size < MIN_TURNS) throw new Error(`the folded game spans ${String(turns.size)} turns, fewer than ${String(MIN_TURNS)}: change the seed`);
  folded = steps;
  return steps;
}

export type FakeReplaySource = ReplaySource & {
  /** The `from` of every page asked for, in order. */
  readonly calls: number[];
};

/**
 * A source over `steps`, a page of at most `REPLAY_PAGE_STEPS` as the engine's. Pages from step
 * `failFrom` on reject with `failure`.
 */
export function fakeReplaySource(
  steps: readonly ReplayStep[],
  options: { failFrom?: number; failure?: ReplayFailure } = {},
): FakeReplaySource {
  const calls: number[] = [];
  return {
    total: steps.length,
    calls,
    page(from, count) {
      calls.push(from);
      if (options.failFrom !== undefined && from >= options.failFrom) {
        return Promise.reject(options.failure ?? new ReplayFailure("error", "the fake source failed"));
      }
      return Promise.resolve(steps.slice(from, from + Math.min(count, REPLAY_PAGE_STEPS)));
    },
  };
}
