// 38 — the accessibility check: axe-core over every route and both boards (issue #757, WCAG 2.2 AA).
//
// Key assertion (issue #757): "axe reports no serious or critical violation on the routes listed
// above, a spec keeps it that way". Each test opens one route, waits for its screen (and, on a board,
// for the animations to settle), and runs `expectNoBlockingViolations` (support/a11y.ts): WCAG 2.0,
// 2.1 and 2.2 A and AA rules, failing on any serious or critical violation not listed with its reason
// in support/axe-exceptions.ts, and on a listed one that no longer fires. One more test plays a card
// by keyboard alone, since drag to play has no keyboard path and click-click is the one that must.
//
// Daily only: the super run plays every spec file, while a pull request's `e2e smoke` plays only
// SMOKE in scripts/shard-specs.mjs, which does not hold 38. Networked: the signed-in routes need
// `E2E=1 cargo run --release -p jackioh-server`. Every wait is a retried DOM assertion.

import { consentTestid, inviteTestid, landingTestid, loginTestid, resetTestid } from "../../../apps/web/src/auth/testids.ts";
import { patchTestid } from "../../../apps/web/src/patches/testids.ts";
import { practiceTestid } from "../../../apps/web/src/practice/testids.ts";
import { statsTestid } from "../../../apps/web/src/stats/testids.ts";
import { expectNoBlockingViolations } from "../../support/a11y.ts";
import { allCardIds } from "../../support/cards.ts";
import { accounts, constants, routes, seedFor, timeouts } from "../../support/config.ts";
import {
  BOARD,
  GAME,
  PLAY_QUEUE,
  PRACTICE_HUD,
  WORKSHOP,
  cardId,
  handCardId,
  promptOf,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { ActionBody, Lane } from "../../support/types.ts";

/** The practice worker loads the engine, the card scripts and the AI before its first answer. */
const BOOT_TIMEOUT = 60_000;

const SEED = seedFor("38-accessibility");

/** The screens whose testids live in their route's component file, which this project does not type-check. */
const SCREEN = {
  account: "account-screen",
  leaderboard: "leaderboard-screen",
  almanac: "almanac",
  privacy: "privacy-screen",
  terms: "terms-screen",
  accessibility: "accessibility-screen",
  copyright: "copyright-screen",
} as const;

function practiceUrl(seed: string): string {
  const params = new URLSearchParams({ seed, difficulty: "easy", deck: "random", seat: "p1", pace: "fast" });
  return `/practice?${params.toString()}`;
}

/** Open `path` signed out, wait for `root`, and check it. */
function checkPublic(route: string, path: string, root: string): void {
  cy.visit(path);
  cy.get(ts(root), { timeout: timeouts.view }).should("be.visible");
  expectNoBlockingViolations(route);
}

/**
 * The code screen and the consent screen as a pending account meets them, drawn for p1. Spec 10
 * redeems the pending fixture, which stays active until the server restarts (R144), and the daily run
 * can put both specs in one shard; so p1's own read of the account is made to say pending (no match,
 * no series), and `owes` adds what it owes. The screens draw only from that read (CLAUDE.md rule 7).
 */
function asPending(owes: Record<string, boolean> = {}): void {
  cy.intercept("GET", "**/api/auth/me", (req) => {
    req.continue((res) => {
      const body = res.body as Record<string, unknown> & { profile: Record<string, unknown> };
      res.body = {
        ...body,
        profile: { ...body.profile, status: "pending" },
        needsInviteCode: true,
        currentMatchId: null,
        currentSeriesId: null,
        ...owes,
      };
    });
  });
}

/** Open `path` as `account`, wait for `root`, and check it. */
function checkSignedIn(route: string, account: ReturnType<typeof accounts.p1>, path: string, root: string): void {
  cy.visitAs(account, path);
  cy.get(ts(root), { timeout: timeouts.view }).should("be.visible");
  expectNoBlockingViolations(route);
}

describe("38 — axe finds no serious or critical violation on any route (issue #757)", () => {
  describe("public pages", () => {
    it("/", () => {
      cy.visit("/");
      cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
      // The fan's cards are dealt in (R704): checked once every one is drawn, so the check sees them.
      cy.get(`${ts(landingTestid.fan)} .landing-fan-card--face`, { timeout: timeouts.view }).should(($cards) => {
        expect($cards.length, "the fan holds its cards").to.be.greaterThan(0);
        for (const card of $cards.toArray()) expect(getComputedStyle(card).opacity, "a dealt card").to.eq("1");
      });
      expectNoBlockingViolations("/");
    });

    it("/login, signing in", () => {
      checkPublic("/login", routes.login(), loginTestid.form);
    });

    it("/login, creating an account (the age screen and the terms box)", () => {
      cy.visit(routes.login());
      cy.get(ts(loginTestid.mode), { timeout: timeouts.view }).click();
      cy.get(ts(loginTestid.form)).should("have.attr", "data-mode", "signUp");
      cy.get(ts(consentTestid.birthYear)).should("be.visible");
      expectNoBlockingViolations("/login sign-up");
    });

    it("/reset-password", () => {
      checkPublic("/reset-password", "/reset-password", resetTestid.screen);
    });

    it("/almanac", () => {
      checkPublic("/almanac", "/almanac", SCREEN.almanac);
    });

    it("/patch-notes", () => {
      checkPublic("/patch-notes", "/patch-notes", patchTestid.screen);
    });

    it("/stats", () => {
      checkPublic("/stats", "/stats", statsTestid.screen);
    });

    it("/privacy", () => {
      checkPublic("/privacy", "/privacy", SCREEN.privacy);
    });

    it("/terms", () => {
      checkPublic("/terms", "/terms", SCREEN.terms);
    });

    it("/accessibility", () => {
      checkPublic("/accessibility", "/accessibility", SCREEN.accessibility);
    });

    it("/copyright", () => {
      checkPublic("/copyright", "/copyright", SCREEN.copyright);
    });
  });

  describe("practice and hotseat, no account", () => {
    it("/practice setup", () => {
      checkPublic("/practice", "/practice", practiceTestid.setup);
    });

    it("/practice board", () => {
      cy.visit(practiceUrl(SEED));
      cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("be.visible");
      cy.get(ts(BOARD), { timeout: BOOT_TIMEOUT }).should("be.visible");
      // The opening mulligan (R265), whose card options are a prompt's.
      cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
      cy.settled();
      expectNoBlockingViolations("/practice board");
    });

    it("hotseat board", () => {
      cy.seedGame({ seed: SEED, a: "01-aggro-a", b: "01-aggro-b" });
      cy.get(ts(GAME), { timeout: timeouts.view }).should("be.visible");
      cy.settled();
      expectNoBlockingViolations("hotseat board");
    });
  });

  describe("signed in", () => {
    beforeEach(() => {
      cy.freeAccount(accounts.p1());
    });

    it("/play", () => {
      checkSignedIn("/play", accounts.p1(), routes.play(), PLAY_QUEUE);
    });

    it("/decks", () => {
      // One full deck, whatever specs ran before this one saved: the list's rows and the open deck's
      // card strips are what this page checks, and the exception for the strips must fire.
      cy.clearDecks(accounts.p1());
      cy.saveDeck(accounts.p1(), { name: "Axe", cards: allCardIds().slice(0, constants.DECK_SIZE) });
      checkSignedIn("/decks", accounts.p1(), routes.deckbuilder(), WORKSHOP);
    });

    it("/account", () => {
      checkSignedIn("/account", accounts.p1(), "/account", SCREEN.account);
    });

    it("/leaderboard", () => {
      checkSignedIn("/leaderboard", accounts.p1(), "/leaderboard", SCREEN.leaderboard);
    });

    it("/invite, the code screen", () => {
      asPending();
      checkSignedIn("/invite", accounts.p1(), routes.invite(), inviteTestid.input);
    });

    it("/invite, the consent screen (R757000, R757001)", () => {
      asPending({ ageCheckOwed: true, termsOwed: true });
      checkSignedIn("/invite consent", accounts.p1(), routes.invite(), consentTestid.prompt);
      cy.get(ts(consentTestid.birthMonth)).should("be.visible");
    });
  });

  describe("keyboard alone", () => {
    it("Enter on a legal hand card and then on a legal zone plays the card there", () => {
      cy.seedGame({ seed: SEED, a: "01-aggro-a", b: "01-aggro-b" });
      // The device follows the seat that answered the last mulligan: hand it to the seat to act.
      cy.gameState().then((state) => {
        cy.jackioh().then((handle) => {
          if (handle.seat !== state.active) cy.handOver();
        });
      });
      cy.settled();
      cy.jackioh().then((handle) => {
        const legal = handle.legal?.() ?? [];
        const play = legal.find(
          (body): body is Extract<ActionBody, { type: "play" }> =>
            body.type === "play" &&
            body.zone?.row === "units" &&
            body.targets === undefined &&
            body.tributes === undefined &&
            body.modes === undefined,
        );
        expect(play, "the opening hand holds a unit with a legal lane").to.not.eq(undefined);
        if (play === undefined || play.zone === undefined) return;
        const lane = play.zone.lane as Lane;
        cy.get(ts(handCardId(play.instanceId))).focus().type("{enter}");
        cy.get(ts(zoneId("you", "units", lane))).focus().type("{enter}");
        cy.settled();
        cy.get(ts(zoneId("you", "units", lane)), { timeout: timeouts.view }).find(ts(cardId(play.instanceId))).should("exist");
        cy.get(ts(handCardId(play.instanceId))).should("not.exist");
        // The board with a unit in a lane, whose zone is then a group around a card that takes focus.
        expectNoBlockingViolations("hotseat board, a unit in play");
      });
    });
  });
});
