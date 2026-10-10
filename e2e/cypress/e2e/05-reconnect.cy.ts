// BUILD M8 05: reload during #65 Masochism Mask's active-player `PendingChoice` (SPEC §8, §6.2).
// Prompts are state (§9.3), so reconnect gets a full view rather than replay (§9.5).
// Turn 7 makes #77 Professor Curvature's R48 modifier live; fingerprint also covers R169 badges.
// It checks testids (BUILD M5-T1, M5-T4), prompt, counters and badges; R79 owns the clock limits
// (BUILD §2). CLAUDE.md rule 7 requires assertions against the DOM `viewFor` renders.
// Needs M6/M7-T1; see e2e/README.md.

import {
  DISCONNECT_GRACE_SECONDS,
  PROMPT_CLOCK_SECONDS,
  TURN_CLOCK_SECONDS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import { CARD_NAMES } from "../../support/cards.ts";
import { accounts, routes, seedFor, server, timeouts } from "../../support/config.ts";
import {
  END_TURN,
  MANA_CRYSTAL,
  MODIFIER_BADGE,
  PROMPT,
  exileCountId,
  graveyardCountId,
  handCountId,
  heroId,
  libraryCountId,
  manaId,
  modifiersId,
  promptOptionId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { ActionInput, Lane, PlayerId, Side } from "../../support/types.ts";

// Local scaffolding; ASK items belong in `e2e/support/**`.

/** ASK: A6 supplies ready-made tokens; localStorage must survive `cy.reload()`. */
const SESSION_STORAGE_KEY = "jackioh.e2e.session";

/** Seat 2 lives in Node for the whole file; `tasks/index.ts` resets the sockets per spec. */
const SEAT_TWO = "seat-two";
const SEAT_TWO_ID: PlayerId = "p2";

const LANES: readonly Lane[] = [1, 2, 3, 4, 5];
const SIDES: readonly Side[] = ["you", "opponent"];

/** `promptOptionId("")` is the prefix, so no spec spells a testid format. */
const OPTION_PREFIX = promptOptionId("");

function spec8Name(index: number): string {
  const name = CARD_NAMES[index];
  if (name === undefined) throw new Error(`no SPEC §8 card #${String(index)}`);
  return name;
}

/** SPEC §8 #65: the Quickdraw Field Spell whose start-of-turn choice this spec reloads on. */
const MASOCHISM_MASK = spec8Name(65);

/** SPEC §8 #77 installs the modifier the reload must rebuild. */
const PROFESSOR_CURVATURE = spec8Name(77);

/** R48/R65's live discount has no `(next turn)` suffix. */
const CURVATURE_LIVE_LABEL = "(4)+ Cost cards cost (1) less";

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

function visitAs(token: string, path: string): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
    },
  });
}

// Reading seat 2's socket.

/** `crates/server/src/db/store.rs` `MatchClocks`, restated structurally (see support/types.ts). */
type Clocks = {
  turnDeadline: number | null;
  promptDeadline: number | null;
  graceDeadline: { p1: number | null; p2: number | null };
  ceilingAt: number;
};

type ClockFrame = { now: number; clocks: Clocks };

type SideLike = { hand?: { instanceId: string }[] | { count: number } };

function handIdsOf(view: Record<string, unknown> | null | undefined): string[] {
  const you = (view?.you ?? {}) as SideLike;
  const hand = you.hand;
  // §10.8: the viewer's own hand is full cards; only the opponent's is a count.
  expect(Array.isArray(hand), "the viewer's own hand arrives as cards, not a count (§10.8)").to.eq(
    true,
  );
  return (Array.isArray(hand) ? hand : []).map((card) => card.instanceId);
}

function clockFrames(name: string): Cypress.Chainable<ClockFrame[]> {
  return cy
    .wsPlayer({ action: "messages", name })
    .then((result) => (result.messages ?? []).filter((message) => message.type === "clock"))
    .then((frames) => frames as unknown as ClockFrame[]);
}

function lastClockFrame(name: string): Cypress.Chainable<ClockFrame> {
  return clockFrames(name).then((frames) => {
    const last = frames.at(-1);
    expect(last, `a clock frame on ${name}'s socket (R79, M7-T1)`).to.not.eq(undefined);
    return last as ClockFrame;
  });
}

