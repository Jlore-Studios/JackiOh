// Spec 20: concurrent mulligans, confirmed concede, and draw offers over browser and socket views.
// CLAUDE.md rule 7: each seat sees only its `viewFor` data (R266, §9.1, §10.8).
// R265 and R268 cover sealed mulligans and their shared clock; R36 and R269 cover draw offers.
// Fixture turns stay actionable under R82 and R244; kept and redrawn cards follow R9 and R10.
// Room setup follows R143 and R257 (SPEC §11; §§2.1, 2.5, 9.5; SURFACE §11.3); clocks follow R79.

import { MULLIGAN_CLOCK_MS } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, routes, seedFor, server, timeouts } from "../../support/config.ts";
import {
  BOARD,
  CLOCK_OPPONENT,
  CLOCK_YOU,
  CONCEDE,
  CONCEDE_CANCEL,
  CONCEDE_CONFIRM,
  CONCEDE_DIALOG,
  DRAW_ACCEPT,
  DRAW_DECLINE,
  DRAW_OFFER,
  DRAW_OFFER_STATUS,
  DRAW_OUTCOME,
  DRAW_TOAST,
  END_TURN,
  MULLIGAN_OPPONENT_READY,
  MULLIGAN_OPPONENT_STATUS,
  MULLIGAN_WAITING,
  OFFER_DRAW,
  PROMPT,
  PROMPT_CLOCK,
  PROMPT_SUBMIT,
  RESULT_OVERLAY,
  handCardId,
  mulliganWaitingCardId,
  promptOf,
  promptOptionId,
  ts,
} from "../../support/testids.ts";
import type { ActionInput, PlayerId } from "../../support/types.ts";
import type { ViewPredicate, WsPlayerResult } from "../../support/commands.ts";

const SEAT_TWO = "seat-two";
const SEAT_ONE_ID: PlayerId = "p1";
const SEAT_TWO_ID: PlayerId = "p2";

const DECK_A = "06-room-a";
const DECK_B = "06-room-b";

const SEEDS = {
  seatTwoFirst: seedFor("20-mulligan-seat-two-first"),
  // R10: this seed makes a redrawn card leave the hand before turn 1.
  browserFirst: seedFor("20-mulligan-browser-first-2"),
  concede: seedFor("20-concede"),
  // R82: this seed would auto-end player 1's first turn.
  draw: seedFor("20-draw-1"),
};

const OPTION_PREFIX = promptOptionId("");
const ANY_OPTION = `[data-testid^="${OPTION_PREFIX}"]`;

// Seat 2's socket

type CardLike = { instanceId: string };
type ViewLike = {
  viewer?: string;
  turn?: number;
  active?: string;
  phase?: string;
  you?: { hand?: CardLike[] | { count: number } };
  opponent?: { hand?: CardLike[] | { count: number } };
  pending?: { forYou: boolean; kind?: string; pendingFor?: string } | null;
  mulligan?: { youReady: boolean; opponentReady: boolean; kept?: string[] };
  drawOffer?: { by: string };
  result?: { winner?: string; reason?: string } | null;
};

type Clocks = { turnDeadline: number | null; promptDeadline: number | null };
type ClockFrame = { now: number; clocks: Clocks };

function asView(view: Record<string, unknown> | null | undefined): ViewLike {
  expect(view, "a PlayerView on seat 2's socket (§10.8)").to.be.an("object");
  return (view ?? {}) as ViewLike;
}

function ownHandIds(view: ViewLike): string[] {
  const hand = view.you?.hand;
  expect(Array.isArray(hand), "the viewer's own hand arrives as cards (§10.8)").to.eq(true);
  return (Array.isArray(hand) ? hand : []).map((card) => card.instanceId);
}

function opponentHandCount(view: ViewLike): number {
  const hand = view.opponent?.hand;
  expect(Array.isArray(hand), "the opponent's hand is a count, never cards (§10.8)").to.eq(false);
  return (hand as { count?: number } | undefined)?.count ?? -1;
}

function sorted(ids: readonly string[]): string[] {
  return [...ids].sort();
}

