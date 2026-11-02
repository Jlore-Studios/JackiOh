// Replay fixtures (#511): a seeded game folded in WebAssembly and read through `replayPage`, which is
// what a practice or online source hands the player, and a fake source over those steps. A test
// helper, so the one place in the replay's tests that reaches the module.

import { REPLAY_PAGE_STEPS, createRng, hashState, registeredCatalog, replayOpen, replayPage, seatToAct, subsystems } from "@jackioh/engine";
import type { GameState, ReplayStep } from "@jackioh/engine";
import type { Action, SideView } from "@jackioh/shared";

import { ANIMATIONS, newEventsSince, planEntries } from "../game/animations.ts";
import { resolveDecks } from "../game/decks.ts";
import { REPLAY_GIF_FPS, REPLAY_GIF_SIZES } from "../replay/config.ts";
import { planFrames } from "../replay/gif/plan.ts";
import type { GifPlanOptions } from "../replay/gif/plan.ts";
import { ReplayFailure } from "../replay/source.ts";
import type { ReplaySource } from "../replay/source.ts";
import { beginGame, catalogVersion, createGame, reduce } from "../wasm/index.ts";
import { seedWhere } from "./seedSearch.ts";

// #2569: the game is the first seed under SEED_PREFIX ("replay-player-0", "replay-player-1", …) whose folded steps
// have what the replay's tests read off them (fitsTheReplayTests), found at run time by seedWhere, so a card
// change that re-deals the game moves the seed and breaks no test. A test that later needs another property of
// the steps adds its check there; it never pins a seed.
const SEED_PREFIX = "replay-player";
/** Two pages' worth of actions and a few over, then a concede: the game ends, in a few turns. */
const GAME_ACTIONS = 2 * REPLAY_PAGE_STEPS + 8;
/** The turns the folded game must span for turn moves to have somewhere to go. */
const MIN_TURNS = 3;
/** The least steps the first turn and the third turn each hold (ReplayPlayer.test.tsx's turn moves). */
const MIN_TURN_STEPS = 2;
/** The first step an animation may start from, as `firstAnimatedStep` in ReplayPlayer.test.tsx: the first two have almost nothing to log. */
const FIRST_ANIMATED_FROM = 3;
/** The steps the GIF export's clip takes, as `slice(8, 14)` in export.test.ts. */
const GIF_CLIP_FROM = 8;
const GIF_CLIP_TO = 14;
/** The GIF plan export.test.ts's "assetsFor" test makes of that clip. */
const GIF_PLAN: GifPlanOptions = {
  ...REPLAY_GIF_SIZES.landscape,
  fps: REPLAY_GIF_FPS,
  catalogVersion: "v",
  portraits: { p1: "vanilla", p2: "gary" },
};

let folded: ReplayStep[] | null = null;

/** Every step of the game one seed deals, as seat p1 saw it: the random policy's actions, then p1 concedes. */
function foldSteps(seed: string): ReplayStep[] {
  const resolved = resolveDecks("cheap20", "cheap20", registeredCatalog());
  if ("error" in resolved) throw new Error(resolved.error);
  const { decks } = resolved;
  let state = beginGame(createGame({ seed, decks })).state;
  const log: Action[] = [];
  const policy = createRng(`policy-${seed}`);
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

  const opened = replayOpen({ seed, decks, log }, { catalogVersion: catalogVersion(), finalHash: hashState(state as GameState) });
  if (opened.kind !== "ready") throw new Error(`the folded game was refused: ${opened.reason}`);
  const steps: ReplayStep[] = [];
  while (steps.length < opened.steps) {
    const page = replayPage(opened.checkpoints, "p1", steps.length, REPLAY_PAGE_STEPS);
    if (page.steps.length === 0) throw new Error(`an empty page at step ${String(steps.length)}`);
    steps.push(...page.steps);
  }
  return steps;
}

/** How many cards a hand holds, listed (the viewer's, and both once the game is over, R434) or counted. */
function held(hand: SideView["hand"]): number {
  return Array.isArray(hand) ? hand.length : hand.count;
}

/**
 * Whether one folded game has what the replay's tests read off its steps: a third page, both hands holding
 * cards at the end, MIN_TURNS turns (the first and third long), an animated step before the last, and a
 * card in the GIF export's clip.
 */
function fitsTheReplayTests(steps: readonly ReplayStep[]): boolean {
  const last = steps.at(-1);
  if (last === undefined || steps.length <= 2 * REPLAY_PAGE_STEPS) return false;
  if (held(last.view.you.hand) === 0 || held(last.view.opponent.hand) === 0) return false;
  const starts = steps.flatMap((step, index) => (index === 0 || steps[index - 1]?.turn !== step.turn ? [index] : []));
  const lengths = starts.map((start, index) => (starts[index + 1] ?? steps.length) - start);
  if (starts.length < MIN_TURNS || (lengths[0] ?? 0) < MIN_TURN_STEPS || (lengths[2] ?? 0) < MIN_TURN_STEPS) return false;
  const animated = steps.findIndex(
    (step, index) =>
      index >= FIRST_ANIMATED_FROM &&
      newEventsSince(steps[index - 1]?.view.events ?? [], step.view.events).some((event) => ANIMATIONS[event.type].durationMs > 0),
  );
  const before = steps[animated - 1];
  const onto = steps[animated];
  if (before === undefined || onto === undefined || onto === last) return false;
  if (!planEntries(newEventsSince(before.view.events, onto.view.events), before.view, false).some((entry) => entry.durationMs > 0)) return false;
  return planFrames(steps.slice(GIF_CLIP_FROM, GIF_CLIP_TO), GIF_PLAN).frames.some((frame) => frame.items.some((item) => item.kind === "card"));
}

/** Every step of the first seeded game that fits the replay's tests, folded once per test file. */
export function foldedReplaySteps(): readonly ReplayStep[] {
  if (folded !== null) return folded;
  const seed = seedWhere(SEED_PREFIX, (candidate) => fitsTheReplayTests(foldSteps(candidate)));
  folded = foldSteps(seed);
  return folded;
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
