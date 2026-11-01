// 42 — Play together (#936, SPEC S7, S8): two accounts become friends, chat, and challenge each other,
// the browser playing seat one and `cy.request` playing seat two, as spec 06 splits them (B54–B58).
//
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` beside the client. Daily only, as
// every spec outside scripts/shard-specs.mjs's SMOKE. No wait depends on a server clock: every wait is a
// retried DOM or HTTP assertion bounded by the poll that brings it.

import { EMOTE_BUBBLE_MAX_MS, EMOTE_BUBBLE_MIN_MS, EMOTE_EMOJI_MS } from "../../../apps/web/src/emotes/config.ts";
import { EMOTE_COOLDOWN_MS } from "../../../apps/web/src/wire/emotes.ts";
import {
  CHAT_POLL_MS,
  MATCH_FOUND_NAV_DELAY_MS,
  PRESENCE_HEARTBEAT_MS,
  SOCIAL_POLL_MS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, routes, seedFor, server, timeouts, type E2EAccount } from "../../support/config.ts";
import { onStep } from "../../support/replays.ts";
import {
  CHALLENGE_ACCEPT,
  CHALLENGE_TOAST,
  CHAT_CLOSE,
  CHAT_INPUT,
  CHAT_MESSAGE,
  CHAT_PANEL,
  CHAT_SEND,
  EMOTE_BUBBLE,
  EMOTE_MUTE,
  EMOTE_MUTE_MENU,
  FRIENDS_BADGE,
  FRIENDS_BUTTON,
  FRIENDS_PANEL,
  MATCH_WATCH_REPLAY,
  OPPONENT_AIM,
  OPPONENT_AIM_ARROW,
  REPLAYS_SHARED_WITH_ME,
  SHARE_FRIEND_OPEN,
  SHARE_FRIEND_OPTION,
  SHARE_FRIEND_UNSHARE,
  TOGETHER_ADD_ERROR,
  TOGETHER_ADD_INPUT,
  TOGETHER_ADD_SENT,
  TOGETHER_ADD_SUBMIT,
  TOGETHER_CHALLENGE,
  TOGETHER_CHALLENGE_FORM,
  TOGETHER_CHALLENGE_SEND,
  TOGETHER_CHAT_OPEN,
  TOGETHER_FRIEND_ROW,
  TOGETHER_REQUEST_DECLINE,
  TOGETHER_REQUEST_ROW,
  TOGETHER_SHARED_OPEN,
  TOGETHER_SHARED_ROW,
  heroId,
  togetherChallengeModeId,
  ts,
  type QueueMode,
} from "../../support/testids.ts";
import type { ActionInput, PlayerId } from "../../support/types.ts";

/** Play together's route, mirroring `paths.together` in apps/web/src/net/navigate.ts. */
const TOGETHER = "/together";

/** A change to the friends list (a request, a friend, a status) lands within its poll and a render. */
const FRIENDS_TIMEOUT_MS = PRESENCE_HEARTBEAT_MS + timeouts.view;

/** A challenge lands within the challenges' poll and a render. */
const CHALLENGE_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

/** An open conversation's reply lands within the chat's poll and a render. */
const CHAT_TIMEOUT_MS = CHAT_POLL_MS + timeouts.view;

/** The challenger's follow: a poll to see the join, the "Match found!" beat, then the board's path. */
const FOLLOW_TIMEOUT_MS = CHALLENGE_TIMEOUT_MS + MATCH_FOUND_NAV_DELAY_MS + timeouts.view;

const MATCH_PATH = /^\/match\/([^/]+)$/;

/** A name no fixture account holds, in the `name#tag` shape the add form takes. */
const UNKNOWN_NAME = "nosuchplayer#9999";

/** What seat one says, and seat two's answer: words the username filter leaves alone. */
const HELLO = "Good luck and have fun";
const REPLY = "You too, see you at the table";

/** No deck to bring: the server deals both seats one (R257), so the challenge needs no loadout. */
const MODE: QueueMode = "random";

const SEAT_TWO = "seat-two";

/** The room's host is seat p1 (§9.5), the browser; the joiner is p2, `wsPlayer`. */
const ONE_SEAT: PlayerId = "p1";
const TWO_SEAT: PlayerId = "p2";

/** The seed fixes the deal (R143), so a retry plays the same match. */
const SEED = seedFor("42-together-live");

