// 58 — the notification feed (issue #4039, R4039000, R4039001): what happened to a player reaches the open tab
// as a toast and stays in the bell's Recent section. Seat one challenges seat two to a daily game and seat two
// accepts, both keep their hands and the move is seat two's, all over HTTP (`support/daily.ts`). Seat one opens
// Play and waits for its social socket to be live (the bell's `data-live`); seat two ends its turn, and seat one's
// tab shows "Your move against …" as a toast and as a `dailyMove` row of the inbox's Recent section.
// Networked (`E2E=1 cargo run --release -p jackioh-server`); daily only, as every spec outside
// scripts/shard-specs.mjs's SMOKE. No wait depends on a server clock: every wait is a retried DOM or HTTP assertion.

import { inboxTestid } from "../../../apps/web/src/inbox/testids.ts";
import { SOCIAL_POLL_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, routes, timeouts } from "../../support/config.ts";
import {
  clearDaily,
  dailyConcedeAs,
  dailyEndTurnAs,
  dailyGameOf,
  dailyKeepHandAs,
  dailyOf,
  makeFriends,
  profileOf,
} from "../../support/daily.ts";
import { ts } from "../../support/testids.ts";

/** No deck to bring: the server deals both seats one (R257). */
const MODE = "random";

/** The shortest time per move the server offers (`DAILY_TIME_PER_MOVE_DAYS`). */
const PER_MOVE_DAYS = 1;

/** The social socket opens and answers its first read within a poll and a render. */
const LIVE_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

/** A row reaches the live socket as it is written; a social read and a render bound it. */
const PUSH_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

describe("58 — the notification feed (issue #4039)", () => {
  it("R4039001 a daily move made over HTTP reaches the open tab as a toast and stays in the inbox's Recent section", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let twoName = "";
    let matchId = "";

    cy.freeAccount(one);
    cy.freeAccount(two);
    profileOf(one).then((profile) => {
      oneId = profile.id;
      oneName = profile.name;
    });
    profileOf(two).then((profile) => {
      twoId = profile.id;
      twoName = profile.name;
    });
    cy.then(() => {
      expect(oneName, "R1435: seat one has a username to be found by").to.not.eq("");
      expect(twoName, "seat two has a username to be told by").to.not.eq("");
      clearDaily(one);
      clearDaily(two);
      makeFriends(two, one, oneName, twoId, oneId);
    });

    // Seat one challenges seat two, who accepts: the game is made.
    cy.then(() => {
      cy.request<{ id: string }>({
        method: "POST",
        url: apiUrl("/api/daily/challenges"),
        headers: bearerHeaders(one.token),
        body: { friendId: twoId, mode: MODE, perMoveDays: PER_MOVE_DAYS },
      })
        .its("status")
        .should("eq", 201);
    });
    cy.then(() => {
      dailyOf(two).then((body) => {
        const challenge = body.challenges.incoming.find((got) => got.from.profileId === oneId);
        expect(challenge, "the daily challenge reached seat two").to.not.eq(undefined);
        cy.request<{ matchId: string }>({
          method: "POST",
          url: apiUrl(`/api/daily/challenges/${challenge?.id ?? ""}/accept`),
          headers: bearerHeaders(two.token),
          body: { mode: MODE },
        }).then((accepted) => {
          expect(accepted.status, "accepting makes the game").to.eq(201);
          matchId = accepted.body.matchId;
          expect(matchId, "the daily game's id").to.not.eq("");
        });
      });
    });

    // Both keep their hands; whoever goes first, seat one ends its turn if it holds the move, so the move is seat two's.
    cy.then(() => {
      dailyKeepHandAs(one, matchId);
      dailyKeepHandAs(two, matchId);
      dailyGameOf(one, matchId).then((game) => {
        if (game.hasMove) dailyEndTurnAs(one, matchId);
      });
    });
    cy.then(() => {
      dailyGameOf(two, matchId).its("hasMove").should("eq", true);
    });

    // Seat one's tab, its social socket live: the server pushes what it writes from here on.
    cy.visitAs(one, routes.play());
    cy.get(ts(inboxTestid.bell), { timeout: LIVE_TIMEOUT_MS }).should("have.attr", "data-live", "true");

    // Seat two ends its turn: seat one is told, as a toast.
    cy.then(() => {
      dailyEndTurnAs(two, matchId);
    });
    cy.then(() => {
      cy.get(`${ts(inboxTestid.toast)}[data-kind="dailyMove"]`, { timeout: PUSH_TIMEOUT_MS }).should("contain.text", `Your move against ${twoName}`);
    });

    // And it stays in the bell's Recent section.
    cy.get(ts(inboxTestid.bell)).click();
    cy.get(ts(inboxTestid.panel), { timeout: timeouts.view }).should("be.visible");
    cy.get(`${ts(inboxTestid.recentRow)}[data-kind="dailyMove"]`, { timeout: timeouts.view }).first().should("contain.text", `Your move against ${twoName}`);

    // The game ends, for the next run: a running daily game counts against the account's cap.
    cy.then(() => {
      dailyConcedeAs(one, matchId);
      clearDaily(one);
      clearDaily(two);
    });
  });
});
