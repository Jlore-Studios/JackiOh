// The practice screen as specs 13 and 40 drive it (#2160): the opening hand kept whole, a prompt's
// first option taken, and the wait through the AI's turns for the human's main phase. All of it
// acts through the UI and reads the DOM, as each spec's own copy did.

import { timeouts } from "./config.ts";
import { ANIMATING, BOARD, END_TURN, PROMPT, PROMPT_SUBMIT, RESULT_OVERLAY, promptOf, promptOptionId, ts } from "./testids.ts";

/** Prompts the human may meet on the way to its main phase: a rare cast-on-draw card's (R58), a card's own question, a Discover. */
const PROMPT_BUDGET = 10;

const ANY_PROMPT_OPTION = `[data-testid^="${promptOptionId("")}"]`;

function isPicked($option: JQuery<HTMLElement>): boolean {
  const marked = $option.closest("[aria-pressed], [data-selected]");
  const element = marked.length > 0 ? marked : $option;
  return element.attr("aria-pressed") === "true" || element.attr("data-selected") === "true";
}

/** The AI can take whole turns between board settlements. */
export function settledThroughAiTurn(): void {
  cy.get(ANIMATING, { timeout: timeouts.game, log: false }).should("not.exist");
}

/**
 * R9: select every mulligan card, then confirm. `bootTimeout` is how long the spec gives the practice
 * worker to boot before the mulligan shows (spec 40's also loads the Luau module).
 */
export function keepWholeHand(bootTimeout: number): void {
  cy.get(promptOf("mulligan"), { timeout: bootTimeout }).should("be.visible");
  cy.get(promptOf("mulligan")).within(() => {
    cy.get(ANY_PROMPT_OPTION).each(($option) => {
      if (isPicked($option)) return;
      cy.wrap($option, { log: false }).click();
    });
    cy.get(ts(PROMPT_SUBMIT)).click();
  });
  settledThroughAiTurn();
}

function answerFirstOption(): void {
  cy.get(PROMPT).first().within(() => {
    cy.get(ANY_PROMPT_OPTION).first().click();
  });
  cy.get("body").then(($body) => {
    const submit = $body.find(ts(PROMPT_SUBMIT));
    if (submit.length > 0 && !submit.is(":disabled")) cy.wrap(submit.first()).click();
  });
  settledThroughAiTurn();
}

type Waited = "turn" | "prompt" | "over";

function waitForHuman(): Cypress.Chainable<Waited> {
  return cy
    .get("body", { timeout: timeouts.game })
    .should(($body) => {
      const board = $body.find(ts(BOARD));
      const endTurn = $body.find(ts(END_TURN));
      const myTurn =
        board.attr("data-active") === "you" &&
        board.attr("data-phase") === "main" &&
        endTurn.length > 0 &&
        !endTurn.is(":disabled");
      const prompt = $body.find(PROMPT).length > 0;
      const over = $body.find(ts(RESULT_OVERLAY)).length > 0;
      expect(myTurn || prompt || over, "the human's main phase, a prompt for the human, or a result").to.eq(true);
    })
    .then(($body): Waited => {
      if ($body.find(ts(RESULT_OVERLAY)).length > 0) return "over";
      if ($body.find(PROMPT).length > 0) return "prompt";
      return "turn";
    });
}

/** Reach the human main phase, resolving at most PROMPT_BUDGET prompts. */
export function reachHumanTurn(prompts = PROMPT_BUDGET): void {
  waitForHuman().then((waited) => {
    expect(waited, "the game is still on when the human is due to act").to.not.eq("over");
    if (waited === "prompt") {
      expect(prompts, "the human's prompts run out").to.be.greaterThan(0);
      answerFirstOption();
      reachHumanTurn(prompts - 1);
      return;
    }
    cy.settled();
  });
}
