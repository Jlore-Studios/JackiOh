// BUILD M8: §9.4 and §9.8 require identical bad-code responses; R107 hides branch timing.
// The §9.4 attempt budget permits one screen failure, three HTTP failures, then redemption; M6-T1 tests timing tightly.
// R111's activation grant enables an L1–L6 legal trio (R250, R252, R253); test order leaves the fixture active.

import {
  CODE_ALPHABET,
  CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
  INVITE_CODE_GROUP_SIZE,
  INVITE_CODE_LENGTH,
  INVITE_CODE_SEPARATOR,
  REDEMPTION_IDENTICAL_ERROR,
  REDEMPTION_RESPONSE_FLOOR_MS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import { CARD_NAMES, cardId } from "../../support/cards.ts";
import { accounts, constants, inviteCodes, routes, seedFor, server } from "../../support/config.ts";
import {
  INVITE_CODE_INPUT,
  INVITE_ERROR,
  INVITE_NOT_NEEDED,
  INVITE_PAUSED,
  INVITE_SUBMIT,
  ts,
} from "../../support/testids.ts";
import { mintId } from "../../support/commands.ts";

/** ASK: `cy.signIn(account)` and its session key belong in `e2e/support/**`. */
const SESSION_STORAGE_KEY = "jackioh.e2e.session";

/** Not R107's floor: bounds localhost transport spread; M6-T1 owns the tight timing assertion. */
const TRANSPORT_JITTER_MS = 100;

const CORE_CARD_COUNT = Object.keys(CARD_NAMES).length;

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

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

function redeem(code: string): Cypress.Chainable<Cypress.Response<RedeemBody>> {
  return cy.request<RedeemBody>({
    method: "POST",
    url: api("/api/codes/redeem"),
    headers: bearer(pendingToken()),
    body: { code },
    failOnStatusCode: false,
  });
}

function me(token: string): Cypress.Chainable<Cypress.Response<MeBody>> {
  return cy.request<MeBody>({
    method: "GET",
    url: api("/api/auth/me"),
    headers: bearer(token),
    failOnStatusCode: false,
  });
}

function visitAs(token: string, path: string): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
    },
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
      url: api("/api/codes/status"),
      headers: bearer(pendingToken()),
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
        url: api(door.path),
        headers: bearer(pendingToken()),
        body: door.body,
        failOnStatusCode: false,
      }).should((response) => {
        expect(response.status, `§9.4's gate closes ${door.method} ${door.path}`).to.eq(403);
        expect(response.body.error.code, "and says why, without naming a rule").to.eq(
          "account_pending",
        );
      });
    }

    visitAs(pendingToken(), routes.deckbuilder());
    cy.location("pathname").should("eq", routes.invite());

    visitAs(pendingToken(), routes.invite());
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

    // R104 normalizes lower-case input to the canonical formatted code.
    // §9.4 permits exactly this UI failure before the three HTTP failures and valid redemption.
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

  it("R111 — a good code activates, grants one copy of every non-token card, and is idempotent", () => {
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

    // Read R111's grant from the §9.1 ledger across SPEC §8, §8.6, and §8.7.
    let expected = new Set<string>();
    cy.request<CatalogBody>({ method: "GET", url: api("/api/catalog") }).then((response) => {
      expected = new Set(
        Object.entries(response.body.defs)
          .filter(([, def]) => !def.token)
          .map(([id]) => id),
      );
      for (let index = 1; index <= CORE_CARD_COUNT; index += 1) {
        expect(expected.has(cardId(index)), `the catalog holds SPEC §8's ${cardId(index)}`).to.eq(true);
      }
    });
    cy.request<CollectionBody>({
      method: "GET",
      url: api("/api/collection"),
      headers: bearer(pendingToken()),
    }).should((response) => {
      expect(response.status, "the gate is open now").to.eq(200);
      const entries = response.body.entries;
      const owned = new Set(entries.map((entry) => entry.cardId));

      expect(
        owned.size,
        `R111 grants the catalog's ${String(expected.size)} non-token cards and no token`,
      ).to.eq(expected.size);
      for (const id of expected) {
        expect(owned.has(id), `R111 granted ${id}`).to.eq(true);
      }
      for (const entry of entries) {
        expect(
          entry.quantity,
          `R111 grants one copy, which with MAX_COPIES = ${String(constants.MAX_COPIES)} is exactly enough: ${entry.cardId}`,
        ).to.eq(constants.MAX_COPIES);
      }
      granted = JSON.stringify(entries);
    });

    // R111 is idempotent; §9.4 makes repeat redemption an account-state conflict.
    redeem(inviteCodes.good()).should((response) => {
      expect(response.status, "an already-active account is 409, not a code failure").to.eq(409);
      expect(
        response.body.error?.message,
        "§9.4's identical error covers the three code kinds, not the account's own state",
      ).to.not.eq(REDEMPTION_IDENTICAL_ERROR);
    });

    cy.then(() => {
      cy.request<CollectionBody>({
        method: "GET",
        url: api("/api/collection"),
        headers: bearer(pendingToken()),
      }).should((response) => {
        expect(
          JSON.stringify(response.body.entries),
          "R111 is idempotent: the second redemption changed nothing",
        ).to.eq(granted);
      });
    });

    cy.installLoadout(accounts.pending(), "10-invite-gate-a").then((installed) => {
      cy.request<{ status: string; mode: string }>({
        method: "POST",
        url: api("/api/queue"),
        headers: bearer(pendingToken()),
        body: { mode: "bo3", trioId: installed.trioId },
      }).should((response) => {
        expect(response.status, "R253: the granted cards make a trio Best of 3 accepts").to.eq(200);
        expect(response.body.mode).to.eq("bo3");
      });
      cy.request({ method: "DELETE", url: api("/api/queue"), headers: bearer(pendingToken()) })
        .its("status")
        .should("eq", 200);
    });

    visitAs(pendingToken(), routes.deckbuilder());
    cy.location("pathname").should("eq", routes.deckbuilder());
  });
});