function stringsIn(value: unknown, into: Set<string> = new Set()): Set<string> {
  if (typeof value === "string") into.add(value);
  else if (Array.isArray(value)) for (const item of value) stringsIn(item, into);
  else if (value !== null && typeof value === "object") for (const item of Object.values(value)) stringsIn(item, into);
  return into;
}

/** R266, §9.1: seat 2's received frames must not name an opponent card. */
function seatTwoNeverSaw(ids: readonly string[], why: string): void {
  cy.wsPlayer({ action: "messages", name: SEAT_TWO }).then((result) => {
    const seen = stringsIn(result.messages ?? []);
    for (const id of ids) expect(seen.has(id), `${why}: ${id}`).to.eq(false);
  });
}

function seatTwoView(): Cypress.Chainable<ViewLike> {
  return cy.wsPlayer({ action: "view", name: SEAT_TWO }).then((result) => asView(result.view));
}

function seatTwoAwaits(where: ViewPredicate): Cypress.Chainable<ViewLike> {
  return cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where }).then((result) => asView(result.view));
}

function seatTwoSends(body: ActionInput): Cypress.Chainable<WsPlayerResult> {
  return cy.wsPlayer({ action: "send", name: SEAT_TWO, body });
}

/** Latest clock frame (R79, R268). */
function seatTwoClock(): Cypress.Chainable<ClockFrame> {
  return cy.wsPlayer({ action: "messages", name: SEAT_TWO }).then((result) => {
    const frames = (result.messages ?? []).filter((message) => message.type === "clock") as unknown as ClockFrame[];
    const last = frames.at(-1);
    expect(last, "a clock frame on seat 2's socket (R79)").to.not.eq(undefined);
    return last as ClockFrame;
  });
}

function seatTwoKeepsWholeHand(): void {
  seatTwoAwaits({ promptKind: "mulligan" }).then((view) => {
    seatTwoSends({ type: "mulligan", keep: ownHandIds(view), playerId: SEAT_TWO_ID });
  });
}

// The browser

/** Opening hand ids from the mulligan picker (R9). */
function browserMulliganIds(): Cypress.Chainable<string[]> {
  return cy
    .get(`${promptOf("mulligan")} ${ANY_OPTION}`, { timeout: timeouts.view })
    .should("have.length.at.least", 1)
    .then(($options) =>
      $options
        .map((_index, element) => (element.getAttribute("data-testid") ?? "").slice(OPTION_PREFIX.length))
        .get(),
    );
}

/** CLAUDE.md rule 7, R266: the browser must not hold an opponent card id. */
function browserNeverShows(ids: readonly string[], why: string): void {
  cy.document({ log: false }).then((doc) => {
    const values: string[] = [];
    for (const element of Array.from(doc.querySelectorAll("*"))) {
      for (const attribute of Array.from(element.attributes)) values.push(attribute.value);
    }
    for (const id of ids) {
      const named = values.some((value) => value === id || value.endsWith(`-${id}`));
      expect(named, `${why} (DOM): ${id}`).to.eq(false);
    }
  });
  cy.window({ log: false }).then((win) => {
    const seen = stringsIn(win.__jackioh?.state ?? null);
    for (const id of ids) expect(seen.has(id), `${why} (dev handle): ${id}`).to.eq(false);
  });
}

function remainingOf($line: JQuery<HTMLElement>): number | null {
  const raw = $line.attr("data-remaining-ms") ?? "";
  return raw === "" ? null : Number(raw);
}

/** R268: both seats and the prompt share a live mulligan deadline. */
function browserShowsMulliganClock(): void {
  for (const line of [CLOCK_YOU, CLOCK_OPPONENT, PROMPT_CLOCK]) {
    cy.get(ts(line), { timeout: timeouts.view })
      .should("have.attr", "data-kind", "mulligan")
      .and("have.attr", "data-total-ms", String(MULLIGAN_CLOCK_MS))
      .and(($line) => {
        const remaining = remainingOf($line);
        expect(remaining, `${line}: the mulligan countdown is running (R268)`).to.be.greaterThan(0);
        expect(remaining, `${line}: and it is at most MULLIGAN_CLOCK_SECONDS`).to.be.at.most(MULLIGAN_CLOCK_MS);
      });
  }
  cy.get(ts(CLOCK_YOU)).then(($line) => {
    const first = remainingOf($line) ?? 0;
    cy.get(ts(CLOCK_YOU), { timeout: timeouts.view }).should(($later) => {
      expect(remainingOf($later), "the mulligan countdown moves on the page (R268)").to.be.lessThan(first);
    });
  });
}

