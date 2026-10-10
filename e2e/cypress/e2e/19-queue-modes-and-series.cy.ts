// BUILD M8 spec 19: Best-of-1, All Random, Conquest, and rooms (R257, R258, R264, R330, R338, R765).
// #21 Hinder is the selected deck's observable opening-hand card.
// Browser p1 uses the UI; p2 seeds matches over HTTP/socket (R143). Opening hands prove selected decks (R245; §10.8).
// Balance each fixture account's rated endings and clean both before every test; pairing allows §9.5's R108 window.
// BUILD M5-T1 fixes testids; M6/M7 supply the server, actor, queue and results.

import {
  MATCHMAKER_SWEEP_INTERVAL_SECONDS,
  RATING_WINDOW_UNCAPPED_AFTER_SECONDS,
  SERIES_POLL_SECONDS,
  SERIES_WINS_NEEDED,
} from "../../../apps/web/src/wire/serverConfig.ts";
import { TOKEN_NAMES, tokenId } from "../../support/cards.ts";
import { INSTALLED_TRIO_NAME, type InstalledLoadout } from "../../support/commands.ts";
import {
  accounts,
  constants,
  routes,
  seedFor,
  server,
  timeouts,
  type E2EAccount,
} from "../../support/config.ts";
import {
  LIVE_GAME_BANNER,
  LIVE_GAME_REJOIN,
  NAV_BACK,
  PLAY_CREATE_ROOM,
  PLAY_DECK_SELECT,
  PLAY_QUEUE,
  PLAY_ROOM_CODE,
  PLAY_ROOM_MODE,
  PLAY_STATUS,
  PLAY_TRIO_SELECT,
  PLAY_VERDICT,
  RESULT_OVERLAY,
  SERIES_BANNER,
  SERIES_BANNER_CONTINUE,
  SERIES_BANNER_RESULT,
  SERIES_LOCK_IN,
  SERIES_OPPONENT_STATUS,
  SERIES_PICKER,
  SERIES_RESULT,
  SERIES_SCORE,
  SERIES_SCREEN,
  handCardId,
  playModeId,
  seriesBannerOpponentDeckId,
  seriesBannerYourDeckId,
  seriesDeckId,
  seriesGameId,
  seriesOpponentDeckId,
  seriesPickId,
  ts,
  type QueueMode,
} from "../../support/testids.ts";

// Constants

const MS_PER_SECOND = 1000;

/** Allow the uncapped §9.5/R108 window, a sweep, poll, and render. */
const PAIRING_TIMEOUT_MS =
  (RATING_WINDOW_UNCAPPED_AFTER_SECONDS + MATCHMAKER_SWEEP_INTERVAL_SECONDS + SERIES_POLL_SECONDS) *
    MS_PER_SECOND +
  timeouts.view;

/** Allow a poll and render. */
const SERIES_POLL_TIMEOUT_MS = SERIES_POLL_SECONDS * MS_PER_SECOND + timeouts.view;

/** R245 Tokens are the only hand cards not from a deck. */
const TOKEN_IDS: ReadonlySet<string> = new Set(Object.keys(TOKEN_NAMES).map((index) => tokenId(index)));

const MATCH_PATH = /^\/match\/([^/]+)$/;
const SERIES_PATH = /^\/series\/([^/]+)$/;

const SEAT_TWO = "seat-two";

/** Best-of-1 selects fixture deck 2, not deck 1. */
const CHOSEN_DECK_INDEX = 1;

const GAME_ONE_PICKS = { browser: CHOSEN_DECK_INDEX, seatTwo: 2 } as const;
/** R330 permits seat two's losing deck again. */
const GAME_TWO_PICKS = { browser: 0, seatTwo: 2 } as const;
/** R332 auto-picks the browser's final deck. */
const GAME_THREE_PICKS = { browser: 2, seatTwo: 0 } as const;

// HTTP contract

type EnqueueBody = {
  ticketId: string;
  status: "open" | "matched" | "cancelled";
  matchId: string | null;
  seriesId: string | null;
  mode: QueueMode;
};

type ErrorBody = { error: { code: string; message: string; details?: unknown } };

type JoinBody = {
  matchId: string | null;
  seriesId: string | null;
  code: string;
  seat: "p1" | "p2";
  mode: QueueMode;
};

