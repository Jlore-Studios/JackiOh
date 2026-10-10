// Lesson traps tests (SPEC §9.10, R293): coach and alternate runs must win, expose mechanics, and replay.

import { beforeAll, describe, expect, it } from "vitest";

import { aiToAct } from "@jackioh/ai";
import { beginGame, createGame, fold, hashState, legalActions, reduce, viewFor, type GameState } from "@jackioh/engine";
import { opponentOf, type Action, type GameEvent, type PlayerId, type PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../../game/animations.ts";
import { COACH_START, coachAck, coachDisplay, coachObserve, type CoachCtx } from "../coach.ts";
import { LESSON_GAME_MS, LESSON_GAME_TIMEOUT_MS, RANDOM_GAME_MS, playLesson, type LessonRun } from "../harness.ts";
import { lessonById } from "../lessons.ts";
import { script } from "./traps.ts";

const LESSON = "traps";
/** The coach's line wins on the player's 7th turn; this leaves one turn of slack. */
const COACH_TURNS_MAX = 8;
/** Coach-passive play must finish within this many human turns. */
const PASSIVE_TURNS_MAX = 8;
/** Autopilot play must finish within this many human turns. */
const AUTOPILOT_TURNS_MAX = 8;
/** Maximum consecutive "Got it" bubbles. */
const GOT_IT_RUN_MAX = 2;
/** Alternate lesson seeds the autopilot must win. */
const ALT_SEEDS = 5;
/** Random-policy seeds for a player who ignores the coach. */
const RANDOM_RUNS = 8;

const GOING_LONG = "core-084";
const HONEYPOT = "core-060";
const SHEEPISH = "core-041";
const FARM = "core-058";
const RUSH_TOKEN = "core-t-rush";
const SHEEP = "core-t-sheep";
const TEMPO_TIMMY = "core-011";

const lesson = lessonById(LESSON);
if (lesson === undefined) throw new Error(`no lesson "${LESSON}"`);

/** Accepted action, emitted events, and surrounding states. */
type Step = { action: Action; events: GameEvent[]; before: GameState; after: GameState };

/** Folds the unredacted log, which test code may inspect. */
function stepsOf(run: LessonRun): Step[] {
  const { seed, decks, handicaps, log } = run.debug;
  let state = beginGame(createGame({ seed, decks, handicaps })).state;
  const steps: Step[] = [];
  for (const action of log) {
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`the log does not fold: ${result.error}`);
    steps.push({ action, events: result.events, before: state, after: result.state });
    state = result.state;
  }
  return steps;
}

/** Rebuilds prompted tip/info sequences shown between human actions. */
function gotItRuns(run: LessonRun): { runs: string[][]; tips: string[] } {
  const { seed, decks, handicaps, log } = run.debug;
  const human = run.humanSeat;
  const ai = opponentOf(human);
  let state = beginGame(createGame({ seed, decks, handicaps })).state;
  let coach = COACH_START;
  let previous: PlayerView | null = null;
  const runs: string[][] = [];
  const tips: string[] = [];
  let current: string[] = [];
  const close = (): void => {
    if (current.length > 0) runs.push(current);
    current = [];
  };
  const observe = (): void => {
    const view = viewFor(state, human);
    const ctx: CoachCtx = {
      view,
      legal: legalActions(state, human),
      fresh: previous === null ? [] : newEventsSince(previous.events, view.events),
      aiToAct: aiToAct(state, ai),
    };
    previous = view;
    coach = coachObserve(run.script, coach, ctx);
    let display = coachDisplay(run.script, coach, ctx);
    while (display.mode === "tip" || (display.mode === "step" && display.ack)) {
      current.push(display.id);
      if (display.mode === "tip") tips.push(display.id);
      coach = coachAck(run.script, coach, ctx);
      display = coachDisplay(run.script, coach, ctx);
    }
    if (display.mode === "step") close();
  };
  observe();
  for (const action of log) {
    if (action.playerId === human) close();
    state = reduce(state, action).state;
    observe();
  }
  close();
  return { runs, tips };
}

function gameTurnOf(seat: PlayerId, n: number): number {
  return seat === "p1" ? 2 * n - 1 : 2 * n;
}