function browserSeesTurnOne(): void {
  cy.get(ts(BOARD), { timeout: timeouts.view })
    .should("have.attr", "data-phase", "main")
    .and("have.attr", "data-turn", "1")
    .and("have.attr", "data-active", "you");
  cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
  cy.get(PROMPT).should("not.exist");
  cy.get(ts(MULLIGAN_WAITING)).should("not.exist");
  // R268: after setup, R79's turn clock replaces the mulligan clock.
  cy.get(ts(PROMPT_CLOCK)).should("not.exist");
  cy.get(ts(CLOCK_YOU))
    .should("not.have.attr", "data-kind", "mulligan")
    .and(($line) => {
      expect(remainingOf($line), "the browser's turn clock is running").to.be.greaterThan(0);
    });
  cy.get(ts(CLOCK_OPPONENT))
    .should("not.have.attr", "data-kind", "mulligan")
    .and(($line) => {
      expect(remainingOf($line), "no clock runs for the seat whose turn it is not").to.eq(null);
    });
}

function seatTwoSeesTurnOne(): Cypress.Chainable<ViewLike> {
  return seatTwoAwaits({ phase: "main" }).then((view) => {
    expect(view.turn, "turn 1 has begun").to.eq(1);
    expect(view.active, "player 1 goes first (§2.1)").to.eq(SEAT_ONE_ID);
    expect(view.mulligan, "R265: the window closed with the second answer").to.eq(undefined);
    expect(view.pending, "and no prompt is open").to.eq(null);
    return view;
  });
}

function waitForMyTurn(): void {
  cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
}

// A room match (§9.5)

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

function openMatch(seed: string): void {
  const seatOne = accounts.p1();
  const seatTwo = accounts.p2();
  // R257, SURFACE §11.3: fixture loadouts provide each seat's deck id.
  cy.installLoadout(seatOne, DECK_A).then((one) => {
    cy.installLoadout(seatTwo, DECK_B).then((two) => {
      // R143: the server honours the room seed.
      cy.request<{ code: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne.token),
        body: { mode: "bo1", deckId: one.deckIds[0], seed },
      }).then((created) => {
        cy.request<{ matchId: string; seat: string }>({
          method: "POST",
          url: api(`/api/rooms/${created.body.code}/join`),
          headers: bearer(seatTwo.token),
          body: { mode: "bo1", deckId: two.deckIds[0] },
        }).then((joined) => {
          expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq(SEAT_TWO_ID);
          const matchId = joined.body.matchId;
          cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: seatTwo.token, matchId });
          cy.visitAs(seatOne, routes.match(matchId));
        });
      });
    });
  });
}

function openMatchAtTurnOne(seed: string): void {
  openMatch(seed);
  cy.keepMulligans();
  seatTwoKeepsWholeHand();
  waitForMyTurn();
}

