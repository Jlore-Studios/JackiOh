// Tutorial path coverage (SPEC §9.10; R290, R291, R294, R314).

import { TUTORIAL_HANDICAP, TUTORIAL_PROGRESS_VERSION, constants, timeouts } from "../../support/config.ts";
import {
  COACH,
  COACH_ACK,
  END_TURN,
  PRACTICE_ERROR,
  PRACTICE_LEAVE,
  PRACTICE_LEAVE_CONFIRM,
  PRACTICE_LEAVE_STAY,
  PRACTICE_LOADING,
  PRACTICE_SETUP,
  TUTORIAL_CONTINUE,
  TUTORIAL_EXIT,
  TUTORIAL_HUD,
  TUTORIAL_PATH,
  handCardId,
  handRegionId,
  healthIs,
  heroId,
  ts,
  tutorialLessonId,
  tutorialStartId,
} from "../../support/testids.ts";
import type { TutorialData, TutorialLessonData } from "../../support/tasks/lessons.ts";
import type { PlayerId } from "../../support/types.ts";
import {
  PRACTICE_PATH,
  TUTORIAL_BOOT_TIMEOUT,
  coachMarkIn,
  debugSnapshot,
  handOf,
  lessonUrl,
  practiceHandle,
  stepCounter,
  storedProgress,
  takeMoment,
  visitTutorial,
  waitForMoment,
} from "../../support/tutorial.ts";

const LATER_LESSON = "traps";

const FAST_LOBBY = `${PRACTICE_PATH}?pace=fast`;

const PHONE = { width: 390, height: 844 } as const;

const PHONE_MOMENTS = 16;

const TIP_BUDGET = 6;

type Status = "locked" | "unlocked" | "completed";

function lessons(): Cypress.Chainable<TutorialData> {
  return cy.task<TutorialData>("tutorialLessons", undefined, { timeout: timeouts.task }).then((data) => {
    expect(data.lessons.length, "the tutorial has lessons").to.be.greaterThan(1);
    data.lessons.forEach((lesson, at) => {
      expect(lesson.number, `lessons.ts lists ${lesson.id} in path order`).to.eq(at + 1);
    });
    return data;
  });
}

function lessonNamed(data: TutorialData, id: string): TutorialLessonData {
  const lesson = data.lessons.find((candidate) => candidate.id === id);
  expect(lesson, `lessons.ts has lesson "${id}"`).to.not.eq(undefined);
  return lesson as TutorialLessonData;
}

function otherSeat(seat: PlayerId): PlayerId {
  return seat === "p1" ? "p2" : "p1";
}

function progressUpTo(data: TutorialData, lesson: TutorialLessonData): { v: number; completed: string[] } {
  return { v: TUTORIAL_PROGRESS_VERSION, completed: data.lessons.slice(0, lesson.number - 1).map((each) => each.id) };
}

