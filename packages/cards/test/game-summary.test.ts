// SPEC §9.11, R376: the engine's `summarizeGame` over real cards. packages/engine
// test/game-summary.test.ts proves it with fixtures; this file plays real games with SPEC §10.7's
// policy, as the fuzz gate does, and checks the record against oracles it does not use:
//
//  - the opening hands are the hands the state holds once the mulligans resolve (p1's less its
//    first turn's draw), and The Coin is in the second seat's;
//  - the plays are the hand cards the logged `play` actions named, less those countered in their
//    announce window (§10.5 step 3a, R448), which were never played, and each as the card it resolves
//    as when step 3 replaced it (C #23 Devil's Pact, R449): these games counter and replace some. #96
//    My Pawn plays out a turn for its owner's opponent inside the action of the attack that sprang it,
//    with no `play` in the log, so a game in which it fired may hold more plays than the log names,
//    never fewer and never out of order;
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

type Drawn = Extract<GameEvent, { type: "drawn" }>;

/**
 * The cards a step drew into each hand from event `from` on, by what became of each card a `drawn`
 * names, whatever came between: it entered the hand (an `addedToHand` names it), burned on a full
 * hand (a `burned` does, R4) or was cast on its draw (§2.4: its `cardPlayed`, or the `countered` or
 * `transformed` that took its place, R448, R449). Only the first reached the hand. A cast's choices
 * are asked before its announce, so a draw whose fate a prompt holds waits in `waiting` for a later
 * step's events.
 */
function drawsKept(
  events: readonly GameEvent[],
  from: number,
  waiting: Drawn[],
): { kept: Record<PlayerId, string[]>; burns: number; casts: number } {
  const kept: Record<PlayerId, string[]> = { p1: [], p2: [] };
  let burns = 0;
  let casts = 0;
  for (const event of events.slice(from)) {
    if (event.type === "drawn") {
      waiting.push(event);
      continue;
    }
    if (!["addedToHand", "burned", "cardPlayed", "countered", "transformed"].includes(event.type)) continue;
    const at = waiting.findIndex((drawn) => "instanceId" in event && drawn.instanceId === event.instanceId);
    const drawn = waiting[at];
    if (drawn === undefined) continue;
    waiting.splice(at, 1);
    if (event.type === "addedToHand") kept[drawn.player].push(drawn.defId);
    else if (event.type === "burned") burns += 1;
    else casts += 1;
  }
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
    let counteredPlays = 0;
    let replacedPlays = 0;

    for (let n = 1; n <= GAMES; n += 1) {
      const seed = `summary-cards-${String(n)}`;
      const shuffled = createRng(`summary-cards-decks-${String(n)}`).shuffle(POOL);
      const decks: [string[], string[]] = [shuffled.slice(0, DECK_SIZE), shuffled.slice(DECK_SIZE, DECK_SIZE * 2)];
      const policy = createRng(`summary-cards-policy-${String(n)}`);

      let state = beginGame(createGame({ seed, decks })).state;
      const log: Action[] = [];
      // A play is open while its card waits in the resolving zone (§10.5), which a prompt can hold past
      // its action (R449's replacement's choices, C #4 Palantir's announce window). Before its announce,
      // a `transformed` naming it is step 3's replacement (R449), the card the play resolves as; after
      // it, a `countered` naming it takes the play back.
      type Play = { id: string; defId: string; open: boolean; announced: boolean };
      const plays: Record<PlayerId, Play[]> = { p1: [], p2: [] };
      const drawn: Record<PlayerId, string[]> = { p1: [], p2: [] };
      let opening: Record<PlayerId, string[]> | null = null;
      let begun = false;
      let pawn = false;
      const castIds: string[] = [];
      const unplaced = new Set<string>();
      const waiting: Drawn[] = [];

      while (state.result === null && log.length < MAX_ACTIONS) {
        const player = seatToAct(state);
        const chosen: ActionBody | null = subsystems.chooseAction(state, player, policy);
        if (chosen === null) throw new Error(`${seed}: no action for ${player}`);
        const action = { ...chosen, playerId: player, nonce: `g${String(log.length)}` } as Action;
        if (action.type === "play") {
          const card = state.players[player].hand.find((instance) => instance.id === action.instanceId);
          if (card !== undefined) plays[player].push({ id: card.id, defId: card.defId, open: true, announced: false });
        }
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(`${seed}: ${action.type} refused: ${result.error}`);
        const events: readonly GameEvent[] = result.events;
        for (const event of events) {
          if (event.type !== "transformed" && event.type !== "cardAnnounced" && event.type !== "countered") continue;
          const play = [...plays.p1, ...plays.p2].find((open) => open.open && open.id === event.instanceId);
          if (play === undefined) continue;
          if (event.type === "cardAnnounced") play.announced = true;
          else if (event.type === "transformed" && !play.announced) {
            play.id = event.newInstanceId;
            play.defId = event.toDefId;
            replacedPlays += 1;
          } else if (event.type === "countered" && play.announced) {
            plays[event.player].splice(plays[event.player].indexOf(play), 1);
            counteredPlays += 1;
          }
        }
        for (const seat of ["p1", "p2"] as const) {
          const resolving = new Set(result.state.players[seat].resolving.map((card) => card.id));
          for (const play of plays[seat]) play.open &&= resolving.has(play.id);
        }
        if (events.some((event) => event.type === "trapFired" && event.defId === MY_PAWN)) pawn = true;
        // A cast on its draw is a `cardPlayed` naming a drawn card before any hand took it: its announce
        // comes between (R448), and its choices may be asked first, in a prompt of their own (R70).
        for (const event of events) {
          if (event.type === "drawn") unplaced.add(event.instanceId);
          else if (event.type === "addedToHand" || event.type === "burned") unplaced.delete(event.instanceId);
          else if (event.type === "cardPlayed" && unplaced.delete(event.instanceId)) castIds.push(event.defId);
        }
        // The draws of the game start at the first `turnStarted`, in the step that resolves the
        // mulligans. When that step leaves p1 in its first turn, p2's hand is its opening hand, and
        // p1's is its opening hand plus turn 1's draws.
        const from = begun ? 0 : events.findIndex((event) => event.type === "turnStarted");
        if (from >= 0) {
          const step = drawsKept(events, from, waiting);
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
      for (const card of waiting) throw new Error(`${seed}: ${card.defId} (${card.instanceId}) was drawn and then nothing became of it`);

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
        const logged = plays[player].map((play) => play.defId);
        if (pawn) expect(subsequence(logged, played), `${seed} ${player}`).toBe(true);
        else expect(played, `${seed} ${player}`).toEqual(logged);
      }
      if (pawn) pawnGames += 1;
      casts += castIds.length;
    }

    // The games cast cards on their draw, after the opening hands too, burned draws on a full hand,
    // countered plays and replaced others, and most of them were held to the exact match of plays.
    expect(casts).toBeGreaterThan(0);
    expect(counteredPlays).toBeGreaterThan(0);
    expect(replacedPlays).toBeGreaterThan(0);
    expect(drawCasts).toBeGreaterThan(0);
    expect(burns).toBeGreaterThan(0);
    expect(pawnGames).toBeLessThan(GAMES / 2);
  });
});
