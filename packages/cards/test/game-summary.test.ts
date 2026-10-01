// SPEC §9.11, R376: the engine's `summarizeGame` over real cards. packages/engine
// test/game-summary.test.ts proves it with fixtures; this file plays real games with SPEC §10.7's
// policy, as the fuzz gate does, and checks the record against oracles it does not use:
//
//  - the opening hands are the hands the state holds once the mulligans resolve (p1's less its
//    first turn's draw), and The Coin is in the second seat's;
//  - the plays are the hand cards the logged `play` actions named. #96 My Pawn plays out a turn for
//    its owner's opponent inside the action of the attack that sprang it, with no `play` in the log,
//    so a game in which it fired may hold more plays than the log names, never fewer and never out
//    of order;
//  - so a cast (#21 Hinder or #90.1 CN-Virus cast on its draw, §2.4), which emits `cardPlayed` with
//    no `play` behind it, is never counted as one: these games cast some, and the plays still match;
//  - the draws are every card drawn from the first turn on, less those burned on a full hand (R4) and
//    those cast on their draw, which never reached the hand: these games burn and cast some.

import { describe, expect, it } from "vitest";
import type { Action, ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import {
  COIN_DEF_ID,
  DECK_SIZE,
  beginGame,
  createGame,
  createRng,
  reduce,
  seatToAct,
  subsystems,
  summarizeGame,
  type GameState,
} from "@jackioh/engine";
import { CATALOG, registerAll } from "../src/index";

const GAMES = 40;
const MAX_ACTIONS = 3000;
const MY_PAWN = "core-096";

const POOL: readonly string[] = Object.entries(CATALOG)
  .filter(([, def]) => def.token !== true && !def.tags.includes("Token"))
  .map(([id]) => id)
  .sort();

function hand(state: GameState, player: PlayerId): string[] {
  return state.players[player].hand.map((card) => card.defId);
}

function sorted(ids: readonly string[]): string[] {
  return [...ids].sort();
}

/**
 * The cards a step drew into each hand from event `from` on, by what became of each card a `drawn`
 * names, whatever came between: it entered the hand (an `addedToHand` names it), burned on a full
 * hand (a `burned` does, R4) or was cast on its draw (a `cardPlayed` does, §2.4). Only the first
 * reached the hand.
 */
function drawsKept(events: readonly GameEvent[], from: number): { kept: Record<PlayerId, string[]>; burns: number; casts: number } {
  const rest = events.slice(from);
  const kept: Record<PlayerId, string[]> = { p1: [], p2: [] };
  let burns = 0;
  let casts = 0;
  rest.forEach((event, at) => {
    if (event.type !== "drawn") return;
    const fate = rest
      .slice(at + 1)
      .find(
        (later) =>
          (later.type === "addedToHand" || later.type === "burned" || later.type === "cardPlayed") &&
          later.instanceId === event.instanceId,
      );
    if (fate?.type === "addedToHand") kept[event.player].push(event.defId);
    else if (fate?.type === "burned") burns += 1;
    else if (fate?.type === "cardPlayed") casts += 1;
    else throw new Error(`${event.defId} (${event.instanceId}) was drawn and then nothing became of it`);
  });
  return { kept, burns, casts };
}

/** Whether `part` is `whole` with some entries left out, in order. */
function subsequence(part: readonly string[], whole: readonly string[]): boolean {
  let at = 0;
  for (const id of whole) if (at < part.length && part[at] === id) at += 1;
  return at === part.length;
}

describe("summarizeGame over real cards (§9.11)", () => {
  it("R376 reads opening hands, plays and casts off real games as the state and the log show them", { timeout: 120_000 }, () => {
    registerAll();
    let pawnGames = 0;
    let casts = 0;
    let burns = 0;
    let drawCasts = 0;

    for (let n = 1; n <= GAMES; n += 1) {
      const seed = `summary-cards-${String(n)}`;
      const shuffled = createRng(`summary-cards-decks-${String(n)}`).shuffle(POOL);
      const decks: [string[], string[]] = [shuffled.slice(0, DECK_SIZE), shuffled.slice(DECK_SIZE, DECK_SIZE * 2)];
      const policy = createRng(`summary-cards-policy-${String(n)}`);

      let state = beginGame(createGame({ seed, decks })).state;
      const log: Action[] = [];
      const plays: Record<PlayerId, string[]> = { p1: [], p2: [] };
      const drawn: Record<PlayerId, string[]> = { p1: [], p2: [] };
      let opening: Record<PlayerId, string[]> | null = null;
      let begun = false;
      let pawn = false;
      const castIds: string[] = [];

      while (state.result === null && log.length < MAX_ACTIONS) {
        const player = seatToAct(state);
        const chosen: ActionBody | null = subsystems.chooseAction(state, player, policy);
        if (chosen === null) throw new Error(`${seed}: no action for ${player}`);
        const action = { ...chosen, playerId: player, nonce: `g${String(log.length)}` } as Action;
        if (action.type === "play") {
          const card = state.players[player].hand.find((instance) => instance.id === action.instanceId);
          if (card !== undefined) plays[player].push(card.defId);
        }
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(`${seed}: ${action.type} refused: ${result.error}`);
        const events: readonly GameEvent[] = result.events;
        if (events.some((event) => event.type === "trapFired" && event.defId === MY_PAWN)) pawn = true;
        events.forEach((event, at) => {
          const previous = events[at - 1];
          if (event.type === "cardPlayed" && previous?.type === "drawn" && previous.instanceId === event.instanceId) {
            castIds.push(event.defId);
          }
        });
        // The draws of the game start at the first `turnStarted`, in the step that resolves the
        // mulligans. When that step leaves p1 in its first turn, p2's hand is its opening hand, and
        // p1's is its opening hand plus turn 1's draws.
        const from = begun ? 0 : events.findIndex((event) => event.type === "turnStarted");
        if (from >= 0) {
          const step = drawsKept(events, from);
          if (!begun && state.phase === "mulligan" && result.state.phase !== "mulligan" && result.state.turn === 1) {
            const p1 = hand(result.state, "p1");
            for (const card of step.kept.p1) p1.splice(p1.lastIndexOf(card), 1);
            opening = { p1, p2: hand(result.state, "p2") };
          }
          begun = true;
          drawn.p1.push(...step.kept.p1);
          drawn.p2.push(...step.kept.p2);
          burns += step.burns;
          drawCasts += step.casts;
        }
        log.push(action);
        state = result.state;
      }
      if (state.result === null) throw new Error(`${seed} did not finish`);

      const summary = summarizeGame({ seed, decks, log });
      if (summary === null) throw new Error(`${seed} has no summary`);
      expect(summary.winner, seed).toBe(state.result.winner);
      expect(summary.reason, seed).toBe(state.result.reason);
      expect(summary.turns, seed).toBe(state.turn);
      expect(summary.first, seed).toBe("p1");

      if (opening !== null) {
        expect(sorted(summary.seats.p1.opening), seed).toEqual(sorted(opening.p1));
        expect(sorted(summary.seats.p2.opening), seed).toEqual(sorted(opening.p2));
      }
      // §2.1, R244: The Coin goes to the seat going second, after its mulligan.
      expect(summary.seats.p2.opening, seed).toContain(COIN_DEF_ID);
      expect(summary.seats.p1.opening, seed).not.toContain(COIN_DEF_ID);

      for (const player of ["p1", "p2"] as const) {
        expect(summary.seats[player].drawn, `${seed} ${player}`).toEqual(drawn[player]);
        const played = summary.seats[player].played;
        if (pawn) expect(subsequence(plays[player], played), `${seed} ${player}`).toBe(true);
        else expect(played, `${seed} ${player}`).toEqual(plays[player]);
      }
      if (pawn) pawnGames += 1;
      casts += castIds.length;
    }

    // The games cast cards on their draw, after the opening hands too, and burned draws on a full
    // hand, and most of them were held to the exact match of plays.
    expect(casts).toBeGreaterThan(0);
    expect(drawCasts).toBeGreaterThan(0);
    expect(burns).toBeGreaterThan(0);
    expect(pawnGames).toBeLessThan(GAMES / 2);
  });
});