/** Keep everything: R9's mulligan answer names the cards kept, so that is the whole hand. */
function seatTwoKeepsMulligan(): void {
  cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { promptKind: "mulligan" } }).then(
    (result) => {
      const body: ActionInput = {
        type: "mulligan",
        keep: handIdsOf(result.view),
        // Discarded by `parseClientMessage`; the actor stamps the authenticated seat itself.
        playerId: SEAT_TWO_ID,
      };
      cy.wsPlayer({ action: "send", name: SEAT_TWO, body });
    },
  );
}

function seatTwoEndsTurn(): void {
  cy.wsPlayer({ action: "awaitView", name: SEAT_TWO, where: { active: SEAT_TWO_ID } });
  cy.wsPlayer({ action: "send", name: SEAT_TWO, body: { type: "endTurn", playerId: SEAT_TWO_ID } });
}

/** The turn is seat 1's again when the client re-enables its own `end-turn` (BUILD M5-T1). */
function waitForMyTurn(): void {
  cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
}

// The view fingerprint.

type ViewFingerprint = {
  /** Every `data-testid` in the document, sorted. */
  testids: string[];
  /** The open prompt's kind, or null when none is open (BUILD M5-T4 `data-prompt-kind`). */
  promptKind: string | null;
  /** Per-side counters and mana, read through the testids support/testids.ts builds. */
  counters: Record<string, string>;
  /** R169 badges include captions: R48's suffix is computed when the view is read. */
  modifiers: Record<string, string[]>;
};

/** Reads the testid vocabulary (BUILD M5-T1), not presentation selectors. */
function readView($body: JQuery<HTMLElement>): ViewFingerprint {
  const testids = $body
    .find("[data-testid]")
    .map((_index, element) => element.getAttribute("data-testid") ?? "")
    .get()
    .sort();

  const prompt = $body.find(PROMPT);

  const counters: Record<string, string> = {};
  for (const side of SIDES) {
    for (const id of [
      handCountId(side),
      libraryCountId(side),
      graveyardCountId(side),
      exileCountId(side),
    ]) {
      counters[id] = $body.find(ts(id)).text().trim();
    }
    counters[manaId(side)] = String($body.find(`${ts(manaId(side))} ${MANA_CRYSTAL}`).length);
  }

  const modifiers: Record<string, string[]> = {};
  for (const side of SIDES) {
    modifiers[modifiersId(side)] = $body
      .find(`${ts(modifiersId(side))} ${MODIFIER_BADGE}`)
      .map(
        (_index, element) =>
          `${element.getAttribute("data-modifier-id") ?? ""}=${(element.textContent ?? "").trim()}`,
      )
      .get();
  }

  return {
    testids,
    promptKind: prompt.length === 0 ? null : (prompt.attr("data-prompt-kind") ?? null),
    counters,
    modifiers,
  };
}

function fingerprint(): Cypress.Chainable<ViewFingerprint> {
  cy.settled();
  return cy.get("body", { log: false }).then(($body) => readView($body));
}

function optionKeysOf(print: ViewFingerprint): string[] {
  return print.testids
    .filter((testid) => testid.startsWith(OPTION_PREFIX))
    .map((testid) => testid.slice(OPTION_PREFIX.length));
}

