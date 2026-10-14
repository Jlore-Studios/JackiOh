// 39 — an online match's replay, between the browser and cy.task("wsPlayer") (#768; §9.5, R768, R768000).
//
// Seat 1 is the browser and seat 2 the Node client, in a Best-of-1 room as spec 06 makes one. Seat 1
// concedes once both mulligans are in, and the match's result panel opens its replay: "Watch the
// replay" at step 0, a step forward and back, and Back to the board; "Share the last turn" at the
// last step, whose turn's GIF is made, downloaded (a headless browser has no share sheet), a GIF and
// under the cap. Then the recent games (`/replays`) list the match and open its replay.
//
// Daily only: a pull request's `e2e smoke` plays SMOKE in scripts/shard-specs.mjs, which does not hold
// 39. Every wait is a retried DOM assertion; the GIF's waits on its Download button.

import { accounts, routes, seedFor, server, timeouts } from "../../support/config.ts";
import type { InstalledLoadout } from "../../support/commands.ts";
import { expectLastTurnGif, onStep, spyOnDownloads, stepForwardAndBack } from "../../support/replays.ts";
import {
  MATCH_SHARE_LAST_TURN,
  MATCH_WATCH_REPLAY,
  REPLAY_BACK,
  REPLAYS_ROOT,
  heroId,
  replaysOnlineId,
  ts,
} from "../../support/testids.ts";
import type { ActionInput, PlayerId } from "../../support/types.ts";

/** Spec 06's session key: the browser's account, seeded before load. */
const SESSION_STORAGE_KEY = "jackioh.e2e.session";

const SEAT_TWO = "seat-two";
const SEAT_TWO_ID: PlayerId = "p2";

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

function deckOf(installed: InstalledLoadout | null): string {
  expect(installed, "cy.installLoadout has answered").to.not.eq(null);
  return installed?.deckIds[0] ?? "";
}

/** The browser as `token`, with downloads spied on (`support/replays.ts`). */
function visitAs(token: string, path: string): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
      spyOnDownloads(win);
    },
  });
}

/** Seat 2 keeps its whole opening hand (R9). */
function seatTwoKeepsMulligan(): void {
  cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { promptKind: "mulligan" } }).then((result) => {
    const view = (result.view ?? {}) as { you?: { hand?: { instanceId: string }[] } };
    const hand = Array.isArray(view.you?.hand) ? view.you.hand : [];
    const body: ActionInput = { type: "mulligan", keep: hand.map((card) => card.instanceId), playerId: SEAT_TWO_ID };
    cy.wsPlayer({ action: "send", name: SEAT_TWO, body });
  });
}

describe("39 — an online match's replay (§9.5, R768000)", () => {
  it("R768000 a finished match's replay opens from its result panel, steps forward and back, shares its last turn as a GIF, and is listed in the recent games", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let roomCode = "";
    let matchId = "";
    let seatOneDecks: InstalledLoadout | null = null;
    let seatTwoDecks: InstalledLoadout | null = null;

    cy.freeAccount(seatOne);
    cy.freeAccount(seatTwo);
    cy.installLoadout(seatOne, "39-replays-a").then((installed) => {
      seatOneDecks = installed;
    });
    cy.installLoadout(seatTwo, "39-replays-b").then((installed) => {
      seatTwoDecks = installed;
    });

    // A Best-of-1 room, made by seat 1 and claimed by seat 2 (§9.5), as spec 06 makes one.
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne.token),
        body: { mode: "bo1", deckId: deckOf(seatOneDecks), seed: seedFor("39-replays") },
      }).then((created) => {
        roomCode = created.body.code;
      });
    });
    cy.then(() => {
      cy.request<{ matchId: string }>({
        method: "POST",
        url: api(`/api/rooms/${roomCode}/join`),
        headers: bearer(seatTwo.token),
        body: { mode: "bo1", deckId: deckOf(seatTwoDecks) },
      }).then((joined) => {
        matchId = joined.body.matchId;
        expect(matchId, "a match id to open a socket onto").to.be.a("string").and.not.eq("");
      });
    });
    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId });
      visitAs(seatOne.token, routes.match(matchId));
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");

    // Both keep their hands, and once the first turn is under way (whichever seat's), seat 1
    // concedes and the match is over for both.
    cy.keepMulligans();
    seatTwoKeepsMulligan();
    cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { turnAtLeast: 1 } });
    cy.concede();
    cy.expectResult("Loss");
    cy.then(() => {
      cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { hasResult: true } });
    });

    // Watch the replay: step 0, a step forward and back, and Back to the board and its result panel.
    cy.get(ts(MATCH_WATCH_REPLAY), { timeout: timeouts.view }).click();
    stepForwardAndBack();
    cy.get(ts(REPLAY_BACK)).click();

    // Share the last turn: the last step, its turn's GIF made and downloaded; then back.
    cy.get(ts(MATCH_SHARE_LAST_TURN), { timeout: timeouts.view }).click();
    expectLastTurnGif();
    cy.get(ts(REPLAY_BACK)).click();
    cy.get(ts(MATCH_WATCH_REPLAY), { timeout: timeouts.view }).should("be.visible");

    // The recent games list the match, and open its replay at step 0.
    cy.then(() => {
      visitAs(seatOne.token, routes.replays());
    });
    cy.get(ts(REPLAYS_ROOT), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      cy.get(ts(replaysOnlineId(matchId)), { timeout: timeouts.view }).within(() => {
        cy.contains("button", "Watch").should("not.be.disabled").click();
      });
    });
    onStep(0);
  });
});
