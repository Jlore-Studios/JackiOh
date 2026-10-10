// Spec 22: lesson 1 follows the coach through the UI to a replay-verified win (SPEC §9.10; R187,
// R290, R294). Reduced motion keeps coach state fresh without changing game rules or fast AI pacing.
// BUILD M8: the lesson owns its seed; waits are retried assertions and selectors are test ids.
// The flow stays generic so lesson content may change if following the coach still wins.

import { TUTORIAL_PROGRESS_KEY, TUTORIAL_PROGRESS_VERSION, timeouts } from "../../support/config.ts";
import {
  ACTION_ERROR,
  COACH,
  TUTORIAL_HUD,
  TUTORIAL_NEXT,
  TUTORIAL_RESULT,
  TUTORIAL_STEP,
  ts,
} from "../../support/testids.ts";
import type { TutorialData } from "../../support/tasks/lessons.ts";
import {
  TUTORIAL_BOOT_TIMEOUT,
  debugSnapshot,
  lessonUrl,
  storedProgress,
  takeMoment,
  tutorialHandle,
  visitTutorial,
  waitForMoment,
  type Moment,
  type StepCounter,
  type Taken,
} from "../../support/tutorial.ts";

/** Lesson 1, the one every player may start (R294). */
const LESSON_ID = "basics";

/** Maximum coach moments before the lesson is considered stuck. */
const MOVE_BUDGET = 400;

type Recorder = { errors: string[] };
type RecorderWindow = { __tutorialRecorder?: Recorder };

/** Record transient `action-error` messages from before the first script runs. */
function installRecorder(win: Cypress.AUTWindow): void {
  const recorder: Recorder = { errors: [] };
  (win as unknown as RecorderWindow).__tutorialRecorder = recorder;
  new win.MutationObserver(() => {
    const error = win.document.querySelector(ts(ACTION_ERROR));
    const text = error?.textContent ?? null;
    if (text !== null && recorder.errors.at(-1) !== text) recorder.errors.push(text);
  }).observe(win.document, { subtree: true, childList: true, characterData: true });
}

function recordedErrors(): Cypress.Chainable<string[]> {
  return cy.window({ log: false }).then((win) => {
    const recorder = (win as unknown as RecorderWindow).__tutorialRecorder;
    expect(recorder, "the recorder installed in onBeforeLoad").to.not.eq(undefined);
    return recorder?.errors ?? [];
  });
}

type Run = {
  counters: StepCounter[];
  taken: Record<Taken, number>;
  playShot: boolean;
  outcome: string | null;
};

function followCoach(run: Run, remaining: number): void {
  waitForMoment().then((moment: Moment) => {
    if (moment.kind === "over") {
      run.outcome = moment.outcome;
      return;
    }
    if (moment.counter !== null) run.counters.push(moment.counter);
    expect(remaining, `lesson 1 ends within ${String(MOVE_BUDGET)} moments`).to.be.greaterThan(0);
    takeMoment(moment, {
      afterPickUp: () => {
        if (run.playShot) return;
        run.playShot = true;
        cy.screenshot("22-tutorial-lesson-one/2-a-play-in-flight", { capture: "viewport" });
      },
    }).then((taken) => {
      run.taken[taken] += 1;
    });
    followCoach(run, remaining - 1);
  });
}

