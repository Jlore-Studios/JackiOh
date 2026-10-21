// 38 — the accessibility check: axe-core over every route and both boards (issue #757, WCAG 2.2 AA).
//
// Key assertion (issue #757): "axe reports no serious or critical violation on the routes listed
// above, a spec keeps it that way". Each test opens one route, waits for its screen (and, on a board,
// for the animations to settle), and runs `expectNoBlockingViolations` (support/a11y.ts): WCAG 2.0,
// 2.1 and 2.2 A and AA rules, failing on any serious or critical violation not listed with its reason
// in support/axe-exceptions.ts, and on a listed one that no longer fires. One more test plays a card
// by keyboard alone, since drag to play has no keyboard path and click-click is the one that must.
//
// #1561 added the routes that came after #757 (`/creator`, `/replays`, `/together`, `/collection`)
// and runs every route again at BUILD M5-T1's phone size, 390x844, whose labels end in " (phone)" so
// an exception in axe-exceptions.ts holds at one width only.
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
  CARD_POOL,
  COLLECTION_SCREEN,
  CREATOR_SCREEN,
  GAME,
  PLAY_QUEUE,
  PRACTICE_HUD,
  REPLAYS_ROOT,
  TOGETHER_ROOT,
  WORKSHOP,
  cardId,
  deckRowId,
  handCardId,
  promptOf,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { ActionBody, Lane } from "../../support/types.ts";

/** The practice worker loads the engine, the card scripts and the AI before its first answer. */
const BOOT_TIMEOUT = 60_000;

const SEED = seedFor("38-accessibility");

/**
 * BUILD M5-T1's two sizes: the Cypress config's 1280x720, whose labels are the routes as #757 named
 * them, and the phone's 390x844, whose labels say so.
 */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, suffix: "" },
  { label: "phone", width: 390, height: 844, suffix: " (phone)" },
] as const;

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
  for (const viewport of VIEWPORTS) {
    /** The label `route` has in axe-exceptions.ts at this width. */
    const at = (route: string): string => route + viewport.suffix;

    describe(`${viewport.label}, ${String(viewport.width)}x${String(viewport.height)}`, () => {
      beforeEach(() => {
        cy.viewport(viewport.width, viewport.height);
      });

      describe("public pages", () => {
        it("/", () => {
          cy.visit("/");
          cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
          // The fan's cards are dealt in (R704): checked once every one is drawn, so the check sees them.
          cy.get(`${ts(landingTestid.fan)} .landing-fan-card--face`, { timeout: timeouts.view }).should(($cards) => {
            expect($cards.length, "the fan holds its cards").to.be.greaterThan(0);
            for (const card of $cards.toArray()) expect(getComputedStyle(card).opacity, "a dealt card").to.eq("1");
          });
          expectNoBlockingViolations(at("/"));
        });

        it("/login, signing in", () => {
          checkPublic(at("/login"), routes.login(), loginTestid.form);
        });

        it("/login, creating an account (the age screen and the terms box)", () => {
          cy.visit(routes.login());
          cy.get(ts(loginTestid.mode), { timeout: timeouts.view }).click();
          cy.get(ts(loginTestid.form)).should("have.attr", "data-mode", "signUp");
          cy.get(ts(consentTestid.birthYear)).should("be.visible");
          expectNoBlockingViolations(at("/login sign-up"));
        });

        it("/reset-password", () => {
          checkPublic(at("/reset-password"), "/reset-password", resetTestid.screen);
        });

        it("/almanac", () => {
          checkPublic(at("/almanac"), "/almanac", SCREEN.almanac);
        });

        it("/patch-notes", () => {
          checkPublic(at("/patch-notes"), "/patch-notes", patchTestid.screen);
        });

        it("/stats", () => {
          checkPublic(at("/stats"), "/stats", statsTestid.screen);
        });

        it("/privacy", () => {
          checkPublic(at("/privacy"), "/privacy", SCREEN.privacy);
        });

        it("/terms", () => {
          checkPublic(at("/terms"), "/terms", SCREEN.terms);
        });

        it("/accessibility", () => {
          checkPublic(at("/accessibility"), "/accessibility", SCREEN.accessibility);
        });

        it("/copyright", () => {
          checkPublic(at("/copyright"), "/copyright", SCREEN.copyright);
        });
      });

      describe("practice and hotseat, no account", () => {
        it("/practice setup", () => {
          checkPublic(at("/practice"), "/practice", practiceTestid.setup);
        });

        it("/practice board", () => {
          cy.visit(practiceUrl(SEED));
          cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("be.visible");
          cy.get(ts(BOARD), { timeout: BOOT_TIMEOUT }).should("be.visible");
          // The opening mulligan (R265), whose card options are a prompt's.
          cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
          cy.settled();
          expectNoBlockingViolations(at("/practice board"));
        });

        it("hotseat board", () => {
          cy.seedGame({ seed: SEED, a: "01-aggro-a", b: "01-aggro-b" });
          cy.get(ts(GAME), { timeout: timeouts.view }).should("be.visible");
          cy.settled();
          expectNoBlockingViolations(at("hotseat board"));
        });
      });

      describe("signed in", () => {
        beforeEach(() => {
          cy.freeAccount(accounts.p1());
        });

        it("/play", () => {
          checkSignedIn(at("/play"), accounts.p1(), routes.play(), PLAY_QUEUE);
        });

        it("/decks", () => {
          // One full deck, whatever specs ran before this one saved: the list's rows and the open deck's
          // card strips are what this page checks, and the exception for the strips must fire.
          cy.clearDecks(accounts.p1());
          cy.saveDeck(accounts.p1(), { name: "Axe", cards: allCardIds().slice(0, constants.DECK_SIZE) }).then((deckId) => {
            cy.visitAs(accounts.p1(), routes.deckbuilder());
            cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("be.visible");
            // A phone shows the list alone until a deck is opened (a desktop opens it already), so
            // its row is pressed, and the pool is checked at both widths.
            cy.get(ts(deckRowId(deckId))).click();
            cy.get(ts(CARD_POOL), { timeout: timeouts.view }).should("be.visible");
            expectNoBlockingViolations(at("/decks"));
          });
        });

        it("/account", () => {
          checkSignedIn(at("/account"), accounts.p1(), "/account", SCREEN.account);
        });

        it("/leaderboard", () => {
          checkSignedIn(at("/leaderboard"), accounts.p1(), "/leaderboard", SCREEN.leaderboard);
        });

        it("/creator", () => {
          checkSignedIn(at("/creator"), accounts.p1(), routes.creator(), CREATOR_SCREEN);
        });

        it("/replays", () => {
          checkSignedIn(at("/replays"), accounts.p1(), routes.replays(), REPLAYS_ROOT);
        });

        it("/together", () => {
          checkSignedIn(at("/together"), accounts.p1(), "/together", TOGETHER_ROOT);
        });

        it("/collection", () => {
          checkSignedIn(at("/collection"), accounts.p1(), routes.collection(), COLLECTION_SCREEN);
        });

        it("/invite, the code screen", () => {
          asPending();
          checkSignedIn(at("/invite"), accounts.p1(), routes.invite(), inviteTestid.input);
        });

        it("/invite, the consent screen (R757000, R757001)", () => {
          asPending({ ageCheckOwed: true, termsOwed: true });
          checkSignedIn(at("/invite consent"), accounts.p1(), routes.invite(), consentTestid.prompt);
          cy.get(ts(consentTestid.birthMonth)).should("be.visible");
        });
      });
    });
  }

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
