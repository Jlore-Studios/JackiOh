// 41 — Play together (#936, SPEC S7, S8): two accounts become friends, chat, and challenge each other,
// the browser playing seat one and `cy.request` playing seat two, as spec 06 splits them.
//
// Key assertions (B54–B58): the landing offers Play together, which opens behind the gate for an active
// account; the add form takes a `name#tag` and shows the server's refusal in its words; a request from
// the other side can be declined from the page, and one sent from the page, once accepted, makes a
// friend row; Chat opens the conversation, a message sent is shown, and the friend's reply arrives
// within a poll; a friend's challenge is a toast whose Accept opens `/play` with the room filled in,
// and joining it reaches the game; a challenge sent from a friend row takes the challenger to the game
// once the friend joins its room.
//
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` beside the client. Daily only, as
// every spec outside scripts/shard-specs.mjs's SMOKE. No wait depends on a server clock: every wait is a
// retried DOM or HTTP assertion bounded by a poll (`CHAT_POLL_MS`, the social screens' cadence).

import { CHAT_POLL_MS, MATCH_FOUND_NAV_DELAY_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, routes, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  CHALLENGE_ACCEPT,
  CHALLENGE_TOAST,
  CHAT_CLOSE,
  CHAT_INPUT,
  CHAT_MESSAGE,
  CHAT_PANEL,
  CHAT_SEND,
  IN_MATCH_CHAT_TOGGLE,
  LANDING_PLAY_TOGETHER,
  PLAY_JOIN_INPUT,
  PLAY_JOIN_SUBMIT,
  TOGETHER_ADD_ERROR,
  TOGETHER_ADD_INPUT,
  TOGETHER_ADD_SENT,
  TOGETHER_ADD_SUBMIT,
  TOGETHER_CHALLENGE,
  TOGETHER_CHALLENGE_FORM,
  TOGETHER_CHALLENGE_SEND,
  TOGETHER_CHAT_OPEN,
  TOGETHER_FRIEND_ROW,
  TOGETHER_OUTGOING_CHALLENGE,
  TOGETHER_REQUEST_DECLINE,
  TOGETHER_REQUEST_ROW,
  heroId,
  playModeId,
  togetherChallengeModeId,
  ts,
  type QueueMode,
} from "../../support/testids.ts";

// Constants

/** Play together's route, mirroring `paths.together` in apps/web/src/net/navigate.ts. */
const TOGETHER = "/together";

/** A social read lands within one poll and a render. */
const POLL_TIMEOUT_MS = CHAT_POLL_MS + timeouts.view;

/** The challenger's follow: a poll to see the join, the "Match found!" beat, then the board's path. */
const FOLLOW_TIMEOUT_MS = POLL_TIMEOUT_MS + MATCH_FOUND_NAV_DELAY_MS + timeouts.view;

/** A match page, capturing its id. */
const MATCH_PATH = /^\/match\/([^/]+)$/;

/** A name no fixture account holds, in the `name#tag` shape the add form takes. */
const UNKNOWN_NAME = "nosuchplayer#9999";

/** What seat one says, and seat two's answer: words the username filter leaves alone. */
const HELLO = "Good luck and have fun";
const REPLY = "You too, see you at the table";

/** No deck to bring: the server deals both seats one (R257), so the challenge needs no loadout. */
const MODE: QueueMode = "random";

// HTTP contract (SPEC S7), as far as this spec reads it

type PlayerRef = { profileId: string; username: string };
type RequestBody = { requestId: string; from: PlayerRef; to: PlayerRef };
type FriendsBody = {
  friends: (PlayerRef & { status: string })[];
  incoming: RequestBody[];
  outgoing: RequestBody[];
  blocked: PlayerRef[];
};
type ChallengeBody = { challengeId: string; from: PlayerRef; to: PlayerRef; roomCode: string; mode: QueueMode };
type ChallengesBody = { incoming: ChallengeBody[]; outgoing: ChallengeBody[] };
type ChatBody = { messages: { id: string; from: string; body: string }[] };
type MeBody = { profile: { id: string }; username?: { name: string } };
type JoinBody = { matchId: string | null; code: string; mode: QueueMode };

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy.request<MeBody>({ method: "GET", url: api("/api/auth/me"), headers: bearer(account) }).its("body");
}

function friendsOf(account: E2EAccount): Cypress.Chainable<FriendsBody> {
  return cy.request<FriendsBody>({ method: "GET", url: api("/api/friends"), headers: bearer(account) }).its("body");
}

function challengesOf(account: E2EAccount): Cypress.Chainable<ChallengesBody> {
  return cy
    .request<ChallengesBody>({ method: "GET", url: api("/api/challenges"), headers: bearer(account) })
    .its("body");
}

/** A friend row of the page, by the profile it shows. */
function friendRow(profileId: string, status?: string): string {
  const row = `${ts(TOGETHER_FRIEND_ROW)}[data-profile-id="${profileId}"]`;
  return status === undefined ? row : `${row}[data-status="${status}"]`;
}

