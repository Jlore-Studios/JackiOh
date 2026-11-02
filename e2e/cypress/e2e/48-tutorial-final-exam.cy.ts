// Spec 48: lesson 5, the final exam, from the path to a replay-verified win by following the coach through
// the UI (SPEC §9.10; R2205000, R2205002, R2205003). Reduced motion keeps the coach fresh without
// changing a rule. The flow stays generic, as spec 22's does, so the lesson's content may change while
// following the coach still wins.

import { TUTORIAL_PROGRESS_KEY, TUTORIAL_PROGRESS_VERSION, timeouts } from "../../support/config.ts";
import {
  ACTION_ERROR,
  COACH,
  FIELD_GUIDE,
  FIELD_GUIDE_CLOSE,
  FIELD_GUIDE_PATH,
  TUTORIAL_HUD,
  TUTORIAL_PLAY_PRACTICE,
  TUTORIAL_RESULT,
  fieldGuideEntryId,
  ts,
  tutorialLessonId,
  tutorialStartId,
} from "../../support/testids.ts";
import type { TutorialData } from "../../support/tasks/lessons.ts";
import {
  PRACTICE_PATH,
  TUTORIAL_BOOT_TIMEOUT,
  debugSnapshot,
  storedProgress,
  takeMoment,
  visitTutorial,
  waitForMoment,
  type Moment,
  type Taken,
} from "../../support/tutorial.ts";

const LESSON_ID = "finale";

/** Maximum coach moments before the lesson is considered stuck. */
const MOVE_BUDGET = 400;

type Run = { taken: Record<Taken, number>; modes: string[]; outcome: string | null };

function followCoach(run: Run, remaining: number): void {
  waitForMoment().then((moment: Moment) => {
    if (moment.kind === "over") {
      run.outcome = moment.outcome;
      return;
    }
    if (moment.coach !== null) run.modes.push(moment.coach.mode);
    expect(remaining, `lesson 5 ends within ${String(MOVE_BUDGET)} moments`).to.be.greaterThan(0);
    takeMoment(moment).then((taken) => {
      run.taken[taken] += 1;
    });
    followCoach(run, remaining - 1);
  });
}

describe("48 — tutorial lesson 5, the final exam, played to a win by following the coach (§9.10, R2205003)", () => {
  it("R2205003 lesson 5 opens after the fourth, deals its Legendary cards with no pool hold, and the coach's line wins it", () => {
    cy.task<TutorialData>("tutorialLessons", undefined, { timeout: timeouts.task }).then((data) => {
      const lesson = data.lessons.find((candidate) => candidate.id === LESSON_ID);
      expect(lesson, `lessons.ts has lesson "${LESSON_ID}"`).to.not.eq(undefined);
      if (lesson === undefined) return;
      expect(lesson.number, "the final exam is the last lesson").to.eq(data.lessons.length);
      const before = data.lessons.filter((each) => each.number < lesson.number).map((each) => each.id);

      visitTutorial(PRACTICE_PATH, { progress: { v: TUTORIAL_PROGRESS_VERSION, completed: before }, reducedMotion: true });
      cy.get(ts(tutorialLessonId(LESSON_ID)), { timeout: TUTORIAL_BOOT_TIMEOUT })
        .should("have.attr", "data-status", "unlocked")
        .and("have.attr", "data-next", "true");
      cy.get(ts(tutorialStartId(LESSON_ID))).click();

      cy.get(ts(TUTORIAL_HUD), { timeout: TUTORIAL_BOOT_TIMEOUT })
        .should("have.attr", "data-lesson", LESSON_ID)
        .and("have.attr", "data-human-seat", lesson.humanSeat)
        .and("contain.text", lesson.title);
      cy.get(ts(COACH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
      cy.screenshot("48-tutorial-final-exam/1-first-moment", { capture: "viewport" });

      debugSnapshot().then((debug) => {
        expect(debug.lesson).to.eq(LESSON_ID);
        expect(debug.seed, "the lesson's own seed").to.eq(lesson.seed);
        const human = lesson.humanSeat === "p1" ? 0 : 1;
        expect(debug.decks[human], "the human plays the lesson's deck").to.deep.eq(lesson.humanDeck);
        expect(debug.handicaps[lesson.humanSeat], "the last lesson holds no pool (R2201001)").to.not.have.property("poolHold");
        expect(lesson.poolHold, "lessons.ts agrees").to.eq(false);
      });

      const run: Run = { taken: { ack: 0, coach: 0, fallback: 0 }, modes: [], outcome: null };
      followCoach(run, MOVE_BUDGET);

      cy.get(ts(TUTORIAL_RESULT), { timeout: timeouts.view })
        .should("be.visible")
        .and("have.attr", "data-outcome", "win")
        .and("have.attr", "data-lesson", LESSON_ID);
      // The last lesson ends the tutorial: its result offers a practice game.
      cy.get(ts(TUTORIAL_PLAY_PRACTICE)).should("be.visible");
      cy.screenshot("48-tutorial-final-exam/2-result", { capture: "viewport" });

      cy.then(() => {
        expect(run.outcome).to.eq("win");
        expect(run.taken.coach, "actions made because the coach asked for them").to.be.greaterThan(0);
        expect(run.modes, "the coach named a discovery").to.include("discovery");
        cy.task("log", `[48] lesson 5 won: ${JSON.stringify(run.taken)}`);
      });
      cy.get(ts(ACTION_ERROR)).should("not.exist");

      storedProgress().then((stored) => {
        expect(stored, `localStorage["${TUTORIAL_PROGRESS_KEY}"]`).to.deep.include({ v: TUTORIAL_PROGRESS_VERSION });
        expect((stored as { completed?: unknown }).completed, "every lesson won").to.include(LESSON_ID);
        expect((stored as { found?: unknown }).found, "the discoveries reached the Field Guide (R2205002)").to.include("quickdraw");
      });

      debugSnapshot().then((debug) => {
        cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
          "replayHash",
          { label: "48-tutorial-final-exam", seed: debug.seed, decks: debug.decks, log: debug.log, state: debug.state, handicaps: debug.handicaps },
          { timeout: timeouts.task },
        ).then((result) => {
          expect(result.errors, "the lesson's log replays with no rejected action").to.deep.eq([]);
          expect(result.replayHash, "the Node fold reaches the browser's state").to.eq(result.browserHash);
        });
      });

      // The Field Guide: what the exam found is there with its rule.
      cy.visit(PRACTICE_PATH);
      cy.get(ts(FIELD_GUIDE_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).click();
      cy.get(ts(FIELD_GUIDE)).should("be.visible");
      cy.get(ts(fieldGuideEntryId("quickdraw"))).should("have.attr", "data-found", "true");
      cy.screenshot("48-tutorial-final-exam/3-field-guide", { capture: "viewport" });
      cy.get(ts(FIELD_GUIDE_CLOSE)).click();
      cy.get(ts(FIELD_GUIDE)).should("not.exist");
    });
  });
});
