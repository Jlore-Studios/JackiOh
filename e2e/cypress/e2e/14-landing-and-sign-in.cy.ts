// Spec 14: landing and sign-in (B16-B18, B22, B23, B29-B31, B37 and B40).
// No server or provider: API calls are intercepted, sessions seed in `onBeforeLoad`, and links use
// `/login` hashes or queries. The CORS stub handles client/API preflights.
// Paste uses a realm-local `ClipboardEvent`; unit tests cover the unhandled path.

import {
  AUTH_PASSWORD_MIN_LENGTH,
  CODE_ALPHABET,
  CODE_ATTEMPT_WINDOW_SECONDS,
  INVITE_CODE_GROUP_SIZE,
  INVITE_CODE_LENGTH,
  INVITE_CODE_SEPARATOR,
  REDEMPTION_IDENTICAL_ERROR,
} from "../../../apps/web/src/wire/serverConfig.ts";
import {
  codeFieldTestid,
  inviteTestid,
  landingTestid,
  loginTestid,
  resetTestid,
  shellTestid,
} from "../../../apps/web/src/auth/testids.ts";
import {
  SESSION_STORAGE_KEY,
  rememberPendingEmail,
  rememberPendingReset,
} from "../../../apps/web/src/net/session.ts";
import { SESSION_STORAGE_KEY as FIXTURE_SESSION_KEY, routes } from "../../support/config.ts";

// Values from config

const TOKEN = "e2e-token-spec-14";
const EMAIL = "e2e-spec14@jackioh.test";

/** R104: a code with no digit. */
const LETTERS = CODE_ALPHABET.replace(/[0-9]/g, "");
const BARE_CODE = LETTERS.slice(0, INVITE_CODE_LENGTH);

function grouped(characters: string, separator = INVITE_CODE_SEPARATOR): string {
  const groups: string[] = [];
  for (let index = 0; index < characters.length; index += INVITE_CODE_GROUP_SIZE) {
    groups.push(characters.slice(index, index + INVITE_CODE_GROUP_SIZE));
  }
  return groups.join(separator);
}

const FULL_CODE = grouped(BARE_CODE);

/** R104 excludes 0, 1, I and O. */
const EXCLUDED = [..."0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"].filter((c) => !CODE_ALPHABET.includes(c));

/** A server 429 sentence the screen must relay verbatim. */
const SERVER_RATE_SENTENCE = "Too many attempts. Wait, then try again. (spec 14 stub)";
const WINDOW_MS = CODE_ATTEMPT_WINDOW_SECONDS * 1000;

/** Provider text that must never reach the page (R193). */
const PROVIDER_TEXT = "PROVIDER-SAYS-7f3a Email link is invalid or has expired";

/** Routes omitted from support/config.ts, copied from navigate.ts because e2e type-checks alone. */
const LANDING = "/";
const PRACTICE = "/practice";
const RESET_PASSWORD = "/reset-password";

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

// Stubbed API

type MeState = "pending" | "active" | "banned" | "failing";

type Api = {
  me: MeState;
  attemptsRemaining: number;
  redeem: "rateLimited" | "invalid";
  /** Server-owned address by bearer token; a forged JWT payload must not choose the account. */
  accounts: Record<string, string>;
};

function corsHeaders(origin: unknown): Record<string, string> {
  return {
    "access-control-allow-origin": typeof origin === "string" ? origin : "*",
    "access-control-allow-methods": "GET, POST, PUT, DELETE, OPTIONS",
    "access-control-allow-headers": "authorization, content-type",
    "access-control-expose-headers": "retry-after",
    "access-control-max-age": "600",
    "content-type": "application/json",
  };
}

function meBody(status: "pending" | "active" | "banned", email: string = EMAIL) {
  return {
    profile: { id: "profile-spec-14", status },
    needsInviteCode: status === "pending",
    emailVerified: true,
    currentMatchId: null,
    currentSeriesId: null,
    email,
  };
}