/** The match socket's path, mirroring `WS_PATH`: the frames recorded are the ones it carries. */
const MATCH_SOCKET_PATH = "/ws/match";

/** The unread badge lands within the unread poll and a render. */
const UNREAD_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

/** A bubble is gone within its longest hold and a render. */
const BUBBLE_GONE_TIMEOUT_MS = EMOTE_BUBBLE_MAX_MS + timeouts.view;

/** The opponent's portrait, whose click opens the mute menu (`emotes-layout.cy.tsx`'s `openMenu`). */
const OPPONENT_PORTRAIT = `${ts(heroId("opponent"))} .hero-portrait`;

/** What is said from the board: words the chat's filter leaves alone, tagged with the run when sent. */
const UNREAD = "A message for you while you play";
const MID_HELLO = "Nice opening, good game so far";
const MID_REPLY = "Thanks, it is close";

/** Seat two's aims (R738): over nothing, and over seat one's hero, both from its first hand card. */
const HAND_SOURCE = { at: "hand", player: TWO_SEAT, index: 0 };
const AIM_HELD = { source: HAND_SOURCE, target: null };
const AIM_AT_HERO = { source: HAND_SOURCE, target: { at: "hero", player: ONE_SEAT } };

/** The only fields a relayed aim end may carry, by its kind, sorted and comma-joined (R738, R97, R177). */
const END_FIELDS: Record<string, string> = { hero: "at,player", zone: "at,lane,player,row", hand: "at,index,player" };

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
type SharedBody = { shared: { matchId: string; sharer: PlayerRef }[] };
type RoomBody = { code: string };

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy.request<MeBody>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).its("body");
}

function friendsOf(account: E2EAccount): Cypress.Chainable<FriendsBody> {
  return cy.request<FriendsBody>({ method: "GET", url: apiUrl("/api/friends"), headers: bearerHeaders(account.token) }).its("body");
}