describe("05 reconnect — a networked game reloaded mid-prompt", () => {
  it("rebuilds the same view and the same open prompt, with the clock still running", () => {
    const seed = seedFor("05-reconnect-14");
    const seatOne = accounts.p1();
    const seatTwo = accounts.p2();
    let matchId = "";
    let before: ViewFingerprint = { testids: [], promptKind: null, counters: {}, modifiers: {} };
    let beforeClock: ClockFrame = {
      now: 0,
      clocks: { turnDeadline: null, promptDeadline: null, graceDeadline: { p1: null, p2: null }, ceilingAt: 0 },
    };
    let clockFramesBefore = 0;

    // Free persistent E2E accounts; fixture decks are saved by id (§9.4, R250, R257; SURFACE §11.3).
    cy.freeAccount(seatOne);
    cy.freeAccount(seatTwo);
    let seatOneDeck = "";
    let seatTwoDeck = "";
    cy.installLoadout(seatOne, "05-reconnect-a").then((installed) => {
      seatOneDeck = installed.deckIds[0];
    });
    cy.installLoadout(seatTwo, "05-reconnect-b").then((installed) => {
      seatTwoDeck = installed.deckIds[0];
    });

    // Seat 1 creates and seat 2 claims the seeded room (§9.5). R143 requires the E2E server to
    // honour the optional §9.3 seed; bodies wait for the installed deck ids.
    cy.then(() => {
      cy.request<{ code: string }>({
        method: "POST",
        url: api("/api/rooms"),
        headers: bearer(seatOne.token),
        body: { mode: "bo1", deckId: seatOneDeck, seed },
      }).then((created) => {
        cy.request<{ matchId: string; seat: string }>({
          method: "POST",
          url: api(`/api/rooms/${created.body.code}/join`),
          headers: bearer(seatTwo.token),
          body: { mode: "bo1", deckId: seatTwoDeck },
        }).then((joined) => {
          expect(joined.body.seat, "the joiner is seat 2 (§9.5)").to.eq(SEAT_TWO_ID);
          matchId = joined.body.matchId;
        });
      });
    });

    cy.then(() => {
      // `wsPlayer` defaults the omitted seat to the joining p2 (ASK: type its `seat` field).
      cy.wsPlayer({
        action: "connect",
        name: SEAT_TWO,
        url: server.ws(),
        token: seatTwo.token,
        matchId,
      });
      visitAs(seatOne.token, routes.match(matchId));
    });

    // R9/R265 mulligans open together; answer browser first, then socket.
    cy.keepMulligans();
    seatTwoKeepsMulligan();
    waitForMyTurn();

    // Player-turn 1: #65 costs 2 (§2.3).
    cy.endTurn();
    seatTwoEndsTurn();
    waitForMyTurn();

    // Player-turn 3: #65 takes a backrow zone (R81).
    cy.playByName(MASOCHISM_MASK, { zone: { side: "you", row: "backrow", lane: 1 } });
    cy.endTurn();
    seatTwoEndsTurn();

    // Player-turn 5: `PendingChoice` permits only `answer`, so wait for its prompt, not end turn.
    cy.waitForPrompt("discover");
    // #65's first option leaves hero health unchanged.
    cy.answerPrompt("discover", { first: 1 });
    cy.noPrompt();
    // R48 makes this discount live on player-turn 7, when the reload occurs.
    cy.playByName(PROFESSOR_CURVATURE, { zone: { side: "you", row: "units", lane: 1 } });
    cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
    cy.endTurn();
    seatTwoEndsTurn();

    // Player-turn 7: the prompt is over a populated badge list.
    cy.waitForPrompt("discover");

    // R169's badge must be present for this self-comparison; R48's suffix is now gone.
    cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
    cy.get(ts(modifiersId("you"))).find(MODIFIER_BADGE).should("have.text", CURVATURE_LIVE_LABEL);

    fingerprint().then((print) => {
      before = print;
      expect(
        print.promptKind,
        "the Mask's start-of-turn `mode` choice (§10.6) of three is drawn as a Discover pop-up (#88)",
      ).to.eq("discover");
      expect(
        optionKeysOf(print).length,
        '#65 base offers three options: "exile the bottom card of your library, lose 3 health, or summon a Spikey Pillow"',
      ).to.eq(3);
      // Compare the populated R169 badge list, not a fragile deterministic instance id.
      const badges = print.modifiers[modifiersId("you")] ?? [];
      expect(badges, "R169's badge list reached the fingerprint").to.have.length(1);
      expect(badges[0] ?? "", "…carrying R48's live caption, not just an id").to.have.string(
        `=${CURVATURE_LIVE_LABEL}`,
      );
      expect(
        print.modifiers[modifiersId("opponent")] ?? [],
        "seat 2 installed none of its own, so the badge above is seat 1's",
      ).to.have.length(0);
    });

    clockFrames(SEAT_TWO).then((frames) => {
      clockFramesBefore = frames.length;
    });
    lastClockFrame(SEAT_TWO).then((frame) => {
      beforeClock = frame;
      expect(frame.clocks.turnDeadline, "the active player's turn clock is armed (R79)").to.not.eq(
        null,
      );
      expect(
        (frame.clocks.turnDeadline ?? 0) - frame.now,
        `at most TURN_CLOCK_SECONDS (${String(TURN_CLOCK_SECONDS)} s) remains on a turn clock (R79)`,
      ).to.be.at.most(TURN_CLOCK_SECONDS * 1000);
      expect(
        frame.clocks.promptDeadline,
        `no ${String(PROMPT_CLOCK_SECONDS)} s prompt clock: R79 arms it only for a prompt held by the non-active player`,
      ).to.eq(null);
    });

    // Reload: attach receives a fresh §9.5 view and clears seat 1's grace.
    cy.reload();

    cy.waitForPrompt("discover");

    // Retry the DOM read: R612's asynchronous `match-ranks` banner can follow the socket view.
    cy.settled();
    cy.get("body", { log: false }).should(($body) => {
      const after = readView($body);
      expect(after.testids, "the same rendered view: every testid, before and after").to.deep.eq(
        before.testids,
      );
      expect(after.promptKind, "the same open prompt kind").to.eq(before.promptKind);
      expect(after.counters, "the same hand, library, graveyard, exile and mana readouts").to.deep.eq(
        before.counters,
      );
      // R169/R48 captions are derived on each fresh §9.5 view read.
      expect(after.modifiers, "the same player modifiers, side by side").to.deep.eq(
        before.modifiers,
      );
      expect(optionKeysOf(after), "the same PendingChoice options (§10.8)").to.deep.eq(
        optionKeysOf(before),
      );
    });

    // Essential reconnected-board testids.
    cy.get(ts(heroId("you"))).should("be.visible");
    cy.get(ts(heroId("opponent"))).should("be.visible");
    for (const lane of LANES) {
      cy.get(ts(zoneId("you", "units", lane))).should("exist");
      cy.get(ts(zoneId("you", "backrow", lane))).should("exist");
    }

    // The clock kept running.
    cy.then(() => {
      cy.wsPlayer({ action: "messages", name: SEAT_TWO }).should((result) => {
        const seen = (result.messages ?? []).filter((message) => message.type === "clock").length;
        expect(
          seen,
          "seat 1's reconnect pushed a fresh clock to seat 2 as well (§9.5: both clients show it)",
        ).to.be.greaterThan(clockFramesBefore);
      });
    });

    cy.then(() => {
      lastClockFrame(SEAT_TWO).should((after) => {
        const beforeRemaining = (beforeClock.clocks.turnDeadline ?? 0) - beforeClock.now;
        const afterRemaining = (after.clocks.turnDeadline ?? 0) - after.now;

        expect(
          after.clocks.turnDeadline,
          "the turn clock was not restarted by the reconnect: the same absolute deadline",
        ).to.eq(beforeClock.clocks.turnDeadline);
        expect(after.now, "the server's clock advanced across the reload").to.be.greaterThan(
          beforeClock.now,
        );
        expect(
          afterRemaining,
          "§9.5: the clock keeps running while a player is disconnected",
        ).to.be.lessThan(beforeRemaining);
        expect(afterRemaining, "and it has not expired: the turn is still seat 1's").to.be.greaterThan(
          0,
        );
        expect(
          after.clocks.ceilingAt,
          "the ceiling is measured from the match's start, not from the reconnect (R79)",
        ).to.eq(beforeClock.clocks.ceilingAt);
        expect(
          after.clocks.graceDeadline.p1,
          `seat 1 returned inside DISCONNECT_GRACE_SECONDS (${String(DISCONNECT_GRACE_SECONDS)} s), so its grace is cleared (§9.5)`,
        ).to.eq(null);
      });
    });

    // Any grace countdown seat 2 did observe while seat 1 was away is bounded by R79's value.
    cy.then(() => {
      clockFrames(SEAT_TWO).should((frames) => {
        for (const frame of frames) {
          const grace = frame.clocks.graceDeadline.p1;
          if (grace === null) continue;
          expect(
            grace - frame.now,
            `a grace countdown never exceeds DISCONNECT_GRACE_SECONDS (${String(DISCONNECT_GRACE_SECONDS)} s, R79)`,
          ).to.be.at.most(DISCONNECT_GRACE_SECONDS * 1000);
        }
      });
    });

    // §9.3: answer the rebuilt `PendingChoice`, not a screenshot.
    cy.then(() => {
      const [first] = optionKeysOf(before);
      expect(first, "an option key to answer with").to.be.a("string");
      cy.answerPrompt("discover", { options: [first ?? ""] });
      cy.noPrompt();
      waitForMyTurn();
    });
  });
});