type MeBody = { currentMatchId: string | null; currentSeriesId?: string | null };

/** R612: read season games, never hidden rating. */
type ProfileBody = { record: { wins: number; losses: number; draws: number } };

type RankBody = { record: { games: number; wins: number; losses: number; draws: number } };

type SeriesView = {
  id: string;
  status: "picking" | "playing" | "over";
  gameNo: number;
  winsNeeded: number;
  maxGames: number;
  currentMatchId: string | null;
  ranked: boolean;
  you: {
    seat: "p1" | "p2";
    wins: number;
    trioName: string;
    decks: { slot: number; name: string; cards: string[]; won: boolean; games: number }[];
    pick: number | null;
    autoPick: boolean;
  };
  opponent: { wins: number; decks: Record<string, unknown>[]; picked: boolean } & Record<string, unknown>;
  games: {
    gameNo: number;
    matchId: string;
    yourSlot: number;
    opponentSlot: number;
    youWentFirst: boolean;
    result: "win" | "loss" | "draw" | null;
  }[];
  result: {
    outcome: "win" | "loss" | "draw" | "abandoned";
    endReason: string;
    ranked: boolean;
  } | null;
};

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

function enqueue(account: E2EAccount, body: Record<string, unknown>): Cypress.Chainable<Cypress.Response<EnqueueBody>> {
  return cy.request<EnqueueBody>({ method: "POST", url: api("/api/queue"), headers: bearer(account), body });
}

function dequeue(account: E2EAccount): void {
  cy.request({ method: "DELETE", url: api("/api/queue"), headers: bearer(account) })
    .its("status")
    .should("eq", 200);
}

function me(account: E2EAccount): Cypress.Chainable<MeBody> {
  return cy
    .request<MeBody>({ method: "GET", url: api("/api/auth/me"), headers: bearer(account) })
    .its("body");
}

function profile(account: E2EAccount): Cypress.Chainable<ProfileBody> {
  return cy
    .request<ProfileBody>({ method: "GET", url: api("/api/profile"), headers: bearer(account) })
    .its("body");
}

function ownRank(account: E2EAccount): Cypress.Chainable<RankBody> {
  return cy
    .request<RankBody>({ method: "GET", url: api("/api/ranked"), headers: bearer(account) })
    .its("body");
}

function seriesAs(account: E2EAccount, seriesId: string): Cypress.Chainable<SeriesView> {
  return cy
    .request<SeriesView>({ method: "GET", url: api(`/api/series/${seriesId}`), headers: bearer(account) })
    .its("body");
}

function pickAs(account: E2EAccount, seriesId: string, slot: number): Cypress.Chainable<SeriesView> {
  return cy
    .request<SeriesView>({
      method: "POST",
      url: api(`/api/series/${seriesId}/pick`),
      headers: bearer(account),
      body: { slot },
    })
    .its("body");
}

function forfeitAs(account: E2EAccount, seriesId: string): Cypress.Chainable<SeriesView> {
  return cy
    .request<SeriesView>({ method: "POST", url: api(`/api/series/${seriesId}/forfeit`), headers: bearer(account) })
    .its("body");
}

// Hand reading

const HAND_CARD = `[data-testid^="${handCardId("")}"]`;

function browserHand(): Cypress.Chainable<string[]> {
  return cy
    .get(HAND_CARD, { timeout: timeouts.view })
    .should("have.length.at.least", constants.OPENING_DRAW[0])
    .then(($cards) => $cards.map((_index, element) => element.getAttribute("data-def-id") ?? "").get());
}

/** §10.8 sends the viewer's own hand as cards. */
function socketHand(view: Record<string, unknown> | null | undefined): string[] {
  const you = (view?.you ?? {}) as { hand?: unknown };
  expect(Array.isArray(you.hand), "the viewer's own hand arrives as cards, not a count (§10.8)").to.eq(true);
  return (Array.isArray(you.hand) ? (you.hand as { defId?: string }[]) : []).map((card) => card.defId ?? "");
}

