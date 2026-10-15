// Spec 39 (#922): a custom card from the creator to a practice game against the AI (SPEC §12).
// A fixture account builds "Cry: deal 2 damage to an enemy unit" with the no-code builder, reads it
// in the code view and saves it (R915010, R915008); finds it again after a reload; puts it in a deck,
// which the workshop marks practice only (R915005); starts practice with that deck, which runs in
// the Luau module (R915009); plays the card and sees the damage; and folds the browser's log, its
// custom card with it, with `jackioh replay` to the same hash (R915006), as specs 01 and 13 do.

import { allCardIds } from "../../support/cards.ts";
import { accounts, constants, routes, seedFor, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  ACTION_ERROR,
  ANIMATING,
  BOARD,
  CREATOR_ADD_HOOK,
  CREATOR_ADD_STEP,
  CREATOR_BUILDER,
  CREATOR_CODE_INPUT,
  CREATOR_LIST_ITEM,
  CREATOR_NAME,
  CREATOR_PREVIEW,
  CREATOR_SAVE,
  CREATOR_SAVED,
  CREATOR_SCREEN,
  CREATOR_SCRIPT_PROBLEM,
  CREATOR_STEP_AMOUNT,
  CREATOR_TARGET,
  CREATOR_VIEW_BUILDER,
  CREATOR_VIEW_CODE,
  DECK_COUNT,
  DECK_PRACTICE_ONLY,
  END_TURN,
  PRACTICE_DECK,
  PRACTICE_HUD,
  PRACTICE_START,
  PROMPT,
  PROMPT_SUBMIT,
  RESULT_OVERLAY,
  WORKSHOP,
  addPoolId,
  deckRowId,
  promptOf,
  promptOptionId,
  ts,
} from "../../support/testids.ts";
import type { Action, PlayerId } from "../../support/types.ts";

/** p2: specs 09 and 19 share p1, and R109 limits how fast one account may write. */
function me(): E2EAccount {
  return accounts.p2();
}

const SEED = seedFor("39-custom-cards");

/** The card the builder makes. */
const NAME = "Spark Lancer";
const DAMAGE = 2;

/** The practice worker loads the engine, the AI and, for this game, the Luau module first. */
const BOOT_TIMEOUT = 90_000;

/** Turns the human may pass before the card is in hand with an enemy unit to aim it at. */
const TURN_BUDGET = 12;

/** Prompts the human may meet on the way (a card's own question, a Discover). */
const PROMPT_BUDGET = 10;

/** `CustomCard` as `GET /api/custom-cards` answers it, as far as this spec reads it. */
type CustomCardBody = { id: string; version: number; frame: { name: string; text: string }; source: string };

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

function customCards(account: E2EAccount): Cypress.Chainable<CustomCardBody[]> {
  return cy
    .request<{ cards: CustomCardBody[] }>({ method: "GET", url: api("/api/custom-cards"), headers: bearer(account) })
    .then((response) => {
      expect(response.status, "GET /api/custom-cards").to.eq(200);
      return response.body.cards;
    });
}

/** R915008: every custom card the account keeps, deleted, so each run starts from none. */
function clearCustomCards(account: E2EAccount): void {
  customCards(account).then((cards) => {
    for (const card of cards) {
      cy.request({ method: "DELETE", url: api(`/api/custom-cards/${card.id}`), headers: bearer(account) })
        .its("status")
        .should("eq", 200);
    }
  });
}

// The practice page's dev handle (routes/practice.tsx), as far as this spec reads it.

type UnitLike = { instanceId: string; defId: string; health: number } | null;
type ViewLike = {
  active: PlayerId;
  viewer: PlayerId;
  phase: string;
  pending: unknown;
  you: { hand: { instanceId: string; defId: string }[] | number; units: UnitLike[]; mana: { current: number } };
  opponent: { units: UnitLike[] };
};
type PracticeDebugLike = {
  seed: string;
  decks: [string[], string[]];
  handicaps: Partial<Record<PlayerId, unknown>>;
  log: Action[];
  state: unknown;
  hash: string;
  dealt?: PlayerId[];
  customCards?: unknown[];
};
type PracticeHandleLike = { snapshot(): Promise<PracticeDebugLike>; readonly view: ViewLike | null; readonly thinking: boolean };

function practiceHandle(): Cypress.Chainable<PracticeHandleLike> {
  return cy
    .window({ timeout: timeouts.view })
    .should((win) => {
      expect((win as unknown as { __jackiohPractice?: unknown }).__jackiohPractice, "window.__jackiohPractice (dev builds)").to.not.eq(
        undefined,
      );
    })
    .then((win) => (win as unknown as { __jackiohPractice: PracticeHandleLike }).__jackiohPractice);
}

