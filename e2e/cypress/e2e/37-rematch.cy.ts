// 37 — rematch offers after a finished match, between the browser and cy.task("wsPlayer") (#728; §9.5, R672).
//
// Key assertion (issue #728): "Each flow above has a case that fails if the rendezvous, stakes check,
// presence gate, or ranked flag regresses". The flows: a room match refuses its double-or-nothing, makes
// nothing of one seat's offer and one unranked rematch of both seats' equal offers; a ranked match makes
// nothing of mismatched stakes and a ranked double-or-nothing of matched ones; the buttons show only while
// the opponent's match socket is open; a Conquest game offers no rematch and continues the series instead.
//
// Daily only: the super run plays every spec file, while a pull request's `e2e smoke` plays only SMOKE in
// scripts/shard-specs.mjs, which does not hold 37. No wait depends on REMATCH_OFFER_TTL_MS: every wait is a
// retried DOM assertion, and server state is read after the UI has changed.

import {
  MATCHMAKER_SWEEP_INTERVAL_SECONDS,
  RATING_WINDOW_FULL_AFTER_SECONDS,
  SERIES_POLL_SECONDS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import { type InstalledLoadout } from "../../support/commands.ts";
import {
  accounts,
  routes,
  seedFor,
  server,
  timeouts,
  type E2EAccount,
} from "../../support/config.ts";
import {
  MATCH_RANKS,
  PLAY_DECK_SELECT,
  PLAY_QUEUE,
  PLAY_STATUS,
  REMATCH_DOUBLE,
  REMATCH_INCOMING,
  REMATCH_OFFER,
  REMATCH_STATUS,
  SERIES_BANNER,
  SERIES_BANNER_CONTINUE,
  heroId,
  playModeId,
  ts,
  type QueueMode,
} from "../../support/testids.ts";

// Constants

/** Seconds to milliseconds. */
const MS_PER_SECOND = 1000;

/** Pairing waits out the rating window's widening to MAX_FAIR_GAP (R640001), a sweep, a poll and a render. */
const PAIRING_TIMEOUT_MS =
  (RATING_WINDOW_FULL_AFTER_SECONDS + MATCHMAKER_SWEEP_INTERVAL_SECONDS + SERIES_POLL_SECONDS) *
    MS_PER_SECOND +
  timeouts.view;

/** The buttons re-read the offers every poll, so a change lands within a poll plus a render. */
const POLL_TIMEOUT_MS = SERIES_POLL_SECONDS * MS_PER_SECOND + timeouts.view;

/** A plain rematch's stakes, mirroring STAKE_NORMAL in crates/server/src/api/rematch.rs. */
const STAKE_NORMAL = 1;
/** Double-or-nothing's stakes, mirroring STAKE_DOUBLE in crates/server/src/api/rematch.rs. */
const STAKE_DOUBLE = 2;

/** The Node second-player client, as in spec 06. */
const SEAT_TWO = "seat-two";

/** A match page, capturing its id. */
const MATCH_PATH = /^\/match\/([^/]+)$/;

/** installLoadout's default deck index. */
const FIXTURE_DECK = 0;

/** The first trio slot a series pick names. */
const FIRST_SLOT = 0;

/** The words Rematch.tsx renders for an incoming offer and an offer that is waiting. */
const WORDS = {
  incomingNormal: "Your opponent wants a rematch.",
  incomingDouble: "Your opponent wants double-or-nothing.",
  offeredNormal: "You offered a rematch.",
} as const;

// HTTP contract

type ErrorBody = { error: { code: string; message: string } };

type JoinBody = {
  matchId: string | null;
  seriesId: string | null;
  code: string;
  seat: "p1" | "p2";
  mode: QueueMode;
};

type MeBody = { currentMatchId: string | null; currentSeriesId?: string | null };

/** Both offers, the opponent's presence, the created game and its mode, as rematch.rs answers them. */
type RematchStatusBody = {
  youOffered: number | null;
  opponentOffer: number | null;
  opponentHere: boolean;
  matchId: string | null;
  mode: QueueMode;
};

type RematchOfferBody = { matchId: string | null };

/** Whether the match is ranked, as ranked.rs answers it. */
type RanksBody = { ranked: boolean };

type SeriesView = { status: "picking" | "playing" | "over"; currentMatchId: string | null };

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy
    .request<MeBody>({ method: "GET", url: api("/api/auth/me"), headers: bearer(account) })
    .its("body");
}

