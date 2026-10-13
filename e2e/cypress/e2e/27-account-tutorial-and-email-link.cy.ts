// Account tutorial sync and email-link landing (SPEC §9.10; R320–R324, including R323).
// Progress only grows, so each test records a newer Show choice and asserts containment.
// BUILD M8: await intercepted requests rather than fixed waits; selectors come from `support/testids.ts`.

import { accounts, server, timeouts, TUTORIAL_PROGRESS_VERSION } from "../../support/config.ts";
import { TUTORIAL_HIDE, TUTORIAL_PATH, TUTORIAL_PATH_HIDDEN, TUTORIAL_SHOW, ts, tutorialLessonId } from "../../support/testids.ts";
import { loginTestid } from "../../../apps/web/src/auth/testids.ts";

const LOGIN_CONFIRMED = loginTestid.confirmed;
const LOGIN_LINK_ERROR = loginTestid.linkError;
import type { TutorialData } from "../../support/tasks/lessons.ts";
import { PRACTICE_PATH, TUTORIAL_BOOT_TIMEOUT, storedProgress, visitTutorial } from "../../support/tutorial.ts";

/** `TutorialProgressView` response from `crates/server/src/api/tutorial.rs`. */
type AccountProgress = { completed: string[]; hiddenChoice: { hidden: boolean; at: number } | null };

const SYNC_TIMEOUT = 20_000;

/** Spec 23's phone viewport. */
const PHONE = { width: 390, height: 844 } as const;

/** Minimum touch target (docs/polish/7-mobile-ux.md). */
const TOUCH_TARGET_PX = 44;

const account = accounts.p1;

function headers(): Record<string, string> {
  return { authorization: `Bearer ${account().token}` };
}

function readAccount(): Cypress.Chainable<AccountProgress> {
  return cy
    .request<{ progress: AccountProgress }>({ method: "GET", url: `${server.http()}/api/tutorial`, headers: headers() })
    .its("body.progress");
}

/** Spy on, never stub, account requests. */
function spyOnTheAccount(): void {
  cy.intercept("GET", "**/api/tutorial").as("load");
  cy.intercept("PUT", "**/api/tutorial").as("save");
}

/** Record Show before the client chooses, so it is the newer choice. */
function showOnTheAccount(): void {
  cy.request({
    method: "PUT",
    url: `${server.http()}/api/tutorial`,
    headers: headers(),
    body: { completed: [], hiddenChoice: { hidden: false, at: Date.now() } },
  });
}

function firstLesson(): Cypress.Chainable<string> {
  return cy.task<TutorialData>("tutorialLessons", undefined, { timeout: timeouts.task }).then((data) => {
    const first = data.lessons[0];
    expect(first, "the tutorial has a first lesson").to.not.eq(undefined);
    return (first as TutorialData["lessons"][number]).id;
  });
}

function visitPractice(completed: string[] | null): void {
  cy.signIn(account());
  visitTutorial(PRACTICE_PATH, {
    progress: completed === null ? null : { v: TUTORIAL_PROGRESS_VERSION, completed },
  });
}