/** Require enough non-Token cards to prove the selected deck. */
function expectDealtFrom(hand: readonly string[], deck: readonly string[], others: readonly (readonly string[])[], label: string): void {
  const drawn = hand.filter((id) => !TOKEN_IDS.has(id));
  expect(drawn.length, `${label}: an opening hand (§2.1)`).to.be.at.least(constants.OPENING_DRAW[0]);
  for (const id of drawn) {
    expect(deck, `${label}: ${id} was drawn from this deck`).to.include(id);
    for (const other of others) {
      expect(other, `${label}: ${id} is not from another deck`).to.not.include(id);
    }
  }
}

// Lobby

function openLobby(account: E2EAccount, mode: QueueMode): void {
  cy.visitAs(account, routes.play());
  cy.get(ts(playModeId(mode)), { timeout: timeouts.view }).check();
  cy.get(ts(playModeId(mode))).should("be.checked");
}

/** Queue browser first so R335 gives it series seat p1. */
function findMatch(): void {
  cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(PLAY_STATUS), { timeout: timeouts.view }).should("be.visible");
}

function landedOn(pattern: RegExp, timeout: number): Cypress.Chainable<string> {
  return cy
    .location("pathname", { timeout })
    .should("match", pattern)
    .then((pathname) => pattern.exec(pathname)?.[1] ?? "");
}

/** §2.5 stamps the socket's seat; `playerId` is only action-shape data (R335). */
function seatTwoConcedes(): void {
  cy.wsPlayer({ action: "send", name: SEAT_TWO, body: { type: "concede", playerId: "p2" } });
}

function installed(value: InstalledLoadout | null): InstalledLoadout {
  expect(value, "cy.installLoadout has answered").to.not.eq(null);
  return value as InstalledLoadout;
}

// ---------------------------------------------------------------------------------------------