function challengesOf(account: E2EAccount): Cypress.Chainable<ChallengesBody> {
  return cy
    .request<ChallengesBody>({ method: "GET", url: apiUrl("/api/challenges"), headers: bearerHeaders(account.token) })
    .its("body");
}

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
    cy.request({ method: "DELETE", url: apiUrl(`/api/blocks/${otherId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
    friendsOf(self).then((body) => {
      if (body.friends.some((friend) => friend.profileId === otherId)) {
        cy.request({ method: "DELETE", url: apiUrl(`/api/friends/${otherId}`), headers: bearerHeaders(self.token), failOnStatusCode: false });
      }
      for (const request of body.outgoing.filter((sent) => sent.to.profileId === otherId)) {
        cy.request({
          method: "DELETE",
          url: apiUrl(`/api/friends/requests/${request.requestId}`),
          headers: bearerHeaders(self.token),
          failOnStatusCode: false,
        });
      }
      for (const request of body.incoming.filter((got) => got.from.profileId === otherId)) {
        cy.request({
          method: "POST",
          url: apiUrl(`/api/friends/requests/${request.requestId}/decline`),
          headers: bearerHeaders(self.token),
          failOnStatusCode: false,
        });
      }
    });
    challengesOf(self).then((body) => {
      for (const challenge of body.outgoing) {
        cy.request({
          method: "DELETE",
          url: apiUrl(`/api/challenges/${challenge.challengeId}`),
          headers: bearerHeaders(self.token),
          failOnStatusCode: false,
        });
      }
      for (const challenge of body.incoming) {
        cy.request({
          method: "POST",
          url: apiUrl(`/api/challenges/${challenge.challengeId}/decline`),
          headers: bearerHeaders(self.token),
          failOnStatusCode: false,
        });
      }
    });
  }
}

/** Make `asked` a friend of `asker`: the request by name, then the accept (B43, B45). */
function befriend(asker: E2EAccount, asked: E2EAccount, askedName: string, askerId: string): void {
  cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(asker.token), body: { username: askedName } })
    .its("status")
    .should("eq", 201);
  friendsOf(asked).then((body) => {
    const request = body.incoming.find((got) => got.from.profileId === askerId);
    expect(request, "B43: the request reached the friend").to.not.eq(undefined);
    cy.request({
      method: "POST",
      url: apiUrl(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
      headers: bearerHeaders(asked.token),
    })
      .its("status")
      .should("eq", 200);
  });
}

/** Seat two keeps its whole opening hand (R9), as spec 39's seat two does. */
function seatTwoKeepsHand(): void {
  cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { promptKind: "mulligan" } }).then((result) => {
    const view = (result.view ?? {}) as { you?: { hand?: { instanceId: string }[] } };
    const hand = Array.isArray(view.you?.hand) ? view.you.hand : [];
    const body: ActionInput = { type: "mulligan", keep: hand.map((card) => card.instanceId), playerId: TWO_SEAT };
    cy.wsPlayer({ action: "send", name: SEAT_TWO, body });
  });
}

// The frames the board's match socket received

type Frame = { type: string; [key: string]: unknown };
type RecordingWindow = Cypress.AUTWindow & { __matchFrames?: Frame[]; __bubblesDrawn?: number };

/**
 * `onBeforeLoad`: every frame the page's `/ws/match` socket receives, as the server wrote it, and how
 * many emote bubbles the board has drawn. A bubble is gone within seconds, so an assertion that none
 * is on screen would only wait it out: the count is what proves one was never drawn.
 */
function recordMatchFrames(win: Cypress.AUTWindow): void {
  const frames: Frame[] = [];
  (win as RecordingWindow).__matchFrames = frames;
  (win as RecordingWindow).__bubblesDrawn = 0;
  new win.MutationObserver((records) => {
    for (const record of records) {
      for (const node of Array.from(record.addedNodes)) {
        if (!(node instanceof win.Element)) continue;
        const drawn = (node.matches(ts(EMOTE_BUBBLE)) ? 1 : 0) + node.querySelectorAll(ts(EMOTE_BUBBLE)).length;
        (win as RecordingWindow).__bubblesDrawn = ((win as RecordingWindow).__bubblesDrawn ?? 0) + drawn;
      }
    }
  }).observe(win.document, { childList: true, subtree: true });
  const RealSocket = win.WebSocket;
  win.WebSocket = new Proxy(RealSocket, {
    construct(target, args: unknown[]) {
      const socket = Reflect.construct(target, args) as WebSocket;
      if (String(args[0]).includes(MATCH_SOCKET_PATH)) {
        socket.addEventListener("message", (event: MessageEvent) => {
          if (typeof event.data !== "string") return;
          let parsed: unknown;
          try {
            parsed = JSON.parse(event.data);
          } catch {
            return;
          }
          if (typeof parsed === "object" && parsed !== null && typeof (parsed as Frame).type === "string") {
            frames.push(parsed as Frame);
          }
        });
      }
      return socket;
    },
  });
}

function matchFramesOf(win: Cypress.AUTWindow): Frame[] {
  return (win as RecordingWindow).__matchFrames ?? [];
}

function bubblesDrawnOf(win: Cypress.AUTWindow): number {
  return (win as RecordingWindow).__bubblesDrawn ?? 0;
}

function aimFramesOf(win: Cypress.AUTWindow): Frame[] {
  return matchFramesOf(win).filter((frame) => frame.type === "aim");
}

/** R738, R97, R177: an end of its kind's own fields alone, and never a position in the receiver's hand. */
function isPublicEnd(end: unknown): boolean {
  if (typeof end !== "object" || end === null) return false;
  const fields = end as Record<string, unknown>;
  const kind = typeof fields.at === "string" ? fields.at : "";
  if (Object.keys(fields).sort().join(",") !== END_FIELDS[kind]) return false;
  return !(kind === "hand" && fields.player === ONE_SEAT);
}

/** R738: every aim the board was sent names public ends only. */
function expectEveryAimPublic(win: Cypress.AUTWindow): void {
  for (const frame of aimFramesOf(win)) {
    const aim = frame.aim as { source?: unknown; target?: unknown } | null;
    if (aim === null) continue;
    expect(isPublicEnd(aim.source), `R738: the aim's source is public: ${JSON.stringify(frame)}`).to.eq(true);
    expect(aim.target === null || isPublicEnd(aim.target), `R738: the aim's target is public: ${JSON.stringify(frame)}`).to.eq(true);
  }
}

describe("42 Play together — friends, chat and challenges between two accounts", () => {
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

    // B54: Play together opens for an active account behind the gate (#1577: visited directly).
    cy.visitAs(one, TOGETHER);
    cy.location("pathname", { timeout: timeouts.view }).should("eq", TOGETHER);
    cy.get(ts(TOGETHER_ADD_INPUT), { timeout: timeouts.view }).should("be.visible");

    // B55: a name nobody holds is refused, in the server's words.
    cy.get(ts(TOGETHER_ADD_INPUT)).type(UNKNOWN_NAME);
    cy.get(ts(TOGETHER_ADD_SUBMIT)).click();
    cy.get(ts(TOGETHER_ADD_ERROR), { timeout: timeouts.view }).should("be.visible").and("not.be.empty");

    // B55: a request from seat two shows as incoming within a poll, and Decline ends it.
    cy.then(() => {
      cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(two.token), body: { username: oneName } })
        .its("status")
        .should("eq", 201);
      cy.get(`${ts(TOGETHER_REQUEST_ROW)}[data-direction="incoming"][data-profile-id="${twoId}"]`, { timeout: FRIENDS_TIMEOUT_MS }).within(
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
          url: apiUrl(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
          headers: bearerHeaders(two.token),
        })
          .its("status")
          .should("eq", 200);
      });
      cy.get(friendRow(twoId), { timeout: FRIENDS_TIMEOUT_MS }).should("exist");
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
      cy.request<ChatBody>({ method: "GET", url: apiUrl(`/api/chat/${oneId}`), headers: bearerHeaders(two.token) })
        .its("body.messages")
        .should((messages: ChatBody["messages"]) => {
          expect(messages.map((message) => message.body)).to.include(HELLO);
        });
      cy.request({ method: "POST", url: apiUrl(`/api/chat/${oneId}`), headers: bearerHeaders(two.token), body: { body: REPLY } })
        .its("status")
        .should("eq", 201);
    });
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="friend"]`, REPLY, { timeout: CHAT_TIMEOUT_MS }).should("be.visible");
    cy.get(ts(CHAT_CLOSE)).click();
    cy.get(ts(CHAT_PANEL)).should("not.exist");

    // B57: seat two challenges seat one (online: its browser sends the heartbeat); the toast's Accept
    // joins the room with the remembered deck and reaches the game (R1831000).
    cy.then(() => {
      cy.request<{ challengeId: string; roomCode: string }>({
        method: "POST",
        url: apiUrl("/api/challenges"),
        headers: bearerHeaders(two.token),
        body: { mode: MODE, friendId: oneId },
      }).then((created) => {
        expect(created.status, "B48: a challenge to an online, idle friend").to.eq(201);
        roomCode = created.body.roomCode;
      });
    });
    cy.then(() => {
      cy.get(`${ts(CHALLENGE_TOAST)}[data-room-code="${roomCode}"]`, { timeout: CHALLENGE_TIMEOUT_MS }).within(() => {
        cy.get(ts(CHALLENGE_ACCEPT)).click();
      });
    });
    landedOn(MATCH_PATH, timeouts.view).then((id) => {
      matchId = id;
      expect(matchId, "the match the challenge's room made").to.not.eq("");
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    // B58, #1831: the board's controls carry the Friends button.
    cy.get(ts(FRIENDS_BUTTON)).should("be.visible");
    // B49: joining the room accepted the challenge, so it is open for neither seat any more.
    cy.then(() => {
      challengesOf(two)
        .its("outgoing")
        .should((outgoing: ChallengeBody[]) => {
          expect(outgoing.map((challenge) => challenge.roomCode)).to.not.include(roomCode);
        });
      cy.concedeAs(one, matchId);
    });

    // B48, B56, #1831: seat one challenges seat two from its Friends panel, on a screen that is not
    // Play together; seat two joins the room, and the shell takes seat one to the game
    // (ChallengeToast's follow).
    cy.request({ method: "POST", url: apiUrl("/api/presence"), headers: bearerHeaders(two.token) }).its("status").should("eq", 204);
    cy.visitAs(one, routes.replays());
    cy.get(ts(FRIENDS_BUTTON), { timeout: timeouts.view }).click();
    cy.then(() => {
      cy.get(`${ts(FRIENDS_PANEL)} ${friendRow(twoId, "online")}`, { timeout: FRIENDS_TIMEOUT_MS }).within(() => {
        cy.get(ts(TOGETHER_CHALLENGE)).click();
      });
    });
    cy.get(ts(TOGETHER_CHALLENGE_FORM), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(togetherChallengeModeId(MODE))).check();
    cy.get(ts(TOGETHER_CHALLENGE_SEND)).should("not.be.disabled").click();
    cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.replays());
    cy.then(() => {
      challengesOf(two).then((body) => {
        const challenge = body.incoming.find((got) => got.from.profileId === oneId);
        expect(challenge, "B48: seat one's challenge reached seat two").to.not.eq(undefined);
        expect(challenge?.mode, "the challenge plays the mode it was sent with").to.eq(MODE);
        cy.request<JoinBody>({
          method: "POST",
          url: apiUrl(`/api/rooms/${challenge?.roomCode ?? ""}/join`),
          headers: bearerHeaders(two.token),
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

  it("B52, B58, B59, R738, R643: a live match between friends relays an aim and an emote, mutes one, chats from the board both ways, and shares its replay", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    // Digits only, so the chat's word filter leaves it alone, and this run's messages tell from older ones.
    const run = String(Date.now());
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let roomCode = "";
    let matchId = "";
    let hiddenId = "";
    let firstEmote = "";
    let secondEmote = "";
    let bubblesBeforeMute = 0;

    cy.freeAccount(one);
    cy.freeAccount(two);
    me(one).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two).then((body) => {
      twoId = body.profile.id;
    });
    cy.then(() => {
      expect(oneName, "R1435: seat one has a username to be found by").to.not.eq("");
      separate([
        [one, twoId],
        [two, oneId],
      ]);
      befriend(two, one, oneName, twoId);
    });
    // R1989000 pins the host to seat one beside the seed, so the spec keeps the seats it was
    // written for.
    cy.then(() => {
      cy.request<RoomBody>({
        method: "POST",
        url: apiUrl("/api/rooms"),
        headers: bearerHeaders(one.token),
        body: { mode: MODE, seed: SEED, seat: ONE_SEAT },
      }).then((created) => {
        roomCode = created.body.code;
        cy.request<JoinBody>({
          method: "POST",
          url: apiUrl(`/api/rooms/${roomCode}/join`),
          headers: bearerHeaders(two.token),
          body: { mode: MODE },
        }).then((joined) => {
          matchId = joined.body.matchId ?? "";
          expect(matchId, "the match the room made").to.not.eq("");
        });
      });
    });
    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: two.token, matchId }).then((result) => {
        expect((result.view as { viewer?: string } | null | undefined)?.viewer, "the joiner holds seat two (§9.5)").to.eq(TWO_SEAT);
      });
      cy.visitAs(one, routes.match(matchId), { onBeforeLoad: recordMatchFrames });
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");

    // Both keep their hands; seat two reads the id of a card it holds, which is what its aim will smuggle.
    cy.keepMulligans();
    seatTwoKeepsHand();
    cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { turnAtLeast: 1 } });
    cy.wsPlayer({ action: "view", name: SEAT_TWO }).then((result) => {
      const view = (result.view ?? {}) as { you?: { hand?: { instanceId: string }[] } };
      hiddenId = view.you?.hand?.[0]?.instanceId ?? "";
      expect(hiddenId, "seat two holds a card to hide").to.not.eq("");
    });

    // R738: seat two aims from its first hand card over nothing, and the board marks the source alone.
    cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "aim", aim: AIM_HELD } });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view })
      .should("have.attr", "data-aim", "hand:p2:0>none")
      .and("have.attr", "data-targeting", "false");
    cy.get(ts(OPPONENT_AIM_ARROW)).should("not.exist");

    // R738: over seat one's hero, the arrow is drawn.
    cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "aim", aim: AIM_AT_HERO } });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view })
      .should("have.attr", "data-aim", "hand:p2:0>hero:p1")
      .and("have.attr", "data-targeting", "true");
    cy.get(ts(OPPONENT_AIM_ARROW)).should("exist");

    // R738, R97, R177: a face-down card is aimed at by its zone. The instance id and index seat two
    // smuggles onto the ends never reach the board: the relay names the zone alone.
    cy.then(() => {
      const zone = { at: "zone", row: "backrow", lane: 1, instanceId: hiddenId, index: 0 };
      cy.wsPlayer({
        action: "sendFrame",
        name: SEAT_TWO,
        frame: { type: "aim", aim: { source: { ...zone, player: TWO_SEAT }, target: { ...zone, player: ONE_SEAT } } },
      });
    });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view }).should("have.attr", "data-aim", "zone:p2:backrow:1>zone:p1:backrow:1");
    cy.get(ts(OPPONENT_AIM_ARROW)).should("exist");
    cy.window().should((win) => {
      expect(aimFramesOf(win).at(-1), "R738: the relayed aim carries public fields only").to.deep.eq({
        type: "aim",
        from: TWO_SEAT,
        aim: {
          source: { at: "zone", player: TWO_SEAT, row: "backrow", lane: 1 },
          target: { at: "zone", player: ONE_SEAT, row: "backrow", lane: 1 },
        },
      });
    });

    // R738: a position in seat one's own hand is no public end. The aim is dropped, and the arrow
    // the board showed is cleared instead.
    cy.wsPlayer({
      action: "sendFrame",
      name: SEAT_TWO,
      frame: { type: "aim", aim: { source: { at: "hand", player: ONE_SEAT, index: 0 }, target: { at: "hero", player: ONE_SEAT } } },
    });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view }).should("not.exist");
    cy.window().should((win) => {
      expect(aimFramesOf(win).at(-1)?.aim, "R738: the aim at seat one's hand reached the board as the end of the aim").to.eq(null);
      expectEveryAimPublic(win);
    });

    // R643, R1342: seat two sends two emotes of the hand it was dealt. Waiting a bubble out clears the
    // limiter's gate, so the second is held back by the mute and not by the rate.
    cy.wsPlayer({ action: "messages", name: SEAT_TWO }).then((result) => {
      const portraits = (result.messages ?? []).filter((message) => message.type === "portraits");
      const hand = (portraits.at(-1)?.emotes ?? []) as string[];
      expect(hand.length, "R1342: seat two was dealt an emote hand").to.be.at.least(2);
      firstEmote = hand[0] ?? "";
      secondEmote = hand[1] ?? "";
      expect(
        Math.min(EMOTE_BUBBLE_MIN_MS, EMOTE_EMOJI_MS),
        "R643: a bubble outlasts the emote cooldown",
      ).to.be.greaterThan(EMOTE_COOLDOWN_MS);
    });
    cy.then(() => {
      cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "emote", emote: firstEmote } });
    });
    cy.get(`${ts(heroId("opponent"))} ${ts(EMOTE_BUBBLE)}`, { timeout: timeouts.view }).should("exist");
    cy.get(ts(EMOTE_BUBBLE), { timeout: BUBBLE_GONE_TIMEOUT_MS }).should("not.exist");

    // R644: the opponent's portrait opens a menu whose one item mutes them for the rest of the match.
    cy.get(OPPONENT_PORTRAIT).click();
    cy.get(`${ts(heroId("opponent"))} ${ts(EMOTE_MUTE_MENU)}`).should("be.visible");
    cy.get(ts(EMOTE_MUTE)).click();
    cy.get(ts(EMOTE_MUTE_MENU)).should("not.exist");
    cy.get(OPPONENT_PORTRAIT).click();
    cy.get(ts(EMOTE_MUTE)).should("be.disabled");
    cy.get(OPPONENT_PORTRAIT).click();
    cy.get(ts(EMOTE_MUTE_MENU)).should("not.exist");
    cy.window().then((win) => {
      bubblesBeforeMute = bubblesDrawnOf(win);
      expect(bubblesBeforeMute, "the first emote's bubble was drawn").to.be.at.least(1);
    });

    // R643: the second emote is relayed all the same, which the aim sent after it shows (a socket's
    // frames arrive in order); the board draws no bubble for it.
    cy.then(() => {
      cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "emote", emote: secondEmote } });
    });
    cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "aim", aim: AIM_AT_HERO } });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view }).should("have.attr", "data-aim", "hand:p2:0>hero:p1");
    cy.window().should((win) => {
      const relayed = matchFramesOf(win).some(
        (frame) => frame.type === "emote" && frame.from === TWO_SEAT && frame.emote === secondEmote,
      );
      expect(relayed, "R643: the server relayed the muted seat's emote").to.eq(true);
    });
    cy.wsPlayer({ action: "sendFrame", name: SEAT_TWO, frame: { type: "aim", aim: null } });
    cy.get(ts(OPPONENT_AIM), { timeout: timeouts.view }).should("not.exist");
    cy.window().should((win) => {
      expect(bubblesDrawnOf(win), "R644: no bubble was drawn for the muted seat's emote").to.eq(bubblesBeforeMute);
    });

    // B58, #1831: a message from seat two while the match runs badges the board's Friends button.
    cy.then(() => {
      cy.request({ method: "POST", url: apiUrl(`/api/chat/${oneId}`), headers: bearerHeaders(two.token), body: { body: `${UNREAD} ${run}` } })
        .its("status")
        .should("eq", 201);
    });
    cy.get(ts(FRIENDS_BADGE), { timeout: UNREAD_TIMEOUT_MS }).should("be.visible");

    // B58, #1831: the button opens the friends panel; Chat on seat two's row opens the conversation,
    // with the message that was waiting.
    cy.get(ts(FRIENDS_BUTTON)).click();
    cy.get(ts(FRIENDS_PANEL)).should("be.visible");
    cy.then(() => {
      cy.get(`${ts(FRIENDS_PANEL)} ${friendRow(twoId)}`, { timeout: timeouts.view }).within(() => {
        cy.get(ts(TOGETHER_CHAT_OPEN)).click();
      });
      cy.get(ts(FRIENDS_PANEL)).should("not.exist");
      cy.get(`${ts(CHAT_PANEL)}[data-friend-id="${twoId}"]`).should("be.visible");
    });
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="friend"]`, `${UNREAD} ${run}`, { timeout: CHAT_TIMEOUT_MS }).should("be.visible");

    // B58: both ways, mid-match: seat one's message reaches seat two, and seat two's reply the board.
    cy.get(ts(CHAT_INPUT)).type(`${MID_HELLO} ${run}`);
    cy.get(ts(CHAT_SEND)).click();
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="you"]`, `${MID_HELLO} ${run}`, { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      cy.request<ChatBody>({ method: "GET", url: apiUrl(`/api/chat/${oneId}`), headers: bearerHeaders(two.token) })
        .its("body.messages")
        .should((messages: ChatBody["messages"]) => {
          expect(messages.map((message) => message.body)).to.include(`${MID_HELLO} ${run}`);
        });
      cy.request({ method: "POST", url: apiUrl(`/api/chat/${oneId}`), headers: bearerHeaders(two.token), body: { body: `${MID_REPLY} ${run}` } })
        .its("status")
        .should("eq", 201);
    });
    cy.contains(`${ts(CHAT_MESSAGE)}[data-from="friend"]`, `${MID_REPLY} ${run}`, { timeout: CHAT_TIMEOUT_MS }).should("be.visible");
    cy.get(ts(CHAT_CLOSE)).click();
    cy.get(ts(CHAT_PANEL)).should("not.exist");
    cy.get(ts(FRIENDS_BADGE), { timeout: timeouts.view }).should("not.exist");
    cy.wsPlayer({ action: "view", name: SEAT_TWO }).then((result) => {
      expect(result.view?.result ?? null, "the match was still running while they chatted").to.eq(null);
    });

    cy.concede();
    cy.expectResult("Loss");
    cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { hasResult: true } });

    // B52, B59: the replay is shared with seat two from the replay player.
    cy.get(ts(MATCH_WATCH_REPLAY), { timeout: timeouts.view }).click();
    onStep(0);
    cy.get(ts(SHARE_FRIEND_OPEN)).click();
    cy.then(() => {
      cy.get(`${ts(SHARE_FRIEND_OPTION)}[data-profile-id="${twoId}"]`, { timeout: timeouts.view }).click();
      cy.get(`${ts(SHARE_FRIEND_UNSHARE)}[data-profile-id="${twoId}"]`, { timeout: timeouts.view }).should(
        "have.attr",
        "data-shared",
        "true",
      );
      cy.request<SharedBody>({ method: "GET", url: apiUrl("/api/replays/shared"), headers: bearerHeaders(two.token) })
        .its("body.shared")
        .should((shared: SharedBody["shared"]) => {
          const entry = shared.find((replay) => replay.matchId === matchId);
          expect(entry?.sharer.profileId, "B59: the replay is shared with seat two by seat one").to.eq(oneId);
        });
    });

    // B59: seat two finds it under Shared with me, from Replays, and watches it.
    cy.visitAs(two, routes.replays());
    cy.get(ts(REPLAYS_SHARED_WITH_ME), { timeout: timeouts.view }).click();
    cy.location("pathname").should("eq", TOGETHER);
    cy.location("hash").should("eq", "#shared");
    cy.then(() => {
      cy.get(`${ts(TOGETHER_SHARED_ROW)}[data-match-id="${matchId}"]`, { timeout: timeouts.view }).within(() => {
        cy.get(ts(TOGETHER_SHARED_OPEN)).click();
      });
    });
    onStep(0);

    // Part the two accounts, for the next run and the next spec.
    cy.then(() => {
      separate([
        [one, twoId],
        [two, oneId],
      ]);
    });
  });
});
