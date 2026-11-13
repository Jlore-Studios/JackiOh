// 56 — the inbox (issue #3729, R3729002): what waits on a player is in the bell's panel, so nothing depends on
// catching a toast. Seat two challenges seat one, who sends its toast away with Later on Collection and then
// accepts the challenge from the bell, landing on the board; and a friend request from seat two is accepted
// from the bell at every one of `MENU_SIZES`, each shooting the open inbox for the pull request
// (`3729-inbox-<size>.png`; a whole shot wants a window larger than the size, `E2E_WINDOW_SIZE` in e2e/README.md).
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server`. Daily only, as every spec outside
// scripts/shard-specs.mjs's SMOKE.

import { inboxTestid } from "../../../apps/web/src/inbox/testids.ts";
import { PRESENCE_HEARTBEAT_MS, SOCIAL_POLL_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { MENU_SIZES, accounts, apiUrl, bearerHeaders, routes, timeouts, type E2EAccount } from "../../support/config.ts";
import { CHALLENGE_LATER, CHALLENGE_TOAST, FRIENDS_BUTTON, heroId, ts } from "../../support/testids.ts";

const MATCH_PATH = /^\/match\/([^/]+)$/;

/** A challenge or a request lands within the social poll and a render. */
const SOCIAL_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;
const FRIENDS_TIMEOUT_MS = PRESENCE_HEARTBEAT_MS + timeouts.view;

type PlayerRef = { profileId: string; username: string };
type RequestBody = { requestId: string; from: PlayerRef; to: PlayerRef };
type FriendsBody = { friends: PlayerRef[]; incoming: RequestBody[]; outgoing: RequestBody[] };
type ChallengeBody = { challengeId: string; from: PlayerRef; to: PlayerRef; roomCode: string };
type ChallengesBody = { incoming: ChallengeBody[]; outgoing: ChallengeBody[] };
type MeBody = { profile: { id: string }; username?: { name: string } };

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy.request<MeBody>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).its("body");
}

function friendsOf(account: E2EAccount): Cypress.Chainable<FriendsBody> {
  return cy.request<FriendsBody>({ method: "GET", url: apiUrl("/api/friends"), headers: bearerHeaders(account.token) }).its("body");
}

function challengesOf(account: E2EAccount): Cypress.Chainable<ChallengesBody> {
  return cy.request<ChallengesBody>({ method: "GET", url: apiUrl("/api/challenges"), headers: bearerHeaders(account.token) }).its("body");
}

/** Leave nothing between the two accounts that an earlier run left: the E2E server keeps its state for its life. */
function separate(one: E2EAccount, twoId: string, two: E2EAccount, oneId: string): void {
  for (const [self, otherId] of [
    [one, twoId],
    [two, oneId],
  ] as const) {
    cy.request({ method: "DELETE", url: apiUrl(`/api/blocks/${otherId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
    friendsOf(self).then((body) => {
      if (body.friends.some((friend) => friend.profileId === otherId)) {
        cy.request({ method: "DELETE", url: apiUrl(`/api/friends/${otherId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
      for (const request of body.outgoing.filter((sent) => sent.to.profileId === otherId)) {
        cy.request({ method: "DELETE", url: apiUrl(`/api/friends/requests/${request.requestId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
      for (const request of body.incoming.filter((got) => got.from.profileId === otherId)) {
        cy.request({ method: "POST", url: apiUrl(`/api/friends/requests/${request.requestId}/decline`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
    });
    challengesOf(self).then((body) => {
      for (const challenge of body.outgoing) {
        cy.request({ method: "DELETE", url: apiUrl(`/api/challenges/${challenge.challengeId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
      for (const challenge of body.incoming) {
        cy.request({ method: "POST", url: apiUrl(`/api/challenges/${challenge.challengeId}/decline`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
    });
  }
}

/** The inbox's rows of one kind. */
function rows(kind: string): string {
  return `${ts(inboxTestid.item)}[data-kind="${kind}"]`;
}

function openInbox(): void {
  cy.get(ts(inboxTestid.bell), { timeout: SOCIAL_TIMEOUT_MS }).should("be.visible").click();
  cy.get(ts(inboxTestid.panel), { timeout: timeouts.view }).should("be.visible");
}

describe("56 — the inbox (issue #3729)", () => {
  const one = accounts.p1;
  const two = accounts.p2;
  let oneId = "";
  let oneName = "";
  let twoId = "";

  before(() => {
    cy.freeAccount(one());
    cy.freeAccount(two());
    me(one()).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two()).then((body) => {
      twoId = body.profile.id;
    });
  });

  beforeEach(() => {
    cy.then(() => {
      expect(oneName, "R1435: seat one has a username to be found by").to.not.eq("");
      separate(one(), twoId, two(), oneId);
    });
  });

  it("R3729002 a challenge is accepted from the inbox after its toast has been sent away", () => {
    let roomCode = "";
    let matchId = "";

    // Friends first: a challenge is for a friend (B48), and seat one's browser sends the heartbeat.
    cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(two().token), body: { username: oneName } })
      .its("status")
      .should("eq", 201);
    friendsOf(one()).then((body) => {
      const request = body.incoming.find((got) => got.from.profileId === twoId);
      expect(request, "the request reached seat one").to.not.eq(undefined);
      cy.request({ method: "POST", url: apiUrl(`/api/friends/requests/${request?.requestId ?? ""}/accept`), headers: bearerHeaders(one().token) })
        .its("status")
        .should("eq", 200);
    });
    cy.visitAs(one(), routes.collection());
    cy.get(ts(FRIENDS_BUTTON), { timeout: FRIENDS_TIMEOUT_MS }).should("be.visible");
    cy.request({ method: "POST", url: apiUrl("/api/presence"), headers: bearerHeaders(one().token) }).its("status").should("eq", 204);

    cy.then(() => {
      cy.request<{ challengeId: string; roomCode: string }>({
        method: "POST",
        url: apiUrl("/api/challenges"),
        headers: bearerHeaders(two().token),
        body: { mode: "random", friendId: oneId },
      }).then((created) => {
        expect(created.status, "B48: a challenge to an online, idle friend").to.eq(201);
        roomCode = created.body.roomCode;
      });
    });

    // The toast pops on Collection; Later sends it away, and the challenge is still waiting in the inbox.
    cy.then(() => {
      cy.get(`${ts(CHALLENGE_TOAST)}[data-room-code="${roomCode}"]`, { timeout: SOCIAL_TIMEOUT_MS }).within(() => {
        cy.get(ts(CHALLENGE_LATER)).click();
      });
      cy.get(`${ts(CHALLENGE_TOAST)}[data-room-code="${roomCode}"]`).should("not.exist");
    });
    cy.location("pathname").should("eq", routes.collection());
    openInbox();
    cy.get(rows("challenge"), { timeout: timeouts.view }).should("have.length", 1).within(() => {
      cy.get(ts(inboxTestid.left)).should("be.visible");
      cy.get(ts(inboxTestid.accept)).should("not.be.disabled").click();
    });
    cy.location("pathname", { timeout: timeouts.view })
      .should("match", MATCH_PATH)
      .then((pathname) => {
        matchId = MATCH_PATH.exec(pathname)?.[1] ?? "";
        expect(matchId, "the match the accepted challenge made").to.not.eq("");
      });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      cy.concedeAs(one(), matchId);
    });
  });

  for (const size of MENU_SIZES) {
    it(`R3729002 a friend request is accepted from the inbox at ${size.label}`, () => {
      cy.viewport(size.width, size.height);
      cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(two().token), body: { username: oneName } })
        .its("status")
        .should("eq", 201);
      cy.visitAs(one(), routes.collection());
      openInbox();
      cy.get(rows("request"), { timeout: SOCIAL_TIMEOUT_MS }).should("have.length", 1);
      cy.screenshot(`3729-inbox-${size.label}`, { capture: "viewport", overwrite: true });
      cy.get(rows("request")).within(() => {
        cy.get(ts(inboxTestid.accept)).click();
      });
      cy.get(rows("request"), { timeout: timeouts.view }).should("not.exist");
      friendsOf(one())
        .its("friends")
        .should((friends: PlayerRef[]) => {
          expect(friends.map((friend) => friend.profileId), "the accepted request made seat two a friend").to.include(twoId);
        });
    });
  }
});