describe("20 — both seats mulligan at once, Concede asks first, and a draw offer is answered", () => {
  it("R265 seat 2 answers first: the browser's picker stays open and says so, and its Ready starts the game", () => {
    openMatch(SEEDS.seatTwoFirst);

    cy.waitForPrompt("mulligan");
    cy.get(ts(MULLIGAN_OPPONENT_STATUS))
      .should("have.attr", "data-ready", "false")
      .and("contain.text", "Opponent is choosing");
    cy.get(ts(PROMPT_SUBMIT)).should("have.text", "Ready");
    browserShowsMulliganClock();

    let seatTwoHand: string[] = [];
    let kept: string[] = [];
    let returned = "";
    let deadline = 0;

    seatTwoAwaits({ promptKind: "mulligan", mulliganOpponentReady: false }).then((view) => {
      expect(view.mulligan, "seat 2 sees the window too (R266)").to.deep.eq({ youReady: false, opponentReady: false });
      expect(view.phase, "setup is nobody's turn (§2.1)").to.eq("mulligan");
      seatTwoHand = ownHandIds(view);
      expect(seatTwoHand.length, "an opening hand to mulligan").to.be.at.least(2);
      // Return one card to exercise a sealed answer.
      kept = seatTwoHand.slice(0, -1);
      returned = seatTwoHand.at(-1) ?? "";
    });

    seatTwoClock().then((frame) => {
      expect(frame.clocks.turnDeadline, "setup runs no turn clock (R268)").to.eq(null);
      expect(frame.clocks.promptDeadline, "the mulligan deadline (R268)").to.be.a("number");
      deadline = frame.clocks.promptDeadline ?? 0;
      const remaining = deadline - frame.now;
      expect(remaining, "the mulligan clock is running").to.be.greaterThan(0);
      expect(remaining, "and is at most MULLIGAN_CLOCK_SECONDS").to.be.at.most(MULLIGAN_CLOCK_MS);
    });

    cy.then(() => {
      seatTwoSends({ type: "mulligan", keep: kept, playerId: SEAT_TWO_ID });
    });
    seatTwoAwaits({ mulliganOpponentReady: false }).then((view) => {
      expect(view.mulligan?.youReady, "seat 2 is ready").to.eq(true);
      expect(sorted(view.mulligan?.kept ?? []), "and its view holds what it kept (R266)").to.deep.eq(sorted(kept));
      expect(view.pending, "it now waits on seat 1's mulligan").to.deep.eq({ forYou: false, pendingFor: SEAT_ONE_ID });
      expect(sorted(ownHandIds(view)), "a sealed answer changes no hand yet (R266)").to.deep.eq(sorted(seatTwoHand));
    });
    seatTwoClock().then((frame) => {
      expect(frame.clocks.promptDeadline, "the deadline did not move when seat 2 answered (R268)").to.eq(deadline);
    });

    cy.get(promptOf("mulligan")).should("be.visible");
    cy.get(promptOf("mulligan"))
      .find(ts(MULLIGAN_OPPONENT_STATUS))
      .should("have.attr", "data-ready", "true")
      .find(ts(MULLIGAN_OPPONENT_READY))
      .should("have.text", "Opponent is ready");
    cy.get(ts(MULLIGAN_WAITING)).should("not.exist");
    browserShowsMulliganClock();
    cy.then(() => {
      browserNeverShows(seatTwoHand, "the browser was never sent a card of seat 2's hand");
    });

    cy.keepMulligans();
    browserSeesTurnOne();
    seatTwoSeesTurnOne().then((view) => {
      const hand = ownHandIds(view);
      for (const id of kept) expect(hand, "a card seat 2 kept is still in its hand (R9)").to.include(id);
      expect(hand, "the card seat 2 sent back was replaced (R9)").to.not.include(returned);
    });
  });

  it("R266 the browser answers first: it waits with its hand marked, and seat 2 learns only that it is ready", () => {
    openMatch(SEEDS.browserFirst);

    cy.waitForPrompt("mulligan");
    browserShowsMulliganClock();
    seatTwoAwaits({ promptKind: "mulligan", mulliganOpponentReady: false });

    browserMulliganIds().then((hand) => {
      expect(hand.length, "an opening hand to mulligan").to.be.at.least(2);
      const back = hand.at(-1) ?? "";
      const keep = hand.slice(0, -1);

      cy.get(ts(promptOptionId(back))).click();
      cy.get(ts(PROMPT_SUBMIT)).should("have.text", "Ready").click();
      cy.settled();

      cy.get(PROMPT).should("not.exist");
      cy.get(ts(MULLIGAN_WAITING), { timeout: timeouts.view })
        .should("be.visible")
        .and("contain.text", "Waiting for your opponent…")
        .and("have.attr", "data-returning", "1");
      for (const id of keep) cy.get(ts(mulliganWaitingCardId(id))).should("have.attr", "data-verdict", "keep");
      cy.get(ts(mulliganWaitingCardId(back))).should("have.attr", "data-verdict", "redraw");
      for (const id of hand) cy.get(ts(handCardId(id))).should("exist");
      cy.get(ts(BOARD)).should("have.attr", "data-phase", "mulligan");
      browserShowsMulliganClock();

      seatTwoAwaits({ mulliganOpponentReady: true }).then((view) => {
        expect(view.pending?.forYou, "seat 2's own mulligan is still open (R265)").to.eq(true);
        expect(view.pending?.kind).to.eq("mulligan");
        expect(view.mulligan, "R266: ready, and not what was kept").to.deep.eq({ youReady: false, opponentReady: true });
        expect(opponentHandCount(view), "a sealed answer changes no hand yet (R266)").to.eq(hand.length);
      });
      seatTwoNeverSaw(hand, "R266: seat 2 was never sent a card of the browser's hand");

      seatTwoKeepsWholeHand();
      browserSeesTurnOne();
      for (const id of keep) cy.get(ts(handCardId(id))).should("exist");
      cy.get(ts(handCardId(back))).should("not.exist");
      seatTwoSeesTurnOne();
      seatTwoNeverSaw(hand, "§9.1: nor after the mulligans resolved");
    });
  });

  it("§2.5 Concede asks first: Keep playing, Escape and a click outside keep the game, and only Concede ends it", () => {
    openMatchAtTurnOne(SEEDS.concede);

    cy.get(ts(CONCEDE)).should("not.be.disabled").click();
    cy.get(ts(CONCEDE_DIALOG))
      .should("be.visible")
      .and("have.attr", "role", "alertdialog")
      .and("have.attr", "aria-modal", "true")
      .and("contain.text", "Concede this game?")
      .and("contain.text", "Your opponent wins and the game ends here.");
    cy.focused().should("have.attr", "data-testid", CONCEDE_CANCEL).and("have.text", "Keep playing");
    cy.get(ts(CONCEDE_CONFIRM)).should("have.text", "Concede");

    cy.get(ts(CONCEDE_CANCEL)).click();
    cy.get(ts(CONCEDE_DIALOG)).should("not.exist");
    cy.focused().should("have.attr", "data-testid", CONCEDE);

    cy.get(ts(CONCEDE)).click();
    cy.get(ts(CONCEDE_DIALOG)).should("be.visible");
    cy.get("body").type("{esc}");
    cy.get(ts(CONCEDE_DIALOG)).should("not.exist");

    cy.get(ts(CONCEDE)).click();
    cy.get(ts(CONCEDE_DIALOG)).should("be.visible").parent().click("topLeft");
    cy.get(ts(CONCEDE_DIALOG)).should("not.exist");
    cy.get(ts(RESULT_OVERLAY)).should("not.exist");

    cy.endTurn();
    seatTwoAwaits({ active: SEAT_TWO_ID }).then((view) => {
      expect(view.result, "the match is still on after three kept-playing answers").to.eq(null);
    });
    cy.then(() => {
      seatTwoSends({ type: "endTurn", playerId: SEAT_TWO_ID });
    });
    waitForMyTurn();

    cy.get(ts(CONCEDE)).click();
    cy.get(ts(CONCEDE_CONFIRM)).click();
    cy.get(ts(CONCEDE_DIALOG)).should("not.exist");
    cy.expectResult("Loss");
    cy.get(ts(RESULT_OVERLAY))
      .should("have.attr", "data-outcome", "loss")
      .and("have.attr", "data-reason", "concede")
      .and("contain.text", "You conceded.");
    seatTwoAwaits({ hasResult: true }).then((view) => {
      expect(view.result?.winner, "seat 2 wins on seat 1's concede").to.eq(SEAT_TWO_ID);
      expect(view.result?.reason, "and the reason is the concede (§2.5)").to.eq("concede");
      expect(view.phase, "the match is over").to.eq("over");
    });
  });

  it("R36 R269 a draw offer: waiting on the offerer's side, Accept and Decline with a sound on the other's; declined, then accepted", () => {
    openMatchAtTurnOne(SEEDS.draw);

    cy.task<WsPlayerResult>(
      "wsPlayer",
      { action: "send", name: SEAT_TWO, body: { type: "offerDraw", playerId: SEAT_TWO_ID } },
      { timeout: timeouts.task },
    ).then((refused) => {
      expect(refused.ok, "R36: the non-active seat cannot offer a draw").to.eq(false);
    });

    cy.get(ts(OFFER_DRAW)).should("not.be.disabled");
    cy.offerDraw();
    cy.get(ts(DRAW_TOAST), { timeout: timeouts.view }).should("have.attr", "data-draw", "waiting");
    cy.get(ts(DRAW_OFFER_STATUS)).should("have.text", "Draw offered — waiting for reply");
    cy.get(ts(DRAW_OFFER)).should("not.exist");
    seatTwoAwaits({ drawOfferBy: SEAT_ONE_ID }).then((view) => {
      expect(view.drawOffer, "R269: the offer stands on seat 2's view").to.deep.eq({ by: SEAT_ONE_ID });
      expect(view.result, "an offer ends nothing by itself").to.eq(null);
    });

    cy.then(() => {
      seatTwoSends({ type: "answerDraw", accept: false, playerId: SEAT_TWO_ID });
    });
    cy.get(ts(DRAW_OUTCOME), { timeout: timeouts.view })
      .should("have.attr", "data-outcome", "declined")
      .and("have.text", "Your opponent declined the draw");
    cy.get(ts(DRAW_TOAST)).should("have.attr", "data-draw", "declined");
    cy.get(ts(DRAW_OFFER_STATUS)).should("not.exist");
    cy.get(ts(OFFER_DRAW)).should("be.disabled");
    seatTwoAwaits({ drawOfferBy: null }).then((view) => {
      expect(view.drawOffer, "R269: an answered offer no longer stands").to.eq(undefined);
      expect(view.result, "and a declined one ends nothing").to.eq(null);
    });

    cy.endTurn();
    seatTwoAwaits({ active: SEAT_TWO_ID });
    cy.window({ log: false }).then((win) => {
      const audio = (win as unknown as { __jackiohAudio?: { clearLog(): void } }).__jackiohAudio;
      expect(audio, "window.__jackiohAudio (dev builds, spec 15)").to.not.eq(undefined);
      audio?.clearLog();
    });
    cy.then(() => {
      seatTwoSends({ type: "offerDraw", playerId: SEAT_TWO_ID });
    });

    cy.get(ts(DRAW_TOAST), { timeout: timeouts.view }).should("have.attr", "data-draw", "offered");
    cy.get(ts(DRAW_OFFER))
      .should("be.visible")
      .and("have.attr", "role", "region")
      .and("contain.text", "Your opponent offers a draw");
    cy.get(ts(DRAW_DECLINE)).should("have.text", "Decline").and("not.be.disabled");
    cy.get(ts(DRAW_ACCEPT)).should("have.text", "Accept").and("not.be.disabled");
    cy.window({ log: false }).should((win) => {
      type Cue = { kind: string; id?: string; params?: { urgent?: boolean } };
      const audio = (win as unknown as { __jackiohAudio?: { log(): readonly Cue[] } }).__jackiohAudio;
      const cues = audio?.log() ?? [];
      const rung = cues.some((cue) => cue.kind === "sfx" && cue.id === "notify" && cue.params?.urgent === true);
      expect(rung, `the urgent notify rang for the offer; the log holds ${JSON.stringify(cues)}`).to.eq(true);
    });
    seatTwoView().then((view) => {
      expect(view.drawOffer, "R269: seat 2's offer stands on its own view").to.deep.eq({ by: SEAT_TWO_ID });
    });

    cy.get(ts(DRAW_ACCEPT)).click();
    cy.expectResult("Draw");
    cy.get(ts(DRAW_OUTCOME)).should("have.attr", "data-outcome", "accepted").and("have.text", "Draw accepted");
    cy.get(ts(RESULT_OVERLAY))
      .should("have.attr", "data-outcome", "draw")
      .and("have.attr", "data-reason", "draw-accepted")
      .and("contain.text", "Game drawn by agreement.");
    seatTwoAwaits({ hasResult: true }).then((view) => {
      expect(view.result?.winner, "both seats see the same ending").to.eq("draw");
      expect(view.result?.reason, "a draw by agreement (§2.5)").to.eq("draw-accepted");
      expect(view.phase, "the match is over").to.eq("over");
    });
  });
});
