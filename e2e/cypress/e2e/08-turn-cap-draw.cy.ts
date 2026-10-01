// BUILD M8 `08-turn-cap-draw.cy.ts` — "Two decks that never fatigue (both seats hold #75 Infinite
// Reserves) or a game seeded near the cap, no lethal".
//
// Key assertions (BUILD M8's table, verbatim):
//
//   "after the 60th player-turn, 30 each, the overlay says Draw (R2, R389)"
//
// R389 doubled R2's cap: "60 player-turns, 30 each". So the assertion is not "some number of turns
// happened" but both halves of it — thirty turns started by each seat, sixty player-turns in all —
// and then §2.5's "End of the 60th turn → Draw" on the overlay. Both heroes are still at
// `HERO_HEALTH` when it appears, which is what makes it the cap's draw and not §2.5's "both heroes
// at 0 or less in the same check".
//
// Why Infinite Reserves: R389's own engine test (packages/engine/test/turn-cap.test.ts) shows that
// two do-nothing 20-card decks now fatigue out before the cap — the second seat's eighth fatigue
// draw kills it at player-turn 48 — so the old do-nothing decks can no longer reach it. The
// 08-reserves decks are 08-do-nothing-a and -b with #100 Ceaseless Void swapped for #75 Infinite
// Reserves, a (0) Field Spell: "Drawing from an empty deck gives you a Rush Token card instead of
// fatigue." Each seat plays it as soon as it holds it, and nothing else. With this spec's seed both
// seats hold it in their opening hands (checked below, so a seed that does not is a loud failure,
// not a fatigue loss), so both libraries run dry into Rush Tokens rather than fatigue, a full hand
// burns them (§2.4), and no hero loses a point of health.
//
// R82 is the other ruling here: "A turn ends by itself when the active player's only legal
// actions are ending the turn, conceding and offering a draw; the engine emits `turnAutoEnded`
// and ends the turn." Every other card in both decks costs 3 or more, and max mana is
// min(turns you have started, 4) (§2.3), so once a seat has played its Infinite Reserves on its
// first turn it has nothing left to do and the engine ends that turn itself — a turn this spec
// never clicks. That is asserted by counting the clicks: fewer presses than player-turns.
//
// House rules (BUILD M8): the seed is set here and overridable with `--expose seed=…`; there is
// no fixed `cy.wait(ms)` — every wait is `cy.settled()` or a retried assertion; every selector
// comes from `e2e/support/testids.ts`.

import { constants, seedFor } from "../../support/config.ts";
import { END_TURN, ts } from "../../support/testids.ts";
import type { GameStateLike, PlayerId } from "../../support/types.ts";

/**
 * Every spec sets a seed (BUILD M8); `--expose seed=…` overrides it. This one deals #75 Infinite
 * Reserves into both opening hands, which the engine's own run of the same game confirms: a draw at
 * the end of player-turn 60 with both heroes untouched.
 */
const SEED = seedFor("08-turn-cap-26");

/** §2.5 / R2 / R389: 60 player-turns, 30 each. `support/config.ts` keeps the number. */
const CAP = constants.TURN_CAP_PLAYER_TURNS;
const TURNS_EACH = CAP / 2;

/** #75, the card each seat plays and the only one. */
const INFINITE_RESERVES = "core-075";

/** One iteration per player-turn and per Infinite Reserves, and slack, so a stuck loop fails instead of hanging. */
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

/** BUILD M5-T3: a hotseat device is handed over, so make sure it is on the seat that has to act. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat names the seat holding the device").to.not.eq(undefined);
    if (handle.seat !== player) cy.handOver();
  });
}

describe("BUILD M8 08 — two decks that never fatigue run out the turn cap as a draw", () => {
  it("R389 ends the game as a draw after the 60th player-turn, and R82 ends the dead turns itself", () => {
    cy.seedGame({ seed: SEED, a: "08-reserves-a", b: "08-reserves-b" });

    // Nothing has happened yet but the opening draws: neither deck holds a Cast-on-draw card, a
    // Quickdraw card or a hand trigger, so no card can resolve unless this spec plays one, and the
    // only one it plays is Infinite Reserves.
    cy.gameState().then((opening) => {
      expect(opening.result, "the game is live after setup").to.eq(null);
      for (const player of ["p1", "p2"] as const) {
        expect(peek(opening, player).hero?.health, `${player} starts at HERO_HEALTH`).to.eq(constants.HERO_HEALTH);
        expect(holdsReserves(opening, player), `the seed deals ${player} Infinite Reserves in its opening hand`).to.eq(
          true,
        );
      }
    });

    // Whoever is to act plays Infinite Reserves if they hold it and otherwise presses `end-turn`,
    // until the engine hands back a result. Every turn this loop sees belongs to a player who has a
    // legal action, because R82 means the engine has already ended any turn that does not — which
    // is why `end-turn` must be live here, and a disabled one is a real failure rather than
    // something to skip.
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

    // "after the 60th player-turn, 30 each": R389's own arithmetic, both halves of it.
    cy.gameState().then((state) => {
      expect(played, "each seat played its Infinite Reserves").to.eq(2);
      expect(peek(state, "p1").turnsStarted, "R389: seat 1 started 30 turns").to.eq(TURNS_EACH);
      expect(peek(state, "p2").turnsStarted, "R389: seat 2 started 30 turns").to.eq(TURNS_EACH);
      expect(state.turn, "R389: the player-turn counter reached the cap").to.be.at.least(CAP);

      // "the overlay says Draw", and it is the cap's draw: §2.5's other draw is two heroes at 0
      // or less in the same check, and neither hero has lost a point of health in sixty turns.
      expect(state.result?.winner, "§2.5: the turn cap is a draw").to.eq("draw");
      expect(peek(state, "p1").hero?.health, "seat 1 never took damage").to.eq(constants.HERO_HEALTH);
      expect(peek(state, "p2").hero?.health, "seat 2 never took damage").to.eq(constants.HERO_HEALTH);

      // R82: some of those sixty player-turns ended without anybody pressing anything — each seat's
      // first, once Infinite Reserves is down and everything left in hand costs 3 or more.
      expect(pressed, "R82: the engine ended at least one dead turn itself").to.be.lessThan(CAP);
    });

    // BUILD M5-T4 `gameOver`: "overlay text Win / Loss / Draw". A draw is the one result that
    // reads the same from both seats, so it is asserted from both (§10.8 orients the view).
    cy.expectResult("Draw");
    cy.handOver();
    cy.expectResult("Draw");
  });
});
