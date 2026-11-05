// 49 — the dev dashboard (issue #3074; SPEC §14.10; R3074000, R3074001, R3074002): a dev account signs in to
// the dashboard (apps/admin) on a seeded server, adds Jlore, sets its rating and reads the whole ladder, each row
// marked person or bot; an account that is no dev account meets the dashboard's refusal and nothing else.
// The fixture account `e2e-econ` is the dev account in end-to-end mode (R1310004); after the reseed it waits at the
// invite gate, so the spec redeems its code, then plays one ranked game against `e2e-p2` over HTTP and sockets, so the
// ladder holds rows. The E2E server is not strict (`DEV_STRICT` off), so no two-step code is asked for.
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` and the dashboard's own client,
// `pnpm --dir apps/admin build:e2e && pnpm --dir apps/admin exec vite preview` (http://localhost:5174, which
// proxies `/api` to the server). Daily only, as every spec outside scripts/shard-specs.mjs's SMOKE.

import {
  MATCHMAKER_SWEEP_INTERVAL_SECONDS,
  RATING_WINDOW_FULL_AFTER_SECONDS,
  SERIES_POLL_SECONDS,
  STARTER_CARDS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import {
  E2E_RESEED_PATH,
  SESSION_STORAGE_KEY,
  accounts,
  adminClient,
  apiUrl,
  bearerHeaders,
  constants,
  inviteCodes,
  seedFor,
  server,
  timeouts,
  type E2EAccount,
} from "../../support/config.ts";
import type { AwaitServerResult } from "../../support/tasks/awaitServer.ts";
import { testids as admin } from "../../../apps/admin/src/testids.ts";

const MS_PER_SECOND = 1000;

/** Pairing waits out the rating window's widening (R640001), a sweep and a poll, as in specs 37 and 45. */
const PAIRING_TIMEOUT_MS =
  (RATING_WINDOW_FULL_AFTER_SECONDS + MATCHMAKER_SWEEP_INTERVAL_SECONDS + SERIES_POLL_SECONDS) * MS_PER_SECOND +
  timeouts.view;

/** How often the Node task asks the server again while it waits. */
const POLL_EVERY_MS = 500;

/** installLoadout's default deck index, the deck seat two queues. */
const FIXTURE_DECK = 0;

const SEED = seedFor("49-dev-dashboard");

/** What the spec gives and sets, its own numbers. */
const JLORE_GIVEN = 250;
const RATING_SET = 1800;

const econ = (): E2EAccount => accounts.econ();

type WalletBody = { jlore: number };
type RatingBody = {
  ladders: Array<{ ladder: string; rating: number; handSet: boolean; earned: { rating: number } | null }>;
};

function ts(id: string): string {
  return `[data-testid="${id}"]`;
}

function read<T extends object>(account: E2EAccount, path: string): Cypress.Chainable<T> {
  return cy
    .request<T>({ method: "GET", url: apiUrl(path), headers: bearerHeaders(account.token) })
    .then((response) => {
      expect(response.status, `GET ${path}`).to.eq(200);
    })
    .its("body");
}

/** The server's answer at `path`, once its `key` is set (support/tasks/awaitServer.ts). */
function awaitServer(account: E2EAccount, path: string, key: string, timeoutMs: number): Cypress.Chainable<unknown> {
  return cy
    .task<AwaitServerResult>(
      "awaitServer",
      { url: server.http(), path, token: account.token, key, timeoutMs, everyMs: POLL_EVERY_MS },
      { timeout: timeoutMs + timeouts.task },
    )
    .then((result) => {
      expect(result.timedOut, `${path} sets ${key} within ${String(timeoutMs)} ms: ${JSON.stringify(result.body)}`).to.eq(
        false,
      );
      return result.body;
    });
}

/** The dashboard, signed in as `account` through the E2E session key (apps/admin's non-production builds read it). */
function openDashboard(account: E2EAccount): void {
  cy.visit(adminClient.url(), {
    onBeforeLoad(win) {
      win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: account.token }));
    },
  });
}

/** R1310004: `e2e-econ` past the invite gate. */
function activateEcon(): void {
  cy.request({
    method: "POST",
    url: apiUrl("/api/codes/redeem"),
    headers: bearerHeaders(econ().token),
    body: { code: inviteCodes.good() },
  }).then((response) => {
    expect(response.status, "§9.4: the good code activates the dev account").to.eq(200);
  });
}

/** One ranked Best of 1 game, `e2e-econ` against `e2e-p2`, which `e2e-p2` concedes: both hold a season row after it. */
function playOneRankedGame(): void {
  let ourDeck = "";
  let theirDeck = "";
  cy.saveDeck(econ(), { name: "Starter deck", cards: STARTER_CARDS.slice(0, constants.DECK_SIZE) }).then((id) => {
    ourDeck = id;
  });
  cy.installLoadout(accounts.p2(), "19-modes-b").then((installed) => {
    theirDeck = installed.deckIds[FIXTURE_DECK] ?? "";
  });
  cy.then(() => {
    for (const [account, deckId] of [
      [econ(), ourDeck],
      [accounts.p2(), theirDeck],
    ] as const) {
      cy.request({
        method: "POST",
        url: apiUrl("/api/queue"),
        headers: bearerHeaders(account.token),
        body: { mode: "bo1", deckId, seed: SEED },
      }).then((response) => {
        expect(response.status, `R143: ${account.email} queues for the ladder with an end-to-end seed`).to.eq(200);
      });
      // R897000: a seat queued over HTTP polls like a queued client.
      cy.stayQueued(account, SERIES_POLL_SECONDS * MS_PER_SECOND);
    }
  });
  awaitServer(econ(), "/api/auth/me", "currentMatchId", PAIRING_TIMEOUT_MS).then((me) => {
    const matchId = (me as { currentMatchId: string }).currentMatchId;
    cy.concedeAs(accounts.p2(), matchId);
  });
  // The result's transaction rates the game (api/results.rs): the dev account's season row has its game.
  awaitServer(econ(), "/api/ranked", "record.games", timeouts.game);
}

describe("49 — the dev dashboard (R3074000)", () => {
  beforeEach(() => {
    cy.request({ method: "POST", url: apiUrl(E2E_RESEED_PATH) }).its("status").should("eq", 200);
  });

  afterEach(() => {
    cy.freeAccount(econ());
    cy.freeAccount(accounts.p2());
  });

  it("R3074000 R3074001 R3074002 a dev account adds Jlore, sets its rating and reads the whole ladder", () => {
    activateEcon();
    playOneRankedGame();
    let jloreBefore = 0;
    read<WalletBody>(econ(), "/api/wallet").then((wallet) => {
      jloreBefore = wallet.jlore;
    });

    // Signed in: who it is, on a server that is not strict.
    openDashboard(econ());
    cy.get(ts(admin.account), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(admin.accountStrict)).invoke("text").should("match", /^No/u);

    // 1. Jlore, through the dev tool the dashboard offers.
    cy.get(ts(admin.jloreInput)).clear();
    cy.get(ts(admin.jloreInput)).type(String(JLORE_GIVEN));
    cy.get(ts(admin.jloreGive)).click();
    cy.get(ts(admin.status), { timeout: timeouts.view }).should("have.attr", "data-state", "done");
    cy.then(() => {
      read<WalletBody>(econ(), "/api/wallet").then((wallet) => {
        expect(wallet.jlore, "§14.10: the dev account's own Jlore, given").to.eq(jloreBefore + JLORE_GIVEN);
      });
    });

    // 2. R3074001: its rating on the Constructed ladder, set by hand, the earned one kept.
    cy.get(ts(admin.ratingInput("constructed"))).clear();
    cy.get(ts(admin.ratingInput("constructed"))).type(String(RATING_SET));
    cy.get(ts(admin.ratingSet("constructed"))).click();
    cy.get(ts(admin.ratingValue("constructed")), { timeout: timeouts.view }).should("contain.text", String(RATING_SET));
    cy.get(ts(admin.ratingReset("constructed"))).should("be.visible");
    read<RatingBody>(econ(), "/api/admin/rating").then((body) => {
      const constructed = body.ladders.find((row) => row.ladder === "constructed");
      expect(constructed?.rating, "R3074001: the rating set").to.eq(RATING_SET);
      expect(constructed?.handSet, "R3074001: hand-set").to.eq(true);
      expect(constructed?.earned, "R3074001: the earned rating kept").to.not.eq(null);
    });

    // 3. R3074002: the whole ladder, each row marked person or bot: both players of the game, the dev account's
    // own row hand-set, read afresh (the session key the visit wrote stays for the reload).
    cy.reload();
    cy.get(ts(admin.leaderboard), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      read<{ profile: { id: string } }>(econ(), "/api/auth/me").then((me) => {
        cy.get(`${ts(admin.leaderboardRow)}[data-profile-id="${me.profile.id}"]`, { timeout: timeouts.view })
          .should("have.attr", "data-kind", "person")
          .and("have.attr", "data-hand-set", "true")
          .and("contain.text", String(RATING_SET));
      });
      read<{ profile: { id: string } }>(accounts.p2(), "/api/auth/me").then((me) => {
        cy.get(`${ts(admin.leaderboardRow)}[data-profile-id="${me.profile.id}"]`).should("have.attr", "data-kind", "person");
      });
    });
    cy.get(ts(admin.leaderboardRow)).should("have.length", 2);

    // R3074000: both writes left their audit rows.
    read<{ actions: Array<{ route: string }> }>(econ(), "/api/admin/audit").then((audit) => {
      const routes = audit.actions.map((action) => action.route);
      expect(routes, "R3074000: an audit row per write").to.include.members(["POST /api/dev/jlore", "PUT /api/admin/rating"]);
    });
  });

  it("R3074000 an account that is no dev account meets the refusal and nothing else", () => {
    openDashboard(accounts.p1());
    cy.get(ts(admin.forbidden), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(admin.account)).should("not.exist");
    cy.get(ts(admin.leaderboard)).should("not.exist");
  });
});