describe("27 — the tutorial kept on the account (R320–R322)", () => {
  beforeEach(() => {
    showOnTheAccount();
  });

  it("a lesson won on this device reaches the account, and a device that never saw the tutorial finds it there", () => {
    firstLesson().then((lessonId) => {
      spyOnTheAccount();
      visitPractice([lessonId]);
      cy.get(ts(tutorialLessonId(lessonId)), { timeout: TUTORIAL_BOOT_TIMEOUT }).should(
        "have.attr",
        "data-status",
        "completed",
      );
      // R321: upload the missing lesson unless this shared server already has it.
      cy.wait("@load", { timeout: SYNC_TIMEOUT }).then((load) => {
        const before = (load.response?.body as { progress: AccountProgress } | undefined)?.progress;
        expect(before, "GET /api/tutorial answered").to.not.eq(undefined);
        if (before?.completed.includes(lessonId) === true) return;
        cy.wait("@save", { timeout: SYNC_TIMEOUT }).its("request.body.completed").should("include", lessonId);
      });
      readAccount().then((progress) => {
        expect(progress.completed, "the account keeps the lesson").to.include(lessonId);
      });

      visitPractice(null);
      cy.get(ts(tutorialLessonId(lessonId)), { timeout: TUTORIAL_BOOT_TIMEOUT }).should(
        "have.attr",
        "data-status",
        "completed",
      );
      storedProgress().should((stored) => {
        expect((stored as { completed?: string[] } | null)?.completed ?? [], "the device now keeps it too").to.include(
          lessonId,
        );
      });
    });
  });

  it("Hide tutorial follows the account to a fresh device, and Show brings the path back everywhere", () => {
    spyOnTheAccount();
    visitPractice(null);
    cy.get(ts(TUTORIAL_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
    cy.wait("@load", { timeout: SYNC_TIMEOUT });
    cy.get(ts(TUTORIAL_HIDE)).should("be.visible").click();

    // R322: one focused Show button replaces the path.
    cy.get(ts(TUTORIAL_PATH)).should("not.exist");
    cy.get(ts(TUTORIAL_PATH_HIDDEN)).should("be.visible");
    cy.get(ts(TUTORIAL_SHOW)).should("be.focused").and("contain.text", "Show tutorial");
    cy.wait("@save", { timeout: SYNC_TIMEOUT }).its("request.body.hiddenChoice.hidden").should("eq", true);
    readAccount().its("hiddenChoice.hidden").should("eq", true);

    visitPractice(null);
    cy.wait("@load", { timeout: SYNC_TIMEOUT });
    cy.get(ts(TUTORIAL_PATH_HIDDEN), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
    cy.get(ts(TUTORIAL_PATH)).should("not.exist");

    cy.get(ts(TUTORIAL_SHOW)).click();
    cy.get(ts(TUTORIAL_PATH)).should("be.visible");
    cy.get(ts(TUTORIAL_HIDE)).should("be.focused");
    cy.wait("@save", { timeout: SYNC_TIMEOUT }).its("request.body.hiddenChoice.hidden").should("eq", false);
    readAccount().its("hiddenChoice.hidden").should("eq", false);

    visitPractice(null);
    cy.get(ts(TUTORIAL_PATH), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
    cy.get(ts(TUTORIAL_PATH_HIDDEN)).should("not.exist");
  });

  it("on a phone, Hide and Show are full-size touch targets and the hidden path scrolls nothing sideways", () => {
    cy.viewport(PHONE.width, PHONE.height);
    spyOnTheAccount();
    visitPractice(null);
    cy.wait("@load", { timeout: SYNC_TIMEOUT });
    cy.get(ts(TUTORIAL_HIDE), { timeout: TUTORIAL_BOOT_TIMEOUT })
      .should("be.visible")
      .then(($hide) => {
        expect($hide[0]?.getBoundingClientRect().height ?? 0, "Hide is a 44px touch target").to.be.at.least(TOUCH_TARGET_PX);
      })
      .click();
    cy.get(ts(TUTORIAL_SHOW))
      .should("be.visible")
      .then(($show) => {
        const box = $show[0]?.getBoundingClientRect();
        expect(box?.height ?? 0, "Show is a 44px touch target").to.be.at.least(TOUCH_TARGET_PX);
        expect(box?.right ?? Infinity, "Show fits the screen").to.be.at.most(PHONE.width);
      });
    cy.document().then((doc) => {
      expect(doc.documentElement.scrollWidth, "no horizontal scroll").to.be.at.most(doc.documentElement.clientWidth);
    });
    cy.wait("@save", { timeout: SYNC_TIMEOUT }).its("request.body.hiddenChoice.hidden").should("eq", true);
    cy.get(ts(TUTORIAL_SHOW)).click();
    cy.get(ts(TUTORIAL_PATH)).should("be.visible");
    cy.wait("@save", { timeout: SYNC_TIMEOUT }).its("request.body.hiddenChoice.hidden").should("eq", false);
  });
});

describe("27 — the email link (R323, R324)", () => {
  // `build:e2e` has no provider: unit tests cover exchange; this spec covers cross-device landing and scrubbing.

  const CODE = "3f9c6c1e-5a6b-4c2d-9e8f-0a1b2c3d4e5f";
  /** R324 message from `AUTH_NOTICES.emailConfirmed`. */
  const CONFIRMED = "Your email is confirmed. Sign in to continue.";

  it("R324 a link's code opened on another device lands on /login, scrubbed, and says the email is confirmed, not an error", () => {
    // An unallowed redirect falls back to the Site URL (`/`).
    cy.visit(`/?code=${CODE}`, {
      onBeforeLoad(win) {
        win.localStorage.clear();
      },
    });
    cy.location("pathname").should("eq", "/login");
    cy.location("search").should("eq", "");
    cy.location("href").should("not.contain", CODE);
    cy.get(ts(LOGIN_CONFIRMED)).should("be.visible").and("have.text", CONFIRMED);
    cy.get(ts(LOGIN_LINK_ERROR)).should("not.exist");
    cy.get('[role="alert"]').should("not.exist");
  });
});
