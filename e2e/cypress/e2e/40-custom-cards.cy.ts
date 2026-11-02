// Spec 40 (#922): a custom card from the creator to a practice game against the AI (SPEC §12). The browser's
// log, its custom card with it, folds with `jackioh replay` to the same hash (R915006).

import { allCardIds } from "../../support/cards.ts";
import { accounts, apiUrl, bearerHeaders, constants, routes, seedFor, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  ACTION_ERROR,
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
  WORKSHOP,
  addPoolId,
  deckRowId,
  promptOf,
  ts,
} from "../../support/testids.ts";
import type { Action, PlayerId } from "../../support/types.ts";
import { keepWholeHand, reachHumanTurn, settledThroughAiTurn } from "../../support/practice.ts";

/** p2: specs 09 and 19 share p1, and R109 limits how fast one account may write. */
function me(): E2EAccount {
  return accounts.p2();
}

const SEED = seedFor("40-custom-cards");

const NAME = "Spark Lancer";
const DAMAGE = 2;

/** The practice worker loads the engine, the AI and, for this game, the Luau module first. */
const BOOT_TIMEOUT = 90_000;

/** SPEC §6: damage a shielding unit does not take whole, so the Cry's target is never one of these. */
const SHIELDING_KEYWORDS = ["Divine Shield", "Armor", "Indestructible"];

/** Turns the human may pass before the card is in hand with an enemy unit to aim it at. */
const TURN_BUDGET = 12;

/** `CustomCard` as `GET /api/custom-cards` answers it, as far as this spec reads it. */
type CustomCardBody = { id: string; version: number; frame: { name: string; text: string }; source: string };

function customCards(account: E2EAccount): Cypress.Chainable<CustomCardBody[]> {
  return cy
    .request<{ cards: CustomCardBody[] }>({ method: "GET", url: apiUrl("/api/custom-cards"), headers: bearerHeaders(account.token) })
    .then((response) => {
      expect(response.status, "GET /api/custom-cards").to.eq(200);
      return response.body.cards;
    });
}

/** R915008: every custom card the account keeps, deleted, so each run starts from none. */
function clearCustomCards(account: E2EAccount): void {
  customCards(account).then((cards) => {
    for (const card of cards) {
      cy.request({ method: "DELETE", url: apiUrl(`/api/custom-cards/${card.id}`), headers: bearerHeaders(account.token) })
        .its("status")
        .should("eq", 200);
    }
  });
}

// The practice page's dev handle (routes/practice.tsx), as far as this spec reads it.

type UnitLike = { instanceId: string; defId: string; health: number; armor: number; keywords: { kind: string }[] } | null;
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

type Play = { card: string; target: string; health: number; lane: number };

function takesWholeHit(unit: NonNullable<UnitLike>): boolean {
  return unit.armor === 0 && !unit.keywords.some((keyword) => SHIELDING_KEYWORDS.includes(keyword.kind));
}

function playable(view: ViewLike, customId: string, cost: number): Play | null {
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const card = hand.find((held) => held.defId === customId);
  const enemy = view.opponent.units.find((unit): unit is NonNullable<UnitLike> => unit !== null && takesWholeHit(unit));
  const lane = view.you.units.findIndex((unit) => unit === null);
  if (card === undefined || enemy === undefined || lane < 0 || view.you.mana.current < cost) return null;
  return { card: card.instanceId, target: enemy.instanceId, health: enemy.health, lane: lane + 1 };
}

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

/** Seeds tried, each a fresh deal, before the spec gives up on the card's coming to the human's opening hand. */
const SEED_BUDGET = 25;

/** The localStorage key under which practice keeps a game in progress. */
const RESUME_KEY = "jackioh.practice.game";

/**
 * Start practice on the saved deck, on the first of `SEED`'s deals that puts the custom card in the human's
 * opening hand, and say its seed. The passive human is beaten on turn 9, so a card left to the draws
 * arrives too late; a deal is found, not assumed, so a change to the deck, the shuffle or the AI moves
 * it without breaking the spec (#1803).
 */
function startWithCardInHand(customId: string, attempt = 0): Cypress.Chainable<string> {
  expect(attempt, `the custom card is in the opening hand of one of ${String(SEED_BUDGET)} deals`).to.be.lessThan(SEED_BUDGET);
  const seed = `${SEED}:${String(attempt)}`;
  const params = new URLSearchParams({ seed, seat: "p2", pace: "fast" });
  // R668: a game left in progress would be picked up instead of the setup (practice/resume.ts's key).
  cy.clearLocalStorage(RESUME_KEY);
  cy.visitAs(me(), `${routes.practice()}?${params.toString()}`);
  cy.get(ts(PRACTICE_DECK), { timeout: timeouts.view }).select("saved:1");
  cy.get(ts(PRACTICE_START), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("exist");
  cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
  return practiceHandle().then((handle) => {
    const hand = handle.view?.you.hand;
    const held = Array.isArray(hand) && hand.some((card) => card.defId === customId);
    return held ? cy.wrap(seed, { log: false }) : startWithCardInHand(customId, attempt + 1);
  });
}

describe("40 custom cards — made in the creator, kept on the account, played in practice and replayed", () => {
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
        // Each attribute check on its own: `not.have.attr` hands the next chainer no element.
        cy.get(ts(deckRowId(deckId)), { timeout: timeouts.view }).should("not.have.attr", "data-unsynced");
        cy.get(ts(deckRowId(deckId))).should("have.attr", "data-practice-only", "true");
        cy.get(ts(deckRowId(deckId))).find(ts(DECK_PRACTICE_ONLY)).should("exist");
        cy.savedDecks(me()).should((saved) => {
          expect(saved.decks[0]?.cards, "the saved deck holds the custom card").to.include(customId);
        });
      });

      // R915009: practice with that deck runs in the Luau module; the AI's deck stays a normal deck.
      startWithCardInHand(customId).then((seed) => {
        cy.wrap(seed, { log: false }).as("seed");
      });
      keepWholeHand(BOOT_TIMEOUT);

      untilPlayable(customId, 1).then((play) => {
        cy.playCard(play.card, { zone: { side: "you", row: "units", lane: play.lane as 1 | 2 | 3 | 4 | 5 } });
        // R81: the Cry's declared target. With one enemy unit the board takes it; with more, it asks for
        // the unit the spec measured.
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
      cy.get<string>("@seed").then((seed) => practiceHandle()
        .then((handle) => handle.snapshot())
        .then((debug) => {
          expect(debug.seed).to.eq(seed);
          expect(debug.customCards, "the game carries its custom card").to.have.length(1);
          expect(debug.decks.some((deck) => deck.includes(customId)), "the human's deck holds it").to.eq(true);
          cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
            "replayHash",
            {
              label: "40-custom-cards",
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
        }));
    });
  });
});
