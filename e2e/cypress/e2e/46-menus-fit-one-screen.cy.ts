// 46 — every menu fits one screen (issue #1581, part of #1576, R1581000).
//
// Key assertion (issue #1581): "a check that fails when a menu scrolls ... visits each menu route at
// a desktop size and a phone size and asserts the document does not scroll vertically". Each test
// opens one menu at BUILD M5-T1's two sizes (`viewports`, support/config.ts: 1280x720 and 390x844),
// waits for the ids that say it has drawn, and asserts the document's scrollHeight is no more than
// the window's innerHeight (and its scrollWidth no more than its innerWidth). Settled layout is read
// by a retried assertion, so a late-arriving list is measured too.
//
// The exceptions, as #1576 states them and R1581000 rules them:
//   - The tavern home (`/`, with the settings open on it) is its main screen and exactly one screen
//     below the floor: `LANDING_SCREENS` windows, not one.
//   - A big list of cards (the almanac's and the deck builder's pool, Collection's Cards, the
//     statistics' table) and every menu's own region (the account's, the practice setup's, the patch
//     notes' ...) scroll inside the menu, never the page. This check reads the page, so it never
//     sees them, and so nothing is exempted for them here.
// Not asked: the boards, and the static legal pages (spec 38 holds those to axe).
//
// Per pull request: SMOKE in scripts/shard-specs.mjs holds 46. Networked: the signed-in menus need
// `E2E=1 cargo run --release -p jackioh-server`; the series screen is answered by an intercept.
// Every wait is a retried DOM assertion.

import { consentTestid, inviteTestid, landingTestid, loginTestid, resetTestid, shellTestid } from "../../../apps/web/src/auth/testids.ts";
import { patchTestid } from "../../../apps/web/src/patches/testids.ts";
import { practiceTestid } from "../../../apps/web/src/practice/testids.ts";
import { statsTestid } from "../../../apps/web/src/stats/testids.ts";
import { SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED } from "../../../apps/web/src/wire/serverConfig.ts";
import { allCardIds, cardId } from "../../support/cards.ts";
import { accounts, constants, routes, timeouts, viewports, type MenuViewport } from "../../support/config.ts";
import {
  CARD_POOL,
  COLLECTION_JLORE,
  COLLECTION_SCREEN,
  CREATOR_SCREEN,
  PLAY_QUEUE,
  PLAY_SIGN_IN,
  REPLAYS_ROOT,
  SERIES_SCORE,
  SHOP_PANEL,
  TOGETHER_ROOT,
  WORKSHOP,
  deckRowId,
  ts,
} from "../../support/testids.ts";
import { SETTINGS_OPEN_NAV, SETTINGS_PANEL } from "../../support/ux.ts";

/**
 * #1576's tavern home: its main screen, and exactly one screen below the floor. The only menu the
 * page may be taller than one window.
 */
const LANDING_SCREENS = 2;

/** Every other menu: the page is the window's height and no more. */
const ONE_SCREEN = 1;

/** A box's edge may sit this far past the window's (a sub-pixel border) and still be inside it. */
const EDGE_SLACK_PX = 1;

const MS_PER_SECOND = 1000;

/** The series the intercept answers: a v4 UUID's shape, nothing the server holds. */
const SERIES_ID = "00000000-0000-4000-8000-000000001581";

/** The card pool of a big list takes a while to draw its first cards on a cold browser. */
const CARD_POOL_TIMEOUT = 30_000;

/** The screens whose testids live in their route's component file, which this project does not type-check. */
const SCREEN = {
  accountRank: "account-rank",
  leaderboardSeason: "leaderboard-season",
  almanac: "almanac",
} as const;