describe("19 queue modes and series — Best of 1, All Random, Conquest and rooms", () => {
  beforeEach(() => {
    cy.freeAccount(accounts.p1());
    cy.freeAccount(accounts.p2());
  });

  it("Best of 1 plays the deck the player chose, not the first saved one (R257, R253)", () => {
    const seed = seedFor("19-bo1");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let matchId = "";

    cy.installLoadout(seatOne, "19-modes-a", { deckIndex: CHOSEN_DECK_INDEX }).then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });

    openLobby(seatOne, "bo1");
    cy.then(() => {
      const chosen = installed(mine).deckIds[CHOSEN_DECK_INDEX] ?? "";
      cy.get(ts(PLAY_DECK_SELECT), { timeout: timeouts.view }).select(chosen);
      cy.get(ts(PLAY_DECK_SELECT)).should("have.value", chosen);
    });
    // R253: the verdict is UX only, but this legal fixture should say ready.
    cy.get(ts(PLAY_VERDICT)).should("have.attr", "data-ready", "true");
    findMatch();

    cy.then(() => {
      enqueue(seatTwo, { mode: "bo1", deckId: installed(theirs).deckIds[0], seed }).should((response) => {
        expect(response.status).to.eq(200);
        expect(response.body.mode, "R257: a Best-of-1 ticket").to.eq("bo1");
        expect(response.body.seriesId, "Best of 1 makes a match, never a series").to.eq(null);
      });
    });

    landedOn(MATCH_PATH, PAIRING_TIMEOUT_MS).then((id) => {
      matchId = id;
    });
    // R259: Best-of-1 has no series banner.
    browserHand().then((hand) => {
      const decks = installed(mine).decks;
      const chosen = decks[CHOSEN_DECK_INDEX] ?? [];
      const others = decks.filter((_deck, at) => at !== CHOSEN_DECK_INDEX);
      expectDealtFrom(hand, chosen, others, "Best of 1: the browser's hand is Deck 2, the chosen deck");
    });
    cy.get(ts(SERIES_BANNER)).should("not.exist");

    cy.then(() => {
      cy.concedeAs(seatOne, matchId).its("ok").should("eq", true);
    });
    me(seatOne).its("currentMatchId").should("eq", null);
    me(seatTwo).its("currentMatchId").should("eq", null);
  });

  it("All Random needs no saved deck: both players are dealt one (R258); the queue takes the player to it from the main menu, whose banner rejoins it while it is live (R765)", () => {
    const seed = seedFor("19-random");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let matchId = "";

    cy.clearDecks(seatOne);
    cy.clearDecks(seatTwo);
    cy.savedDecks(seatTwo).its("decks").should("have.length", 0);
    cy.savedDecks(seatOne).its("decks").should("have.length", 0);

    openLobby(seatOne, "random");
    cy.get(ts(PLAY_DECK_SELECT)).should("not.exist");
    cy.get(ts(PLAY_TRIO_SELECT)).should("not.exist");
    findMatch();

    // R765: a queued ticket follows the player to the main menu.
    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");

    cy.then(() => {
      enqueue(seatTwo, { mode: "random", seed }).should((response) => {
        expect(response.status, "R258: All Random is queued with no deck at all").to.eq(200);
        expect(response.body.mode).to.eq("random");
      });
    });

    landedOn(MATCH_PATH, PAIRING_TIMEOUT_MS).then((id) => {
      matchId = id;
      cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId }).then(
        (result) => {
          const hand = socketHand(result.view);
          expect(hand.length, "seat two was dealt a deck and drew from it").to.be.at.least(constants.OPENING_DRAW[0]);
          for (const card of hand) {
            // R258/R380 can deal from every set.
            expect(card, "a catalog id").to.match(/^(core|classic|classicplus)-\d{3}$/);
          }
        },
      );
    });
    browserHand().should("have.length.at.least", constants.OPENING_DRAW[0]);

    // R765/§9.5: the main-menu banner rejoins the same live match.
    cy.visit("/");
    cy.then(() => {
      cy.get(ts(LIVE_GAME_BANNER), { timeout: timeouts.view }).should("have.attr", "data-kind", "match");
      cy.get(ts(LIVE_GAME_REJOIN)).should("have.attr", "href", `/match/${matchId}`).click();
      cy.location("pathname").should("eq", `/match/${matchId}`);
    });
    browserHand().should("have.length.at.least", constants.OPENING_DRAW[0]);

    cy.then(() => {
      seatTwoConcedes();
    });
    cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.view }).should("contain.text", "Win");
    me(seatOne).its("currentMatchId").should("eq", null);
    cy.visit("/");
    cy.get(`[data-testid="landing"][data-account="signed-in"]`, { timeout: timeouts.view }).should("exist");
    cy.get(ts(LIVE_GAME_BANNER)).should("not.exist");
  });

  it("a Conquest series: sealed picks, the picked decks, won decks locked, the last deck picked for you, three wins end it and rate it once (R330–R338, R262)", () => {
    // R335 seeds the games; R635 withholds Hinder until mulligans, avoiding R431/R682 hand changes.
    const seed = seedFor("19-series-0");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let mine: InstalledLoadout | null = null;
    let theirs: InstalledLoadout | null = null;
    let seriesId = "";
    const matchIds: string[] = [];
    let before: ProfileBody = { record: { wins: 0, losses: 0, draws: 0 } };
    let rankedBefore: RankBody = { record: { games: 0, wins: 0, losses: 0, draws: 0 } };

    cy.installLoadout(seatOne, "19-modes-a", { deckIndex: CHOSEN_DECK_INDEX }).then((value) => {
      mine = value;
    });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });
    profile(seatOne).then((body) => {
      before = body;
    });
    ownRank(seatOne).then((body) => {
      rankedBefore = body;
    });

    const lockIn = (slot: number): void => {
      cy.get(ts(SERIES_PICKER), { timeout: timeouts.view }).should("have.attr", "data-state", "choosing");
      cy.get(ts(seriesPickId(slot))).should("not.be.disabled").check();
      cy.get(ts(seriesPickId(slot))).should("be.checked");
      cy.get(ts(SERIES_LOCK_IN)).should("not.be.disabled").click();
    };

    /** R334: a concession loses this game, not the series. */
    const playAndWin = (gameNo: number, picks: { browser: number; seatTwo: number }): void => {
      landedOn(MATCH_PATH, SERIES_POLL_TIMEOUT_MS).then((id) => {
        matchIds.push(id);
        expect(matchIds, `game ${String(gameNo)} is a match of its own`).to.have.length(gameNo);
      });
      cy.get(ts(SERIES_BANNER), { timeout: timeouts.view }).should("be.visible");
      browserHand().then((hand) => {
        const decks = installed(mine).decks;
        expectDealtFrom(
          hand,
          decks[picks.browser] ?? [],
          decks.filter((_deck, at) => at !== picks.browser),
          `game ${String(gameNo)}: the browser plays the deck it picked`,
        );
      });
      cy.then(() => {
        const matchId = matchIds[gameNo - 1] ?? "";
        cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId }).then((result) => {
          const decks = installed(theirs).decks;
          expectDealtFrom(
            socketHand(result.view),
            decks[picks.seatTwo] ?? [],
            decks.filter((_deck, at) => at !== picks.seatTwo),
            `game ${String(gameNo)}: seat two plays the deck it picked`,
          );
        });
        seatTwoConcedes();
      });
      cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.view }).should("contain.text", "Win");
    };

    const continueToPick = (wins: number): void => {
      cy.get(ts(SERIES_BANNER_RESULT)).should("not.exist");
      cy.get(ts(SERIES_BANNER_CONTINUE), { timeout: SERIES_POLL_TIMEOUT_MS }).should("be.visible").click();
      landedOn(SERIES_PATH, timeouts.view).then((id) => {
        expect(id).to.eq(seriesId);
      });
      cy.get(ts(SERIES_SCREEN), { timeout: timeouts.view }).should("have.attr", "data-status", "picking");
      cy.get(ts(SERIES_SCORE)).should("have.attr", "data-you", String(wins)).and("have.attr", "data-opponent", "0");
    };

    // Queue Conquest
    openLobby(seatOne, "bo3");
    cy.then(() => {
      cy.get(ts(PLAY_TRIO_SELECT), { timeout: timeouts.view }).select(installed(mine).trioId);
      cy.get(ts(PLAY_TRIO_SELECT)).should("have.value", installed(mine).trioId);
    });
    cy.get(ts(PLAY_VERDICT)).should("have.attr", "data-ready", "true");
    findMatch();
    cy.then(() => {
      enqueue(seatTwo, { mode: "bo3", trioId: installed(theirs).trioId, seed }).should((response) => {
        expect(response.status).to.eq(200);
        expect(response.body.mode, "R257: a Conquest ticket").to.eq("bo3");
        expect(response.body.matchId, "R338: a series opens on its pick phase, not on a match").to.eq(null);
      });
    });

    landedOn(SERIES_PATH, PAIRING_TIMEOUT_MS).then((id) => {
      seriesId = id;
    });
    cy.get(ts(SERIES_SCREEN), { timeout: timeouts.view }).should("have.attr", "data-status", "picking");
    cy.get(ts(SERIES_SCORE)).should("have.attr", "data-you", "0").and("have.attr", "data-opponent", "0");

    // R331: picks are sealed until both players lock in.
    lockIn(GAME_ONE_PICKS.browser);
    cy.get(ts(SERIES_PICKER)).should("have.attr", "data-state", "waiting");
    cy.get(ts(seriesDeckId(GAME_ONE_PICKS.browser))).should("have.attr", "data-picked", "true");
    cy.get(ts(SERIES_OPPONENT_STATUS)).should("have.attr", "data-picked", "false");
    cy.then(() => {
      seriesAs(seatTwo, seriesId).should((view) => {
        expect(view.status).to.eq("picking");
        expect(view.opponent.picked, "R331: seat two sees THAT the browser picked").to.eq(true);
        // R336: the opponent projection excludes its selected deck.
        expect(Object.keys(view.opponent).sort(), "R336: no pick and no deck in the opponent's projection").to.deep.eq([
          "decks",
          "picked",
          "wins",
        ]);
        for (const deck of view.opponent.decks) {
          expect(Object.keys(deck).sort(), "R336: an opponent slot is its number and whether it has won").to.deep.eq([
            "slot",
            "won",
          ]);
        }
        expect(view.you.pick, "seat two has not picked yet").to.eq(null);
      });
      cy.request<ErrorBody>({
        method: "POST",
        url: api(`/api/series/${seriesId}/pick`),
        headers: bearer(seatOne),
        body: { slot: 0 },
        failOnStatusCode: false,
      })
        .its("status")
        .should("eq", 409);
      pickAs(seatTwo, seriesId, GAME_ONE_PICKS.seatTwo).should((view) => {
        expect(view.status, "R331: both picked, so game 1 starts at once").to.eq("playing");
        expect(view.currentMatchId, "and the answer names it").to.be.a("string");
      });
    });

    // Game 1
    playAndWin(1, GAME_ONE_PICKS);

    // R330: a won deck is locked; a losing deck returns.
    continueToPick(1);
    cy.get(ts(seriesGameId(1))).should("have.attr", "data-result", "win");
    cy.get(ts(seriesDeckId(GAME_ONE_PICKS.browser))).should("have.attr", "data-won", "true");
    cy.get(ts(seriesPickId(GAME_ONE_PICKS.browser))).should("be.disabled");
    cy.get(ts(seriesOpponentDeckId(GAME_ONE_PICKS.seatTwo))).should("have.attr", "data-won", "false");
    cy.then(() => {
      seriesAs(seatOne, seriesId).should((view) => {
        expect(view.gameNo, "the series is picking for game 2").to.eq(2);
        const game = view.games[0];
        expect(game?.yourSlot).to.eq(GAME_ONE_PICKS.browser);
        expect(game?.opponentSlot, "a played slot is shown once both picks are in").to.eq(GAME_ONE_PICKS.seatTwo);
        expect(game?.youWentFirst, "R335: series seat p1, the older ticket, goes first in game 1").to.eq(true);
        expect(view.result, "a conceded game did not end the series").to.eq(null);
      });
    });
    lockIn(GAME_TWO_PICKS.browser);
    cy.then(() => {
      pickAs(seatTwo, seriesId, GAME_TWO_PICKS.seatTwo).should((view) => {
        expect(view.status, "R330: seat two plays the deck that lost again, and game 2 starts").to.eq("playing");
      });
    });
    playAndWin(2, GAME_TWO_PICKS);

    // R332: auto-pick the browser's final deck.
    continueToPick(2);
    cy.get(ts(SERIES_PICKER)).should("have.attr", "data-state", "waiting").and("have.attr", "data-auto", "true");
    cy.get(ts(seriesDeckId(GAME_THREE_PICKS.browser))).should("have.attr", "data-picked", "true");
    cy.then(() => {
      seriesAs(seatTwo, seriesId).its("opponent.picked").should("eq", true);
      pickAs(seatTwo, seriesId, GAME_THREE_PICKS.seatTwo).its("status").should("eq", "playing");
    });
    playAndWin(3, GAME_THREE_PICKS);

    // Series result
    cy.get(ts(SERIES_BANNER_RESULT), { timeout: SERIES_POLL_TIMEOUT_MS }).should("have.attr", "data-outcome", "win");
    for (const slot of [0, 1, 2]) {
      cy.get(ts(seriesBannerYourDeckId(slot))).should("have.attr", "data-won", "true");
      cy.get(ts(seriesBannerOpponentDeckId(slot))).should("have.attr", "data-won", "false");
    }
    cy.then(() => {
      cy.visitAs(seatOne, routes.series(seriesId));
    });
    cy.get(ts(SERIES_RESULT), { timeout: timeouts.view }).should("have.attr", "data-outcome", "win");
    cy.get(ts(SERIES_SCORE))
      .should("have.attr", "data-you", String(SERIES_WINS_NEEDED))
      .and("have.attr", "data-opponent", "0");

    // R262/R604: the series moves rating once; R612 reads its season record, not hidden points.
    cy.then(() => {
      seriesAs(seatOne, seriesId).then((view) => {
        expect(view.status).to.eq("over");
        expect(view.games, `the series ended at ${String(SERIES_WINS_NEEDED)} wins, one with each deck`).to.have.length(
          SERIES_WINS_NEEDED,
        );
        expect(view.games[1]?.youWentFirst, "R335: series seat p2 goes first in even games").to.eq(false);
        const result = view.result;
        expect(result?.outcome).to.eq("win");
        expect(result?.endReason).to.eq("decided");
        expect(result?.ranked, "a won queue-paired series moved the rating").to.eq(true);
        profile(seatOne).should((after) => {
          expect(after.record.wins - before.record.wins, "R262: every game is recorded as a win").to.eq(SERIES_WINS_NEEDED);
        });
        ownRank(seatOne).should((after) => {
          expect(
            after.record.games - rankedBefore.record.games,
            "R262: the series moved the season record by exactly one rated game",
          ).to.eq(1);
          expect(after.record.wins - rankedBefore.record.wins, "the series counts as one win").to.eq(1);
        });
      });
    });

    // Both players can queue again.
    for (const account of [seatOne, seatTwo]) {
      cy.then(() => {
        me(account).should((body) => {
          expect(body.currentMatchId).to.eq(null);
          expect(body.currentSeriesId ?? null, "a finished series holds nobody").to.eq(null);
        });
        const decks = account === seatOne ? installed(mine) : installed(theirs);
        enqueue(account, { mode: "bo1", deckId: decks.deckIds[0] }).its("status").should("eq", 200);
        dequeue(account);
      });
    }
  });

  it("a Conquest room refuses a Best-of-1 joiner and makes a series for a trio (R264)", () => {
    const seed = seedFor("19-room");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let theirs: InstalledLoadout | null = null;
    let code = "";
    let seriesId = "";

    cy.installLoadout(seatOne, "19-modes-a", { deckIndex: CHOSEN_DECK_INDEX });
    cy.installLoadout(seatTwo, "19-modes-b").then((value) => {
      theirs = value;
    });

    openLobby(seatOne, "bo3");
    cy.get(ts(PLAY_TRIO_SELECT), { timeout: timeouts.view }).find("option:selected").should("have.text", INSTALLED_TRIO_NAME);
    cy.get(ts(PLAY_CREATE_ROOM)).should("not.be.disabled").click();
    cy.get(ts(PLAY_ROOM_MODE), { timeout: timeouts.view }).should("have.attr", "data-mode", "bo3");
    cy.get(ts(PLAY_ROOM_CODE))
      .invoke("text")
      .then((text) => {
        code = text.trim();
        expect(code, "R79: a room code").to.have.length(constants.ROOM_CODE_LENGTH);
      });

    // R264: a Best-of-1 joiner is refused with the room mode.
    cy.then(() => {
      cy.request<ErrorBody>({
        method: "POST",
        url: api(`/api/rooms/${code}/join`),
        headers: bearer(seatTwo),
        body: { mode: "bo1", deckId: installed(theirs).deckIds[0], seed },
        failOnStatusCode: false,
      }).should((response) => {
        expect(response.status, "R264: a choice in another mode is a conflict").to.eq(409);
        expect(response.body.error.code).to.eq("conflict");
        expect(response.body.error.details, "and names the room's mode").to.deep.eq({ mode: "bo3" });
        expect(response.body.error.message).to.eq("This room plays Conquest: pick one of your trios.");
      });
    });
    cy.then(() => {
      cy.request<JoinBody>({
        method: "POST",
        url: api(`/api/rooms/${code}/join`),
        headers: bearer(seatTwo),
        body: { mode: "bo3", trioId: installed(theirs).trioId, seed },
      }).should((response) => {
        expect(response.status).to.eq(200);
        expect(response.body.mode).to.eq("bo3");
        expect(response.body.seat, "the joiner is series seat p2").to.eq("p2");
        expect(response.body.matchId, "R264: a Conquest join makes the series, no match yet").to.eq(null);
        expect(response.body.seriesId).to.be.a("string").and.not.eq("");
        seriesId = response.body.seriesId ?? "";
      });
    });

    landedOn(SERIES_PATH, SERIES_POLL_TIMEOUT_MS).then((id) => {
      expect(id, "the host lands on the series the join made").to.eq(seriesId);
    });
    cy.get(ts(SERIES_SCREEN), { timeout: timeouts.view }).should("have.attr", "data-status", "picking");

    // R334: forfeit between games to balance fixture endings.
    cy.then(() => {
      forfeitAs(seatOne, seriesId).should((view) => {
        expect(view.status).to.eq("over");
        expect(view.result?.outcome, "R334: a forfeit loses the series").to.eq("loss");
        expect(view.result?.endReason).to.eq("forfeit");
      });
      seriesAs(seatTwo, seriesId).its("result.outcome").should("eq", "win");
    });
    me(seatOne).its("currentSeriesId").should("eq", null);
    me(seatTwo).its("currentSeriesId").should("eq", null);
  });
});