function landedOn(pattern: RegExp, timeout: number): Cypress.Chainable<string> {
  return cy
    .location("pathname", { timeout })
    .should("match", pattern)
    .then((pathname) => pattern.exec(pathname)?.[1] ?? "");
}

function installed(value: InstalledLoadout | null): InstalledLoadout {
  expect(value, "cy.installLoadout has answered").to.not.eq(null);
  return value as InstalledLoadout;
}

/** Offer a rematch of `matchId` at `stakes` as `account`; null while the seats disagree. */
function offerAs(account: E2EAccount, matchId: string, stakes: number): Cypress.Chainable<RematchOfferBody> {
  return cy
    .request<RematchOfferBody>({
      method: "POST",
      url: api(`/api/matches/${matchId}/rematch`),
      headers: bearer(account),
      body: { stakes },
    })
    .its("body");
}

/** Read `account`'s rematch status for `matchId`. */
function statusAs(account: E2EAccount, matchId: string): Cypress.Chainable<RematchStatusBody> {
  return cy
    .request<RematchStatusBody>({
      method: "GET",
      url: api(`/api/matches/${matchId}/rematch`),
      headers: bearer(account),
    })
    .its("body");
}

/** The refusal a rematch endpoint answers when the match cannot offer one. */
function refused(
  account: E2EAccount,
  method: "GET" | "POST",
  matchId: string,
  stakes?: number,
): Cypress.Chainable<Cypress.Response<ErrorBody>> {
  return cy.request<ErrorBody>({
    method,
    url: api(`/api/matches/${matchId}/rematch`),
    headers: bearer(account),
    ...(stakes === undefined ? {} : { body: { stakes } }),
    failOnStatusCode: false,
  });
}

/** Whether `matchId` is ranked. */
function ranksOf(account: E2EAccount, matchId: string): Cypress.Chainable<RanksBody> {
  return cy
    .request<RanksBody>({ method: "GET", url: api(`/api/matches/${matchId}/ranks`), headers: bearer(account) })
    .its("body");
}

/** Both fixture accounts sit in `matchId` (or in none). */
function bothIn(matchId: string | null): void {
  for (const account of [accounts.p1(), accounts.p2()]) {
    me(account).its("currentMatchId").should("eq", matchId);
  }
}

/** Seat two concedes over its own socket (§2.5 stamps the socket's seat; `playerId` is only action-shape data). */
function seatTwoConcedes(): void {
  cy.wsPlayer({ action: "send", name: SEAT_TWO, body: { type: "concede", playerId: "p2" } });
}

/** The match page the offers made, reached from `from`. */
function movedOn(from: string, timeout: number): Cypress.Chainable<string> {
  return cy
    .location("pathname", { timeout })
    .should((pathname) => {
      expect(pathname, "the offers made a new game").to.match(MATCH_PATH);
      expect(pathname, "the new game is not the old one").to.not.eq(routes.match(from));
    })
    .then((pathname) => MATCH_PATH.exec(pathname)?.[1] ?? "");
}

/**
 * A finished Best-of-1 room match: seat one hosts, seat two joins, the browser plays seat one while
 * the Node client plays seat two, and seat two concedes. Yields the finished match's id.
 */