function landedOn(pattern: RegExp, timeout: number): Cypress.Chainable<string> {
  return cy
    .location("pathname", { timeout })
    .should("match", pattern)
    .then((pathname) => pattern.exec(pathname)?.[1] ?? "");
}

/**
 * Leave nothing between the two accounts that an earlier run (or a failed one) left: no block either
 * way, no friendship, no request and no open challenge. The E2E server keeps its state for its life.
 */
function separate(pairs: readonly [E2EAccount, string][]): void {
  for (const [self, otherId] of pairs) {
    cy.request({ method: "DELETE", url: api(`/api/blocks/${otherId}`), headers: bearer(self), failOnStatusCode: false });
    friendsOf(self).then((body) => {
      if (body.friends.some((friend) => friend.profileId === otherId)) {
        cy.request({ method: "DELETE", url: api(`/api/friends/${otherId}`), headers: bearer(self), failOnStatusCode: false });
      }
      for (const request of body.outgoing.filter((sent) => sent.to.profileId === otherId)) {
        cy.request({
          method: "DELETE",
          url: api(`/api/friends/requests/${request.requestId}`),
          headers: bearer(self),
          failOnStatusCode: false,
        });
      }
      for (const request of body.incoming.filter((got) => got.from.profileId === otherId)) {
        cy.request({
          method: "POST",
          url: api(`/api/friends/requests/${request.requestId}/decline`),
          headers: bearer(self),
          failOnStatusCode: false,
        });
      }
    });
    challengesOf(self).then((body) => {
      for (const challenge of body.outgoing) {
        cy.request({
          method: "DELETE",
          url: api(`/api/challenges/${challenge.challengeId}`),
          headers: bearer(self),
          failOnStatusCode: false,
        });
      }
      for (const challenge of body.incoming) {
        cy.request({
          method: "POST",
          url: api(`/api/challenges/${challenge.challengeId}/decline`),
          headers: bearer(self),
          failOnStatusCode: false,
        });
      }
    });
  }
}

