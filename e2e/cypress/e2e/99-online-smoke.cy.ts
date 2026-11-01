/**
 * The deployed stack, in a real Chromium, with two real accounts. Every other spec drives a LOCAL
 * client against fixtures (ASSUMPTION A6), so none exercises what breaks in a deployment: Vercel
 * serving the bundle, Supabase issuing a token to a real password, Render verifying it, Postgres
 * behind Render. SKIPPED BY DEFAULT: behind `--env online=true`, never part of the M8 gate.
 *
 * Run it with:
 *   E2E_BASE_URL=https://jackioh.com pnpm exec cypress run --browser chrome \
 *     --spec cypress/e2e/99-online-smoke.cy.ts --env online=true,...
 */

/** Cypress 16: `Cypress.env()` is gone; spec-visible values come from `expose`. */
function exposed(key: string, fallback = ""): string {
  const value: unknown = Cypress.expose(key);
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

const ONLINE = exposed("online") === "true";
const SUPABASE_URL = exposed("supabaseUrl");
const SUPABASE_KEY = exposed("supabaseKey");
const SERVER = exposed("serverUrl");
const PASSWORD = exposed("testPassword");
const P1 = exposed("player1", "player1@example.com");
const P2 = exposed("player2", "player2@example.com");

/** A token straight from the auth provider, the same grant the login form uses (SPEC §9.2). */
function tokenFor(email: string): Cypress.Chainable<string> {
  return cy
    .request({
      method: "POST",
      url: `${SUPABASE_URL}/auth/v1/token?grant_type=password`,
      headers: { apikey: SUPABASE_KEY, Authorization: `Bearer ${SUPABASE_KEY}` },
      body: { email, password: PASSWORD },
    })
    .then((res) => {
      expect(res.status, `${email} signs in`).to.eq(200);
      return String(res.body.access_token);
    });
}

/**
 * The account's second saved deck, oldest first in `GET /api/decks` order (R257's `{ deckIndex: 1 }`).
 * The server takes a Best-of-1 choice only as `{ mode: "bo1", deckId }` (docs/v0.3.0/SURFACE.md §11.3).
 */
function secondDeckId(token: string): Cypress.Chainable<string> {
  return cy
    .request({ url: `${SERVER}/api/decks`, headers: { Authorization: `Bearer ${token}` } })
    .then((res) => {
      expect(res.status, "the saved decks").to.eq(200);
      const decks = (res.body.decks ?? []) as readonly { id?: unknown }[];
      const id = decks[1]?.id;
      expect(id, "a second saved deck").to.be.a("string");
      return String(id);
    });
}

/**
 * Clears queue AND match state, so the spec is re-runnable: a live match left behind makes every later
 * `POST /api/rooms` a 409, and no HTTP route leaves a match (only `concede`, R79's clocks or the reaper),
 * so it goes through the database (`support/tasks/onlineReset.ts`), inert unless `E2E_DATABASE_URL` is set.
 */
function resetOnlineState(): void {
  cy.task("onlineReset").then((result) => {
    const r = result as { ok: boolean; skipped?: string; error?: string; cleared?: unknown };
    if (!r.ok) {
      throw new Error(
        `onlineReset did not run: ${r.skipped ?? r.error ?? "unknown"}. ` +
          "Set E2E_DATABASE_URL so the spec can clear match state between runs.",
      );
    }
    cy.task("log", `onlineReset cleared ${JSON.stringify(r.cleared)}`);
  });
}

/** Signs in through the real form, from a clean session, and waits for the gate to settle. */
function signInThroughForm(email: string): void {
  cy.visit("/login", {
    onBeforeLoad(win) {
      // A different account in the same browser: drop whatever token is there first.
      win.sessionStorage.removeItem("jackioh.session");
      win.localStorage.removeItem("jackioh.session");
      win.localStorage.removeItem("jackioh.e2e.session");
    },
  });
  cy.get('[data-testid="login-email"]').clear().type(email);
  cy.get('[data-testid="login-password"]').clear().type(PASSWORD, { log: false });
  cy.get('[data-testid="login-submit"]').click();
  // Issue #479: a sign-in lands on the main menu, which says so once the server has answered.
  cy.location("pathname", { timeout: 40_000 }).should("eq", "/");
  cy.get('[data-testid="landing"]', { timeout: 40_000 }).should("have.attr", "data-account", "signed-in");
}

(ONLINE ? describe : describe.skip)("the deployed stack, two real accounts", () => {
  before(() => {
    expect(SUPABASE_URL, "supabaseUrl env").to.not.eq("");
    expect(SERVER, "serverUrl env").to.not.eq("");
    expect(PASSWORD, "testPassword env").to.not.eq("");
    // R1435: an account `seed-accounts` made before migration 0028 owes the username prompt, which
    // would stand in front of every gated screen below. "Skip for now" answers it, and answering it
    // again changes nothing.
    for (const email of [P1, P2]) {
      tokenFor(email).then((token) =>
        cy
          .request({
            method: "POST",
            url: `${SERVER}/api/username/skip`,
            headers: { Authorization: `Bearer ${token}` },
            body: {},
          })
          .then((res) => {
            expect(res.status, `${email} has answered the username prompt`).to.eq(200);
          }),
      );
    }
  });

  it("serves the client and the deep links the SPA router needs", () => {
    for (const path of ["/", "/login", "/decks", "/play"]) {
      cy.request({ url: path, failOnStatusCode: false }).then((res) => {
        expect(res.status, `GET ${path}`).to.eq(200);
      });
    }
  });

  it("the client it serves is pointed at the deployed server, not localhost", () => {
    cy.request("/").then((res) => {
      const main = /\/assets\/index-[A-Za-z0-9_-]+\.js/.exec(String(res.body))?.[0];
      expect(main, "a main bundle is referenced").to.be.a("string");
      cy.request(String(main)).then((js) => {
        const body = String(js.body);
        expect(body, "the Render URL is compiled in").to.contain(SERVER);
        expect(body, "the localhost fallback is not in use").to.not.contain("localhost:8787");
      });
    });
  });

  it("player 1 signs in THROUGH THE FORM and lands past the invite gate", () => {
    cy.visit("/login");
    cy.get('[data-testid="login-email"]').type(P1);
    cy.get('[data-testid="login-password"]').type(PASSWORD, { log: false });
    cy.get('[data-testid="login-submit"]').click();

    // Issue #479: a sign-in lands on the main menu.
    cy.location("pathname", { timeout: 40_000 }).should("eq", "/");
    cy.get('[data-testid="landing-account"]', { timeout: 40_000 }).should("exist");
    cy.get('[data-testid="login-error"]').should("not.exist");

    // Past the invite gate: the gate leaves an active account on /decks; a pending one would be
    // bounced to /invite. (#1577: the landing offers no Build decks door.)
    cy.visit("/decks");
    cy.get('[data-testid="deck-list"]', { timeout: 40_000 }).should("exist");
    cy.location("pathname").should("eq", "/decks");
  });

  it("the collection and the three seeded decks come back from Render", () => {
    tokenFor(P1).then((token) => {
      cy.request({ url: `${SERVER}/api/collection`, headers: { Authorization: `Bearer ${token}` } })
        .then((res) => {
          expect(res.status).to.eq(200);
          expect(res.body.entries, "cards owned").to.have.length.greaterThan(0);
        });
      // R254: migration 0007 turned the seeded loadout into three saved decks and one trio. The
      // queue bodies below send the second of them by id (`secondDeckId`).
      cy.request({ url: `${SERVER}/api/decks`, headers: { Authorization: `Bearer ${token}` } })
        .then((res) => {
          expect(res.status).to.eq(200);
          expect(res.body.decks, "the saved decks").to.have.length.at.least(3);
          expect(res.body.trios, "and a trio").to.have.length.at.least(1);
        });
    });
  });

  it("two accounts meet in one match through a room code", () => {
    let code = "";
    let t1 = "";
    let t2 = "";
    let d1 = "";
    let d2 = "";

    resetOnlineState();
    tokenFor(P1).then((token) => {
      t1 = token;
    });
    tokenFor(P2).then((token) => {
      t2 = token;
    });
    cy.then(() =>
      secondDeckId(t1).then((id) => {
        d1 = id;
      }),
    );
    cy.then(() =>
      secondDeckId(t2).then((id) => {
        d2 = id;
      }),
    );

    cy.then(() =>
      cy
        .request({
          method: "POST",
          url: `${SERVER}/api/rooms`,
          headers: { Authorization: `Bearer ${t1}` },
          body: { mode: "bo1", deckId: d1 },
        })
        .then((res) => {
          expect(res.status, "room created").to.eq(200);
          code = String(res.body.code ?? res.body.roomCode ?? "");
          expect(code, "a 6-character room code (R79)").to.have.length(6);
        }),
    );

    // Player 2 claims it. §9.5: this is the atomic join that flips the match live.
    cy.then(() =>
      cy
        .request({
          method: "POST",
          url: `${SERVER}/api/rooms/${code}/join`,
          headers: { Authorization: `Bearer ${t2}` },
          body: { mode: "bo1", deckId: d2 },
        })
        .then((res) => {
          expect(res.status, "room joined").to.eq(200);
          expect(String(res.body.matchId ?? ""), "the join returns a match id").to.have.length
            .greaterThan(0);
        }),
    );
  });

  /**
   * Both accounts in the browser, not just on the wire, one after the other since Cypress drives one
   * browser. The host does not navigate on its own when its room is claimed, so player 1 visits
   * /match/<id> directly.
   */
  it("both accounts reach the real board in the browser", () => {
    let code = "";
    let matchId = "";

    resetOnlineState();

    signInThroughForm(P1);
    cy.visit("/play");
    cy.get('[data-testid="play-create-room"]').click();
    cy.get('[data-testid="play-room-code"]', { timeout: 40_000 })
      .invoke("text")
      .then((text) => {
        code = String(text).trim();
        expect(code, "a 6-character room code (R79)").to.have.length(6);
      });

    // Player 2, a different account in the same browser, claims it through the UI.
    cy.then(() => {
      signInThroughForm(P2);
      cy.visit("/play");
      cy.get('[data-testid="play-join-code"]').type(code);
      cy.get('[data-testid="play-join-submit"]').click();

      // The joiner is the one the client navigates.
      cy.location("pathname", { timeout: 60_000 }).should("match", /^\/match\//);
      cy.location("pathname").then((path) => {
        matchId = path.replace("/match/", "");
        expect(matchId, "a match id in the URL").to.have.length.greaterThan(0);
      });
      cy.get('[data-testid="board"]', { timeout: 60_000 }).should("exist");
    });

    // Player 1 opens the same match and gets their own seat's view.
    cy.then(() => {
      signInThroughForm(P1);
      cy.visit(`/match/${matchId}`);
      cy.get('[data-testid="board"]', { timeout: 60_000 }).should("exist");
    });
  });

  /**
   * The ranked queue tells the SECOND player the match id: a 500 there once left a live match the
   * client never learned of, which is why this asserts the RESPONSE, not just the database.
   */
  it("the ranked queue pairs two accounts and tells the second one the match id", () => {
    resetOnlineState();

    let t1 = "";
    let t2 = "";
    let d1 = "";
    let d2 = "";
    tokenFor(P1).then((token) => {
      t1 = token;
    });
    tokenFor(P2).then((token) => {
      t2 = token;
    });
    cy.then(() =>
      secondDeckId(t1).then((id) => {
        d1 = id;
      }),
    );
    cy.then(() =>
      secondDeckId(t2).then((id) => {
        d2 = id;
      }),
    );

    cy.then(() =>
      cy
        .request({
          method: "POST",
          url: `${SERVER}/api/queue`,
          headers: { Authorization: `Bearer ${t1}` },
          body: { mode: "bo1", deckId: d1 },
        })
        .then((res) => {
          expect(res.status, "first enqueue").to.eq(200);
          expect(res.body.status, "nobody to pair with yet").to.eq("open");
          expect(res.body.matchId, "and so no match").to.eq(null);
        }),
    );

    cy.then(() =>
      cy
        .request({
          method: "POST",
          url: `${SERVER}/api/queue`,
          headers: { Authorization: `Bearer ${t2}` },
          body: { mode: "bo1", deckId: d2 },
        })
        .then((res) => {
          expect(res.status, "second enqueue — 500 was the bug").to.eq(200);
          expect(String(res.body.matchId ?? ""), "the pairing is reported to the caller").to.have
            .length.greaterThan(0);
        }),
    );
  });

  /**
   * Only one side's HTTP response carries the match id, so `/play` waits on `/api/auth/me`'s
   * `currentMatchId` (§9.5, cleared by every ending). Player 1 queues IN THE BROWSER; player 2 is
   * paired from outside it, so nothing but the poll can move player 1.
   */
  it("the player who waited is taken to the board with no id handed to them", () => {
    resetOnlineState();

    signInThroughForm(P1);
    cy.visit("/play");
    cy.get('[data-testid="play-queue"]').click();
    // Queued, with nobody to pair against.
    cy.get('[data-testid="play-status"]', { timeout: 30_000 }).should("exist");
    cy.location("pathname").should("eq", "/play");

    // Player 2 joins the queue from outside the browser; player 1's tab is told nothing.
    tokenFor(P2).then((t2) => {
      secondDeckId(t2).then((deckId) => {
        cy.request({
          method: "POST",
          url: `${SERVER}/api/queue`,
          headers: { Authorization: `Bearer ${t2}` },
          body: { mode: "bo1", deckId },
        }).then((res) => {
          expect(res.status, "player 2 enqueues").to.eq(200);
        });
      });
    });

    cy.location("pathname", { timeout: 60_000 }).should("match", /^\/match\//);
    cy.get('[data-testid="board"]', { timeout: 60_000 }).should("exist");
  });

  /** The account screen: who is signed in, their record, and signing out. */
  it("the account screen shows who you are signed in as, and your record", () => {
    signInThroughForm(P1);
    cy.visit("/account");

    cy.get('[data-testid="account-email"]', { timeout: 40_000 }).should("contain.text", P1);
    cy.get('[data-testid="account-status"]').should("have.attr", "data-status", "active");
    // Seeded accounts have played during these runs, so the record is real data off `results`.
    // R612: the screen shows the visible rank, never the hidden rating.
    cy.get('[data-testid="account-rank"]').invoke("text").should("match", /Grape|Raisin|Jlorious/);
    cy.get('[data-testid="account-record"]').invoke("text").should("match", /^\d+–\d+–\d+$/);
    cy.get('[data-testid="account-win-rate"]').should("exist");
  });

  it("signing out clears the session and an anonymous visitor is sent back to sign in", () => {
    signInThroughForm(P1);
    cy.visit("/account");
    cy.get('[data-testid="account-sign-out"]', { timeout: 40_000 }).click();

    // A real navigation, so the token is gone from storage as well as from memory.
    cy.location("pathname", { timeout: 40_000 }).should("eq", "/");
    cy.window().then((win) => {
      expect(win.sessionStorage.getItem("jackioh.session"), "the token is cleared from the tab").to.eq(null);
      expect(win.localStorage.getItem("jackioh.session"), "and none is left in localStorage").to.eq(null);
    });

    // And the gate now refuses a guarded screen.
    cy.visit("/decks");
    cy.location("pathname", { timeout: 40_000 }).should("eq", "/login");
  });

  it("every inner screen offers a way back, and the landing offers the account", () => {
    signInThroughForm(P1);

    for (const path of ["/play", "/account"]) {
      cy.visit(path);
      cy.get('[data-testid="nav-back"], [data-testid="account-screen"]', { timeout: 40_000 }).should(
        "exist",
      );
    }

    cy.visit("/play");
    cy.get('[data-testid="nav-back"]').click();
    cy.location("pathname", { timeout: 40_000 }).should("eq", "/");
    cy.get('[data-testid="landing-account"]').should("exist");
  });

  it("the password can be revealed while typing it", () => {
    cy.visit("/login");
    cy.get('[data-testid="login-password"]').should("have.attr", "type", "password");
    cy.get('[data-testid="login-password"]').type("hunter22222");
    cy.get('[data-testid="login-toggle-password"]').click();
    cy.get('[data-testid="login-password"]')
      .should("have.attr", "type", "text")
      .and("have.value", "hunter22222");
  });
});
