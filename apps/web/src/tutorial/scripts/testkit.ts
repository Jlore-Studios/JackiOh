// Shared helpers of the lesson tests (`*.test.ts` beside this file), `lessonSuite` among them (R293,
// R2205000, R2205001, R2205003). Only tests import this file.

import { beforeAll, expect, it } from "vitest";

import { aiToAct } from "@jackioh/ai";
import { TUTORIAL_BARRED_RARITIES, TUTORIAL_CARD_SET } from "@jackioh/engine/config";
import { beginGame, createGame, fold, hashState, legalActions, reduce, registeredCatalog, viewFor, type GameState } from "@jackioh/engine";
import { opponentOf, type Action, type GameEvent, type PlayerId, type PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../../game/animations.ts";
import { sweep } from "../../test/daily.ts";
import { coachAck, coachDisplay, coachObserve, coachStart, isBubble, type CoachCtx } from "../coach.ts";
import { TUTORIAL_BUBBLES_PER_MOVE_MAX } from "../config.ts";
import { LESSON_GAME_TIMEOUT_MS, RANDOM_GAME_MS, playLesson, type LessonPolicy, type LessonRun } from "../harness.ts";
import { lessonById } from "../lessons.ts";

const catalog = registeredCatalog();

/** One accepted action of the game and everything it emitted, unredacted, with the states around it. */
export type Step = { action: Action; events: GameEvent[]; before: GameState; after: GameState };

/** Fold the run's log one action at a time, keeping every event (the test may read what the page may not). */
export function stepsOf(run: LessonRun): Step[] {
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

/**
 * Every run of "Got it" bubbles the coach showed on this line, with no move of the player's and no step
 * asking for one between them, rebuilt from the run's log as the harness's coach read it (its tips prove it).
 */
export function gotItRuns(run: LessonRun): { runs: string[][]; tips: string[] } {
  const { seed, decks, handicaps, log } = run.debug;
  const human = run.humanSeat;
  const ai = opponentOf(human);
  let state = beginGame(createGame({ seed, decks, handicaps })).state;
  let coach = coachStart([]);
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
      defOf: (defId) => catalog[defId],
    };
    previous = view;
    coach = coachObserve(run.script, coach, ctx);
    let display = coachDisplay(run.script, coach, ctx);
    while (isBubble(display) && display.ack) {
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

/** The player-turn number (the engine's `turn`) of the human's own turn `n` (p1 takes the odd ones). */
export function gameTurnOf(seat: PlayerId, n: number): number {
  return seat === "p1" ? 2 * n - 1 : 2 * n;
}

/** The rarities no lesson before the last deals or generates (R2201001, R2205003). */
const SHOWPIECES: readonly string[] = ["Legendary", "Mythic"];

function cardsIn(state: GameState): string[] {
  const ids: string[] = [];
  for (const player of Object.values(state.players)) {
    for (const zone of [player.hand, player.library, player.graveyard, player.exile ?? []]) {
      for (const card of zone) ids.push(card.defId);
    }
  }
  const json = JSON.stringify(state.players);
  for (const match of json.matchAll(/"defId":"([^"]+)"/g)) if (match[1] !== undefined) ids.push(match[1]);
  return ids;
}

/** Every Legendary or Mythic card a run's game ever held or showed, anywhere. */
export function showpiecesIn(run: LessonRun): string[] {
  const seen = new Set<string>();
  for (const step of stepsOf(run)) {
    for (const id of [...cardsIn(step.after), ...JSON.stringify(step.events).matchAll(/"defId":"([^"]+)"/g)].map((each) =>
      typeof each === "string" ? each : (each[1] ?? ""),
    )) {
      const def = catalog[id];
      if (def !== undefined && SHOWPIECES.includes(def.rarity)) seen.add(id);
    }
  }
  return [...seen].sort();
}

type SuiteOptions = {
  /** The most of the player's own turns the coach's line may take. */
  coachTurnsMax: number;
  /** Policy seeds for the random player in the daily run (its first game runs on every pull request). */
  randomRuns: number;
};

/** R2533000: whether the tutorial opponent may play a card: Training Grounds, neither Legendary nor Mythic, a token by its printed rarity. */
function isTutorialCard(defId: string): boolean {
  const def = catalog[defId];
  if (def === undefined) return false;
  const rarity = def.rarity === "Token" ? def.printedRarity : def.rarity;
  return def.set === TUTORIAL_CARD_SET && !TUTORIAL_BARRED_RARITIES.some((barred) => barred === rarity);
}

/** R2533000: the cards the AI seat played in a run that it may not play, with where. */
export function barredPlays(run: LessonRun): string[] {
  const ai = opponentOf(run.humanSeat);
  return stepsOf(run)
    .flatMap((step) => step.events)
    .flatMap((event) => (event.type === "cardPlayed" && event.player === ai && !isTutorialCard(event.defId) ? [event.defId] : []));
}

/** The policies other than the coach's own line that must each still end in the human's win (R2205001). */
const OTHER_POLICIES: readonly LessonPolicy[] = ["coach-passive", "autopilot", "reckless"];

/**
 * The tests every lesson passes, played through the real practice core (harness.ts). Call it inside a
 * `describe` whose title names the rulings, so `cargo jackioh spec check` finds them in the lesson's own file.
 */
export function lessonSuite(lessonId: string, options: SuiteOptions): void {
  const lesson = lessonById(lessonId);
  if (lesson === undefined) throw new Error(`no lesson "${lessonId}"`);
  const human: PlayerId = lesson.humanSeat;

  {
    let coach: LessonRun;

    beforeAll(() => {
      coach = playLesson(lessonId, { policy: "coach" });
    }, LESSON_GAME_TIMEOUT_MS);

    it(`R293 R2205000 R2434001 R2533000 ${lessonId}: the coach's line wins, every moment shows and is done, every listed mechanic is found, and the AI plays only its own cards`, () => {
      expect(coach.winner, "the human wins").toBe(human);
      expect(coach.view.result?.reason).not.toBe("turn-cap");
      expect(coach.humanTurns, "within the lesson's turns").toBeLessThanOrEqual(options.coachTurnsMax);
      expect(coach.coach.finished).toBe(true);
      const shown = coach.shown.map((entry) => entry.id);
      for (const step of coach.script.steps) {
        expect(shown, `${step.id} shows`).toContain(step.id);
        if (step.final === true) expect(coach.outcomes[step.id], `${step.id} is up when the game ends`).toBeUndefined();
        else expect(coach.outcomes[step.id], `${step.id} is done`).toBe("done");
      }
      for (const id of lesson.discoveries) expect(coach.coach.found, `${id} is found`).toContain(id);
      expect(coach.script.teaches, "the script teaches the lesson's list").toEqual(lesson.discoveries);
      expect(coach.humanActions.filter((entry) => entry.refused !== null), "the engine takes every move").toEqual([]);
      expect(barredPlays(coach), "the AI plays only Training Grounds cards that are not Legendary or Mythic").toEqual([]);
    });

    it(`R2205000 ${lessonId}: the coach never shows more than ${String(TUTORIAL_BUBBLES_PER_MOVE_MAX)} "Got it"s in a row on its line`, () => {
      const { runs } = gotItRuns(coach);
      for (const run of runs) expect(run.length, `"Got it" ${run.join(" -> ")}`).toBeLessThanOrEqual(TUTORIAL_BUBBLES_PER_MOVE_MAX);
    });

    for (const policy of OTHER_POLICIES) {
      it(
        `R293 R2205001 R2434001 R2533000 ${lessonId}: the ${policy} player ends the lesson in the human's win, with no loss, draw or stall, and the AI plays only its own cards`,
        () => {
          const run = playLesson(lessonId, { policy });
          expect(run.winner).toBe(human);
          expect(run.view.result?.reason).not.toBe("turn-cap");
          expect(run.coach.finished).toBe(true);
          expect(run.humanActions.filter((entry) => entry.refused !== null)).toEqual([]);
          expect(barredPlays(run), "the AI plays only Training Grounds cards that are not Legendary or Mythic").toEqual([]);
        },
        LESSON_GAME_TIMEOUT_MS,
      );
    }

    it(
      `R293 R2205001 R2434001 R2533000 ${lessonId}: a player who makes random moves still wins, never stalls or breaks the coach, and the AI plays only its own cards`,
      () => {
        for (let n = 1; n <= sweep(options.randomRuns); n += 1) {
          const run = playLesson(lessonId, { policy: "random", policySeed: `${lessonId}:random:${String(n)}` });
          expect(run.winner, `policy seed ${String(n)}: the human wins`).toBe(human);
          expect(run.coach.finished, `policy seed ${String(n)}: the coach is finished`).toBe(true);
          expect(barredPlays(run), `policy seed ${String(n)}: the AI plays only its own cards`).toEqual([]);
        }
      },
      options.randomRuns * RANDOM_GAME_MS,
    );

    it(
      `R293 R2205000 ${lessonId}: the lesson replays from its seed, decks and handicaps`,
      () => {
        const { seed, decks, handicaps, log, hash } = coach.debug;
        expect(seed).toBe(lesson.seed);
        const replayed = fold({ seed, decks, handicaps, log });
        expect(replayed.errors).toEqual([]);
        expect(hashState(replayed.state)).toBe(hash);
        const again = playLesson(lessonId, { policy: "coach" });
        expect(again.debug.log).toEqual(log);
        expect(again.debug.hash).toBe(hash);
        expect(again.shown).toEqual(coach.shown);
        expect(again.discoveries).toEqual(coach.discoveries);
      },
      LESSON_GAME_TIMEOUT_MS,
    );

    if (lesson.poolHold) {
      it(
        `R2205003 ${lessonId}: no Legendary or Mythic card shows up, drawn or generated, on the coach's line or the reckless player's`,
        () => {
          expect(showpiecesIn(coach)).toEqual([]);
          expect(showpiecesIn(playLesson(lessonId, { policy: "reckless" }))).toEqual([]);
        },
        LESSON_GAME_TIMEOUT_MS,
      );
    } else {
      it(`R2205003 ${lessonId}: the last lesson deals the Legendary and Mythic cards, and the coach's line plays them`, () => {
        const plays = stepsOf(coach).flatMap((step) => step.events).filter((event) => event.type === "cardPlayed" && event.player === human);
        const played = new Set(plays.map((event) => (event.type === "cardPlayed" ? event.defId : "")));
        const held = lesson.humanDeck.filter((id) => SHOWPIECES.includes(catalog[id]?.rarity ?? ""));
        expect(held.length, "the deck holds them").toBeGreaterThan(0);
        for (const id of held) expect(played, `${id} is played`).toContain(id);
      });
    }
  }
}
