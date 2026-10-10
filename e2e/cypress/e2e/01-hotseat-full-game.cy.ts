// BUILD M8: a seeded hotseat game completes through the UI; its final hash matches replay (V19).
// The browser records actions, then `jackioh replay` folds and hashes them; equality proves accepted client actions in order (M5-T3).
// The seed is overridable for §4's 20-seed run; waits use `cy.settled` or retry, and selectors come from `support/testids.ts`.
// Fixture decks are choice-free, so generic play and attack open no `PendingChoice` (R81); seedGame and board clicks answer M5-T2.

import { seedFor, timeouts } from "../../support/config.ts";
import {
  LEGAL,
  RESULT_OVERLAY,
  cardId,
  handCardId,
  heroId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { GameStateLike, Lane, PlayerId, Row } from "../../support/types.ts";

/** BUILD M8's seed; `--expose seed=…` overrides it for the nightly sweep. */
const SEED = seedFor("01-hotseat");

/** §2.5 / R2 cap player turns; allow slack for `turnAutoEnded`. */
const MAX_TURNS = 40;
const MAX_PLAYS_PER_TURN = 12;

const ROWS: readonly Row[] = ["units", "backrow"];
const LANES: readonly Lane[] = [1, 2, 3, 4, 5];

function other(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}

/** Read only likely instance IDs; assertions stay on DOM `viewFor` (CLAUDE.md rule 7). */
type SidePeek = {
  hand?: { id: string; defId: string }[];
  units?: ({ id: string }[] | null)[];
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

function handOf(state: GameStateLike, player: PlayerId): string[] {
  return (peek(state, player).hand ?? []).map((card) => card.id);
}

/** §3.2: only a Stack pile's top card is active. */
function unitsOf(state: GameStateLike, player: PlayerId): string[] {
  return (peek(state, player).units ?? []).flatMap((pile) => {
    const top = pile === null ? undefined : pile[0];
    return top === undefined ? [] : [top.id];
  });
}

/** Ask client `data-legal`, which mirrors engine `legalActions` (BUILD M5-T2). */
function firstLegal(testids: readonly string[]): Cypress.Chainable<string | null> {
  return cy.get("body", { log: false }).then(($body) => {
    const found = testids.find((testid) => $body.find(`${ts(testid)}${LEGAL}`).length > 0);
    // Cypress treats bare `null` as the current subject, so wrap the answer.
    return cy.wrap(found ?? null, { log: false });
  });
}

/** BUILD M5-T3: hand the hotseat device to the active seat. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}

function playWhilePossible(budget: number): void {
  if (budget <= 0) return;
  cy.gameState().then((state) => {
    if (state.result !== null) return;
    firstLegal(handOf(state, state.active).map(handCardId)).then((card) => {
      if (card === null) return;
      cy.get(ts(card)).click();
      cy.settled();
      // R81 / M5-T2: the zone is in `play`; click only a highlighted zone.
      firstLegal(ROWS.flatMap((row) => LANES.map((lane) => zoneId("you", row, lane)))).then((zone) => {
        if (zone !== null) {
          cy.get(ts(zone)).click();
          cy.settled();
        }
        playWhilePossible(budget - 1);
      });
    });
  });
}

/** Target an offered hero or unit; Taunt (§4.2 step 3) steers the run. */
function attackWith(attackers: readonly string[], index: number): void {
  const attacker = attackers[index];
  if (attacker === undefined) return;
  cy.gameState().then((state) => {
    if (state.result !== null) return;
    firstLegal([cardId(attacker)]).then((selectable) => {
      if (selectable === null) {
        attackWith(attackers, index + 1);
        return;
      }
      cy.get(ts(cardId(attacker))).click();
      const targets = [heroId("opponent"), ...unitsOf(state, other(state.active)).map(cardId)];
      firstLegal(targets).then((target) => {
        // M5-T2: clicking a `switchPosition`-only unit again resets selection.
        cy.get(ts(target ?? cardId(attacker))).click();
        cy.settled();
        attackWith(attackers, index + 1);
      });
    });
  });
}

function takeTurns(remaining: number): void {
  cy.gameState().then((state) => {
    expect(remaining, "the game reached a result inside the turn budget").to.be.greaterThan(0);
    if (state.result !== null) return;
    // Fixture decks are choice-free; a prompt here means contract drift.
    expect(state.pending, "01-aggro-a/b open no prompt once the mulligan is answered").to.eq(null);

    const me = state.active;
    ensureSeat(me);
    playWhilePossible(MAX_PLAYS_PER_TURN);

    cy.gameState().then((afterPlays) => {
      if (afterPlays.result !== null) return;
      attackWith(unitsOf(afterPlays, me), 0);

      cy.gameState().then((afterAttacks) => {
        if (afterAttacks.result !== null) return;
        // R82: auto-ended turns only hand the device over.
        if (afterAttacks.active === me) cy.endTurn();
        else cy.handOver();
        takeTurns(remaining - 1);
      });
    });
  });
}

describe("BUILD M8 01 — a seeded hotseat game played to completion through the UI", () => {
  it("shows the result overlay and the recorded log folds to the same state hash", () => {
    cy.seedGame({ seed: SEED, a: "01-aggro-a", b: "01-aggro-b" });

    takeTurns(MAX_TURNS);

    // §10.8: `viewFor` makes Win / Loss viewer-relative.
    cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.game }).should("be.visible");
    cy.jackioh().then((handle) => {
      const result = handle.state.result;
      expect(result, "the engine ended the game (§2.5)").to.not.eq(null);
      if (result === null) return;
      const shown = result.winner === "draw" ? "Draw" : result.winner === handle.seat ? "Win" : "Loss";
      cy.get(ts(RESULT_OVERLAY)).should("contain.text", shown);
      cy.get(ts(RESULT_OVERLAY)).should("have.attr", "data-reason", result.reason);
    });

    // `cy.replayCheck` folds the log with `jackioh replay` and compares `hashState` (§12).
    // It also requires the fold to accept every recorded action.
    cy.replayCheck("01-hotseat-full-game");
  });
});
