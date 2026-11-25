// TEMPORARY (issue #4783), deleted before its pull request merges: scans "tg-finale-<n>" seeds for one on
// which the finale passes lessonSuite's checks, since the session building the patch cannot run the web's
// tests. It runs only under `.github/workflows/tmp-finale-seed-scan.yml`, which sets SCAN_FROM and SCAN_TO.

import { it } from "vitest";

import { registeredCatalog } from "@jackioh/engine";

import { TUTORIAL_BUBBLES_PER_MOVE_MAX } from "../config.ts";
import { playLesson, type LessonPolicy } from "../harness.ts";
import { lessonById } from "../lessons.ts";
import { barredPlays, gotItRuns, stepsOf } from "./testkit.ts";

const FROM = Number(process.env.SCAN_FROM ?? "0");
const TO = Number(process.env.SCAN_TO ?? "0");

function problems(seed: string): string[] {
  const lesson = lessonById("finale");
  if (lesson === undefined) return ["no lesson"];
  (lesson as { seed: string }).seed = seed;
  const catalog = registeredCatalog();
  const human = lesson.humanSeat;
  const out: string[] = [];
  const coach = playLesson("finale", { policy: "coach" });
  if (coach.winner !== human) out.push("the coach's line loses");
  if (coach.view.result?.reason === "turn-cap") out.push("turn cap");
  if (coach.humanTurns > 11) out.push(`${String(coach.humanTurns)} turns`);
  if (!coach.coach.finished) out.push("coach unfinished");
  const shown = coach.shown.map((entry) => entry.id);
  for (const step of coach.script.steps) {
    if (!shown.includes(step.id)) out.push(`${step.id} unshown`);
    const outcome = coach.outcomes[step.id];
    if (step.final === true ? outcome !== undefined : outcome !== "done") out.push(`${step.id} ${String(outcome)}`);
  }
  for (const id of lesson.discoveries) if (!coach.coach.found.includes(id)) out.push(`${id} unfound`);
  if (coach.humanActions.some((entry) => entry.refused !== null)) out.push("a move refused");
  if (barredPlays(coach).length > 0) out.push("a barred AI play");
  for (const run of gotItRuns(coach).runs) if (run.length > TUTORIAL_BUBBLES_PER_MOVE_MAX) out.push("too many Got its");
  const played = new Set(
    stepsOf(coach)
      .flatMap((step) => step.events)
      .flatMap((event) => (event.type === "cardPlayed" && event.player === human ? [event.defId] : [])),
  );
  for (const id of lesson.humanDeck) {
    const rarity = catalog[id]?.rarity ?? "";
    if ((rarity === "Legendary" || rarity === "Mythic") && !played.has(id)) out.push(`${id} unplayed`);
  }
  if (out.length > 0) return out;
  const others: readonly LessonPolicy[] = ["coach-passive", "autopilot", "reckless"];
  for (const policy of others) {
    const run = playLesson("finale", { policy });
    const bad =
      run.winner !== human ||
      run.view.result?.reason === "turn-cap" ||
      !run.coach.finished ||
      run.humanActions.some((entry) => entry.refused !== null) ||
      barredPlays(run).length > 0;
    if (bad) out.push(`${policy} fails`);
  }
  const random = playLesson("finale", { policy: "random", policySeed: "finale:random:1" });
  if (random.winner !== human || !random.coach.finished || barredPlays(random).length > 0) out.push("random 1 fails");
  return out;
}

it.skipIf(FROM === 0)(
  "TEMPORARY the finale's seed scan",
  () => {
    const good: string[] = [];
    for (let n = FROM; n <= TO && good.length < 3; n += 1) {
      const seed = `tg-finale-${String(n)}`;
      const found = problems(seed);
      console.log(`SCAN ${seed}: ${found.length === 0 ? "PASS" : found.join("; ")}`);
      if (found.length === 0) good.push(seed);
    }
    console.log(`SCAN-GOOD ${good.join(" ")}`);
  },
  3_600_000,
);
