// #1050 44: watching a friend's live game (#936, SPEC S7, §9.5 "Watching a friend"; B50, B56).
// Three accounts, because a watcher holds no seat (`watchable_in` refuses one that does): `one` hosts an
// unranked room and is the friend watched (seat p1), `pending` joins as seat p2, and `two` watches in the
// browser. Neither seat attaches a socket (R744: a seat that never attached starts no grace), so the match
// just stands while it is watched. `pending` is the fixture account waiting at the invite gate, so `before`
// reseeds the server and redeems the good code for it: only spec 10 reads that account, and it reseeds first. The spec opens no socket itself: an account is one watcher and a
// newer socket replaces the older.
// "Full" (`SPECTATORS_PER_MATCH`) takes nine friend accounts and the E2E server seeds three: the server's
// `b50_b51_a_full_match_is_forbidden_to_one_watcher_more` covers it, and WatchRoute.test.tsx its `forbidden`.
// A watcher who comes after `SPECTATE_DELAY_MS` is sent the first view at once, so the waiting panel shows
// only to one who comes sooner; a page visit alone can take that long, so the browser is on Play together
// before the join. Networked (`E2E=1 cargo run --release -p jackioh-server`); daily only, as every spec
// outside scripts/shard-specs.mjs's SMOKE. `SPECTATE_DELAY_MS` is the server's, fixed at compile time.

import { SPECTATE_DELAY_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { E2E_RESEED_PATH, accounts, apiUrl, bearerHeaders, inviteCodes, seedFor, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  GAME,
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

// HTTP contract (SPEC S7), as far as this spec reads it

type MeBody = { profile: { id: string }; username?: { name: string } };
type JoinBody = { matchId: string; seat: string };

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy.request<MeBody>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).its("body");
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

/** `PUT /api/settings` with the account's `allowSpectators` alone. */
function allowSpectators(account: E2EAccount, allow: boolean): Cypress.Chainable<Cypress.Response<{ settings: { allowSpectators: boolean } }>> {
  return cy.request<{ settings: { allowSpectators: boolean } }>({
    method: "PUT",
    url: apiUrl("/api/settings"),
    headers: bearerHeaders(account.token),
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

  it("B50: waits out the delay, then draws the friend's seat read-only, and is ended when the friend turns watching off", () => {
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

    // §9.5: a friend allows watching until they say otherwise.
    cy.request<{ settings: { allowSpectators: boolean } }>({ method: "GET", url: apiUrl("/api/settings"), headers: bearerHeaders(one.token) })
      .its("body.settings.allowSpectators")
      .should("eq", true);

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
      cy.request({ method: "DELETE", url: apiUrl(`/api/friends/${twoId}`), headers: bearerHeaders(one.token) }).its("status").should("eq", 204);
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