/** Verifies every backrow mechanic appears on the coach line. */
function expectMechanics(run: LessonRun, steps: Step[]): void {
  const human = run.humanSeat;
  const ai = opponentOf(human);

  const opening = viewFor(beginGame(createGame({ seed: run.debug.seed, decks: run.debug.decks, handicaps: run.debug.handicaps })).state, human);
  expect(Array.isArray(opening.you.hand) ? opening.you.hand.map((card) => card.defId) : [], "Quickdraw: Going Long starts in the opening hand").toContain(GOING_LONG);

  const set = steps.find((step) =>
    step.events.some((event) => event.type === "summoned" && event.player === human && event.row === "backrow" && event.defId === HONEYPOT),
  );
  expect(set, "the human sets Bear Honeypot in the backrow").toBeDefined();
  if (set !== undefined) {
    const aiSees = viewFor(set.after, ai).opponent.backrow;
    expect(aiSees.some((card) => card !== null && card.faceDown), "the AI sees the trap face-down").toBe(true);
  }

  const mine = steps.find((step) => step.events.some((event) => event.type === "trapFired" && event.controller === human && event.defId === HONEYPOT));
  expect(mine, "the human's Bear Honeypot fires").toBeDefined();
  expect(mine?.action.playerId, "it fires during the AI's turn").toBe(ai);
  expect(
    mine?.events.filter((event) => event.type === "summoned" && event.player === human && event.defId === RUSH_TOKEN).length,
    "Tokens: it summons two Rush Tokens",
  ).toBe(2);

  const farmed = steps.find((step) => {
    const started = step.events.findIndex((event) => event.type === "turnStarted" && event.player === human);
    if (started < 0) return false;
    const drawn = step.events.findIndex((event, index) => index > started && event.type === "drawn");
    const window = step.events.slice(started, drawn < 0 ? undefined : drawn);
    return window.some((event) => event.type === "summoned" && event.player === human && event.defId === RUSH_TOKEN);
  });
  expect(farmed, "Rush Token Farm makes a Rush Token at the start of the human's turn").toBeDefined();
  if (farmed !== undefined) {
    expect(farmed.before.players[human].backrow.some((card) => card?.defId === FARM), "the farm is in the backrow").toBe(true);
  }

  const armored = steps.find((step) => viewFor(step.after, human).you.hero.armor === 2);
  expect(armored, "Going Long gives the hero Armor 2").toBeDefined();

  const aiSet = steps.find((step) =>
    step.events.some((event) => event.type === "summoned" && event.player === ai && event.row === "backrow" && event.defId === SHEEPISH),
  );
  expect(aiSet, "the AI sets Sheepish").toBeDefined();
  if (aiSet === undefined) return;
  expect(aiSet.action.playerId, "on its own turn").toBe(ai);
  // Rule 7: a bare face-down marker must not identify the card.
  const seen = viewFor(aiSet.after, human).opponent.backrow.filter((card) => card !== null);
  expect(seen, "the human sees only a face-down card").toContainEqual({ faceDown: true, cost: 1 });

  const theirs = steps.find((step) => step.events.some((event) => event.type === "trapFired" && event.controller === ai && event.defId === SHEEPISH));
  expect(theirs, "the AI's Sheepish fires").toBeDefined();
  expect(theirs?.action.playerId, "during the human's turn, on the human's play").toBe(human);
  expect(theirs?.before.turn, "on the human's turn right after the AI set it").toBe(aiSet.before.turn + 1);
  expect(
    theirs?.events.some((event) => event.type === "transformed" && event.fromDefId === TEMPO_TIMMY && event.toDefId === SHEEP),
    "Sheepish turns the bait, Tempo Timmy, into a Sheep",
  ).toBe(true);
  const baitId = theirs?.action.type === "play" ? theirs.action.instanceId : undefined;
  const bait = run.humanActions.find((entry) => entry.action.type === "play" && entry.action.instanceId === baitId);
  expect(bait?.byCoach, "the coach named the bait").toBe(true);
  expect(run.shown.map((entry) => entry.id), "the coach asked for the bait").toContain("bait");
  expect(run.outcomes["bait"], "and the player gave it").toBe("done");

  for (const id of ["honeypot-fired", "tokens", "armor", "enemy-facedown", "enemy-trap"]) {
    expect(run.tips, `tip ${id} shows`).toContain(id);
  }
}

