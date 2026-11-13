// 53 — daily games (#3727, R3727000): a move whose time runs out loses the game for the seat that held it.
// Seat one challenges seat two from the New daily game dialog in the browser; seat two accepts and both
// keep their hands over HTTP (`support/daily.ts`), and the move is seat one's. The E2E server's daily clock
// (`POST /api/e2e/daily-clock`) then moves past the hold's deadline and the sweep settles it: seat one's
// board says "Lost on time", its list "Lost on time" among the finished, and seat two's read says it won.
// Networked (`E2E=1 cargo run --release -p jackioh-server`); daily only. The clock is put back after.

import { PRESENCE_HEARTBEAT_MS, SOCIAL_POLL_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, routes, timeouts } from "../../support/config.ts";
import {
  clearDaily,
  dailyEndTurnAs,
  dailyGameOf,
  dailyKeepHandAs,
  dailyOf,
  makeFriends,
  profileOf,
  setDailyClock,
  sweepDaily,
} from "../../support/daily.ts";
import {
  DAILY_CLOCK,
  DAILY_FRIEND,
  DAILY_GAME_ROW,
  DAILY_NEW_DIALOG,
  DAILY_NEW_GAME,
  DAILY_SECTION,
  DAILY_SEND,
  DAILY_TIME_LEFT,
  RESULT_OVERLAY,
  dailyKindId,
  dailyModeId,
  ts,
} from "../../support/testids.ts";

/** A move made over HTTP reaches the screen within a social read and a read of the list or board. */
const MOVE_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

/** The New daily game dialog's friends list is read when it opens; a friend made just before is in it. */
const FRIENDS_TIMEOUT_MS = PRESENCE_HEARTBEAT_MS + timeouts.view;

/** How far past the hold's deadline the clock is moved: well clear of any rounding to the hour. */
const PAST_DEADLINE_MS = 60 * 60 * 1000;

/** No deck to bring: the server deals both seats one (R257). */
const MODE = "random";

/** The words the board and the list say to the seat whose move ran out (apps/web/src/daily). */
const LOST_ON_TIME = "Lost on time";

function gameRow(matchId: string): string {
  return `${ts(DAILY_GAME_ROW)}[data-match-id="${matchId}"]`;
}

describe("53 Daily games — a move whose time runs out", () => {
  beforeEach(() => {
    setDailyClock(0);
  });

  afterEach(() => {
    setDailyClock(0);
  });

  it("R3727000 the seat holding the move when its time runs out loses: its board says Lost on time", () => {
    const one = accounts.p1();
    const two = accounts.p2();
    let oneId = "";
    let oneName = "";
    let twoId = "";
    let matchId = "";

    cy.freeAccount(one);
    cy.freeAccount(two);
    profileOf(one).then((profile) => {
      oneId = profile.id;
      oneName = profile.name;
    });
    profileOf(two).then((profile) => {
      twoId = profile.id;
    });
    cy.then(() => {
      expect(oneName, "R1435: seat one has a username to be found by").to.not.eq("");
      clearDaily(one);
      clearDaily(two);
      makeFriends(two, one, oneName, twoId, oneId);
    });

    // The New daily game dialog: Challenge a friend, All random, the first time per move.
    cy.visitAs(one, routes.daily());
    cy.get(ts(DAILY_NEW_GAME), { timeout: timeouts.view }).click();
    cy.get(ts(DAILY_NEW_DIALOG), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(dailyKindId("friend"))).click();
    cy.then(() => {
      cy.get(ts(DAILY_FRIEND), { timeout: FRIENDS_TIMEOUT_MS }).select(twoId);
    });
    cy.get(ts(dailyModeId(MODE))).click();
    cy.get(ts(DAILY_SEND), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(DAILY_NEW_DIALOG), { timeout: timeouts.view }).should("not.exist");
    cy.get(`${ts(DAILY_SECTION)}[data-section="outgoing"]`, { timeout: timeouts.view }).should("exist");

    // Seat two accepts; both keep their hands; seat two ends its turn if it went first.
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
          matchId = accepted.body.matchId;
          expect(matchId, "the daily game's id").to.not.eq("");
        });
      });
    });
    cy.then(() => {
      dailyKeepHandAs(two, matchId);
      dailyKeepHandAs(one, matchId);
      dailyGameOf(two, matchId).then((game) => {
        if (game.hasMove) dailyEndTurnAs(two, matchId);
      });
    });

    // Seat one holds the move, and its board says so.
    cy.then(() => {
      dailyGameOf(one, matchId).its("hasMove").should("eq", true);
      cy.visitAs(one, routes.dailyGame(matchId));
    });
    cy.get(ts(DAILY_CLOCK), { timeout: MOVE_TIMEOUT_MS }).should("have.attr", "data-holder", "you");

    // The clock passes the hold's deadline (its whole time per move after it began), and the sweep settles it.
    cy.then(() => {
      dailyGameOf(one, matchId).then((game) => {
        const hold = game.holds.find((entry) => entry.player === game.seat);
        expect(hold, "seat one holds the move").to.not.eq(undefined);
        setDailyClock((hold?.deadline ?? 0) - (hold?.since ?? 0) + PAST_DEADLINE_MS);
        sweepDaily();
      });
    });

    // Seat one's board: the game is over, lost on time.
    cy.then(() => {
      cy.visitAs(one, routes.dailyGame(matchId));
    });
    cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.view })
      .should("have.attr", "data-outcome", "loss")
      .and("have.attr", "data-reason", "disconnect")
      .and("contain.text", LOST_ON_TIME);
    cy.get(ts(DAILY_CLOCK)).should("not.exist");
    cy.then(() => {
      dailyGameOf(one, matchId).then((game) => {
        expect(game.over, "the game is over").to.eq(true);
        expect(game.lostOnTime, "R3727000: it ended on time").to.eq(true);
      });
      dailyGameOf(two, matchId).its("lostOnTime").should("eq", true);
    });

    // Seat one's list has it among the finished, lost on time.
    cy.then(() => {
      cy.visitAs(one, routes.daily());
      cy.get(`${ts(DAILY_SECTION)}[data-section="finished"] ${gameRow(matchId)}`, { timeout: timeouts.view })
        .find(ts(DAILY_TIME_LEFT))
        .should("contain.text", LOST_ON_TIME);
      clearDaily(one);
      clearDaily(two);
    });
  });
});