function expectPath(data: TutorialData, statuses: readonly Status[], next: string | null): void {
  cy.get(ts(TUTORIAL_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
  data.lessons.forEach((lesson, at) => {
    const status = statuses[at];
    cy.get(ts(tutorialLessonId(lesson.id))).should("have.attr", "data-status", status ?? "");
    if (lesson.id === next) cy.get(ts(tutorialLessonId(lesson.id))).should("have.attr", "data-next", "true");
    else cy.get(ts(tutorialLessonId(lesson.id))).should("not.have.attr", "data-next");
    const button = cy.get(ts(tutorialStartId(lesson.id)));
    if (status === "locked") button.should("have.attr", "aria-disabled", "true");
    else button.should("not.have.attr", "aria-disabled");
  });
  cy.get(ts(TUTORIAL_PATH)).should(
    "have.attr",
    "data-complete",
    statuses.every((status) => status === "completed") ? "true" : "false",
  );
}

function startFromPath(lesson: TutorialLessonData): void {
  cy.get(ts(tutorialStartId(lesson.id))).should("not.have.attr", "aria-disabled");
  cy.get(ts(tutorialStartId(lesson.id))).click();
  cy.get(ts(TUTORIAL_HUD), { timeout: TUTORIAL_BOOT_TIMEOUT })
    .should("have.attr", "data-lesson", lesson.id)
    .and("have.attr", "data-human-seat", lesson.humanSeat)
    .and("have.attr", "data-ai-seat", otherSeat(lesson.humanSeat))
    .and("contain.text", `Lesson ${String(lesson.number)}`);
  practiceHandle();
}

/** R291, R290: a lesson uses its fixed seed, decks and tutorial handicap. */
function expectLessonGame(data: TutorialData, lesson: TutorialLessonData): void {
  debugSnapshot().then((debug) => {
    const human = lesson.humanSeat === "p1" ? 0 : 1;
    const aiSeat = otherSeat(lesson.humanSeat);
    expect(debug.lesson, "the debug record names the lesson").to.eq(lesson.id);
    expect(debug.seed, "the lesson's own seed").to.eq(lesson.seed);
    expect(debug.humanSeat, "the lesson's own seat").to.eq(lesson.humanSeat);
    expect(debug.decks[human], "the human plays the lesson's humanDeck").to.deep.eq(lesson.humanDeck);
    expect(debug.decks[1 - human], "the AI plays the lesson's aiDeck").to.deep.eq(lesson.aiDeck);
    expect(debug.handicaps[aiSeat], "the AI seat's handicap is the tutorial's (R290)").to.deep.eq(TUTORIAL_HANDICAP);
    expect(debug.handicaps[lesson.humanSeat], "the human plays unhandicapped").to.eq(undefined);
    expect(data.aiTutorial, "AI_TUTORIAL in the engine still says the same").to.deep.eq(TUTORIAL_HANDICAP);
    expect(lesson.aiDeck, "the AI's deck is exactly the handicap's deck size (R184)").to.have.length(TUTORIAL_HANDICAP.deckSize);
  });
}

function exitLesson(): void {
  cy.get(ts(TUTORIAL_EXIT)).click();
  cy.get(ts(PRACTICE_LEAVE)).should("be.visible");
  cy.get(ts(PRACTICE_LEAVE_CONFIRM)).click();
  cy.get(ts(TUTORIAL_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
  cy.get(ts(TUTORIAL_HUD)).should("not.exist");
}

/** R314: no Skip controls in the HUD or coach bubble. */
function expectNoSkip(doc: Document): void {
  for (const region of [TUTORIAL_HUD, COACH]) {
    const root = doc.querySelector(ts(region));
    const skips = Array.from(root?.querySelectorAll("button, [data-testid]") ?? []).filter((element) =>
      /skip/i.test([element.textContent ?? "", element.getAttribute("aria-label") ?? "", element.getAttribute("data-testid") ?? ""].join(" ")),
    );
    expect(
      skips.map((element) => element.outerHTML),
      `no Skip step in ${region}`,
    ).to.deep.eq([]);
  }
}

function coachOnAStep(budget = TIP_BUDGET): void {
  cy.get(ts(COACH), { timeout: TUTORIAL_BOOT_TIMEOUT })
    .should(($coach) => {
      expect($coach, "the coach has caught up with the board").not.to.have.attr("data-stale");
    })
    .then(($coach) => {
      if ($coach.attr("data-coach-mode") !== "tip") return;
      expect(budget, "the tips in the way run out").to.be.greaterThan(0);
      const tip = $coach.attr("data-coach-step") ?? "";
      cy.get(ts(COACH_ACK)).click();
      cy.get(ts(COACH)).should(($now) => {
        expect($now.attr("data-coach-step"), `the tip ${tip} is gone`).to.not.eq(tip);
      });
      coachOnAStep(budget - 1);
    });
}

type Opening = { instanceIds: string[]; defIds: string[] };

/** The opening hand must agree with the board (§2.1). */
function openingHand(lesson: TutorialLessonData): Cypress.Chainable<Opening> {
  debugSnapshot().then((debug) => {
    expect(debug.log.filter((action) => action.playerId === lesson.humanSeat), "the player has done nothing yet").to.deep.eq([]);
  });
  return practiceHandle().then((handle) => {
    const view = handle.view;
    expect(view, "the page holds a view").to.not.eq(null);
    const hand = view === null ? [] : handOf(view);
    expect(hand.length, "an opening hand is dealt (§2.1)").to.be.at.least(Math.min(...constants.OPENING_DRAW));
    for (const card of hand) cy.get(ts(handCardId(card.instanceId))).should("exist");
    return cy.wrap<Opening>(
      { instanceIds: hand.map((card) => card.instanceId), defIds: hand.map((card) => card.defId) },
      { log: false },
    );
  });
}

type Box = { left: number; top: number; right: number; bottom: number };

function boxOf(element: Element): Box {
  const rect = element.getBoundingClientRect();
  return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
}

function overlapArea(a: Box, b: Box): number {
  const width = Math.min(a.right, b.right) - Math.max(a.left, b.left);
  const height = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
  return width > 0 && height > 0 ? width * height : 0;
}

/** On a phone the coach must not cover End turn or the hand. */
function expectCoachClearOfEndTurn(when: string): void {
  cy.get(ts(COACH)).should("be.visible").and("have.attr", "data-coach-dock", "panel");
  cy.get(ts(END_TURN)).should("be.visible");
  cy.document({ log: false }).then((doc) => {
    const coach = doc.querySelector(ts(COACH));
    const endTurn = doc.querySelector(ts(END_TURN));
    const hand = doc.querySelector(ts(handRegionId("you")));
    expect(coach, "the coach").to.not.eq(null);
    expect(endTurn, "End turn").to.not.eq(null);
    expect(hand, "your hand").to.not.eq(null);
    if (coach === null || endTurn === null || hand === null) return;
    const a = boxOf(coach);
    const b = boxOf(endTurn);
    const c = boxOf(hand);
    const mark = JSON.stringify(coachMarkIn(doc));
    expect(overlapArea(a, b), `${when}: the coach ${mark} ${JSON.stringify(a)} and End turn ${JSON.stringify(b)} overlap`).to.eq(0);
    expect(overlapArea(a, c), `${when}: the coach ${mark} ${JSON.stringify(a)} and your hand ${JSON.stringify(c)} overlap`).to.eq(0);
    for (const [name, box] of [["the coach", a], ["End turn", b]] as const) {
      expect(box.left, `${name} inside the screen`).to.be.at.least(0);
      expect(box.right, `${name} inside the screen`).to.be.at.most(PHONE.width);
      expect(box.bottom, `${name} inside the screen`).to.be.at.most(PHONE.height);
    }
    expect(c.top, "your hand on screen").to.be.lessThan(PHONE.height);
  });
}

function phoneMoments(remaining: number): void {
  waitForMoment().then((moment) => {
    expect(moment.kind, "the lesson is still on").to.not.eq("over");
    if (moment.kind === "over") return;
    cy.document({ log: false }).then((doc) => {
      if (doc.querySelector(ts(COACH)) !== null) expectCoachClearOfEndTurn(`moment ${String(PHONE_MOMENTS - remaining + 1)} (${moment.kind})`);
    });
    if (moment.kind === "turn") return;
    expect(remaining, `the player's first turn comes within ${String(PHONE_MOMENTS)} moments`).to.be.greaterThan(0);
    takeMoment(moment);
    phoneMoments(remaining - 1);
  });
}

describe("23 — the tutorial's lesson path: progress, no Skip step and Exit, a lesson's fixed deck, the phone (§9.10)", () => {
  it("R294 anonymous /practice: the path tops the page, lesson 1 is open, the others are locked and start nothing", () => {
    lessons().then((data) => {
      visitTutorial(PRACTICE_PATH, { progress: null });
      const first = data.lessons[0] as TutorialLessonData;
      expectPath(
        data,
        data.lessons.map((_lesson, at) => (at === 0 ? "unlocked" : "locked")),
        first.id,
      );
      cy.get(ts(TUTORIAL_CONTINUE)).should("not.exist");

      cy.get(ts(PRACTICE_SETUP)).should("be.visible");
      cy.document().then((doc) => {
        const path = doc.querySelector(ts(TUTORIAL_PATH));
        const setup = doc.querySelector(ts(PRACTICE_SETUP));
        expect(doc.querySelector("section"), "the path is the page's first section").to.eq(path);
        if (path === null || setup === null) return;
        expect(path.compareDocumentPosition(setup) & Node.DOCUMENT_POSITION_FOLLOWING, "the setup comes after the path").to.not.eq(0);
        expect(path.getBoundingClientRect().bottom, "the path sits above the setup").to.be.at.most(setup.getBoundingClientRect().top);
      });

      for (const locked of data.lessons.slice(1)) {
        cy.get(ts(tutorialStartId(locked.id))).click();
        cy.get(ts(PRACTICE_LOADING)).should("not.exist");
        cy.get(ts(TUTORIAL_HUD)).should("not.exist");
        cy.get(ts(PRACTICE_SETUP)).should("be.visible");
        cy.get(ts(tutorialLessonId(locked.id))).should("have.attr", "data-status", "locked");
      }
      cy.window().then((win) => {
        expect((win as unknown as { __jackiohPractice?: unknown }).__jackiohPractice, "no game was started").to.eq(undefined);
      });
      cy.location("search").should("eq", "");
      storedProgress().should("eq", null);
    });
  });

  it("R294 saved progress shows lesson 1 done and lesson 2 next, survives a reload, and a corrupt value is no progress", () => {
    lessons().then((data) => {
      const [first, second] = data.lessons as [TutorialLessonData, TutorialLessonData];
      const done = { v: TUTORIAL_PROGRESS_VERSION, completed: [first.id] };
      const afterOne: Status[] = data.lessons.map((_lesson, at) => (at === 0 ? "completed" : at === 1 ? "unlocked" : "locked"));

      visitTutorial(PRACTICE_PATH, { progress: done });
      expectPath(data, afterOne, second.id);
      cy.get(ts(tutorialStartId(first.id))).should("contain.text", "Replay");
      cy.get(ts(tutorialStartId(second.id))).should("contain.text", "Start");
      cy.get(ts(TUTORIAL_CONTINUE)).should("be.visible").and("contain.text", `Lesson ${String(second.number)}`);

      cy.reload();
      expectPath(data, afterOne, second.id);
      storedProgress().should("deep.eq", done);

      visitTutorial(PRACTICE_PATH, { progress: "{bad json" });
      expectPath(
        data,
        data.lessons.map((_lesson, at) => (at === 0 ? "unlocked" : "locked")),
        first.id,
      );
      cy.get(ts(TUTORIAL_CONTINUE)).should("not.exist");
      cy.get(ts(PRACTICE_SETUP)).should("be.visible");
      cy.get(ts(PRACTICE_ERROR)).should("not.exist");
    });
  });

  it("R290 R291 R314 lesson 2 from the path: its seed, decks and handicap; no Skip step anywhere; Exit asks, Stay stays, Exit leaves", () => {
    lessons().then((data) => {
      const [first, second] = data.lessons as [TutorialLessonData, TutorialLessonData];
      const done = { v: TUTORIAL_PROGRESS_VERSION, completed: [first.id] };
      const afterOne: Status[] = data.lessons.map((_lesson, at) => (at === 0 ? "completed" : at === 1 ? "unlocked" : "locked"));

      visitTutorial(FAST_LOBBY, { progress: done });
      expectPath(data, afterOne, second.id);
      startFromPath(second);
      expectLessonGame(data, second);
      cy.get(ts(heroId("opponent"))).find(healthIs(TUTORIAL_HANDICAP.heroHealth)).should("exist");

      coachOnAStep();
      stepCounter().then((counter) => {
        expect(counter.of, "the HUD counts the lesson's steps").to.be.greaterThan(0);
      });
      cy.get(ts(TUTORIAL_HUD)).find(ts(TUTORIAL_EXIT)).should("be.visible").and("have.attr", "aria-label", "Exit tutorial");
      cy.document().should((doc) => {
        expectNoSkip(doc);
      });

      debugSnapshot().then((before) => {
        cy.get(ts(TUTORIAL_EXIT)).click();
        cy.get(ts(PRACTICE_LEAVE)).should("be.visible");
        cy.get(ts(PRACTICE_LEAVE_STAY)).click();
        cy.get(ts(PRACTICE_LEAVE)).should("not.exist");
        cy.get(ts(TUTORIAL_HUD)).should("have.attr", "data-lesson", second.id);
        debugSnapshot().then((after) => {
          expect(after.seed, "the same game").to.eq(before.seed);
          expect(after.lesson, "the same lesson").to.eq(before.lesson);
          // The AI may have moved, so the prior log must remain as a prefix.
          expect(after.log.slice(0, before.log.length), "the game was not restarted").to.deep.eq(before.log);
        });
      });

      exitLesson();
      cy.get(ts(PRACTICE_SETUP)).should("be.visible");
      expectPath(data, afterOne, second.id);
      storedProgress().should("deep.eq", done);
    });
  });

  it("R291 a later lesson deals its fixed opening hand, Quickdraw card included, and deals it again the second time", () => {
    lessons().then((data) => {
      const later = lessonNamed(data, LATER_LESSON);
      const progress = progressUpTo(data, later);
      visitTutorial(FAST_LOBBY, { progress });
      cy.get(ts(tutorialLessonId(later.id)), { timeout: TUTORIAL_BOOT_TIMEOUT })
        .should("have.attr", "data-status", "unlocked")
        .and("have.attr", "data-next", "true");

      startFromPath(later);
      expectLessonGame(data, later);
      openingHand(later).then((first) => {
        for (const defId of first.defIds) expect(later.humanDeck, `${defId} comes from the lesson's deck`).to.include(defId);
        const quickdraw = later.humanDeck.filter((defId) => data.quickdraw.includes(defId));
        cy.task("log", `[23] ${later.id} opening hand ${JSON.stringify(first.defIds)}; Quickdraw in its deck ${JSON.stringify(quickdraw)}`);
        for (const defId of quickdraw) {
          expect(first.defIds, `the Quickdraw card ${defId} starts in the opening hand`).to.include(defId);
        }

        exitLesson();
        startFromPath(later);
        expectLessonGame(data, later);
        openingHand(later).then((second) => {
          expect(second.defIds, "the same cards, in the same order").to.deep.eq(first.defIds);
          expect(second.instanceIds, "the same instances").to.deep.eq(first.instanceIds);
        });
      });
      exitLesson();
      storedProgress().should("deep.eq", progress);
    });
  });

  it("the phone: the path lists every lesson with no horizontal scroll, and the coach keeps off End turn up to the first turn", () => {
    cy.viewport(PHONE.width, PHONE.height);
    lessons().then((data) => {
      visitTutorial(PRACTICE_PATH, { progress: null });
      cy.get(ts(TUTORIAL_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
      for (const lesson of data.lessons) {
        cy.get(ts(tutorialLessonId(lesson.id))).scrollIntoView();
        cy.get(ts(tutorialLessonId(lesson.id))).should("be.visible");
        cy.get(ts(tutorialStartId(lesson.id))).should("be.visible");
        cy.get(ts(tutorialLessonId(lesson.id))).should(($lesson) => {
          const box = boxOf($lesson[0] as Element);
          expect(box.left, `${lesson.id} inside the screen`).to.be.at.least(0);
          expect(box.right, `${lesson.id} inside the screen`).to.be.at.most(PHONE.width);
        });
      }
      cy.window().then((win) => {
        const root = win.document.documentElement;
        expect(root.scrollWidth, "no horizontal scroll").to.be.at.most(root.clientWidth);
        expect(win.document.body.scrollWidth, "no horizontal scroll in the body").to.be.at.most(root.clientWidth);
      });
      cy.get(ts(tutorialLessonId((data.lessons[0] as TutorialLessonData).id))).scrollIntoView();
      cy.screenshot("23-tutorial-path/phone-path", { capture: "viewport" });

      const first = data.lessons[0] as TutorialLessonData;
      visitTutorial(lessonUrl(first.id), { progress: null });
      cy.get(ts(TUTORIAL_HUD), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("have.attr", "data-lesson", first.id);
      cy.get(ts(COACH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
      cy.screenshot("23-tutorial-path/phone-lesson-first-bubble", { capture: "viewport" });
      phoneMoments(PHONE_MOMENTS);
      cy.screenshot("23-tutorial-path/phone-lesson-first-turn", { capture: "viewport" });
      cy.window().then((win) => {
        const root = win.document.documentElement;
        expect(root.scrollWidth, "no horizontal scroll in a lesson").to.be.at.most(root.clientWidth);
      });
    });
  });
});
