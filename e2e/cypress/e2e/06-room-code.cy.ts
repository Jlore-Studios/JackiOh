// BUILD M8 06: browser and Node client complete a room-code match (M6; e2e/README.md).
// R79/R104 define the imported room-code length and alphabet; R110 makes finished players queueable.
// Seat 2 claims `POST /api/rooms/:code/join` before opening its socket (§9.5).
// CLAUDE.md rule 7/§10.8: Node sees the opponent's hand count, while the browser renders its own.
// R767 covers a guest following the room link and choosing its deck.

import { CODE_ALPHABET, ROOM_CODE_LENGTH } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, constants, routes, seedFor, server, timeouts } from "../../support/config.ts";
import {
  END_TURN,
  MANA_CRYSTAL,
  PLAY_DECK_SELECT,
  PLAY_JOIN_INPUT,
  PLAY_JOIN_SUBMIT,
  PLAY_STATUS,
  handCardId,
  handCountId,
  heroId,
  manaId,
  playModeId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { InstalledLoadout } from "../../support/commands.ts";
import type { ActionInput, Lane, PlayerId, Row, Side } from "../../support/types.ts";

// Local scaffolding; ASK items belong in `e2e/support/**`.

/** ASK (support/commands.ts + support/config.ts): `cy.signIn(account)` and the session key. */
const SESSION_STORAGE_KEY = "jackioh.e2e.session";

const SEAT_TWO = "seat-two";
const HOST = "host";
const SEAT_ONE_ID: PlayerId = "p1";
const SEAT_TWO_ID: PlayerId = "p2";

const LANES: readonly Lane[] = [1, 2, 3, 4, 5];
const ROWS: readonly Row[] = ["units", "backrow"];
const SIDES: readonly Side[] = ["you", "opponent"];

/** `handCardId("")` is the prefix, so no spec spells a testid format. */
const HAND_CARD_PREFIX = handCardId("");

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

/** The fixture's deck id out of `cy.installLoadout`'s answer (deck index 0, the default). */
function deckOf(installed: InstalledLoadout | null): string {
  expect(installed, "cy.installLoadout has answered").to.not.eq(null);
  return installed?.deckIds[0] ?? "";
}

/** The link the room ticket shares: it mirrors `roomLinkPath` in `apps/web/src/net/roomLink.ts`. */
function inviteLink(code: string, mode: string): string {
  return `${routes.play()}?${new URLSearchParams({ room: code, mode }).toString()}`;
}

function visitAs(token: string, path: string): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
    },
  });
}

// Reading seat 2's view.

type CardLike = { instanceId: string };
type SideLike = {
  player?: string;
  hero?: { health?: number; armor?: number };
  hand?: CardLike[] | { count: number };
  units?: unknown[];
  backrow?: unknown[];
  libraryCount?: number;
};

type ViewLike = {
  viewer?: string;
  turn?: number;
  active?: string;
  phase?: string;
  you?: SideLike;
  opponent?: SideLike;
  result?: { winner?: string; reason?: string } | null;
};

function asView(view: Record<string, unknown> | null | undefined): ViewLike {
  expect(view, "a PlayerView on seat 2's socket (§10.8)").to.be.an("object");
  return (view ?? {}) as ViewLike;
}

function ownHandIds(view: Record<string, unknown> | null | undefined): string[] {
  const hand = asView(view).you?.hand;
  expect(Array.isArray(hand), "the viewer's own hand arrives as cards, not a count (§10.8)").to.eq(
    true,
  );
  return (Array.isArray(hand) ? hand : []).map((card) => card.instanceId);
}

/** §10.8: "counts for the opponent's hand". A number here, never an array. */
function opponentHandCount(view: Record<string, unknown> | null | undefined): number {
  const hand = asView(view).opponent?.hand;
  expect(
    Array.isArray(hand),
    "CLAUDE.md rule 7 / §10.8: the opponent's hand is a count, never cards",
  ).to.eq(false);
  const count = (hand as { count?: number } | undefined)?.count;
  expect(count, "the opponent's hand count").to.be.a("number");
  return count ?? -1;
}

