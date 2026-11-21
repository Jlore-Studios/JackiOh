// TEMPORARY seed scan for lesson "finale" (issue #4353, pull request #4356); never committed. Copy it to
// apps/web/src/tutorial/scripts/finale-seed-scan.test.ts, build the WASM module (sh scripts/build-wasm.sh),
// then run
//   FINALE_SCAN=1 FINALE_SCAN_FROM=1 FINALE_SCAN_TO=200 pnpm --dir apps/web exec vitest run src/tutorial/scripts/finale-seed-scan.test.ts
// (without FINALE_SCAN=1 it is skipped).
// It prints one "FINALE-SCAN tg-finale-<n>: <first check failed | PASS>" line per seed and stops after
// FINALE_SCAN_WANT passing seeds (default 3). A seed passes when it meets every check that
// lessonSuite("finale", { coachTurnsMax: 11, randomRuns: 8 }) (testkit.ts) and lessons.test.ts's R2201000 idle
// check make, the daily sweep's 8 random games included.

import { expect, it } from "vitest";

import { registeredCatalog } from "@jackioh/engine";

import { TUTORIAL_BUBBLES_PER_MOVE_MAX } from "../config.ts";
import { playLesson, type LessonPolicy, type LessonRun } from "../harness.ts";
import { lessonById } from "../lessons.ts";
import { barredPlays, gotItRuns, stepsOf } from "./testkit.ts";

const LESSON = "finale";
const COACH_TURNS_MAX = 11;
const RANDOM_RUNS = 8;
const SHOWPIECES: readonly string[] = ["Legendary", "Mythic"];
const IDLE_MOVES: readonly string[] = ["mulligan", "answer", "endTurn"];
const OTHER_POLICIES: readonly LessonPolicy[] = ["coach-passive", "autopilot", "reckless"];

const catalog = registeredCatalog();
const found = lessonById(LESSON);
if (found === undefined) throw new Error(`no lesson "${LESSON}"`);
const lesson = found;
const human = lesson.humanSeat;

/** Why `run` is not a clean win for the human, or null when it is. */
function notAWin(run: LessonRun): string | null {
  if (run.winner !== human) return `winner ${String(run.winner)}`;
  if (run.view.result?.reason === "turn-cap") return "turn cap";
  if (!run.coach.finished) return "coach not finished";
  if (run.humanActions.some((entry) => entry.refused !== null)) return "a move refused";
  const barred = barredPlays(run);
  if (barred.length > 0) return `the AI played ${barred.join(", ")}`;
  return null;
}

/** The first check `seed` fails, or null when it passes them all. */
function failure(seed: string): string | null {
  const coach = playLesson(LESSON, { policy: "coach", seed });
  const lost = notAWin(coach);
  if (lost !== null) return `coach: ${lost}`;
  if (coach.humanTurns > COACH_TURNS_MAX) return `coach: ${String(coach.humanTurns)} turns`;
  const shown = coach.shown.map((entry) => entry.id);
  for (const step of coach.script.steps) {
    if (!shown.includes(step.id)) return `coach: ${step.id} never shows`;
    const outcome = coach.outcomes[step.id];
    if (step.final === true ? outcome !== undefined : outcome !== "done") return `coach: ${step.id} is ${String(outcome)}`;
  }
  for (const id of lesson.discoveries) if (!coach.coach.found.includes(id)) return `coach: ${id} not found`;
  if (gotItRuns(coach).runs.some((run) => run.length > TUTORIAL_BUBBLES_PER_MOVE_MAX)) return "coach: too many Got its in a row";
  const played = new Set(
    stepsOf(coach)
      .flatMap((step) => step.events)
      .flatMap((event) => (event.type === "cardPlayed" && event.player === human ? [event.defId] : [])),
  );
  const unplayed = lesson.humanDeck.filter((id) => SHOWPIECES.includes(catalog[id]?.rarity ?? "") && !played.has(id));
  if (unplayed.length > 0) return `coach: unplayed ${unplayed.join(", ")}`;

  for (const policy of OTHER_POLICIES) {
    const why = notAWin(playLesson(LESSON, { policy, seed }));
    if (why !== null) return `${policy}: ${why}`;
  }
  const idle = playLesson(LESSON, { policy: "idle", seed });
  if (idle.winner !== human) return `idle: winner ${String(idle.winner)}`;
  if (idle.view.result?.reason === "turn-cap") return "idle: turn cap";
  if (idle.humanActions.some((entry) => entry.refused !== null)) return "idle: a move refused";
  if (idle.humanActions.some(({ action }) => !IDLE_MOVES.includes(action.type))) return "idle: a move beyond ending the turn";
  for (let n = 1; n <= RANDOM_RUNS; n += 1) {
    const run = playLesson(LESSON, { policy: "random", seed, policySeed: `${LESSON}:random:${String(n)}` });
    if (run.winner !== human) return `random ${String(n)}: winner ${String(run.winner)}`;
    if (!run.coach.finished) return `random ${String(n)}: coach not finished`;
    if (barredPlays(run).length > 0) return `random ${String(n)}: the AI played ${barredPlays(run).join(", ")}`;
  }
  return null;
}

it.runIf(process.env.FINALE_SCAN === "1")("finale seed scan", () => {
  const from = Number(process.env.FINALE_SCAN_FROM ?? "1");
  const to = Number(process.env.FINALE_SCAN_TO ?? "200");
  const want = Number(process.env.FINALE_SCAN_WANT ?? "3");
  const passing: string[] = [];
  for (let n = from; n <= to && passing.length < want; n += 1) {
    const seed = `tg-finale-${String(n)}`;
    let why: string | null;
    try {
      why = failure(seed);
    } catch (error) {
      why = `threw ${String(error)}`;
    }
    console.log(`FINALE-SCAN ${seed}: ${why ?? "PASS"}`);
    if (why === null) passing.push(seed);
  }
  console.log(`FINALE-SCAN passing: ${passing.join(", ")}`);
  expect(passing.length).toBeGreaterThan(0);
}, 7_200_000);