/** R1581000: the page is no taller than `screens` windows and no wider than one. */
function expectOnScreen(viewport: MenuViewport, screens: number): void {
  cy.window().should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.innerWidth, "the viewport asked for").to.eq(viewport.width);
    expect(win.innerHeight, "the viewport asked for").to.eq(viewport.height);
    const page = win.document.documentElement;
    expect(page.scrollHeight, `the page's height at ${where}`).to.be.at.most(win.innerHeight * screens);
    expect(page.scrollWidth, `the page's width at ${where}`).to.be.at.most(win.innerWidth);
  });
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

/**
 * A series between its games, choosing a deck (the shape of `SeriesView`, apps/web/src/net/api.ts):
 * three decks to pick from and the clock running, which is the screen at its tallest. `now` is the
 * server's own clock (R333), here this browser's.
 */
function seriesPicking(now: number): Record<string, unknown> {
  return {
    id: SERIES_ID,
    status: "picking",
    gameNo: 1,
    winsNeeded: SERIES_WINS_NEEDED,
    maxGames: SERIES_MAX_GAMES,
    pickDeadline: now + SERIES_PICK_SECONDS * MS_PER_SECOND,
    now,
    currentMatchId: null,
    ranked: true,
    you: {
      seat: "p1",
      wins: 0,
      trioName: "Main trio",
      decks: ["Aggro", "Control", "Ramp"].map((name, slot) => ({
        slot,
        name,
        cards: [cardId(slot + 1)],
        won: false,
        games: 0,
      })),
      pick: null,
      autoPick: false,
    },
    opponent: {
      wins: 0,
      decks: [0, 1, 2].map((slot) => ({ slot, won: false })),
      picked: false,
    },
    games: [],
    result: null,
  };
}