describe("41 Play together — friends, chat and challenges between two accounts", () => {
  it("adds a friend by name, chats both ways, and plays a challenge each way", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let twoName = "";
    let roomCode = "";
    let matchId = "";

    cy.freeAccount(one);
    cy.freeAccount(two);
    me(one).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two).then((body) => {
      twoId = body.profile.id;
      twoName = body.username?.name ?? "";
    });
    cy.then(() => {
      expect(oneName, "R1435: seat one has a username to be found by").to.not.eq("");
      expect(twoName, "R1435: seat two has a username to be found by").to.not.eq("");
      separate([
        [one, twoId],
        [two, oneId],
      ]);
    });

    // B54: the landing offers Play together, which opens for an active account behind the gate.
    cy.visitAs(one, "/");
    cy.get(ts(LANDING_PLAY_TOGETHER), { timeout: timeouts.view }).click();
    cy.location("pathname", { timeout: timeouts.view }).should("eq", TOGETHER);
    cy.get(ts(TOGETHER_ADD_INPUT), { timeout: timeouts.view }).should("be.visible");

    // B55: a name nobody holds is refused, in the server's words.
    cy.get(ts(TOGETHER_ADD_INPUT)).type(UNKNOWN_NAME);
    cy.get(ts(TOGETHER_ADD_SUBMIT)).click();
    cy.get(ts(TOGETHER_ADD_ERROR), { timeout: timeouts.view }).should("be.visible").and("not.be.empty");

    // B55: a request from seat two shows as incoming within a poll, and Decline ends it.
    cy.then(() => {
      cy.request({ method: "POST", url: api("/api/friends/requests"), headers: bearer(two), body: { username: oneName } })
        .its("status")
        .should("eq", 201);
      cy.get(`${ts(TOGETHER_REQUEST_ROW)}[data-direction="incoming"][data-profile-id="${twoId}"]`, { timeout: POLL_TIMEOUT_MS }).within(
        () => {
          cy.get(ts(TOGETHER_REQUEST_DECLINE)).click();
        },
      );
      cy.get(`${ts(TOGETHER_REQUEST_ROW)}[data-profile-id="${twoId}"]`, { timeout: timeouts.view }).should("not.exist");
      friendsOf(two).its("outgoing").should("have.length", 0);
    });

    // B55: seat one adds seat two by name#tag; seat two accepts (B45), and the friend row appears.
    cy.then(() => {
      cy.get(ts(TOGETHER_ADD_INPUT)).clear().type(twoName);
    });
    cy.get(ts(TOGETHER_ADD_SUBMIT)).click();
    cy.get(ts(TOGETHER_ADD_SENT), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      cy.get(`${ts(TOGETHER_REQUEST_ROW)}[data-direction="outgoing"][data-profile-id="${twoId}"]`, {
        timeout: timeouts.view,
      }).should("exist");
      friendsOf(two).then((body) => {
        const request = body.incoming.find((got) => got.from.profileId === oneId);
        expect(request, "B43: seat one's request reached seat two").to.not.eq(undefined);
        cy.request({
          method: "POST",
          url: api(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
          headers: bearer(two),
        })
          .its("status")
          .should("eq", 200);
      });
      cy.get(friendRow(twoId), { timeout: POLL_TIMEOUT_MS }).should("exist");
    });

    // B58: Chat opens the conversation; a message sent shows at once, and the reply within a poll.
    cy.then(() => {
      cy.get(friendRow(twoId)).within(() => {
        cy.get(ts(TOGETHER_CHAT_OPEN)).click();
      });
    });
    cy.get(ts(CHAT_PANEL), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(CHAT_INPUT)).type(HELLO);
    cy.get(ts(CHAT_SEND)).click();
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="you"]`, HELLO, { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      // B53: seat two reads it, being a friend, and answers.
      cy.request<ChatBody>({ method: "GET", url: api(`/api/chat/${oneId}`), headers: bearer(two) })
        .its("body.messages")
        .should((messages: ChatBody["messages"]) => {
          expect(messages.map((message) => message.body)).to.include(HELLO);
        });
      cy.request({ method: "POST", url: api(`/api/chat/${oneId}`), headers: bearer(two), body: { body: REPLY } })
        .its("status")
        .should("eq", 201);
    });
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="friend"]`, REPLY, { timeout: POLL_TIMEOUT_MS }).should("be.visible");
    cy.get(ts(CHAT_CLOSE)).click();
    cy.get(ts(CHAT_PANEL)).should("not.exist");

    // B57: seat two challenges seat one (online: its browser sends the heartbeat); the toast's Accept
    // opens /play with the room's code and mode filled in, and Join reaches the game.
    cy.then(() => {
      cy.request<{ challengeId: string; roomCode: string }>({
        method: "POST",
        url: api("/api/challenges"),
        headers: bearer(two),
        body: { mode: MODE, friendId: oneId },
      }).then((created) => {
        expect(created.status, "B48: a challenge to an online, idle friend").to.eq(201);
        roomCode = created.body.roomCode;
      });
    });
    cy.then(() => {
      cy.get(`${ts(CHALLENGE_TOAST)}[data-room-code="${roomCode}"]`, { timeout: POLL_TIMEOUT_MS }).within(() => {
        cy.get(ts(CHALLENGE_ACCEPT)).click();
      });
    });
    cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.play());
    cy.then(() => {
      cy.get(ts(PLAY_JOIN_INPUT), { timeout: timeouts.view }).should("have.value", roomCode);
    });
    cy.get(ts(playModeId(MODE))).should("be.checked");
    cy.get(ts(PLAY_JOIN_SUBMIT)).should("not.be.disabled").click();
    landedOn(MATCH_PATH, timeouts.view).then((id) => {
      matchId = id;
      expect(matchId, "the match the challenge's room made").to.not.eq("");
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    // B58: the board rail carries the chat toggle.
    cy.get(ts(IN_MATCH_CHAT_TOGGLE)).should("be.visible");
    // B49: joining the room accepted the challenge, so it is open for neither seat any more.
    cy.then(() => {
      challengesOf(two)
        .its("outgoing")
        .should((outgoing: ChallengeBody[]) => {
          expect(outgoing.map((challenge) => challenge.roomCode)).to.not.include(roomCode);
        });
      cy.concedeAs(one, matchId);
    });

    // B48, B56: seat one challenges seat two from its friend row; seat two joins the room, and the
    // shell takes seat one to the game (ChallengeToast's follow).
    cy.request({ method: "POST", url: api("/api/presence"), headers: bearer(two) }).its("status").should("eq", 204);
    cy.visitAs(one, TOGETHER);
    cy.then(() => {
      cy.get(friendRow(twoId, "online"), { timeout: POLL_TIMEOUT_MS }).within(() => {
        cy.get(ts(TOGETHER_CHALLENGE)).click();
      });
    });
    cy.get(ts(TOGETHER_CHALLENGE_FORM), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(togetherChallengeModeId(MODE))).check();
    cy.get(ts(TOGETHER_CHALLENGE_SEND)).should("not.be.disabled").click();
    cy.get(ts(TOGETHER_OUTGOING_CHALLENGE), { timeout: timeouts.view }).should("exist");
    cy.then(() => {
      challengesOf(two).then((body) => {
        const challenge = body.incoming.find((got) => got.from.profileId === oneId);
        expect(challenge, "B48: seat one's challenge reached seat two").to.not.eq(undefined);
        expect(challenge?.mode, "the challenge plays the mode it was sent with").to.eq(MODE);
        cy.request<JoinBody>({
          method: "POST",
          url: api(`/api/rooms/${challenge?.roomCode ?? ""}/join`),
          headers: bearer(two),
          body: { mode: MODE },
        }).then((joined) => {
          matchId = joined.body.matchId ?? "";
          expect(matchId, "B49: accepting is joining the room").to.not.eq("");
        });
      });
    });
    cy.then(() => {
      cy.location("pathname", { timeout: FOLLOW_TIMEOUT_MS }).should("eq", routes.match(matchId));
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");

    // Free both accounts, and part them, for the next run and the next spec.
    cy.then(() => {
      cy.concedeAs(two, matchId);
      separate([
        [one, twoId],
        [two, oneId],
      ]);
    });
  });
});
