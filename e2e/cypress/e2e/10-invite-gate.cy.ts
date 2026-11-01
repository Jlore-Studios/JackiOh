// BUILD M8: §9.4 and §9.8 require identical bad-code responses; R107 hides branch timing.
// The §9.4 attempt budget permits one screen failure, three HTTP failures, then redemption.
// R111's activation grant is the starter cards (R1310000, §14.12): enough for a deck Best of 1 accepts
// (L2, L3, L5, L6; R250, R253), not for a trio's three DECK_SIZE decks of distinct cards (L4).
// Test order leaves the fixture active.

import {
  CODE_ALPHABET,
  CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
  INVITE_CODE_GROUP_SIZE,
  INVITE_CODE_LENGTH,
  INVITE_CODE_SEPARATOR,
  REDEMPTION_IDENTICAL_ERROR,
  REDEMPTION_RESPONSE_FLOOR_MS,
  STARTER_CARDS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import {
  E2E_RESEED_PATH,
  accounts,
  apiUrl,
  bearerHeaders,
  constants,
  inviteCodes,
  routes,
  seedFor,
} from "../../support/config.ts";
import {
  INVITE_CODE_INPUT,
  INVITE_ERROR,
  INVITE_NOT_NEEDED,
  INVITE_PAUSED,
  INVITE_SUBMIT,
  ts,
} from "../../support/testids.ts";
import { mintId } from "../../support/commands.ts";

/** Not R107's floor: bounds localhost transport spread; M6-T1 owns the tight timing assertion. */
const TRANSPORT_JITTER_MS = 100;

/** The deck the activated account saves and queues: starter cards alone (R1310000). */
const STARTER_DECK_FIXTURE = "10-invite-gate-a";

function pendingToken(): string {
  return accounts.pending().token;
}

type MeBody = {
  profile: { id: string; status: string };
  needsInviteCode: boolean;
  emailVerified: boolean;
};

type RedeemBody = {
  status?: string;
  needsInviteCode?: boolean;
  error?: { code: string; message: string; details?: unknown };
};

type CatalogBody = { version: string; defs: Record<string, { token: boolean }> };

type CollectionBody = {
  catalogVersion: string;
  entries: { cardId: string; quantity: number }[];
};

type ReseedBody = { profiles: Record<string, string>; grantedCards: number; codes: string[] };

function redeem(code: string): Cypress.Chainable<Cypress.Response<RedeemBody>> {
  return cy.request<RedeemBody>({
    method: "POST",
    url: apiUrl("/api/codes/redeem"),
    headers: bearerHeaders(pendingToken()),
    body: { code },
    failOnStatusCode: false,
  });
}

function me(token: string): Cypress.Chainable<Cypress.Response<MeBody>> {
  return cy.request<MeBody>({
    method: "GET",
    url: apiUrl("/api/auth/me"),
    headers: bearerHeaders(token),
    failOnStatusCode: false,
  });
}

/** §9.4 format over R104's alphabet. */
function expectWellFormedInviteCode(formatted: string): void {
  const groups = formatted.split(INVITE_CODE_SEPARATOR);
  expect(
    groups.length,
    `§9.4: ${String(INVITE_CODE_LENGTH / INVITE_CODE_GROUP_SIZE)} groups of ${String(INVITE_CODE_GROUP_SIZE)}`,
  ).to.eq(INVITE_CODE_LENGTH / INVITE_CODE_GROUP_SIZE);
  for (const group of groups) {
    expect(group.length, `§9.4: groups of ${String(INVITE_CODE_GROUP_SIZE)}`).to.eq(
      INVITE_CODE_GROUP_SIZE,
    );
  }
  const bare = groups.join("");
  expect(bare.length, `§9.4: ${String(INVITE_CODE_LENGTH)} characters, 80 bits over 32 symbols`).to.eq(
    INVITE_CODE_LENGTH,
  );
  for (const character of bare) {
    expect(
      CODE_ALPHABET.includes(character),
      `R104: "${character}" is in the code alphabet ${CODE_ALPHABET}`,
    ).to.eq(true);
  }
}

describe("10 invite gate — a pending account", () => {
  const seed = seedFor("10-invite-gate");

  before(() => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");
    // §9.4 fixtures must be valid codes, not malformed inputs.
    for (const code of [
      inviteCodes.good(),
      inviteCodes.missing(),
      inviteCodes.expired(),
      inviteCodes.exhausted(),
    ]) {
      expectWellFormedInviteCode(code);
    }
    // R144: the server reseeds the account and code this file spends, so it passes run after run.
    cy.request<ReseedBody>({ method: "POST", url: apiUrl(E2E_RESEED_PATH) }).should((response) => {
      expect(response.status, "R144: an E2E server reseeds on demand").to.eq(200);
      expect(response.body.codes, "the good, expired and exhausted codes; never the missing one").to.deep.eq([
        "good",
        "expired",
        "exhausted",
      ]);
      expect(Object.keys(response.body.profiles).length, "every fixture account").to.eq(
        Object.keys(accounts).length,
      );
    });
  });

  it("the code screen is shown, and a pending account can reach nothing else", () => {
    me(pendingToken()).should((response) => {
      expect(response.status, "`/api/auth/me` is `auth: \"user\"` — this *is* the code screen's read").to.eq(
        200,
      );
      expect(response.body.profile.status).to.eq("pending");
      expect(response.body.needsInviteCode, "so the client knows to show the code screen").to.eq(
        true,
      );
      expect(response.body.emailVerified, "§9.4 makes a verified email a precondition").to.eq(true);
    });

    cy.request<{ redemptionEnabled: boolean; retryAfterMs: number }>({
      method: "GET",
      url: apiUrl("/api/codes/status"),
      headers: bearerHeaders(pendingToken()),
    }).should((response) => {
      expect(response.status).to.eq(200);
      expect(response.body.redemptionEnabled, "§9.4's circuit breaker is closed").to.eq(true);
      expect(response.body.retryAfterMs).to.eq(0);
    });

    // §9.4 gates collection, saved loadouts (R250, R252), and every queue mode (R257).
    const someId = mintId();
    const gated: { method: "GET" | "PUT" | "DELETE" | "POST"; path: string; body?: Record<string, unknown> }[] = [
      { method: "GET", path: "/api/collection" },
      { method: "GET", path: "/api/decks" },
      { method: "PUT", path: `/api/decks/${someId}`, body: { name: "Gate", cards: [], catalogVersion: "whatever" } },
      { method: "DELETE", path: `/api/decks/${someId}` },
      { method: "PUT", path: `/api/trios/${someId}`, body: { name: "Gate", deckIds: [null, null, null] } },
      { method: "POST", path: "/api/queue", body: { mode: "bo1", deckId: someId } },
      { method: "POST", path: "/api/queue", body: { mode: "random" } },
    ];
    for (const door of gated) {
      cy.request<{ error: { code: string } }>({
        method: door.method,
        url: apiUrl(door.path),
        headers: bearerHeaders(pendingToken()),
        body: door.body,
        failOnStatusCode: false,
      }).should((response) => {
        expect(response.status, `§9.4's gate closes ${door.method} ${door.path}`).to.eq(403);
        expect(response.body.error.code, "and says why, without naming a rule").to.eq(
          "account_pending",
        );
      });
    }

    cy.visitAs(accounts.pending(), routes.deckbuilder());
    // The redirect is the gate's own read of /api/auth/me (net/gate.ts), just answered as pending.
    // When it never comes, the message names the panel that held the page.
    cy.window().should((win) => {
      const shown = win.document.body.innerText.replace(/\s+/gu, " ").slice(0, 400);
      expect(win.location.pathname, `the gate sends a pending account to /invite; the page shows: ${shown}`).to.eq(
        routes.invite(),
      );
    });

    cy.visitAs(accounts.pending(), routes.invite());
    cy.location("pathname").should("eq", routes.invite());

    // A13: assert rendered controls, not only the `/invite` route.
    cy.get(ts(INVITE_CODE_INPUT)).should("be.visible").and("have.value", "");
    cy.get(ts(INVITE_CODE_INPUT)).should(
      "have.attr",
      "placeholder",
      Array.from({ length: INVITE_CODE_LENGTH / INVITE_CODE_GROUP_SIZE }, () =>
        "X".repeat(INVITE_CODE_GROUP_SIZE),
      ).join(INVITE_CODE_SEPARATOR),
    );
    cy.get(ts(INVITE_SUBMIT)).should("be.visible").and("be.disabled");
    cy.get(ts(INVITE_ERROR)).should("not.exist");
    cy.get(ts(INVITE_PAUSED)).should("not.exist");
    cy.get(ts(INVITE_NOT_NEEDED)).should("not.exist");

    // R104 normalizes lower-case input; this is §9.4's one screen failure.
    cy.get(ts(INVITE_CODE_INPUT)).type(inviteCodes.missing().toLowerCase());
    cy.get(ts(INVITE_CODE_INPUT)).should("have.value", inviteCodes.missing());
    cy.get(ts(INVITE_SUBMIT)).should("not.be.disabled").click();

    // R145: render the server's §9.4 rejection verbatim.
    cy.get(ts(INVITE_ERROR)).should("have.text", REDEMPTION_IDENTICAL_ERROR);
    cy.location("pathname").should("eq", routes.invite());
    cy.get(ts(INVITE_NOT_NEEDED)).should("not.exist");
    me(pendingToken()).should((response) => {
      expect(response.body.profile.status, "a refused code activates nothing").to.eq("pending");
      expect(response.body.needsInviteCode).to.eq(true);
    });
  });

  it("R107 — a missing, an expired and an exhausted code are indistinguishable", () => {
    const kinds: { name: string; code: string }[] = [
      { name: "missing", code: inviteCodes.missing() },
      { name: "expired", code: inviteCodes.expired() },
      { name: "exhausted", code: inviteCodes.exhausted() },
    ];
    expect(
      kinds.length,
      `three samples keeps the file inside §9.4's ${String(CODE_ATTEMPTS_PER_PROFILE_PER_HOUR)}-attempt-per-hour budget`,
    ).to.be.lessThan(CODE_ATTEMPTS_PER_PROFILE_PER_HOUR);

    const bodies: string[] = [];
    const statuses: number[] = [];
    const durations: number[] = [];

    for (const kind of kinds) {
      redeem(kind.code).then((response) => {
        expect(response.status, `${kind.name}: §9.4's rejection is a 400 invalid_code`).to.eq(400);
        expect(response.body.error?.code, `${kind.name}: the same error code`).to.eq("invalid_code");
        expect(response.body.error?.message, `${kind.name}: the one client-facing sentence`).to.eq(
          REDEMPTION_IDENTICAL_ERROR,
        );
        expect(
          response.body.error?.details,
          `${kind.name}: no \`details\` — §9.4 keeps the operator-facing reason in code_attempts, "never returned to the client"`,
        ).to.eq(undefined);
        expect(
          response.duration,
          `${kind.name}: R107 pads every redemption response to ${String(REDEMPTION_RESPONSE_FLOOR_MS)} ms`,
        ).to.be.at.least(REDEMPTION_RESPONSE_FLOOR_MS);

        statuses.push(response.status);
        durations.push(response.duration);
        bodies.push(JSON.stringify(response.body));
      });
    }

    cy.then(() => {
      expect(
        new Set(bodies).size,
        `§9.4: "an identical error" — one response body for all three kinds:\n  ${bodies.join("\n  ")}`,
      ).to.eq(1);
      expect(new Set(statuses).size, "and one status").to.eq(1);

      const spread = Math.max(...durations) - Math.min(...durations);
      expect(
        spread,
        `§9.4: "in identical time" — R107's ${String(REDEMPTION_RESPONSE_FLOOR_MS)} ms floor leaves only transport between them (${durations.join(", ")} ms)`,
      ).to.be.at.most(TRANSPORT_JITTER_MS);
    });
  });

  it("R111 — a good code activates, grants one copy of each starter card, and is idempotent", () => {
    let granted = "";

    redeem(inviteCodes.good()).should((response) => {
      expect(response.status, "§9.4: redemption flips pending to active").to.eq(200);
      expect(response.body.status).to.eq("active");
      expect(response.body.needsInviteCode, "so the code screen goes away").to.eq(false);
      expect(
        response.duration,
        `R107 pads the success too, or the floor itself would be the oracle (${String(REDEMPTION_RESPONSE_FLOOR_MS)} ms)`,
      ).to.be.at.least(REDEMPTION_RESPONSE_FLOOR_MS);
    });

    me(pendingToken()).should((response) => {
      expect(response.body.profile.status).to.eq("active");
      expect(response.body.needsInviteCode).to.eq(false);
    });

    // Read R111's grant from the §9.1 ledger: `STARTER_CARDS` (R1310000).
    cy.request<CatalogBody>({ method: "GET", url: apiUrl("/api/catalog") }).then((response) => {
      for (const id of STARTER_CARDS) {
        const def = response.body.defs[id];
        expect(def, `the catalog holds the starter card ${id}`).to.not.eq(undefined);
        expect(def?.token === true, `the starter card ${id} is not a token`).to.eq(false);
      }
    });
    cy.request<CollectionBody>({
      method: "GET",
      url: apiUrl("/api/collection"),
      headers: bearerHeaders(pendingToken()),
    }).should((response) => {
      expect(response.status, "the gate is open now").to.eq(200);
      const entries = response.body.entries;

      expect(
        entries.map((entry) => entry.cardId).sort(),
        `R111, R1310000: the grant is the ${String(STARTER_CARDS.length)} starter cards and nothing else`,
      ).to.deep.eq([...STARTER_CARDS].sort());
      for (const entry of entries) {
        expect(
          entry.quantity,
          `R111 grants one copy, which with MAX_COPIES = ${String(constants.MAX_COPIES)} is exactly enough: ${entry.cardId}`,
        ).to.eq(constants.MAX_COPIES);
      }
      granted = JSON.stringify(entries);
    });

    redeem(inviteCodes.good()).should((response) => {
      expect(response.status, "an already-active account is 409, not a code failure").to.eq(409);
    // R111 is idempotent; §9.4 makes repeat redemption an account-state conflict.
      expect(
        response.body.error?.message,
        "§9.4's identical error covers the three code kinds, not the account's own state",
      ).to.not.eq(REDEMPTION_IDENTICAL_ERROR);
    });

    cy.then(() => {
      cy.request<CollectionBody>({
        method: "GET",
        url: apiUrl("/api/collection"),
        headers: bearerHeaders(pendingToken()),
      }).should((response) => {
        expect(
          JSON.stringify(response.body.entries),
          "R111 is idempotent: the second redemption changed nothing",
        ).to.eq(granted);
      });
    });

    // R253: the granted cards make a deck Best of 1 accepts, L5 (owned) included. The fixture is
    // starter cards alone, which is checked here so a list that moves fails with its own words.
    cy.fixture(`decks/${STARTER_DECK_FIXTURE}.json`).then((raw: { cards: string[] }) => {
      for (const id of raw.cards) {
        expect(STARTER_CARDS, `decks/${STARTER_DECK_FIXTURE}.json holds starter cards only`).to.include(id);
      }
      cy.clearDecks(accounts.pending())
        .then((catalogVersion) => cy.saveDeck(accounts.pending(), { name: "Deck 1", cards: raw.cards, catalogVersion }))
        .then((deckId) => {
          cy.request<{ status: string; mode: string }>({
            method: "POST",
            url: apiUrl("/api/queue"),
            headers: bearerHeaders(pendingToken()),
            body: { mode: "bo1", deckId },
          }).should((response) => {
            expect(response.status, "R253, R1310000: the starter cards make a deck Best of 1 accepts").to.eq(200);
            expect(response.body.mode).to.eq("bo1");
          });
          cy.request({ method: "DELETE", url: apiUrl("/api/queue"), headers: bearerHeaders(pendingToken()) })
            .its("status")
            .should("eq", 200);
        });
    });

    cy.visitAs(accounts.pending(), routes.deckbuilder());
    cy.location("pathname").should("eq", routes.deckbuilder());
  });
});