function payloadEmail(token: string): string | null {
  const payload = token.split(".")[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/g, "+").replace(/_/g, "/");
    const claims = JSON.parse(atob(base64 + "=".repeat((4 - (base64.length % 4)) % 4))) as { email?: unknown };
    return typeof claims.email === "string" ? claims.email : null;
  } catch {
    return null;
  }
}

function bearerOf(header: string | string[] | undefined): string {
  const value = Array.isArray(header) ? (header[0] ?? "") : (header ?? "");
  return value.replace(/^Bearer /, "");
}

/** A live API state: StrictMode may read twice, so tests change fields rather than a counter. */
function stubApi(initial: Partial<Api> = {}): Api {
  const api: Api = { me: "pending", attemptsRemaining: 5, redeem: "rateLimited", accounts: {}, ...initial };

  // Cypress tries newest routes first; this catch-all must be registered first.
  cy.intercept({ pathname: /^\/api\// }, (req) => {
    const headers = corsHeaders(req.headers.origin);
    if (req.method === "OPTIONS") {
      req.reply({ statusCode: 204, headers });
      return;
    }
    req.reply({
      statusCode: 404,
      headers,
      body: { error: { code: "not_found", message: `spec 14 has no stub for ${req.url}` } },
    });
  });

  cy.intercept({ method: "GET", pathname: "/api/auth/me" }, (req) => {
    const headers = corsHeaders(req.headers.origin);
    if (api.me === "failing") {
      req.reply({ statusCode: 500, headers, body: { error: { code: "internal", message: "the server fell over" } } });
      return;
    }
    const bearer = bearerOf(req.headers.authorization);
    const email = api.accounts[bearer] ?? payloadEmail(bearer) ?? EMAIL;
    req.reply({ statusCode: 200, headers, body: meBody(api.me, email) });
  }).as("me");

  cy.intercept({ method: "GET", pathname: "/api/codes/status" }, (req) => {
    req.reply({
      statusCode: 200,
      headers: corsHeaders(req.headers.origin),
      body: { redemptionEnabled: true, retryAfterMs: 0, attemptsRemaining: api.attemptsRemaining },
    });
  }).as("status");

  cy.intercept({ method: "POST", pathname: "/api/codes/redeem" }, (req) => {
    const headers = corsHeaders(req.headers.origin);
    if (api.redeem === "invalid") {
      req.reply({
        statusCode: 400,
        headers,
        body: { error: { code: "invalid_code", message: REDEMPTION_IDENTICAL_ERROR } },
      });
      return;
    }
    req.reply({
      statusCode: 429,
      headers: { ...headers, "retry-after": String(Math.ceil(WINDOW_MS / 1000)) },
      body: { error: { code: "rate_limited", message: SERVER_RATE_SENTENCE, details: { retryAfterMs: WINDOW_MS } } },
    });
  }).as("redeem");

  return api;
}

/** No request may reach the auth provider. */
function forbidProvider(): void {
  cy.intercept({ pathname: /\/auth\/v1\// }, () => {
    throw new Error("spec 14 must never reach the auth provider");
  }).as("provider");
}

function visitSignedIn(path: string): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem(FIXTURE_SESSION_KEY, JSON.stringify({ accessToken: TOKEN }));
    },
  });
}

function sessionKeysIn(win: Window): { real: string | null; fixture: string | null } {
  return {
    // R632: a real sign-in stays in sessionStorage.
    real: win.sessionStorage.getItem(SESSION_STORAGE_KEY) ?? win.localStorage.getItem(SESSION_STORAGE_KEY),
    fixture: win.localStorage.getItem(FIXTURE_SESSION_KEY),
  };
}

// Emailed links

