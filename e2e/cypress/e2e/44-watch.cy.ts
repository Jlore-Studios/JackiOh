// #1050 44: watching a friend's live game (#936, SPEC S7, §9.5 "Watching a friend"; B50, B56).
// Three accounts, because a watcher holds no seat: `one` hosts an unranked room (seat p1), `pending` joins it as p2,
// and `two` watches in the browser. Neither seat attaches a socket (R744: a seat that never attached starts no grace),
// so the match stands while it is watched. `pending` is the fixture account waiting at the invite gate, so `before`
// reseeds the server (R144) and redeems the good code for it; only spec 10 reads that account and it reseeds first.
// The waiting panel shows only to a watcher who comes within `SPECTATE_DELAY_MS` of the join, so the browser is on
// Play together first. "Full" (`SPECTATORS_PER_MATCH`) is the server's test: the E2E server seeds three friends.
// R2990000: a watcher's emote from the sideline strip reaches a seat live, the joiner's Node socket standing in
// for the player, with the hand the watch socket dealt (R2990001).
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` beside the client. Daily only, as every spec
// outside scripts/shard-specs.mjs's SMOKE.

import { SPECTATE_DELAY_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { E2E_RESEED_PATH, accounts, apiUrl, bearerHeaders, inviteCodes, seedFor, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  EMOTE_MENU,
  GAME,
  SIDELINE_EMOTE,
  SIDELINE_MENU,
  SIDELINE_STRIP,
  TOGETHER_FRIEND_ROW,
  TOGETHER_REQUEST_ACCEPT,
  TOGETHER_REQUEST_ROW,
  TOGETHER_ROOT,
  TOGETHER_WATCH,
  WATCH_BACK,
  WATCH_REFUSED,
  WATCH_SCREEN,
  WATCH_STATUS,
  WATCH_WAITING,
  emoteItemId,
  heroId,
  ts,
} from "../../support/testids.ts";

/** Mirrors `paths.together` in apps/web/src/net/navigate.ts. */
const TOGETHER = "/together";

/** `WATCH_PATH` in apps/web/src/together/WatchRoute.tsx. */
const WATCH_SOCKET_PATH = "/ws/watch";

/** The room's host is seat p1 (§9.5), so the friend watched sees the board from there. */
const HOST_SEAT = "p1";

const SEED = seedFor("44-watch");

/** The first view is due `SPECTATE_DELAY_MS` after the match began, and a render after that. */
const FIRST_VIEW_TIMEOUT_MS = SPECTATE_DELAY_MS + timeouts.view;

/** The browser's `Date.now()` against the server's monotonic clock, which `SPECTATE_DELAY_MS` is measured on. */
const CLOCK_SLACK_MS = 1_000;

/** R2990000: a watcher's emote is relayed live, never `SPECTATE_DELAY_MS` behind, so a view push's wait covers it. */
const RELAY_TIMEOUT_MS = timeouts.view;

/** The joiner's Node socket, the seat a watcher's emote is relayed to (R2990000). */
const JOINER = "joiner";
const JOINER_SEAT = "p2";

// HTTP contract (SPEC S7), as far as this spec reads it

type MeBody = { profile: { id: string }; username?: { name: string } };
type JoinBody = { matchId: string; seat: string };
type PlayerRef = { profileId: string; username: string };
type FriendsBody = { friends: PlayerRef[]; incoming: { requestId: string; from: PlayerRef }[] };

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy.request<MeBody>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).its("body");
}

function friendsOf(account: E2EAccount): Cypress.Chainable<FriendsBody> {
  return cy.request<FriendsBody>({ method: "GET", url: apiUrl("/api/friends"), headers: bearerHeaders(account.token) }).its("body");
}

/** B43, B45: `asked` and `asker` friends, by a request and its accept, unless they already are. */
function befriend(asker: E2EAccount, askerId: string, asked: E2EAccount, askedName: string): void {
  friendsOf(asked).then((body) => {
    if (body.friends.some((friend) => friend.profileId === askerId)) return;
    if (!body.incoming.some((request) => request.from.profileId === askerId)) {
      cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(asker.token), body: { username: askedName } })
        .its("status")
        .should("eq", 201);
    }
    friendsOf(asked).then((after) => {
      const request = after.incoming.find((got) => got.from.profileId === askerId);
      expect(request, "B43: the request reached the friend").to.not.eq(undefined);
      cy.request({
        method: "POST",
        url: apiUrl(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
        headers: bearerHeaders(asked.token),
      })
        .its("status")
        .should("eq", 200);
    });
  });
}

/**
 * `name`'s Node socket's frames, read again until one is `what` or RELAY_TIMEOUT_MS has passed. A task is
 * not retried by `.should`, and specs never sleep (`cy.wait(ms)` is refused), so each read is one more task.
 */
function untilReceived(name: string, match: (frame: Frame) => boolean, what: string, deadline = Date.now() + RELAY_TIMEOUT_MS): void {
  cy.wsPlayer({ action: "messages", name }).then((result) => {
    if ((result.messages ?? []).some(match)) return;
    expect(Date.now(), `${what} within RELAY_TIMEOUT_MS`).to.be.below(deadline);
    untilReceived(name, match, what, deadline);
  });
}

function friendRow(profileId: string): string {
  return `${ts(TOGETHER_FRIEND_ROW)}[data-profile-id="${profileId}"]`;
}

function incomingRequestRow(profileId: string): string {
  return `${ts(TOGETHER_REQUEST_ROW)}[data-direction="incoming"][data-profile-id="${profileId}"]`;
}

/** The watch link a friend row's Watch follows (`watchLink` in apps/web/src/together/watchLink.ts). */
function watchPath(matchId: string, friendId: string): string {
  return `/watch/${matchId}?${new URLSearchParams({ friend: friendId }).toString()}`;
}

// The frames the watch screen's socket received

type Frame = { type: string; [key: string]: unknown };
type Recorded = { at: number; frame: Frame };
type RecordingWindow = Cypress.AUTWindow & { __watchFrames?: Recorded[] };
type HandLike = unknown[] | { count?: unknown };
type ViewLike = { viewer?: string; you?: { hand?: HandLike }; opponent?: { hand?: HandLike } };

/** `onBeforeLoad`: every frame the page's `/ws/watch` sockets receive, and when. */
function recordWatchFrames(win: Cypress.AUTWindow): void {
  const frames: Recorded[] = [];
  (win as RecordingWindow).__watchFrames = frames;
  const RealSocket = win.WebSocket;
  win.WebSocket = new Proxy(RealSocket, {
    construct(target, args: unknown[]) {
      const socket = Reflect.construct(target, args) as WebSocket;
      if (String(args[0]).includes(WATCH_SOCKET_PATH)) {
        socket.addEventListener("message", (event: MessageEvent) => {
          if (typeof event.data !== "string") return;
          let parsed: unknown;
          try {
            parsed = JSON.parse(event.data);
          } catch {
            return;
          }
          if (typeof parsed === "object" && parsed !== null && typeof (parsed as Frame).type === "string") {
            frames.push({ at: Date.now(), frame: parsed as Frame });
          }
        });
      }
      return socket;
    },
  });
}

function recordedOf(win: Cypress.AUTWindow): Recorded[] {
  return (win as RecordingWindow).__watchFrames ?? [];
}

function framesOf(win: Cypress.AUTWindow): Frame[] {
  return recordedOf(win).map((recorded) => recorded.frame);
}

/** The refused screen, with the way back, and the `forbidden` frame behind it and no view. */
function expectRefused(): void {
  cy.get(ts(WATCH_REFUSED), { timeout: timeouts.view }).should("be.visible");
  cy.get(ts(WATCH_SCREEN)).should("have.attr", "data-connection", "refused");
  cy.get(ts(WATCH_BACK)).should("be.visible");
  cy.window().should((win) => {
    const frames = framesOf(win);
    expect(
      frames.some((frame) => frame.type === "error" && frame.code === "forbidden"),
      "B50: a watcher who may not watch is told forbidden",
    ).to.eq(true);
    expect(
      frames.some((frame) => frame.type === "view"),
      "B50: and is never sent a view",
    ).to.eq(false);
  });
}

describe("44 watching a friend's live game", () => {
  before(() => {
    // R144: the server puts back the fixture accounts and codes, so `pending` is waiting at the
    // gate again whatever ran before and the code is unspent; redeeming it makes the third account.
    cy.request({ method: "POST", url: apiUrl(E2E_RESEED_PATH) }).its("status").should("eq", 200);
    cy.request<{ status: string }>({
      method: "POST",
      url: apiUrl("/api/codes/redeem"),
      headers: bearerHeaders(accounts.pending().token),
      body: { code: inviteCodes.good() },
    })
      .its("body.status")
      .should("eq", "active");
  });

  it("B50 R2615000: waits out the delay, then draws the friend's seat read-only, and is ended when the friend removes the watcher", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    const joiner = accounts.pending();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let twoName = "";
    let matchId = "";
    let joinRequestedAt = 0;

    me(one).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two).then((body) => {
      twoId = body.profile.id;
      twoName = body.username?.name ?? "";
    });
    cy.then(() => {
      expect(oneName, "R1435: the friend watched has a username to be found by").to.not.eq("");
      expect(twoName, "R1435: the watcher has a username to be found by").to.not.eq("");
    });

    // R2615000: no setting turns watching off; the account's settings carry no switch.
    cy.request<{ settings: Record<string, unknown> }>({ method: "GET", url: apiUrl("/api/settings"), headers: bearerHeaders(one.token) })
      .its("body.settings")
      .should("not.have.property", "allowSpectators");

    // B43: `one` asks `two` to be friends, and `two` opens Play together, where the request waits.
    cy.then(() => {
      cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(one.token), body: { username: twoName } })
        .its("status")
        .should("eq", 201);
    });
    cy.visitAs(two, TOGETHER, { onBeforeLoad: recordWatchFrames });
    cy.then(() => {
      cy.get(incomingRequestRow(oneId), { timeout: timeouts.view }).should("be.visible");
    });

    // No deck to bring: the server deals both seats one (R257). The seed fixes the deal (R143), and R1989000
    // pins the host to p1 beside it, so the spec keeps the seats it was written for.
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: apiUrl("/api/rooms"),
        headers: bearerHeaders(one.token),
        body: { mode: "random", seed: SEED, seat: HOST_SEAT },
      }).then((created) => {
        joinRequestedAt = Date.now();
        cy.request<JoinBody>({
          method: "POST",
          url: apiUrl(`/api/rooms/${created.body.code}/join`),
          headers: bearerHeaders(joiner.token),
          body: { mode: "random" },
        }).then((joined) => {
          expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq("p2");
          matchId = joined.body.matchId;
          expect(matchId, "a match to watch").to.not.eq("");
        });
      });
    });

    // B45: `two` accepts, and the page reads the friends list again. B56: the new friend's row offers
    // Watch, and the click opens the watch screen in the page already loaded.
    cy.then(() => {
      cy.get(incomingRequestRow(oneId)).within(() => {
        cy.get(ts(TOGETHER_REQUEST_ACCEPT)).click();
      });
      cy.get(friendRow(oneId), { timeout: timeouts.view })
        .should("have.attr", "data-status", "inMatch")
        .and("have.attr", "data-watchable", "true")
        .within(() => {
          cy.get(ts(TOGETHER_WATCH)).click();
        });
      cy.location("pathname").should("eq", `/watch/${matchId}`);
      cy.location("search").should("eq", `?${new URLSearchParams({ friend: oneId }).toString()}`);
    });

    // B50: the socket is open and the screen waits for the first, delayed, view.
    cy.get(ts(WATCH_SCREEN)).should("have.attr", "data-connection", "open");
    cy.get(ts(WATCH_WAITING)).should("be.visible");

    // B50: the first view comes SPECTATE_DELAY_MS on, drawn from the friend's seat, read-only.
    cy.then(() => {
      cy.get(ts(WATCH_STATUS), { timeout: FIRST_VIEW_TIMEOUT_MS })
        .should("have.attr", "data-connection", "open")
        .and("contain.text", oneName);
    });
    cy.get(ts(WATCH_WAITING)).should("not.exist");
    cy.get(ts(GAME)).should("have.attr", "data-replay", "true").and("have.attr", "data-viewer", HOST_SEAT);
    cy.get(ts(heroId("you"))).should("exist");

    // B50, §10.8: what the socket carried, not only what the board drew.
    cy.window().should((win) => {
      const views = recordedOf(win).filter((recorded) => recorded.frame.type === "view");
      expect(views.length, "at least one view reached the watcher").to.be.greaterThan(0);
      expect(
        (views[0]?.at ?? 0) - joinRequestedAt,
        "B50: the first view comes SPECTATE_DELAY_MS after the state it shows",
      ).to.be.at.least(SPECTATE_DELAY_MS - CLOCK_SLACK_MS);
      for (const { frame } of views) {
        const view = frame.view as ViewLike;
        expect(frame.legal, "B50: a watcher is offered nothing").to.deep.eq([]);
        expect(view.viewer, "the board is the friend's seat's").to.eq(HOST_SEAT);
        expect(Array.isArray(view.you?.hand), "the friend's own hand is the friend's to see").to.eq(true);
        const opponentHand = view.opponent?.hand;
        expect(Array.isArray(opponentHand), "§10.8: the other seat's hand is no list of cards").to.eq(false);
        expect(
          typeof (opponentHand as { count?: unknown } | undefined)?.count,
          "§10.8: the other seat's hand is a count",
        ).to.eq("number");
      }
    });

    // B50, B45: the friend removes the watcher, and the watcher is let go at once (R2615000: the
    // friendship is the way out).
    cy.then(() => {
      cy.request({ method: "DELETE", url: apiUrl(`/api/friends/${twoId}`), headers: bearerHeaders(one.token) }).its("status").should("eq", 204);
    });
    cy.get(ts(WATCH_REFUSED), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(WATCH_SCREEN)).should("have.attr", "data-connection", "refused");
    cy.window().should((win) => {
      const last = framesOf(win).at(-1);
      expect(last?.type, "B50: the last thing the watcher is sent").to.eq("error");
      expect(last?.code, "B50: a revoked watcher is told forbidden").to.eq("forbidden");
    });

    // No longer friends: a watch link is refused the same way, and the way back leads to Play together.
    cy.then(() => {
      cy.visitAs(two, watchPath(matchId, oneId), { onBeforeLoad: recordWatchFrames });
    });
    expectRefused();
    cy.get(ts(WATCH_BACK)).click();
    cy.location("pathname").should("eq", TOGETHER);
    cy.get(ts(TOGETHER_ROOT), { timeout: timeouts.view }).should("exist");

    // Free the accounts for the next spec.
    cy.then(() => {
      cy.concedeAs(one, matchId);
    });
  });

  it("R2990000 a watcher's emote from the sidelines reaches a player", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    const joiner = accounts.pending();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let twoName = "";
    let matchId = "";
    let picked = "";

    me(one).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two).then((body) => {
      twoId = body.profile.id;
      twoName = body.username?.name ?? "";
    });

    // The spec's first test ends with `one` removing `two`, so they are made friends again.
    cy.then(() => {
      befriend(one, oneId, two, twoName);
    });

    // A room as the first test opens one, and the joiner's seat on a Node socket, which the relay reaches.
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: apiUrl("/api/rooms"),
        headers: bearerHeaders(one.token),
        body: { mode: "random", seed: SEED, seat: HOST_SEAT },
      }).then((created) => {
        cy.request<JoinBody>({
          method: "POST",
          url: apiUrl(`/api/rooms/${created.body.code}/join`),
          headers: bearerHeaders(joiner.token),
          body: { mode: "random" },
        }).then((joined) => {
          expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq(JOINER_SEAT);
          matchId = joined.body.matchId;
        });
      });
    });
    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: JOINER, url: server.ws(), token: joiner.token, matchId, seat: JOINER_SEAT });
    });

    // `two` watches `one` and waits out the delay for the first view.
    cy.then(() => {
      cy.visitAs(two, watchPath(matchId, oneId), { onBeforeLoad: recordWatchFrames });
      cy.get(ts(WATCH_STATUS), { timeout: FIRST_VIEW_TIMEOUT_MS })
        .should("have.attr", "data-connection", "open")
        .and("contain.text", oneName);
    });
    cy.get(ts(GAME)).should("have.attr", "data-replay", "true");

    // R2990001: the watch socket dealt the watcher its own hand of emoji, which the strip's menu offers.
    cy.window().then((win) => {
      const dealt = framesOf(win).find((frame) => frame.type === "spectatorHand");
      const hand = Array.isArray(dealt?.emotes) ? (dealt.emotes as string[]) : [];
      expect(hand.length, "R2990001: the watcher was dealt a hand").to.be.greaterThan(0);
      picked = hand[0] ?? "";
      cy.get(ts(SIDELINE_MENU)).click();
      cy.get(`${ts(EMOTE_MENU)} [role="menuitem"]`).then((items) => {
        expect([...items].map((item) => item.getAttribute("data-testid")), "the menu offers the dealt hand").to.deep.eq(hand.map(emoteItemId));
      });
    });
    cy.then(() => {
      cy.get(`${ts(EMOTE_MENU)} ${ts(emoteItemId(picked))}`).click();
      // Shown at once as the watcher's own, since the server never echoes it.
      cy.get(`${ts(SIDELINE_STRIP)} ${ts(SIDELINE_EMOTE)}[data-self="true"][data-emote="${picked}"]`).should("contain.text", "You");
    });

    // R2990000: the seat hears it live, under the watcher's name.
    cy.then(() => {
      untilReceived(
        JOINER,
        (frame) => frame.type === "spectatorEmote" && frame.watcher === twoName && frame.emote === picked,
        "the watcher's spectatorEmote at the joiner's seat",
      );
    });
    cy.window().then((win) => {
      expect(
        framesOf(win).some((frame) => frame.type === "spectatorEmote" && frame.watcher === twoName),
        "R2990000: the watcher is never sent its own emote back",
      ).to.eq(false);
    });

    // Free the accounts for the next spec, no longer friends, as the first test leaves them.
    cy.then(() => {
      cy.wsPlayer({ action: "disconnect", name: JOINER });
      cy.concedeAs(one, matchId);
      cy.request({ method: "DELETE", url: apiUrl(`/api/friends/${twoId}`), headers: bearerHeaders(one.token) }).its("status").should("eq", 204);
    });
  });
});