const ANY_PROMPT_OPTION = `[data-testid^="${promptOptionId("")}"]`;

function isPicked($option: JQuery<HTMLElement>): boolean {
  const marked = $option.closest("[aria-pressed], [data-selected]");
  const element = marked.length > 0 ? marked : $option;
  return element.attr("aria-pressed") === "true" || element.attr("data-selected") === "true";
}

/** The AI can take whole turns between board settlements. */
function settledThroughAiTurn(): void {
  cy.get(ANIMATING, { timeout: timeouts.game, log: false }).should("not.exist");
}

/** R9: keep the whole opening hand. */
function keepWholeHand(): void {
  cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
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
        board.attr("data-active") === "you" && board.attr("data-phase") === "main" && endTurn.length > 0 && !endTurn.is(":disabled");
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

function reachHumanTurn(prompts = PROMPT_BUDGET): void {
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

type Play = { card: string; target: string; health: number; lane: number };

/** The card in hand, an enemy unit to aim it at, an empty lane of the human's and the mana to pay. */
function playable(view: ViewLike, customId: string, cost: number): Play | null {
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const card = hand.find((held) => held.defId === customId);
  const enemy = view.opponent.units.find((unit): unit is NonNullable<UnitLike> => unit !== null);
  const lane = view.you.units.findIndex((unit) => unit === null);
  if (card === undefined || enemy === undefined || lane < 0 || view.you.mana.current < cost) return null;
  return { card: card.instanceId, target: enemy.instanceId, health: enemy.health, lane: lane + 1 };
}

/** Pass turns until the custom card can be played at an enemy unit, then say how. */
function untilPlayable(customId: string, cost: number, turns = TURN_BUDGET): Cypress.Chainable<Play> {
  reachHumanTurn();
  return practiceHandle().then((handle) => {
    const view = handle.view;
    expect(view, "the page holds a view").to.not.eq(null);
    const play = view === null ? null : playable(view, customId, cost);
    if (play !== null) return cy.wrap(play, { log: false });
    expect(turns, `the custom card is playable at an enemy unit within ${String(TURN_BUDGET)} turns`).to.be.greaterThan(0);
    cy.get(ts(END_TURN)).should("not.be.disabled").click();
    settledThroughAiTurn();
    return untilPlayable(customId, cost, turns - 1);
  });
}

/** Twenty distinct Core ids with no token, from the catalog's order, for the deck's other nineteen. */
function deckWith(customId: string): string[] {
  return [customId, ...allCardIds().slice(0, constants.DECK_SIZE - 1)];
}

describe("39 custom cards — made in the creator, kept on the account, played in practice and replayed", () => {
  before(() => {
    expect(SEED, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");
  });

  beforeEach(() => {
    cy.freeAccount(me());
    cy.clearDecks(me());
    clearCustomCards(me());
  });

  it("R915010 R915008 R915005 R915009 R915006 a card built with blocks is saved, kept, decked, played against the AI and replayed", () => {
    // R915010: the builder writes kit Luau, which the code view shows; the Luau module checks it.
    cy.visitAs(me(), routes.creator());
    cy.get(ts(CREATOR_SCREEN), { timeout: timeouts.view }).should("exist");
    cy.get(ts(CREATOR_NAME)).clear();
    cy.get(ts(CREATOR_NAME)).type(NAME);
    cy.get(ts(CREATOR_TARGET)).select("an enemy unit");
    cy.get(ts(CREATOR_ADD_HOOK)).click();
    cy.get(ts(CREATOR_ADD_STEP)).click();
    cy.get(ts(CREATOR_STEP_AMOUNT)).clear();
    cy.get(ts(CREATOR_STEP_AMOUNT)).type(String(DAMAGE));
    cy.get(ts(CREATOR_PREVIEW)).should("contain.text", `Deal ${String(DAMAGE)} damage to an enemy unit.`);

    cy.get(ts(CREATOR_VIEW_CODE)).click();
    cy.get(ts(CREATOR_CODE_INPUT))
      .invoke("val")
      .should("contain", 'target = "an enemy unit"')
      .and("contain", "c:damage(c:target(), n.dmg)");
    cy.get(ts(CREATOR_VIEW_BUILDER)).click();
    cy.get(ts(CREATOR_BUILDER)).should("exist");
    cy.get(ts(CREATOR_SCRIPT_PROBLEM)).should("not.exist");

    // R915008: saved on the account, which minted its id.
    cy.get(ts(CREATOR_SAVE), { timeout: BOOT_TIMEOUT }).should("not.be.disabled").click();
    cy.get(ts(CREATOR_SAVED), { timeout: timeouts.view }).should("exist");
    customCards(me()).then((cards) => {
      expect(cards, "one custom card on the account").to.have.length(1);
      const [card] = cards;
      expect(card?.id).to.match(/^custom-[0-9a-f]{32}$/u);
      expect(card?.frame.name).to.eq(NAME);
      expect(card?.source).to.contain("c:damage(c:target(), n.dmg)");
      cy.wrap(card?.id ?? "", { log: false }).as("customId");
    });

    // A reload, a fresh read of the account: the card is still there.
    cy.reload();
    cy.get(ts(CREATOR_LIST_ITEM), { timeout: timeouts.view }).should("have.length", 1).and("contain.text", NAME);

    // R915005: the deck that holds it is marked practice only.
    cy.get<string>("@customId").then((customId) => {
      const cards = deckWith(customId).slice(1);
      cy.saveDeck(me(), { name: "Custom deck", cards }).then((deckId) => {
        cy.visitAs(me(), routes.deckbuilder());
        cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("exist");
        cy.get(ts(deckRowId(deckId))).click();
        cy.get(ts(addPoolId(customId)), { timeout: timeouts.view }).click();
        cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", String(constants.DECK_SIZE));
        cy.get(ts(deckRowId(deckId)), { timeout: timeouts.view })
          .should("not.have.attr", "data-unsynced")
          .and("have.attr", "data-practice-only", "true")
          .find(ts(DECK_PRACTICE_ONLY))
          .should("exist");
        cy.savedDecks(me()).should((saved) => {
          expect(saved.decks[0]?.cards, "the saved deck holds the custom card").to.include(customId);
        });
      });

      // R915009: practice with that deck runs in the Luau module; the AI's deck stays a normal deck.
      const params = new URLSearchParams({ seed: SEED, seat: "p2", pace: "fast" });
      cy.visitAs(me(), `${routes.practice()}?${params.toString()}`);
      cy.get(ts(PRACTICE_DECK), { timeout: timeouts.view }).select("saved:1");
      cy.get(ts(PRACTICE_START), { timeout: timeouts.view }).should("not.be.disabled").click();
      cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("exist");
      keepWholeHand();

      untilPlayable(customId, 1).then((play) => {
        cy.playCard(play.card, { zone: { side: "you", row: "units", lane: play.lane as 1 | 2 | 3 | 4 | 5 } });
        // R81: the Cry's declared target. With one enemy unit the board takes it as the only one;
        // with more, it asks, and the spec names the unit it measured.
        cy.get("body").then(($body) => {
          if ($body.find(promptOf("target")).length > 0) cy.answerPrompt("target", { cards: [play.target] });
        });
        cy.settled();
        cy.get(ts(ACTION_ERROR)).should("not.exist");
        practiceHandle().should((handle) => {
          const unit = handle.view?.opponent.units.find((held) => held?.instanceId === play.target) ?? null;
          if (play.health > DAMAGE) {
            expect(unit?.health, "the Cry dealt its damage to the enemy unit").to.eq(play.health - DAMAGE);
          } else {
            expect(unit, "the Cry destroyed the enemy unit").to.eq(null);
          }
        });
      });

      // R915006: the browser's log folds, with its custom card, to the browser's hash.
      practiceHandle()
        .then((handle) => handle.snapshot())
        .then((debug) => {
          expect(debug.seed).to.eq(SEED);
          expect(debug.customCards, "the game carries its custom card").to.have.length(1);
          expect(debug.decks.some((deck) => deck.includes(customId)), "the human's deck holds it").to.eq(true);
          cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
            "replayHash",
            {
              label: "39-custom-cards",
              seed: debug.seed,
              decks: debug.decks,
              log: debug.log,
              state: debug.state,
              handicaps: debug.handicaps,
              customCards: debug.customCards,
              ...(debug.dealt === undefined ? {} : { dealt: debug.dealt }),
            },
            { timeout: timeouts.task },
          ).then((result) => {
            expect(result.errors, "the recorded log replays with no rejected action").to.deep.eq([]);
            expect(result.browserHash, "hashState of the page's final state is the page's own hash").to.eq(debug.hash);
            expect(result.replayHash, "the CLI's fold, through the card kit, reaches the browser's state").to.eq(result.browserHash);
          });
        });
    });
  });
});