function seatTwoKeepsMulligan(): void {
  cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { promptKind: "mulligan" } }).then(
    (result) => {
      // R9: the answer names the cards kept, so keeping everything is the whole hand.
      const body: ActionInput = {
        type: "mulligan",
        keep: ownHandIds(result.view),
        // Discarded by `parseClientMessage`; the actor stamps the authenticated seat.
        playerId: SEAT_TWO_ID,
      };
      cy.wsPlayer({ action: "send", name: SEAT_TWO, body });
    },
  );
}

/** The turn is seat 1's again when the client re-enables its own `end-turn` (BUILD M5-T1). */
function waitForMyTurn(): void {
  cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
}

/** How many hand cards the client is rendering right now. */
function renderedHandCards(): Cypress.Chainable<number> {
  cy.settled();
  return cy.get("body", { log: false }).then(
    ($body) =>
      $body
        .find("[data-testid]")
        .map((_index, element) => element.getAttribute("data-testid") ?? "")
        .get()
        .filter((testid) => testid.startsWith(HAND_CARD_PREFIX)).length,
  );
}

describe("06 room code — a networked match between a browser and a Node client", () => {
  it("opens a room, plays it out and leaves both players queue-eligible", () => {
    const seed = seedFor("06-room-code");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let roomCode = "";
    let matchId = "";
    let seatOneDecks: InstalledLoadout | null = null;
    let seatTwoDecks: InstalledLoadout | null = null;

    // Persistent E2E accounts need freeing; save each fixture deck (§9.4, R250).
    cy.freeAccount(seatOne);
    cy.freeAccount(seatTwo);
    cy.installLoadout(seatOne, "06-room-a").then((installed) => {
      seatOneDecks = installed;
    });
    cy.installLoadout(seatTwo, "06-room-b").then((installed) => {
      seatTwoDecks = installed;
    });

    // ASK: create through §9.5's endpoint until the room screen has testids. R143 requires the
    // E2E server to honour this §9.3 seed; R264 makes a Best-of-1 room carry one deck id.
    cy.then(() => {
      cy.request<{ code: string; expiresAt: number; mode: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne.token),
        body: { mode: "bo1", deckId: deckOf(seatOneDecks), seed },
      }).then((created) => {
        roomCode = created.body.code;

        expect(
          constants.ROOM_CODE_LENGTH,
          "support/config.ts's mirror still agrees with crates/server/src/config.rs (R79)",
        ).to.eq(ROOM_CODE_LENGTH);
        expect(roomCode.length, `R79: a room code is ${String(ROOM_CODE_LENGTH)} characters`).to.eq(
          ROOM_CODE_LENGTH,
        );
        for (const character of roomCode) {
          expect(
            CODE_ALPHABET.includes(character),
            `R104: "${character}" is in the invite-code alphabet ${CODE_ALPHABET}`,
          ).to.eq(true);
        }
        expect(created.body.mode, "R264: the room plays the mode it was made with").to.eq("bo1");
        expect(created.body.expiresAt, "the room's TTL is returned so a client can show it").to.be.a(
          "number",
        );
      });
    });

    // Seat 2 atomically claims the room (§9.5).
    cy.then(() => {
      cy.request<{ matchId: string; seriesId: string | null; code: string; seat: string; mode: string }>({
        method: "POST",
        url: api(`/api/rooms/${roomCode}/join`),
        headers: bearer(seatTwo.token),
        body: { mode: "bo1", deckId: deckOf(seatTwoDecks) },
      }).then((joined) => {
        expect(joined.body.code, "the claim names the code it claimed").to.eq(roomCode);
        expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq(SEAT_TWO_ID);
        expect(joined.body.mode, "R264: the join plays the room's mode").to.eq("bo1");
        expect(joined.body.seriesId, "a Best-of-1 room makes a match, not a series").to.eq(null);
        matchId = joined.body.matchId;
        expect(matchId, "a match id to open a socket onto").to.be.a("string").and.not.eq("");
      });
    });

    // Both connect.
    cy.then(() => {
      // `wsPlayer` defaults the omitted seat to p2 (ASK: type its `seat` field).
      cy.wsPlayer({
        action: "connect",
        name: SEAT_TWO,
        url: server.ws(),
        token: seatTwo.token,
        matchId,
      }).should((result) => {
        // Attach pushes a fresh §9.5 view.
        const view = asView(result.view);
        expect(view.viewer, "seat 2's view is seat 2's").to.eq(SEAT_TWO_ID);
      });
      visitAs(seatOne.token, routes.match(matchId));
    });

    // Both see the board.
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
    cy.get(ts(heroId("opponent"))).should("be.visible");
    for (const side of SIDES) {
      for (const row of ROWS) {
        for (const lane of LANES) {
          cy.get(ts(zoneId(side, row, lane))).should("exist");
        }
      }
      cy.get(ts(handCountId(side))).should("exist");
    }

    cy.then(() => {
      cy.wsPlayer({ action: "view", name: SEAT_TWO }).should((result) => {
        const view = asView(result.view);
        expect(view.you?.player, "seat 2 is p2").to.eq(SEAT_TWO_ID);
        expect(view.opponent?.player, "and its opponent is p1").to.eq(SEAT_ONE_ID);
        expect(view.you?.units, `${String(constants.UNIT_ZONES)} unit lanes (§3.1)`).to.have.length(
          constants.UNIT_ZONES,
        );
        expect(
          view.you?.backrow,
          `${String(constants.BACKROW_ZONES)} backrow lanes (§3.1)`,
        ).to.have.length(constants.BACKROW_ZONES);
        expect(view.you?.hero?.health, "heroes start at HERO_HEALTH (§2)").to.eq(
          constants.HERO_HEALTH,
        );
        expect(view.opponent?.hero?.health).to.eq(constants.HERO_HEALTH);
      });
    });

    // R265 mulligans are simultaneous; R266 lets Node observe seat 1's browser answer.
    cy.keepMulligans();
    cy.then(() => {
      cy.wsPlayer({
        action: "awaitView",
        name: SEAT_TWO,
        where: { mulliganOpponentReady: true, promptKind: "mulligan" },
      });
    });
    seatTwoKeepsMulligan();
    waitForMyTurn();

    // §2.1/§2.3 place mana crystals after mulligan, when a player has started a turn.
    cy.get(`${ts(manaId("you"))} ${MANA_CRYSTAL}`)
      .its("length")
      .should("be.within", 1, constants.MAX_MANA);

    // CLAUDE.md rule 7: browser renders only seat 1's hand.
    cy.then(() => {
      cy.wsPlayer({ action: "view", name: SEAT_TWO }).then((result) => {
        const held = opponentHandCount(result.view);
        expect(held, "an opening hand (§2.1: OPENING_DRAW)").to.be.within(
          constants.OPENING_DRAW[0],
          constants.HAND_CAP,
        );
        renderedHandCards().should((rendered) => {
          expect(
            rendered,
            "CLAUDE.md rule 7: the browser renders seat 1's own hand and not one card more",
          ).to.eq(held);
        });
      });
    });

    // Node -> actor -> browser.
    cy.endTurn();
    cy.then(() => {
      // ASK: type `turnAtLeast`; a post-ack view is not pre-action.
      cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { active: SEAT_TWO_ID } }).should(
        (result) => {
          expect(
            asView(result.view).turn,
            "seat 2's first turn is player-turn 2 (R2: the cap counts player-turns)",
          ).to.be.at.least(2);
        },
      );
      cy.wsPlayer({
        action: "send",
        name: SEAT_TWO,
        body: { type: "endTurn", playerId: SEAT_TWO_ID },
      });
    });
    waitForMyTurn();
    cy.then(() => {
      cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { active: SEAT_ONE_ID } }).should(
        (result) => {
          expect(
            asView(result.view).turn,
            "the turn came back to seat 1 as player-turn 3 (R2)",
          ).to.be.at.least(3);
        },
      );
    });

    // §2.5 draw answer ends this nonlethal fixture before R79's ceiling.
    cy.offerDraw();
    cy.then(() => {
      cy.wsPlayer({
        action: "send",
        name: SEAT_TWO,
        body: { type: "answerDraw", accept: true, playerId: SEAT_TWO_ID },
      });
    });

    cy.expectResult("Draw");
    cy.then(() => {
      cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { hasResult: true } }).should(
        (result) => {
          const view = asView(result.view);
          expect(view.result?.winner, "both seats see the same ending (§9.5)").to.eq("draw");
          expect(view.phase, "and the match is over (§10.1)").to.eq("over");
        },
      );
    });

    // §9.5 clears both players; queue verifies it without pairing them. R253/R110 apply.
    for (const seat of [
      { token: seatOne.token, decks: () => seatOneDecks },
      { token: seatTwo.token, decks: () => seatTwoDecks },
    ]) {
      const token = seat.token;
      cy.then(() => {
        cy.request<{ profile: { status: string } }>({
          method: "GET",
          url: api("/api/auth/me"),
          headers: bearer(token),
        })
          .its("body.profile.status")
          .should("eq", "active");
        cy.request({
          method: "POST",
          url: api("/api/queue"),
          headers: bearer(token),
          body: { mode: "bo1", deckId: deckOf(seat.decks()) },
        })
          .its("status")
          .should("eq", 200);
        cy.request({ method: "DELETE", url: api("/api/queue"), headers: bearer(token) })
          .its("status")
          .should("eq", 200);
      });
    }
  });

  it("R767 the guest opens the host's invite link, picks a deck, presses Join, and both reach the match", () => {
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let roomCode = "";
    let matchId = "";
    let seatOneDecks: InstalledLoadout | null = null;
    let seatTwoDecks: InstalledLoadout | null = null;

    cy.freeAccount(seatOne);
    cy.freeAccount(seatTwo);
    cy.installLoadout(seatOne, "06-room-a").then((installed) => {
      seatOneDecks = installed;
    });
    cy.installLoadout(seatTwo, "06-room-b").then((installed) => {
      seatTwoDecks = installed;
    });

    // Host makes a Best-of-1 room and hands over its link.
    cy.then(() => {
      cy.request<{ code: string; mode: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne.token),
        body: { mode: "bo1", deckId: deckOf(seatOneDecks), seed: seedFor("06-room-link") },
      }).then((created) => {
        roomCode = created.body.code;
        expect(created.body.mode, "R264: the room plays the mode it was made with").to.eq("bo1");
      });
    });

    // The guest link fills the form without joining.
    cy.then(() => {
      visitAs(seatTwo.token, inviteLink(roomCode, "bo1"));
      cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.play());
      cy.location("search").should("eq", "");
      cy.get(ts(PLAY_JOIN_INPUT), { timeout: timeouts.view }).should("have.value", roomCode);
      cy.get(ts(playModeId("bo1"))).should("be.checked");
      cy.get(ts(PLAY_STATUS)).should("contain.text", "Pick a deck for Best of 1, then press Join.");
      cy.focused({ timeout: timeouts.view }).should("have.attr", "data-testid", PLAY_DECK_SELECT);
    });

    // R264: the guest still picks a deck and presses Join.
    cy.then(() => {
      cy.get(ts(PLAY_DECK_SELECT)).select(deckOf(seatTwoDecks));
    });
    cy.get(ts(PLAY_JOIN_SUBMIT)).should("not.be.disabled").click();

    cy.location("pathname", { timeout: timeouts.view })
      .should("match", /^\/match\/([^/]+)$/)
      .then((pathname) => {
        matchId = /^\/match\/([^/]+)$/.exec(pathname)?.[1] ?? "";
        expect(matchId, "the match the join made").to.not.eq("");
      });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");

    // The host reaches the same match.
    cy.then(() => {
      cy.wsPlayer({
        action: "connect",
        name: HOST,
        url: server.ws(),
        token: seatOne.token,
        matchId,
        seat: SEAT_ONE_ID,
      }).should((result) => {
        expect(asView(result.view).viewer, "the host's view is the host's").to.eq(SEAT_ONE_ID);
      });
    });

    // Free both accounts for the next spec.
    cy.then(() => {
      cy.concedeAs(seatOne, matchId).its("ok").should("eq", true);
    });
  });
});