describe("22 — tutorial lesson 1, played to a win by following the coach (§9.10, R293)", () => {
  it("R293 the coach's line wins lesson 1 through the UI, saves the lesson and replays in Node", () => {
    cy.task<TutorialData>("tutorialLessons", undefined, { timeout: timeouts.task }).then((data) => {
      const lesson = data.lessons.find((candidate) => candidate.id === LESSON_ID);
      expect(lesson, `lessons.ts has lesson "${LESSON_ID}"`).to.not.eq(undefined);
      expect(lesson?.number, "basics is lesson 1").to.eq(1);
      if (lesson === undefined) return;

      visitTutorial(lessonUrl(LESSON_ID), {
        progress: null,
        reducedMotion: true,
        onBeforeLoad: installRecorder,
      });

      cy.get(ts(TUTORIAL_HUD), { timeout: TUTORIAL_BOOT_TIMEOUT })
        .should("have.attr", "data-lesson", LESSON_ID)
        .and("have.attr", "data-human-seat", lesson.humanSeat)
        .and("contain.text", `Lesson ${String(lesson.number)}`)
        .and("contain.text", lesson.title);
      cy.get(ts(COACH), { timeout: TUTORIAL_BOOT_TIMEOUT })
        .should("be.visible")
        .and("have.attr", "data-coach-mode", "step")
        .and("have.attr", "data-coach-dock", "float")
        .and("not.have.attr", "data-stale");
      cy.get(ts(COACH)).invoke("attr", "data-coach-step").should("be.a", "string").and("not.be.empty");
      cy.get(ts(TUTORIAL_STEP)).should("contain.text", "Step 1 of");
      tutorialHandle().then((handle) => {
        expect(handle.lessonId).to.eq(LESSON_ID);
        expect(handle.display.mode, "the coach opens on a step").to.eq("step");
        expect(handle.display.stepNumber, "the coach opens on step 1").to.eq(1);
      });
      debugSnapshot().then((debug) => {
        expect(
          debug.log.filter((action) => action.playerId === lesson.humanSeat),
          "nothing has been done yet",
        ).to.deep.eq([]);
      });
      cy.screenshot("22-tutorial-lesson-one/1-first-coach-step", { capture: "viewport" });

      const run: Run = { counters: [], taken: { ack: 0, coach: 0, fallback: 0 }, playShot: false, outcome: null };
      followCoach(run, MOVE_BUDGET);

      cy.get(ts(TUTORIAL_RESULT), { timeout: timeouts.view })
        .should("be.visible")
        .and("have.attr", "data-outcome", "win")
        .and("have.attr", "data-lesson", LESSON_ID);
      cy.get(ts(TUTORIAL_NEXT)).should("be.visible");
      cy.screenshot("22-tutorial-lesson-one/3-result", { capture: "viewport" });

      cy.then(() => {
        expect(run.outcome, "the lesson's outcome").to.eq("win");
        expect(run.playShot, "the coach asked for at least one play").to.eq(true);
        expect(run.taken.coach, "actions made because the coach asked for them").to.be.greaterThan(0);
        expect(run.taken.ack, "bubbles read with Got it").to.be.greaterThan(0);
        const steps = run.counters.map((counter) => counter.step);
        steps.forEach((step, at) => {
          if (at > 0) expect(step, `the step counter never goes back (moment ${String(at)})`).to.be.at.least(steps[at - 1] ?? 0);
        });
        expect(new Set(run.counters.map((counter) => counter.of)).size, "one step count for the whole lesson").to.eq(1);
        expect(Math.max(...steps), "the step counter advanced past step 1").to.be.greaterThan(1);
        cy.task("log", `[22] lesson 1 won: ${JSON.stringify(run.taken)}; steps seen ${JSON.stringify([...new Set(steps)])} of ${String(run.counters[0]?.of ?? 0)}`);
      });

      cy.get(ts(ACTION_ERROR)).should("not.exist");
      recordedErrors().then((errors) => {
        expect(errors, "action-error never appeared").to.deep.eq([]);
      });

      storedProgress().then((stored) => {
        expect(stored, `localStorage["${TUTORIAL_PROGRESS_KEY}"]`).to.deep.include({ v: TUTORIAL_PROGRESS_VERSION });
        expect((stored as { completed?: unknown }).completed, "the completed lessons").to.include(LESSON_ID);
      });

      debugSnapshot().then((debug) => {
        expect(debug.lesson, "the debug record names the lesson").to.eq(LESSON_ID);
        expect(debug.seed, "the lesson's own seed").to.eq(lesson.seed);
        expect(debug.humanSeat).to.eq(lesson.humanSeat);
        const aiSeat = lesson.humanSeat === "p1" ? "p2" : "p1";
        expect(debug.handicaps[aiSeat], "the AI seat plays with the tutorial handicap (R290)").to.deep.eq(data.aiTutorial);
        expect(debug.log.some((action) => action.playerId === aiSeat), "the AI acted").to.eq(true);

        cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
          "replayHash",
          {
            label: "22-tutorial-lesson-one",
            seed: debug.seed,
            decks: debug.decks,
            log: debug.log,
            state: debug.state,
            handicaps: debug.handicaps,
          },
          { timeout: timeouts.task },
        ).then((result) => {
          expect(result.errors, "the lesson's log replays with no rejected action").to.deep.eq([]);
          expect(result.browserHash, "hashState of the page's final state is the page's own hash").to.eq(debug.hash);
          expect(result.replayHash, "the Node fold, tutorial handicap included, reaches the browser's state").to.eq(
            result.browserHash,
          );
        });
      });
    });
  });
});
