// 52 — daily games (#3727): seat one challenges seat two to a daily game from the friends list in the
// browser, seat two accepts and moves over HTTP (`support/daily.ts`), and "Your move" passes between them:
// on the Daily screen, on the board's clock, and back again once seat two ends its turn. Networked
// (`E2E=1 cargo run --release -p jackioh-server`); daily only, as every spec outside
// scripts/shard-specs.mjs's SMOKE. No wait depends on a server clock: every wait is a retried DOM or HTTP
// assertion bounded by the poll that brings it (the social read's `dailyWaiting`).

import { PRESENCE_HEARTBEAT_MS, SOCIAL_POLL_MS } from "../../../apps/web/src/wire/serverConfig.ts";
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
import {
  DAILY_BOARD,
  DAILY_CLOCK,
  DAILY_GAME_ROW,
  DAILY_SECTION,
  DAILY_TIME_LEFT,
  END_TURN,
  PLAY_DAILY,
  PLAY_DAILY_BADGE,
  TOGETHER_CHALLENGE,
  TOGETHER_CHALLENGE_FORM,
  TOGETHER_CHALLENGE_SEND,
  TOGETHER_FRIEND_ROW,
  dailyModeId,
  togetherChallengeKindId,
  ts,
} from "../../support/testids.ts";

/** Mirrors `paths.together` in apps/web/src/net/navigate.ts. */
const TOGETHER = "/together";

/** A friend made over HTTP reaches the open list within its poll and a render. */
const FRIENDS_TIMEOUT_MS = PRESENCE_HEARTBEAT_MS + timeouts.view;

/** A move made over HTTP reaches the screen within a social read (its `dailyWaiting`) and a read of the list or board. */
const MOVE_TIMEOUT_MS = SOCIAL_POLL_MS + timeouts.view;

/** No deck to bring: the server deals both seats one (R257). */
const MODE = "random";

function gameRow(matchId: string): string {
  return `${ts(DAILY_GAME_ROW)}[data-match-id="${matchId}"]`;
}

function inSection(section: string, matchId: string): string {
  return `${ts(DAILY_SECTION)}[data-section="${section}"] ${gameRow(matchId)}`;
}

describe("52 Daily games — a friend's daily challenge, and the move passing between them", () => {
  it("challenges a friend to a daily game in the browser; the friend accepts and moves; Your move passes back and forth", () => {
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

    // #3727: Challenge is offered for any friend, online or not; its dialog's Daily sends a daily challenge.
    cy.visitAs(one, TOGETHER);
    cy.then(() => {
      cy.get(`${ts(TOGETHER_FRIEND_ROW)}[data-profile-id="${twoId}"]`, { timeout: FRIENDS_TIMEOUT_MS }).within(() => {
        cy.get(ts(TOGETHER_CHALLENGE)).click();
      });
    });
    cy.get(ts(TOGETHER_CHALLENGE_FORM), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(togetherChallengeKindId("daily"))).click();
    cy.get(ts(TOGETHER_CHALLENGE_FORM)).should("have.attr", "data-kind", "daily");
    cy.get(ts(dailyModeId(MODE)), { timeout: timeouts.view }).click();
    cy.get(ts(TOGETHER_CHALLENGE_SEND), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(TOGETHER_CHALLENGE_FORM), { timeout: timeouts.view }).should("not.exist");
    // A daily challenge waits: the challenger stays where they were.
    cy.location("pathname").should("eq", TOGETHER);

    // Seat two finds it among its challenges and accepts with its own pick.
    cy.then(() => {
      dailyOf(two).then((body) => {
        const challenge = body.challenges.incoming.find((got) => got.from.profileId === oneId);
        expect(challenge, "the daily challenge reached seat two").to.not.eq(undefined);
        expect(challenge?.mode, "it plays the mode it was sent with").to.eq(MODE);
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

    // Both seats hold the mulligan: the game waits on seat one too, on its Daily screen.
    cy.then(() => {
      cy.visitAs(one, routes.daily());
      cy.get(inSection("yourMove", matchId), { timeout: MOVE_TIMEOUT_MS }).should("have.attr", "data-has-move", "true");
    });

    // Both keep their hands; whoever goes first, seat two ends its turn if it holds the move, so the move is seat one's.
    cy.then(() => {
      dailyKeepHandAs(two, matchId);
      dailyKeepHandAs(one, matchId);
      dailyGameOf(two, matchId).then((game) => {
        if (game.hasMove) dailyEndTurnAs(two, matchId);
      });
    });

    // Seat one: Your move on the list, then on the board's clock.
    cy.then(() => {
      cy.visitAs(one, routes.daily());
      cy.get(inSection("yourMove", matchId), { timeout: MOVE_TIMEOUT_MS }).within(() => {
        cy.get(ts(DAILY_TIME_LEFT)).should("contain.text", "Your move");
        cy.get("a").click();
      });
      cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.dailyGame(matchId));
    });
    cy.get(ts(DAILY_CLOCK), { timeout: timeouts.view }).should("have.attr", "data-holder", "you").and("contain.text", "Your move");

    // End turn and send: the move goes to seat two.
    cy.get(ts(END_TURN), { timeout: timeouts.view }).should("contain.text", "End turn and send").and("not.be.disabled").click();
    cy.get(ts(DAILY_BOARD), { timeout: timeouts.view }).should("have.attr", "data-has-move", "false");
    cy.get(ts(DAILY_CLOCK)).should("have.attr", "data-holder", "opponent").and("contain.text", "Their move");
    cy.then(() => {
      dailyGameOf(two, matchId).its("hasMove").should("eq", true);
    });

    // Seat two ends its turn: the move comes back, and the open board reads it (the social read's dailyWaiting).
    cy.then(() => {
      dailyEndTurnAs(two, matchId);
    });
    cy.get(ts(DAILY_BOARD), { timeout: MOVE_TIMEOUT_MS }).should("have.attr", "data-has-move", "true");
    cy.get(ts(DAILY_CLOCK)).should("have.attr", "data-holder", "you");

    // The Play screen's Daily games carries the count of games waiting on seat one.
    cy.visitAs(one, routes.play());
    cy.get(ts(PLAY_DAILY), { timeout: timeouts.view }).should("have.attr", "href", routes.daily());
    cy.get(ts(PLAY_DAILY_BADGE), { timeout: MOVE_TIMEOUT_MS }).should("be.visible");

    // Seat two sees the game under Their move on its own read; then the game ends, for the next run.
    cy.then(() => {
      dailyOf(two).then((body) => {
        const row = body.games.find((game) => game.matchId === matchId);
        expect(row?.hasMove, "the move is seat one's, not seat two's").to.eq(false);
      });
      dailyConcedeAs(one, matchId);
      clearDaily(one);
      clearDaily(two);
    });
  });
});