describe("R293 lesson traps", () => {
  let coach: LessonRun;
  let steps: Step[];
  const human: PlayerId = lesson.humanSeat;

  beforeAll(() => {
    coach = playLesson(LESSON, { policy: "coach" });
    steps = stepsOf(coach);
  }, LESSON_GAME_TIMEOUT_MS);

  it("R293 traps: following the coach wins the lesson, and every step shows and is done", () => {
    expect(coach.winner, "the human wins").toBe(human);
    expect(coach.view.result?.reason).toBe("hero-death");
    expect(coach.humanTurns, "within the lesson's turns").toBeLessThanOrEqual(COACH_TURNS_MAX);
    expect(coach.coach.finished).toBe(true);

    const shown = coach.shown.map((entry) => entry.id);
    for (const step of script.steps) {
      expect(shown, `${step.id} shows`).toContain(step.id);
      // The final step remains visible when its game-ending action resolves.
      if (step.final === true) expect(coach.outcomes[step.id], `${step.id} is still up when the game ends`).toBeUndefined();
      else expect(coach.outcomes[step.id], `${step.id} is done`).toBe("done");
    }
    expect(shown, "every step shows in the script's order").toEqual(script.steps.map((step) => step.id));

    for (const [id, n] of [["move-2", 2], ["move-3", 3], ["move-4", 4]] as const) {
      expect(coach.shown.find((entry) => entry.id === id)?.turn, `${id} covers the rest of turn ${String(n)}`).toBe(gameTurnOf(human, n));
    }

    expect(coach.humanActions.filter((entry) => !entry.byCoach).map((entry) => entry.action.type)).toEqual([]);
    expect(coach.humanActions.filter((entry) => entry.refused !== null)).toEqual([]);
  });

  it("R293 traps: the lesson's mechanics come up on the coach line", () => {
    expectMechanics(coach, steps);
  });

  it('R293 traps: the coach never shows more than two "Got it"s in a row on its line', () => {
    const { runs, tips } = gotItRuns(coach);
    expect(tips, "the rebuilt line shows the tips the coach showed").toEqual(coach.tips);
    for (const run of runs) expect(run.length, `"Got it" ${run.join(" -> ")}`).toBeLessThanOrEqual(GOT_IT_RUN_MAX);
    // The AI's busy turns produce each pair; tokens wait for the player's turn.
    expect(runs, "the hidden trap, then the trap springing").toContainEqual(["trap-waits", "honeypot-fired"]);
    expect(runs, "the Armor, then the AI's face-down card").toContainEqual(["armor", "enemy-facedown"]);
  });

  it(
    "R293 traps: a beginner who plays only what the coach names still wins",
    () => {
      const run = playLesson(LESSON, { policy: "coach-passive" });
      expect(run.winner, "the human wins").toBe(human);
      expect(run.view.result?.reason).toBe("hero-death");
      expect(run.humanTurns, "within the lesson's turns").toBeLessThanOrEqual(PASSIVE_TURNS_MAX);
      expect(run.coach.finished).toBe(true);
      expect(run.humanActions.filter((entry) => !entry.byCoach).map((entry) => entry.action.type)).toEqual([]);
      expect(run.humanActions.filter((entry) => entry.refused !== null)).toEqual([]);
      expectMechanics(run, stepsOf(run));
    },
    LESSON_GAME_TIMEOUT_MS,
  );

  it(
    "R293 traps: a sensible beginner who ignores the coach still wins",
    () => {
      const run = playLesson(LESSON, { policy: "autopilot" });
      expect(run.winner).toBe(human);
      expect(run.humanTurns).toBeLessThanOrEqual(AUTOPILOT_TURNS_MAX);
      expect(run.coach.finished).toBe(true);
    },
    LESSON_GAME_TIMEOUT_MS,
  );

  it(
    "R293 traps: a sensible beginner who ignores the coach wins on other seeds of the same decks too",
    () => {
      for (let n = 1; n <= ALT_SEEDS; n += 1) {
        const seed = `${lesson.seed}:alt:${String(n)}`;
        const run = playLesson(LESSON, { policy: "autopilot", seed });
        expect(run.winner, `seed ${seed}`).toBe(human);
        expect(run.coach.finished, `seed ${seed}: the coach is finished`).toBe(true);
      }
    },
    ALT_SEEDS * LESSON_GAME_MS,
  );

  it(
    "R293 traps: a player who ignores the coach never stalls or breaks it",
    () => {
      for (let n = 1; n <= RANDOM_RUNS; n += 1) {
        const run = playLesson(LESSON, { policy: "random", policySeed: `${LESSON}:random:${String(n)}` });
        expect(run.view.result, `policy seed ${String(n)} reaches a result`).not.toBeNull();
        expect(run.coach.finished, `policy seed ${String(n)}: the coach is finished`).toBe(true);
        for (const step of script.steps) {
          const outcome = run.outcomes[step.id];
          if (outcome !== undefined) expect(["done", "moot", "expired"]).toContain(outcome);
        }
      }
    },
    RANDOM_RUNS * RANDOM_GAME_MS,
  );

  it(
    "R293 traps: the lesson replays from its seed, decks and handicaps",
    () => {
      const { seed, decks, handicaps, log, hash } = coach.debug;
      expect(seed).toBe(lesson.seed);
      const replayed = fold({ seed, decks, handicaps, log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hash);

      const again = playLesson(LESSON, { policy: "coach" });
      expect(again.debug.log).toEqual(log);
      expect(again.debug.hash).toBe(hash);
      expect(again.shown).toEqual(coach.shown);
      expect(again.tips).toEqual(coach.tips);
    },
    LESSON_GAME_TIMEOUT_MS,
  );
});