function base64url(text: string): string {
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** An unsigned JWT for link tests; the client never verifies it. */
function unsignedJwt(email: string): string {
  const header = base64url(JSON.stringify({ alg: "none", typ: "JWT" }));
  const payload = base64url(
    JSON.stringify({ sub: "user-spec-14", email, role: "authenticated", exp: Math.floor(Date.now() / 1000) + 3600 }),
  );
  return `${header}.${payload}.unsigned`;
}

function tokenLink(type: string, accessToken: string): string {
  const params = new URLSearchParams({
    access_token: accessToken,
    refresh_token: "refresh-spec-14",
    expires_at: String(Math.floor(Date.now() / 1000) + 3600),
    expires_in: "3600",
    token_type: "bearer",
    type,
  });
  return `${routes.login()}#${params.toString()}`;
}

function errorLink(params: Record<string, string>, where: "fragment" | "query"): string {
  const encoded = new URLSearchParams(params).toString();
  return where === "fragment" ? `${routes.login()}#${encoded}` : `${routes.login()}?${encoded}`;
}

function expectScrubbed(secret: string): void {
  cy.location("hash").should("eq", "");
  cy.location("href").should("not.contain", secret);
}

// Code field

function codeInput(): Cypress.Chainable<JQuery<HTMLElement>> {
  return cy.get(byTestid(inviteTestid.input));
}

function paste(text: string): void {
  codeInput().then(($input) => {
    const input = $input[0];
    if (input === undefined) throw new Error("the code input is not mounted");
    const win = input.ownerDocument.defaultView;
    if (win === null) throw new Error("the code input has no window");
    const data = new win.DataTransfer();
    data.setData("text/plain", text);
    input.focus();
    input.dispatchEvent(new win.ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  });
}

function openInviteScreen(api: Partial<Api> = {}): Api {
  const live = stubApi({ me: "pending", ...api });
  visitSignedIn(routes.invite());
  codeInput().should("be.visible");
  return live;
}

// B37: landing page

describe("B37 the landing page", () => {
  it("B37 anonymous: the three CTAs link to /practice, /play and /decks, and the corner offers sign-in", () => {
    stubApi();
    cy.visit(LANDING);

    cy.get(byTestid(landingTestid.root)).should("be.visible");
    cy.get(byTestid(landingTestid.playAi)).should("be.visible").closest("a").should("have.attr", "href", PRACTICE);
    cy.get(byTestid(landingTestid.playOnline)).should("be.visible").closest("a").should("have.attr", "href", routes.play());
    cy.get(byTestid(landingTestid.buildDecks)).should("be.visible").closest("a").should("have.attr", "href", routes.deckbuilder());
    cy.get(byTestid(landingTestid.signIn)).should("be.visible");
    cy.get(byTestid(landingTestid.account)).should("not.exist");

    cy.get(byTestid(landingTestid.signIn)).click();
    cy.location("pathname").should("eq", routes.login());
    cy.get(byTestid(loginTestid.form)).should("be.visible");
  });

  it("B37 signed in: the corner offers landing-account and no sign-in", () => {
    stubApi({ me: "active" });
    visitSignedIn(LANDING);

    cy.get(byTestid(landingTestid.account)).should("be.visible");
    cy.get(byTestid(landingTestid.signIn)).should("not.exist");
  });

  it("B37 Build decks while anonymous goes through the gate to /login", () => {
    stubApi();
    cy.visit(LANDING);
    cy.get(byTestid(landingTestid.buildDecks)).click();
    cy.location("pathname").should("eq", routes.login());
  });

  it("B37 Play vs AI moves the URL to /practice", () => {
    stubApi();
    cy.visit(LANDING);
    cy.get(byTestid(landingTestid.playAi)).click();
    cy.location("pathname").should("eq", PRACTICE);
  });
});

// B16-B18: typing and pasting a code

describe("B16-B18 the code field", () => {
  it("B16 typing eight lower-case letters shows two upper-case groups, the separator added, caret at the end", () => {
    openInviteScreen();
    const eight = BARE_CODE.slice(0, 2 * INVITE_CODE_GROUP_SIZE);

    codeInput().type(eight.toLowerCase());

    codeInput().should("have.value", grouped(eight));
    codeInput().should(($input) => {
      const input = $input[0] as HTMLInputElement | undefined;
      expect(input?.selectionStart, "the caret is at the end").to.eq(input?.value.length);
    });
    cy.get(byTestid(codeFieldTestid.root)).should("have.attr", "data-complete", "false");
  });

  it("B17 pasting an invite sentence fills the field with the one code in it", () => {
    openInviteScreen();

    paste(`Your invite: ${FULL_CODE.toLowerCase()}.`);

    codeInput().should("have.value", FULL_CODE);
    cy.get(byTestid(codeFieldTestid.root)).should("have.attr", "data-complete", "true");
    cy.get(byTestid(inviteTestid.submit)).should("be.enabled");
  });

  it("B17 a padded paste longer than the old 19-character field is never cut short", () => {
    openInviteScreen();
    const padded = ` ${grouped(BARE_CODE.toLowerCase(), " - ")} `;
    expect(padded.length, "the premise: longer than the old maxLength of 19").to.be.greaterThan(19);

    paste(padded);

    codeInput().should("have.value", FULL_CODE);
    cy.get(byTestid(codeFieldTestid.root)).should("have.attr", "data-complete", "true");
  });

  it("B18 typing an excluded character leaves the value alone and names it; the next accepted key clears the hint", () => {
    openInviteScreen();
    const three = BARE_CODE.slice(0, INVITE_CODE_GROUP_SIZE - 1);

    codeInput().type(`${three.toLowerCase()}0`);

    codeInput().should("have.value", three);
    cy.get(byTestid(codeFieldTestid.hint))
      .should("be.visible")
      .and("have.attr", "data-kind", "excluded")
      .invoke("text")
      .then((text) => {
        for (const character of EXCLUDED) expect(text, `the hint names ${character}`).to.contain(character);
      });
    cy.get(byTestid(codeFieldTestid.root)).should("have.attr", "data-problem", "excluded");

    const fourth = BARE_CODE.charAt(INVITE_CODE_GROUP_SIZE - 1);
    codeInput().type(fourth.toLowerCase());

    codeInput().should("have.value", BARE_CODE.slice(0, INVITE_CODE_GROUP_SIZE));
    cy.get(byTestid(codeFieldTestid.hint)).should("not.exist");
  });

  it("B18 each of R104's excluded characters, in either case, is refused and named", () => {
    openInviteScreen();
    for (const character of EXCLUDED) {
      for (const typed of new Set([character, character.toLowerCase()])) {
        codeInput().clear();
        codeInput().type(`${BARE_CODE.charAt(0).toLowerCase()}${typed}`);
        codeInput().should("have.value", BARE_CODE.charAt(0));
        cy.get(byTestid(codeFieldTestid.hint)).should("have.attr", "data-kind", "excluded");
        cy.get(byTestid(codeFieldTestid.hint)).should("contain.text", character);
      }
    }
  });
});

// B22: a rate limit is a rate limit (R192)

describe("B22 a rate-limited redemption", () => {
  it("B22 shows the server's sentence and the wait, disables submit, and never shows R145's error", () => {
    const api = openInviteScreen({ redeem: "rateLimited" });
    paste(FULL_CODE);
    cy.get(byTestid(inviteTestid.submit)).should("be.enabled");
    cy.then(() => {
      api.attemptsRemaining = 4;
    });

    cy.get(byTestid(inviteTestid.submit)).click();

    cy.wait("@redeem").its("request.body").should("deep.equal", { code: FULL_CODE });
    cy.get(byTestid(inviteTestid.error)).should("have.text", SERVER_RATE_SENTENCE);
    cy.get(byTestid(inviteTestid.rateLimited))
      .should("be.visible")
      .and("have.attr", "data-retry-after-ms", String(WINDOW_MS));
    cy.get(byTestid(inviteTestid.submit)).should("be.disabled");
    cy.get("body").should("not.contain.text", REDEMPTION_IDENTICAL_ERROR);
  });

  it("B22 an invalid code still reads R145's identical sentence, with no rate-limit panel", () => {
    openInviteScreen({ redeem: "invalid" });
    paste(FULL_CODE);
    cy.get(byTestid(inviteTestid.submit)).click();

    cy.wait("@redeem");
    cy.get(byTestid(inviteTestid.error)).should("have.text", REDEMPTION_IDENTICAL_ERROR);
    cy.get(byTestid(inviteTestid.rateLimited)).should("not.exist");
  });
});

// B23: code-screen exit

/** `navTestid.back` is in JSX, outside this bundle. */
const NAV_BACK = "nav-back";

describe("B23 the invite screen's way out", () => {
  it("B23 offers back, the account's email and sign-out; sign-out clears both keys and lands on /", () => {
    stubApi({ me: "pending" });
    cy.visit(routes.invite(), {
      onBeforeLoad(win) {
        win.localStorage.setItem(FIXTURE_SESSION_KEY, JSON.stringify({ accessToken: TOKEN }));
        win.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
      },
    });
    codeInput().should("be.visible");

    cy.get(byTestid(NAV_BACK)).should("be.visible");
    cy.get(byTestid(inviteTestid.accountEmail)).should("have.text", EMAIL);
    cy.get(byTestid(inviteTestid.signOut)).should("be.visible").click();

    cy.location("pathname").should("eq", LANDING);
    cy.get(byTestid(landingTestid.root)).should("be.visible");
    cy.get(byTestid(landingTestid.signIn)).should("be.visible");
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });
});

// Failed sign-in keeps the sign-in screen
// Without a provider, submissions fail before a request; the screen must retain the form.

const PASSWORD = "e2e-spec14-password";

function submitSignIn(): void {
  cy.get(byTestid(loginTestid.email)).clear().type(EMAIL);
  cy.get(byTestid(loginTestid.password)).clear().type(PASSWORD, { log: false });
  cy.get(byTestid(loginTestid.submit)).click();
}

function expectStillSigningIn(): void {
  cy.get(byTestid(loginTestid.error)).should("be.visible").and("have.attr", "role", "alert");
  cy.get(byTestid(loginTestid.error)).invoke("text").should("not.be.empty");
  cy.location("pathname").should("eq", routes.login());
  cy.get(byTestid(loginTestid.form)).should("have.attr", "data-mode", "signIn");
  cy.get(byTestid(loginTestid.email)).should("have.value", EMAIL);
  cy.get(byTestid(loginTestid.password)).should("have.value", PASSWORD);
  cy.get(byTestid(loginTestid.submit)).should("be.enabled");
  cy.window().then((win) => {
    expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
  });
}

function storedValues(win: Window): string[] {
  const values: string[] = [];
  for (const storage of [win.sessionStorage, win.localStorage]) {
    for (let index = 0; index < storage.length; index += 1) {
      values.push(storage.getItem(storage.key(index) ?? "") ?? "");
    }
  }
  return values;
}

describe("#479 a failed sign-in stays on the sign-in screen", () => {
  it("#479 from the main menu: the sentence shows above the form, which keeps what was typed", () => {
    stubApi();
    forbidProvider();
    cy.visit(LANDING);
    cy.get(byTestid(landingTestid.signIn)).click();
    cy.location("pathname").should("eq", routes.login());

    submitSignIn();
    expectStillSigningIn();
  });

  it("#479 sent by a protected screen: a failure stays on the sign-in screen, which keeps no way back to that screen", () => {
    stubApi();
    forbidProvider();
    cy.visit(routes.play());
    cy.location("pathname").should("eq", routes.login());

    submitSignIn();
    expectStillSigningIn();
    // A sign-in returns to the main menu, not its protected origin.
    cy.window().then((win) => {
      for (const value of storedValues(win)) expect(value).not.to.contain(routes.play());
    });

    cy.get(byTestid(NAV_BACK)).click();
    cy.location("pathname").should("eq", LANDING);
    cy.get(byTestid(landingTestid.root)).should("be.visible");
  });
});

// B29, B30: emailed links (R193)

describe("B29 a confirmation link", () => {
  it("R193 B29 whose email matches the sign-up this browser started still signs nothing in: the player signs in with their password", () => {
    // A link is not proof that this browser owns the address.
    stubApi({ me: "pending" });
    forbidProvider();
    const jwt = unsignedJwt(EMAIL);

    cy.visit(tokenLink("signup", jwt), {
      onBeforeLoad() {
        rememberPendingEmail(EMAIL);
      },
    });

    cy.get(byTestid(loginTestid.confirmed)).should("be.visible");
    cy.location("pathname").should("eq", routes.login());
    cy.get(byTestid(loginTestid.password)).should("be.visible").and("have.value", "");
    expectScrubbed(jwt);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
      expect(win.sessionStorage.getItem(SESSION_STORAGE_KEY)).to.eq(null);
      expect(win.localStorage.getItem(SESSION_STORAGE_KEY)).to.eq(null);
    });
  });

  it("B29 with no sign-up remembered stores nothing, says the email is confirmed and fills in nothing", () => {
    stubApi();
    forbidProvider();
    const jwt = unsignedJwt(EMAIL);

    cy.visit(tokenLink("signup", jwt));

    cy.get(byTestid(loginTestid.confirmed)).should("be.visible");
    // A link could be anyone's; its address must not arm this browser's guard.
    cy.get(byTestid(loginTestid.email)).should("have.value", "");
    cy.location("pathname").should("eq", routes.login());
    expectScrubbed(jwt);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("B29 whose email differs from the remembered sign-up stores nothing (login CSRF)", () => {
    stubApi();
    forbidProvider();
    const jwt = unsignedJwt("attacker@jackioh.test");

    cy.visit(tokenLink("signup", jwt), {
      onBeforeLoad() {
        rememberPendingEmail(EMAIL);
      },
    });

    cy.get(byTestid(loginTestid.confirmed)).should("be.visible");
    cy.location("pathname").should("eq", routes.login());
    expectScrubbed(jwt);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("R193 a token whose payload names the remembered address, but which the server says is another account's, stores nothing", () => {
    const api = stubApi();
    forbidProvider();
    // The readable, unverified payload claims the victim's address.
    const jwt = unsignedJwt(EMAIL);
    api.accounts[jwt] = "attacker@jackioh.test";

    cy.visit(tokenLink("signup", jwt), {
      onBeforeLoad() {
        rememberPendingEmail(EMAIL);
      },
    });

    cy.get(byTestid(loginTestid.confirmed)).should("be.visible");
    cy.location("pathname").should("eq", routes.login());
    cy.get(byTestid(loginTestid.email)).should("have.value", "");
    expectScrubbed(jwt);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });
});

describe("B30 a recovery link and an expired link", () => {
  it("B30 a recovery link goes to /reset-password, shows the address, and keeps the session out of storage", () => {
    stubApi();
    forbidProvider();
    const jwt = unsignedJwt(EMAIL);

    // R193: only a reset this browser asked for is accepted.
    cy.visit(tokenLink("recovery", jwt), {
      onBeforeLoad() {
        rememberPendingReset(EMAIL);
      },
    });

    cy.location("pathname").should("eq", RESET_PASSWORD);
    cy.get(byTestid(resetTestid.email)).should("contain.text", EMAIL);
    expectScrubbed(jwt);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("R193 a recovery link this browser never asked for holds nothing until the player types the address it was sent to", () => {
    stubApi();
    forbidProvider();
    const jwt = unsignedJwt("attacker@jackioh.test");

    cy.visit(tokenLink("recovery", jwt));

    cy.get(byTestid(loginTestid.recoveryClaim)).should("be.visible");
    cy.get(byTestid(loginTestid.form)).should("have.attr", "data-mode", "claimReset");
    cy.get(byTestid(loginTestid.email)).should("have.value", "");
    cy.location("pathname").should("eq", routes.login());
    expectScrubbed(jwt);

    cy.get(byTestid(loginTestid.email)).type(EMAIL);
    cy.get(byTestid(loginTestid.submit)).click();
    cy.get(byTestid(loginTestid.emailError)).should("be.visible");
    cy.location("pathname").should("eq", routes.login());
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("R193 a reset asked for on another device works here once the player types its address", () => {
    stubApi();
    forbidProvider();
    const jwt = unsignedJwt(EMAIL);

    cy.visit(tokenLink("recovery", jwt));

    cy.get(byTestid(loginTestid.recoveryClaim)).should("be.visible");
    cy.get(byTestid(loginTestid.email)).type(EMAIL.toUpperCase());
    cy.get(byTestid(loginTestid.submit)).click();
    cy.location("pathname").should("eq", RESET_PASSWORD);
    cy.get(byTestid(resetTestid.email)).should("contain.text", EMAIL);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  for (const where of ["fragment", "query"] as const) {
    it(`B30 an otp_expired link in the ${where} shows the link error, resend and forgot, and never the provider's text`, () => {
      stubApi();
      forbidProvider();

      cy.visit(
        errorLink({ error: "access_denied", error_code: "otp_expired", error_description: PROVIDER_TEXT }, where),
      );

      cy.get(byTestid(loginTestid.linkError)).should("be.visible");
      cy.get(byTestid(loginTestid.resend)).should("be.visible");
      cy.get(byTestid(loginTestid.forgot)).should("be.visible");
      cy.get("body").should("not.contain.text", "PROVIDER-SAYS-7f3a");
      cy.location("pathname").should("eq", routes.login());
      cy.location("hash").should("eq", "");
      cy.location("search").should("eq", "");
    });
  }

  it("B30 another link error shows the same link error, and markup in its description is never rendered", () => {
    stubApi();
    forbidProvider();

    cy.visit(
      errorLink(
        {
          error: "server_error",
          error_code: "unexpected_failure",
          error_description: '<img src="x" data-spec14="xss" onerror="window.__spec14Xss = true">',
        },
        "fragment",
      ),
    );

    cy.get(byTestid(loginTestid.linkError)).should("be.visible");
    cy.get('[data-spec14="xss"]').should("not.exist");
    cy.get("body").should("not.contain.text", "onerror");
    cy.window().should("not.have.property", "__spec14Xss");
    cy.location("hash").should("eq", "");
  });

  it("B30 the forgot action beside a link error opens the forgot form", () => {
    stubApi();
    forbidProvider();
    cy.visit(errorLink({ error: "access_denied", error_code: "otp_expired" }, "fragment"));

    cy.get(byTestid(loginTestid.forgot)).click();

    cy.get(byTestid(loginTestid.form)).should("have.attr", "data-mode", "forgot");
  });
});

// B31: /reset-password

describe("B31 the reset screen", () => {
  function openWithRecoveryLink(): void {
    stubApi();
    forbidProvider();
    cy.visit(tokenLink("recovery", unsignedJwt(EMAIL)), {
      onBeforeLoad() {
        rememberPendingReset(EMAIL);
      },
    });
    cy.location("pathname").should("eq", RESET_PASSWORD);
    cy.get(byTestid(resetTestid.form)).should("be.visible");
  }

  it("B31 refuses a password shorter than AUTH_PASSWORD_MIN_LENGTH without sending it", () => {
    openWithRecoveryLink();
    const short = "p".repeat(AUTH_PASSWORD_MIN_LENGTH - 1);

    cy.get(byTestid(resetTestid.password)).type(short);
    cy.get(byTestid(resetTestid.confirm)).type(short);
    cy.get(byTestid(resetTestid.submit)).click();

    cy.get(byTestid(resetTestid.passwordError)).should("be.visible");
    cy.get(byTestid(resetTestid.password)).should("have.attr", "aria-invalid", "true");
    cy.location("pathname").should("eq", RESET_PASSWORD);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("B31 refuses a confirmation that does not match, without sending it", () => {
    openWithRecoveryLink();
    const password = "p".repeat(AUTH_PASSWORD_MIN_LENGTH + 4);

    cy.get(byTestid(resetTestid.password)).type(password);
    cy.get(byTestid(resetTestid.confirm)).type(`${password}x`);
    cy.get(byTestid(resetTestid.submit)).click();

    cy.get(byTestid(resetTestid.confirmError)).should("be.visible");
    cy.location("pathname").should("eq", RESET_PASSWORD);
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("B31 the new password can be shown and hidden", () => {
    openWithRecoveryLink();
    cy.get(byTestid(resetTestid.password)).should("have.attr", "type", "password");
    cy.get(byTestid(resetTestid.togglePassword)).click();
    cy.get(byTestid(resetTestid.password)).should("have.attr", "type", "text");
    cy.get(byTestid(resetTestid.togglePassword)).click();
    cy.get(byTestid(resetTestid.password)).should("have.attr", "type", "password");
  });

  it("B31 without a recovery session it shows reset-no-link; request-new opens the forgot form", () => {
    stubApi();
    cy.visit(RESET_PASSWORD);

    cy.get(byTestid(resetTestid.noLink)).should("be.visible");
    cy.get(byTestid(resetTestid.form)).should("not.exist");
    cy.get(byTestid(resetTestid.requestNew)).click();

    cy.location("pathname").should("eq", routes.login());
    cy.get(byTestid(loginTestid.form)).should("have.attr", "data-mode", "forgot");
  });

  it("B31 without a recovery session, back-to-sign-in goes to /login", () => {
    stubApi();
    cy.visit(RESET_PASSWORD);

    cy.get(byTestid(resetTestid.backToSignIn)).click();

    cy.location("pathname").should("eq", routes.login());
    cy.get(byTestid(loginTestid.form)).should("be.visible");
  });
});

// B40: no dead ends in the shell

describe("B40 the shell's exits", () => {
  it("B40 the gate's error panel offers retry, home and sign-out; retry opens the gate once /api/auth/me answers", () => {
    const api = stubApi({ me: "failing" });
    visitSignedIn(routes.invite());

    cy.get(byTestid(shellTestid.error)).should("be.visible");
    cy.get(byTestid(shellTestid.retry)).should("be.visible");
    cy.get(byTestid(shellTestid.home)).should("be.visible");
    cy.get(byTestid(shellTestid.signOut)).should("be.visible");

    cy.then(() => {
      api.me = "pending";
    });
    cy.get(byTestid(shellTestid.retry)).click();

    codeInput().should("be.visible");
    cy.get(byTestid(shellTestid.error)).should("not.exist");
    cy.location("pathname").should("eq", routes.invite());
  });

  it("B40 gate-home on the error panel goes to the landing page", () => {
    stubApi({ me: "failing" });
    visitSignedIn(routes.deckbuilder());

    cy.get(byTestid(shellTestid.home)).click();

    cy.location("pathname").should("eq", LANDING);
    cy.get(byTestid(landingTestid.root)).should("be.visible");
  });

  it("B40 the banned panel offers home and sign-out; sign-out clears the session and loads /", () => {
    stubApi({ me: "banned" });
    visitSignedIn(routes.deckbuilder());

    cy.get(byTestid(shellTestid.home)).should("be.visible");
    cy.get(byTestid(shellTestid.signOut)).should("be.visible").click();

    cy.location("pathname").should("eq", LANDING);
    cy.get(byTestid(landingTestid.root)).should("be.visible");
    cy.get(byTestid(landingTestid.signIn)).should("be.visible");
    cy.window().then((win) => {
      expect(sessionKeysIn(win)).to.deep.equal({ real: null, fixture: null });
    });
  });

  it("B40 the 404 panel says so in a player's words and offers a way home", () => {
    stubApi();
    cy.visit("/no-such-screen");

    cy.get(byTestid(shellTestid.notFound)).should("be.visible").and("contain.text", "That page doesn’t exist.");
    cy.get(byTestid(shellTestid.notFoundHome)).click();

    cy.location("pathname").should("eq", LANDING);
    cy.get(byTestid(landingTestid.root)).should("be.visible");
  });
});
