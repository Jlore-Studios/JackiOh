// The turn cap doubled (docs/classic-sets.md B4.3, R389, rewriting R2): 60 player-turns, 30 each, then
// the game is a draw. Its knock-on is proved too: two 20-card decks that do nothing now fatigue out
// before the cap, so fatigue is the usual end of a long game and the cap a backstop for games that
// never fatigue (§2.4, R3).

import type { Action, ActionInput } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { DECK_SIZE, TURN_CAP_PLAYER_TURNS } from "../src/config";
import { beginGame, reduce } from "../src/reduce";
import { type GameState } from "../src/state";
import { vanillaDeck } from "./fixtures/catalog";
import { newGame, put, slot } from "./fixtures/harness";
import { infiniteReserves } from "./fixtures/scripts";

let nonce = 0;

function act(state: GameState, body: ActionInput): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `cap${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** Past both mulligans, in p1's main phase on turn 1, with nothing ending a turn but End turn. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(`turn-cap-${seed}`, [vanillaDeck(DECK_SIZE, 1), vanillaDeck(DECK_SIZE, 21)])).state;
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
  }
  return state;
}

/** Both players press End turn until the game is over, and nothing else. */
function passUntilOver(state: GameState): GameState {
  let next = state;
  for (let step = 0; step <= 2 * TURN_CAP_PLAYER_TURNS && next.result === null; step += 1) {
    next = act(next, { type: "endTurn", playerId: next.active });
  }
  return next;
}

describe("B4.3: the turn cap (R389)", () => {
  it("R389 the cap is 60 player-turns, 30 each: a game that never fatigues is a draw at the end of the 60th", () => {
    expect(TURN_CAP_PLAYER_TURNS).toBe(60);
    const state = playing("reserves");
    // #75 Infinite Reserves turns every empty-library draw into a card, so no hero ever fatigues.
    put(state, infiniteReserves.id, slot("p1", "backrow", 1));
    put(state, infiniteReserves.id, slot("p2", "backrow", 1));
    const over = passUntilOver(state);
    expect(over.result).toEqual({ winner: "draw", reason: "turn-cap" });
    expect(over.turn).toBe(TURN_CAP_PLAYER_TURNS);
    expect(over.players.p1.turnsStarted).toBe(30);
    expect(over.players.p2.turnsStarted).toBe(30);
  });

  it("R389 two do-nothing 20-card decks fatigue out before the cap, so fatigue ends such a game (§2.4, R3)", () => {
    const over = passUntilOver(playing("fatigue"));
    expect(over.result?.reason).toBe("hero-death");
    expect(over.turn).toBeLessThan(TURN_CAP_PLAYER_TURNS);
    // B4.3's arithmetic: the second player's library empties first (four opening cards to three),
    // and their eighth fatigue draw kills them on their 24th turn, player-turn 48.
    expect(over.turn).toBe(48);
    expect(over.result?.winner).toBe("p1");
  });
});
