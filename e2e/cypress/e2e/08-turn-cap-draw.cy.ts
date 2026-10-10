// BUILD M8: R389 doubles R2's cap to 60 player-turns, 30 per seat; §2.5 makes it a draw.
// #75 Infinite Reserves replaces fatigue with Rush Tokens; both opening hands hold it (§2.4).
// R82 auto-ends no-action turns after Reserves; max mana stays below the other cards' costs (§2.3).

import { constants, seedFor } from "../../support/config.ts";
import { END_TURN, ts } from "../../support/testids.ts";
import type { GameStateLike, PlayerId } from "../../support/types.ts";

/** BUILD M8 seed: both seats open #75 Infinite Reserves; `--expose seed=…` overrides it. */
const SEED = seedFor("08-turn-cap-26");

const CAP = constants.TURN_CAP_PLAYER_TURNS;
const TURNS_EACH = CAP / 2;

const INFINITE_RESERVES = "core-075";

/** Bounds the recursion so a stalled game fails. */
const STEP_BUDGET = CAP + 6;

type SidePeek = {
  hero?: { health: number };
  mana?: { current: number; max: number };
  hand?: { id: string; defId: string }[];
  turnsStarted?: number;
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

function holdsReserves(state: GameStateLike, player: PlayerId): boolean {
  return (peek(state, player).hand ?? []).some((card) => card.defId === INFINITE_RESERVES);
}

/** BUILD M5-T3: move the hotseat device to the acting player. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat names the seat holding the device").to.not.eq(undefined);
    if (handle.seat !== player) cy.handOver();
  });
}

describe("BUILD M8 08 — two decks that never fatigue run out the turn cap as a draw", () => {
  it("R389 ends the game as a draw after the 60th player-turn, and R82 ends the dead turns itself", () => {
    cy.seedGame({ seed: SEED, a: "08-reserves-a", b: "08-reserves-b" });

    cy.gameState().then((opening) => {
      expect(opening.result, "the game is live after setup").to.eq(null);
      for (const player of ["p1", "p2"] as const) {
        expect(peek(opening, player).hero?.health, `${player} starts at HERO_HEALTH`).to.eq(constants.HERO_HEALTH);
        expect(holdsReserves(opening, player), `the seed deals ${player} Infinite Reserves in its opening hand`).to.eq(
          true,
        );
      }
    });

    // R82 already ended any no-action turn, so a disabled end-turn is a failure.
    let pressed = 0;
    let played = 0;
    const runOut = (left: number): void => {
      cy.gameState().then((state) => {
        if (state.result !== null) return;
        expect(left, `the game ended inside ${STEP_BUDGET} steps`).to.be.greaterThan(0);
        ensureSeat(state.active);
        if (holdsReserves(state, state.active)) {
          cy.playByName("Infinite Reserves", { zone: { side: "you", row: "backrow", lane: 1 } });
          played += 1;
        } else {
          cy.get(ts(END_TURN)).should("not.be.disabled");
          cy.endTurn();
          pressed += 1;
        }
        runOut(left - 1);
      });
    };
    runOut(STEP_BUDGET);

    cy.gameState().then((state) => {
      expect(played, "each seat played its Infinite Reserves").to.eq(2);
      expect(peek(state, "p1").turnsStarted, "R389: seat 1 started 30 turns").to.eq(TURNS_EACH);
      expect(peek(state, "p2").turnsStarted, "R389: seat 2 started 30 turns").to.eq(TURNS_EACH);
      expect(state.turn, "R389: the player-turn counter reached the cap").to.be.at.least(CAP);

      expect(state.result?.winner, "§2.5: the turn cap is a draw").to.eq("draw");
      expect(peek(state, "p1").hero?.health, "seat 1 never took damage").to.eq(constants.HERO_HEALTH);
      expect(peek(state, "p2").hero?.health, "seat 2 never took damage").to.eq(constants.HERO_HEALTH);

      expect(pressed, "R82: the engine ended at least one dead turn itself").to.be.lessThan(CAP);
    });

    // BUILD M5-T4: §10.8 requires checking the draw from both seats.
    cy.expectResult("Draw");
    cy.handOver();
    cy.expectResult("Draw");
  });
});