describe("46 — every menu fits one screen (issue #1581)", () => {
  for (const viewport of [viewports.desktop, viewports.phone]) {
    describe(`${viewport.label}, ${String(viewport.width)}x${String(viewport.height)}`, () => {
      beforeEach(() => {
        cy.viewport(viewport.width, viewport.height);
      });

      describe("signed out", () => {
        beforeEach(() => {
          cy.signOut();
        });

        it("R1581000 /", () => {
          cy.visit("/");
          cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
          // The fan's cards are dealt in (R704): measured once every one is drawn.
          cy.get(`${ts(landingTestid.fan)} .landing-fan-card--face`, { timeout: timeouts.view }).should(($cards) => {
            expect($cards.length, "the fan holds its cards").to.be.greaterThan(0);
            for (const card of $cards.toArray()) expect(getComputedStyle(card).opacity, "a dealt card").to.eq("1");
          });
          expectOnScreen(viewport, LANDING_SCREENS);
        });

        it("R1581000 /, the settings open", () => {
          cy.visit("/");
          cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
          cy.get(ts(SETTINGS_OPEN_NAV)).click();
          cy.get(ts(SETTINGS_PANEL), { timeout: timeouts.view }).should("be.visible");
          // The dialog is inside the window, whatever its tab holds: a tab's body scrolls inside it.
          cy.get(ts(SETTINGS_PANEL)).should(($panel) => {
            const box = $panel[0]?.getBoundingClientRect();
            expect(box?.top, "the settings' top edge").to.be.at.least(0);
            expect(box?.bottom, "the settings' bottom edge").to.be.at.most(viewport.height + EDGE_SLACK_PX);
          });
          expectOnScreen(viewport, LANDING_SCREENS);
        });

        it("R1581000 /login", () => {
          cy.visit(routes.login());
          cy.get(ts(loginTestid.form), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /login, creating an account", () => {
          cy.visit(routes.login());
          cy.get(ts(loginTestid.mode), { timeout: timeouts.view }).click();
          cy.get(ts(loginTestid.form)).should("have.attr", "data-mode", "signUp");
          cy.get(ts(consentTestid.birthYear)).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /reset-password", () => {
          cy.visit("/reset-password");
          cy.get(ts(resetTestid.screen), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /almanac", () => {
          cy.visit("/almanac");
          cy.get(ts(SCREEN.almanac), { timeout: timeouts.view }).should("be.visible");
          cy.get(ts(CARD_POOL), { timeout: CARD_POOL_TIMEOUT }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /patch-notes", () => {
          cy.visit("/patch-notes");
          cy.get(ts(patchTestid.screen), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /stats", () => {
          cy.visit("/stats");
          cy.get(ts(statsTestid.summaryTiles), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /practice", () => {
          cy.visit(routes.practice());
          cy.get(ts(practiceTestid.setup), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /play, signed out", () => {
          cy.visit(routes.play());
          cy.get(ts(PLAY_SIGN_IN), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /replays, signed out", () => {
          cy.visit(routes.replays());
          cy.get(ts(REPLAYS_ROOT), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 a path nothing is at", () => {
          cy.visit("/no-such-menu");
          cy.get(ts(shellTestid.notFound), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });
      });

      describe("signed in", () => {
        before(() => {
          cy.freeAccount(accounts.p1());
        });

        it("R1581000 /play", () => {
          cy.visitAs(accounts.p1(), routes.play());
          cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /decks", () => {
          cy.visitAs(accounts.p1(), routes.deckbuilder());
          cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /decks, a deck open", () => {
          // One full deck, whatever specs ran before this one saved: the open deck's pool is the big
          // list this menu holds.
          cy.clearDecks(accounts.p1());
          cy.saveDeck(accounts.p1(), { name: "Fits", cards: allCardIds().slice(0, constants.DECK_SIZE) }).then((deckId) => {
            cy.visitAs(accounts.p1(), routes.deckbuilder());
            cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("be.visible");
            // A phone shows the list alone until a deck is opened (a desktop opens it already).
            cy.get(ts(deckRowId(deckId))).click();
            cy.get(ts(CARD_POOL), { timeout: CARD_POOL_TIMEOUT }).should("be.visible");
            expectOnScreen(viewport, ONE_SCREEN);
          });
        });

        it("R1581000 /account", () => {
          cy.visitAs(accounts.p1(), "/account");
          cy.get(ts(SCREEN.accountRank), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /leaderboard", () => {
          cy.visitAs(accounts.p1(), "/leaderboard");
          cy.get(ts(SCREEN.leaderboardSeason), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /creator", () => {
          cy.visitAs(accounts.p1(), routes.creator());
          cy.get(ts(CREATOR_SCREEN), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /replays", () => {
          cy.visitAs(accounts.p1(), routes.replays());
          cy.get(ts(REPLAYS_ROOT), { timeout: timeouts.view }).find('[role="status"]').should("not.exist");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /together", () => {
          cy.visitAs(accounts.p1(), "/together");
          cy.get(ts(TOGETHER_ROOT), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /collection", () => {
          cy.visitAs(accounts.p1(), routes.collection());
          cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /collection, Cards", () => {
          cy.visitAs(accounts.p1(), routes.collectionCards());
          cy.get(ts(COLLECTION_JLORE), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /collection, Cosmetics", () => {
          cy.visitAs(accounts.p1(), `${routes.collection()}?section=cosmetics`);
          cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /shop", () => {
          cy.visitAs(accounts.p1(), routes.shop());
          cy.get(ts(SHOP_PANEL), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /series, choosing a deck", () => {
          cy.intercept("GET", `**/api/series/${SERIES_ID}`, { statusCode: 200, body: seriesPicking(Date.now()) });
          cy.visitAs(accounts.p1(), routes.series(SERIES_ID));
          cy.get(ts(SERIES_SCORE), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /invite, the code screen", () => {
          asPending();
          cy.visitAs(accounts.p1(), routes.invite());
          cy.get(ts(inviteTestid.input), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });

        it("R1581000 /invite, the consent screen", () => {
          asPending({ ageCheckOwed: true, termsOwed: true });
          cy.visitAs(accounts.p1(), routes.invite());
          cy.get(ts(consentTestid.birthMonth), { timeout: timeouts.view }).should("be.visible");
          expectOnScreen(viewport, ONE_SCREEN);
        });
      });
    });
  }
});