function finishedRoomMatch(seed: string, seatOneDeck: string, seatTwoDeck: string): Cypress.Chainable<string> {
  const seatOne = accounts.p1();
  const seatTwo = accounts.p2();
  let code = "";
  let matchId = "";
  cy.request<{ code: string }>({
    method: "POST",
    url: api("/api/rooms"),
    headers: bearer(seatOne),
    body: { mode: "bo1", deckId: seatOneDeck, seed },
  }).then((created) => {
    code = created.body.code;
  });
  cy.then(() => {
    cy.request<JoinBody>({
      method: "POST",
      url: api(`/api/rooms/${code}/join`),
      headers: bearer(seatTwo),
      body: { mode: "bo1", deckId: seatTwoDeck },
    }).then((joined) => {
      matchId = joined.body.matchId ?? "";
      expect(matchId, "the join made a match").to.not.eq("");
    });
  });
  cy.then(() => {
    cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId });
    cy.visitAs(seatOne, routes.match(matchId));
  });
  cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
  cy.then(() => {
    seatTwoConcedes();
  });
  cy.expectResult("Win");
  return cy.then(() => matchId);
}

// ---------------------------------------------------------------------------------------------

describe("37 rematch — offers after a finished match (§9.5, R672)", () => {
  beforeEach(() => {
    cy.freeAccount(accounts.p1());
    cy.freeAccount(accounts.p2());
  });

  it("a room match: R672 refuses its double-or-nothing, one seat's offer makes nothing, and both seats' offers make one unranked rematch", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let matchId = "";
    let rematchId = "";

    cy.installLoadout(seatOne, "19-modes-a").then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });
    cy.then(() => {
      finishedRoomMatch(
        seedFor("37-rematch-room"),
        installed(mine).deckIds[FIXTURE_DECK] ?? "",
        installed(theirs).deckIds[FIXTURE_DECK] ?? "",
      ).then((id) => {
        matchId = id;
      });
    });

    cy.get(ts(REMATCH_OFFER), { timeout: POLL_TIMEOUT_MS }).should("be.visible").and("not.be.disabled");
    cy.get(ts(MATCH_RANKS), { timeout: timeouts.view }).should("contain.text", "Unranked match");
    cy.get(ts(REMATCH_DOUBLE)).should("be.disabled");
    cy.then(() => {
      refused(seatTwo, "POST", matchId, STAKE_DOUBLE).should((response) => {
        expect(response.status).to.eq(422);
        expect(response.body.error.code).to.eq("double_requires_ranked");
      });
    });

    cy.get(ts(REMATCH_OFFER)).click();
    cy.get(ts(REMATCH_STATUS), { timeout: timeouts.view }).should("contain.text", WORDS.offeredNormal);
    cy.then(() => {
      statusAs(seatTwo, matchId).should((body) => {
        expect(body).to.deep.eq({
          youOffered: null,
          opponentOffer: STAKE_NORMAL,
          opponentHere: true,
          matchId: null,
          mode: "bo1",
        });
      });
      bothIn(null);
      cy.location("pathname").should("eq", routes.match(matchId));
    });

    cy.then(() => {
      offerAs(seatTwo, matchId, STAKE_NORMAL).should((body) => {
        expect(body.matchId, "equal offers made one rematch").to.be.a("string").and.not.eq("").and.not.eq(matchId);
        rematchId = body.matchId ?? "";
      });
    });
    cy.then(() => {
      cy.location("pathname", { timeout: POLL_TIMEOUT_MS }).should("eq", routes.match(rematchId));
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      offerAs(seatOne, matchId, STAKE_NORMAL).its("matchId").should("eq", rematchId);
      bothIn(rematchId);
    });

    cy.then(() => {
      // A room's rematch is unranked: only a ranked match rates (R604).
      ranksOf(seatOne, rematchId).its("ranked").should("eq", false);
    });
    cy.then(() => {
      cy.concedeAs(seatOne, rematchId).its("ok").should("eq", true);
    });
  });

  it("a ranked match: mismatched stakes make nothing, and the R672 double-or-nothing both seats offer is ranked", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let matchId = "";
    let doubleId = "";

    cy.installLoadout(seatOne, "19-modes-a").then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });

    cy.visitAs(seatOne, routes.play());
    cy.get(ts(playModeId("bo1")), { timeout: timeouts.view }).check();
    cy.get(ts(playModeId("bo1"))).should("be.checked");
    cy.then(() => {
      const chosen = installed(mine).deckIds[FIXTURE_DECK] ?? "";
      cy.get(ts(PLAY_DECK_SELECT), { timeout: timeouts.view }).select(chosen);
      cy.get(ts(PLAY_DECK_SELECT)).should("have.value", chosen);
    });
    cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(PLAY_STATUS), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      cy.request({
        method: "POST",
        url: api("/api/queue"),
        headers: bearer(seatTwo),
        body: { mode: "bo1", deckId: installed(theirs).deckIds[FIXTURE_DECK] ?? "", seed: seedFor("37-rematch-ranked") },
      }).should((response) => {
        expect(response.status, "R143: the queue takes an end-to-end seed").to.eq(200);
      });
      // R897000: seat two is a queued client, so it polls like one while the rating window widens.
      cy.stayQueued(seatTwo, SERIES_POLL_SECONDS * MS_PER_SECOND);
    });
    landedOn(MATCH_PATH, PAIRING_TIMEOUT_MS).then((id) => {
      matchId = id;
    });

    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId });
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      seatTwoConcedes();
    });
    cy.expectResult("Win");
    cy.then(() => {
      // A queue match rates (R604).
      ranksOf(seatOne, matchId).its("ranked").should("eq", true);
    });

    cy.get(ts(MATCH_RANKS), { timeout: timeouts.view }).should("contain.text", "Ranked match");
    cy.get(ts(REMATCH_OFFER), { timeout: POLL_TIMEOUT_MS }).should("be.visible");
    cy.get(ts(REMATCH_DOUBLE)).should("not.be.disabled");
    cy.then(() => {
      offerAs(seatTwo, matchId, STAKE_DOUBLE).its("matchId").should("eq", null);
    });

    cy.get(ts(REMATCH_INCOMING), { timeout: POLL_TIMEOUT_MS }).should("contain.text", WORDS.incomingDouble);
    cy.get(ts(REMATCH_OFFER)).click();
    cy.get(ts(REMATCH_STATUS), { timeout: timeouts.view }).should("contain.text", WORDS.offeredNormal);
    cy.get(ts(REMATCH_INCOMING)).should("contain.text", WORDS.incomingDouble);
    cy.then(() => {
      statusAs(seatTwo, matchId).should((body) => {
        expect(body.youOffered).to.eq(STAKE_DOUBLE);
        expect(body.opponentOffer).to.eq(STAKE_NORMAL);
        expect(body.matchId).to.eq(null);
      });
      bothIn(null);
      cy.location("pathname").should("eq", routes.match(matchId));
    });

    cy.get(ts(REMATCH_DOUBLE)).should("not.be.disabled").click();
    cy.then(() => {
      movedOn(matchId, timeouts.view).then((id) => {
        doubleId = id;
      });
    });
    cy.then(() => {
      statusAs(seatTwo, matchId).its("matchId").should("eq", doubleId);
      bothIn(doubleId);
    });

    cy.then(() => {
      ranksOf(seatOne, doubleId).its("ranked").should("eq", true);
    });
    cy.then(() => {
      cy.concedeAs(seatOne, doubleId).its("ok").should("eq", true);
    });
  });

  it("the buttons show only while the opponent's match socket is open (§9.5)", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let matchId = "";

    cy.installLoadout(seatOne, "19-modes-a").then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });
    cy.then(() => {
      finishedRoomMatch(
        seedFor("37-rematch-presence"),
        installed(mine).deckIds[FIXTURE_DECK] ?? "",
        installed(theirs).deckIds[FIXTURE_DECK] ?? "",
      ).then((id) => {
        matchId = id;
      });
    });

    cy.get(ts(REMATCH_OFFER), { timeout: POLL_TIMEOUT_MS }).should("be.visible");
    cy.then(() => {
      statusAs(seatOne, matchId).its("opponentHere").should("eq", true);
    });

    cy.then(() => {
      cy.wsPlayer({ action: "disconnect", name: SEAT_TWO });
    });
    cy.get(ts(REMATCH_OFFER), { timeout: POLL_TIMEOUT_MS }).should("not.exist");
    cy.get(ts(REMATCH_DOUBLE), { timeout: POLL_TIMEOUT_MS }).should("not.exist");
    cy.then(() => {
      statusAs(seatOne, matchId).its("opponentHere").should("eq", false);
    });

    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId }).should(
        (result) => {
          expect(result.view?.["result"] ?? null, "a finished match's view carries its result").to.not.eq(null);
        },
      );
    });
    cy.get(ts(REMATCH_OFFER), { timeout: POLL_TIMEOUT_MS }).should("be.visible");
    cy.then(() => {
      statusAs(seatOne, matchId).its("opponentHere").should("eq", true);
    });
  });

  it("a Conquest game offers no rematch: the series continues instead (§9.5)", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let code = "";
    let seriesId = "";
    let gameId = "";

    cy.installLoadout(seatOne, "19-modes-a").then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne),
        body: { mode: "bo3", trioId: installed(mine).trioId, seed: seedFor("37-rematch-conquest") },
      }).then((created) => {
        code = created.body.code;
      });
    });
    cy.then(() => {
      cy.request<JoinBody>({
        method: "POST",
        url: api(`/api/rooms/${code}/join`),
        headers: bearer(seatTwo),
        body: { mode: "bo3", trioId: installed(theirs).trioId },
      }).then((joined) => {
        // A Conquest join makes the series, no match yet (R264).
        expect(joined.body.matchId).to.eq(null);
        expect(joined.body.seriesId, "the join made a series").to.be.a("string").and.not.eq("");
        seriesId = joined.body.seriesId ?? "";
      });
    });
    cy.then(() => {
      cy.request({
        method: "POST",
        url: api(`/api/series/${seriesId}/pick`),
        headers: bearer(seatOne),
        body: { slot: FIRST_SLOT },
      })
        .its("status")
        .should("eq", 200);
    });
    cy.then(() => {
      cy.request<SeriesView>({
        method: "POST",
        url: api(`/api/series/${seriesId}/pick`),
        headers: bearer(seatTwo),
        body: { slot: FIRST_SLOT },
      }).then((second) => {
        expect(second.body.status).to.eq("playing");
        expect(second.body.currentMatchId, "both picks named the game").to.be.a("string").and.not.eq("");
        gameId = second.body.currentMatchId ?? "";
      });
    });

    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId: gameId });
      cy.visitAs(seatOne, routes.match(gameId));
    });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    cy.then(() => {
      seatTwoConcedes();
    });
    cy.expectResult("Win");

    cy.get(ts(SERIES_BANNER), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(SERIES_BANNER_CONTINUE), { timeout: POLL_TIMEOUT_MS }).should("be.visible");
    cy.get(ts(REMATCH_OFFER)).should("not.exist");
    cy.get(ts(REMATCH_DOUBLE)).should("not.exist");
    cy.then(() => {
      refused(seatOne, "GET", gameId).should((response) => {
        expect(response.status).to.eq(422);
        expect(response.body.error.code).to.eq("series_game");
      });
      refused(seatTwo, "POST", gameId, STAKE_NORMAL).should((response) => {
        expect(response.status).to.eq(422);
        expect(response.body.error.code).to.eq("series_game");
      });
    });
    cy.then(() => {
      cy.request({
        method: "POST",
        url: api(`/api/series/${seriesId}/forfeit`),
        headers: bearer(seatTwo),
      })
        .its("body.status")
        .should("eq", "over");
    });
  });
});

