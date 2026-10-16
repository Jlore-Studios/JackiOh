// #1050 44: watching a friend's live game (#936, SPEC S7, §9.5 "Watching a friend"; B50, B56).
//
// Three accounts, because a watcher holds no seat (`watchable_in` refuses one that does): `one`
// hosts an unranked room and is the friend watched (seat p1), `pending` joins it as seat p2, and
// `two` is the friend who watches, in the browser. Neither seat attaches a socket (R744: a seat that
// never attached starts no grace), so the match just stands while it is watched. `pending` is the
// fixture account waiting at the invite gate, so `before` reseeds the server (R144) and redeems the
// good code for it, which leaves nothing for a spec after this one: only spec 10 reads that account
// and it reseeds first.
//
// What it asserts, by behaviour (every frame read is one the watch screen's own socket received;
// an account is one watcher and a newer socket replaces the older, so the spec opens none itself):
// - B56: a friend's row in a live unranked match offers Watch, and the click opens `/watch/:matchId`
//   with the friend in the query;
// - B50: the screen waits (its waiting panel) until the first view, which comes `SPECTATE_DELAY_MS`
//   after the state it shows, then draws the friend's board read-only from the friend's seat;
// - B50, §10.8: every `view` frame carries `legal: []`, the friend's seat's hand in full and the
//   other seat's hand as a count alone;
// - B50 (revocation): the friend turning `allowSpectators` off mid-watch ends it with a `forbidden`
//   frame and the refused screen;
// - B50 (refusal): with watching off, and with the friends no longer friends, the watch link is
//   refused the same way, and the way back leads to Play together.
//
// "Full" (`SPECTATORS_PER_MATCH`) is not covered here: an account is one watcher, so filling a match
// takes nine friend accounts and the E2E server seeds three. The server's own test covers it
// (`b50_b51_a_full_match_is_forbidden_to_one_watcher_more`) and the client shows it as any other
// `forbidden` (WatchRoute.test.tsx).
//
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` beside the client. Daily only,
// as every spec outside scripts/shard-specs.mjs's SMOKE. `SPECTATE_DELAY_MS` is the server's, fixed
// at compile time, so the first view takes about 30 s to arrive and the spec waits it out.

import { SPECTATE_DELAY_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { E2E_RESEED_PATH, accounts, inviteCodes, seedFor, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  GAME,
  TOGETHER_FRIEND_ROW,
  TOGETHER_ROOT,
  TOGETHER_WATCH,
  WATCH_BACK,
  WATCH_REFUSED,
  WATCH_SCREEN,
  WATCH_STATUS,
  WATCH_WAITING,
  heroId,
  ts,
} from "../../support/testids.ts";

// Constants

/** Play together's route, mirroring `paths.together` in apps/web/src/net/navigate.ts. */
const TOGETHER = "/together";

/** The watch socket's path, `WATCH_PATH` in apps/web/src/together/WatchRoute.tsx. */
const WATCH_SOCKET_PATH = "/ws/watch";

/** The room's host is seat p1 (§9.5), so the friend watched sees the board from there. */
const HOST_SEAT = "p1";

const SEED = seedFor("44-watch");

/** The first view is due `SPECTATE_DELAY_MS` after the match began, and a render after that. */
const FIRST_VIEW_TIMEOUT_MS = SPECTATE_DELAY_MS + timeouts.view;

/** The browser's `Date.now()` against the server's monotonic clock, which `SPECTATE_DELAY_MS` is measured on. */
const CLOCK_SLACK_MS = 1_000;

// HTTP contract (SPEC S7), as far as this spec reads it

type PlayerRef = { profileId: string; username: string };
type RequestBody = { requestId: string; from: PlayerRef; to: PlayerRef };
type FriendsBody = { friends: PlayerRef[]; incoming: RequestBody[]; outgoing: RequestBody[]; blocked: PlayerRef[] };
type MeBody = { profile: { id: string }; username?: { name: string } };
type JoinBody = { matchId: string; seat: string };

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

/** A friend row of the page, by the profile it shows. */
function friendRow(profileId: string): string {
  return `${ts(TOGETHER_FRIEND_ROW)}[data-profile-id="${profileId}"]`;
}

/** The watch link a friend row's Watch follows (`watchLink` in apps/web/src/together/watchLink.ts). */
function watchPath(matchId: string, friendId: string): string {
  return `/watch/${matchId}?${new URLSearchParams({ friend: friendId }).toString()}`;
}

/** `PUT /api/settings` with the account's `allowSpectators` alone. */
function allowSpectators(account: E2EAccount, allow: boolean): Cypress.Chainable<Cypress.Response<{ settings: { allowSpectators: boolean } }>> {
  return cy.request<{ settings: { allowSpectators: boolean } }>({
    method: "PUT",
    url: api("/api/settings"),
    headers: bearer(account),
    body: { allowSpectators: allow },
  });
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
    cy.request({ method: "POST", url: api(E2E_RESEED_PATH) }).its("status").should("eq", 200);
    cy.request<{ status: string }>({
      method: "POST",
      url: api("/api/codes/redeem"),
      headers: bearer(accounts.pending()),
      body: { code: inviteCodes.good() },
    })
      .its("body.status")
      .should("eq", "active");
  });

  it("B50: waits out the delay, then draws the friend's seat read-only, and is ended when the friend turns watching off", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    const joiner = accounts.pending();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let matchId = "";
    let joinRequestedAt = 0;

    me(one).then((body) => {
      oneId = body.profile.id;
      oneName = body.username?.name ?? "";
    });
    me(two).then((body) => {
      twoId = body.profile.id;
    });
    cy.then(() => {
      expect(oneName, "R1435: the friend watched has a username to be found by").to.not.eq("");
    });

    // §9.5: a friend allows watching until they say otherwise.
    cy.request<{ settings: { allowSpectators: boolean } }>({ method: "GET", url: api("/api/settings"), headers: bearer(one) })
      .its("body.settings.allowSpectators")
      .should("eq", true);

    // Two friends: `two` asks, `one` accepts.
    cy.then(() => {
      cy.request({ method: "POST", url: api("/api/friends/requests"), headers: bearer(two), body: { username: oneName } })
        .its("status")
        .should("eq", 201);
      friendsOf(one).then((body) => {
        const request = body.incoming.find((got) => got.from.profileId === twoId);
        expect(request, "B43: the request reached the friend").to.not.eq(undefined);
        cy.request({
          method: "POST",
          url: api(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
          headers: bearer(one),
        })
          .its("status")
          .should("eq", 200);
      });
    });

    // An unranked match: `one` hosts the room (seat p1), `pending` joins it (p2). No deck to bring:
    // the server deals both seats one (R257). The seed fixes the deal (R143).
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(one),
        body: { mode: "random", seed: SEED },
      }).then((created) => {
        joinRequestedAt = Date.now();
        cy.request<JoinBody>({
          method: "POST",
          url: api(`/api/rooms/${created.body.code}/join`),
          headers: bearer(joiner),
          body: { mode: "random" },
        }).then((joined) => {
          expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq("p2");
          matchId = joined.body.matchId;
          expect(matchId, "a match to watch").to.not.eq("");
        });
      });
    });

    // B56: the friend's row offers Watch, and the click opens the watch screen. Nothing sits between
    // the join and this, so the waiting panel (up to the delay) is still showing when it is asserted.
    cy.visitAs(two, TOGETHER, { onBeforeLoad: recordWatchFrames });
    cy.then(() => {
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

    // B50, B46: the friend turns watching off, and the watcher is let go at once.
    cy.then(() => {
      allowSpectators(one, false).its("body.settings.allowSpectators").should("eq", false);
    });
    cy.get(ts(WATCH_REFUSED), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(WATCH_SCREEN)).should("have.attr", "data-connection", "refused");
    cy.window().should((win) => {
      const last = framesOf(win).at(-1);
      expect(last?.type, "B50: the last thing the watcher is sent").to.eq("error");
      expect(last?.code, "B50: a revoked watcher is told forbidden").to.eq("forbidden");
    });

    // With watching off, the row no longer offers Watch, and a watch link is refused.
    cy.visitAs(two, TOGETHER);
    cy.then(() => {
      cy.get(friendRow(oneId), { timeout: timeouts.view })
        .should("have.attr", "data-status", "inMatch")
        .and("have.attr", "data-watchable", "false");
      cy.get(friendRow(oneId)).find(ts(TOGETHER_WATCH)).should("not.exist");
      cy.visitAs(two, watchPath(matchId, oneId), { onBeforeLoad: recordWatchFrames });
    });
    expectRefused();

    // Watching back on, but no longer friends: the same refusal, and the way back leads to Play together.
    cy.then(() => {
      allowSpectators(one, true).its("status").should("eq", 200);
      cy.request({ method: "DELETE", url: api(`/api/friends/${twoId}`), headers: bearer(one) }).its("status").should("eq", 204);
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
});
