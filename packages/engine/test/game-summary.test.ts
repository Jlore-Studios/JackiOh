// SPEC §9.11, R376: `summarizeGame` reads a finished game's record off `(seed, decks, log)`. Each
// figure is checked against an oracle the summary does not use: the hands the state holds once the
// mulligans resolve, the hand card each logged `play` names, and the drawn cards the hand holds once
// the step that drew them is done. The fixture Hinder is cast on draw (§2.4) and the fixture Going
// Long is a Quickdraw card (R225), so a cast and a Quickdraw deal are both in these games, and a full
// hand burns a draw (R4). packages/cards test/game-summary.test.ts repeats the oracles over real cards.

import type { Action, ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { DECK_SIZE, TURN_CAP_PLAYER_TURNS } from "../src/config";
import { summarizeGame } from "../src/gameSummary";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { fold } from "../src/replay";
import type { GameState } from "../src/state";
import { vanillaDeck } from "./fixtures/catalog";
import { newGame, playRandomGame } from "./fixtures/harness";

const P1_DECK = ["fx-hinder", "fx-going-long", ...vanillaDeck(DECK_SIZE - 2, 1)];
const P2_DECK = vanillaDeck(DECK_SIZE, 21);

function hand(state: GameState, player: PlayerId): string[] {
  return state.players[player].hand.map((card) => card.defId);
}

function sorted(ids: readonly string[]): string[] {
  return [...ids].sort();
}

type Step = { events: readonly GameEvent[]; after: GameState };

/**
 * The cards a step drew into each hand from event `from` on, read off the state the step left rather
 * than the order of its events: each card a `drawn` names that its seat holds once the step is done.
 * A card burned on a full hand (R4) or cast on its draw (§2.4) is in no hand, and no fixture card in
 * these decks moves a card out of a hand in the step that drew it, so that hand settles it.
 */
function drawsKept(step: Step, from = 0): Record<PlayerId, string[]> {
  const drawn: Record<PlayerId, string[]> = { p1: [], p2: [] };
  for (const event of step.events.slice(from)) {
    if (event.type !== "drawn") continue;
    if (step.after.players[event.player].hand.some((card) => card.id === event.instanceId)) {
      drawn[event.player].push(event.defId);
    }
  }
  return drawn;
}

/** Where the draws of the game start: the first `turnStarted` (§2.1), or -1 in a step before it. */
function turnStartIn(events: readonly GameEvent[]): number {
  return events.findIndex((event) => event.type === "turnStarted");
}

/** Steps a hand-written game, keeping the log and every step's events. */
function stepper(seed: string, decks: [string[], string[]]) {
  const begun = beginGame(newGame(seed, decks));
  let state = begun.state;
  const log: Action[] = [];
  const steps: { before: GameState; after: GameState; events: GameEvent[] }[] = [];
  return {
    get state() {
      return state;
    },
    log,
    steps,
    act(player: PlayerId, body: ActionBody): void {
      const action = { ...body, playerId: player, nonce: `s${String(log.length)}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(`${body.type} refused: ${result.error}`);
      steps.push({ before: state, after: result.state, events: result.events });
      log.push(action);
      state = result.state;
    },
  };
}

describe("summarizeGame (§9.11)", () => {
  it("R376 records each seat's deck, opening hand, draws and plays, and how the game ended", () => {
    const game = stepper("summary-hand-written", [P1_DECK, P2_DECK]);
    const dealt = game.state;
    const p1Dealt = dealt.players.p1.hand;
    // R225: the Quickdraw card is always dealt. p1 sends back one other card; p2 keeps its hand.
    expect(hand(dealt, "p1")).toContain("fx-going-long");
    const returned = p1Dealt.find((card) => card.defId !== "fx-going-long");
    if (returned === undefined) throw new Error("p1 was dealt nothing but the Quickdraw card");
    game.act("p1", { type: "mulligan", keep: p1Dealt.filter((card) => card !== returned).map((card) => card.id) });
    game.act("p2", { type: "mulligan", keep: dealt.players.p2.hand.map((card) => card.id) });

    // The step that resolved both mulligans also began turn 1 and drew its card.
    const resolved = game.steps[1];
    if (resolved === undefined) throw new Error("no resolving step");
    expect(resolved.after.turn).toBe(1);
    const turnOneDraws = drawsKept(resolved, turnStartIn(resolved.events));

    const play = legalActions(game.state, "p1").find((action) => action.type === "play");
    if (play === undefined || play.type !== "play") throw new Error("p1 has no play on turn 1");
    const playedDef = game.state.players.p1.hand.find((card) => card.id === play.instanceId)?.defId;
    game.act("p1", play);
    game.act("p1", { type: "endTurn" });
    game.act("p2", { type: "endTurn" });
    game.act("p1", { type: "concede" });

    const summary = summarizeGame({ seed: "summary-hand-written", decks: [P1_DECK, P2_DECK], log: game.log });
    if (summary === null) throw new Error("no summary of a finished game");

    expect(summary.first).toBe("p1");
    expect(summary.winner).toBe("p2");
    expect(summary.reason).toBe("concede");
    expect(summary.turns).toBe(game.state.turn);
    expect(summary.seats.p1.deck).toEqual(P1_DECK);
    expect(summary.seats.p2.deck).toEqual(P2_DECK);

    // The hand once the mulligans resolved: what the step left, less the first turn's draw.
    const p1Opening = [...hand(resolved.after, "p1")];
    for (const card of turnOneDraws.p1) p1Opening.splice(p1Opening.lastIndexOf(card), 1);
    expect(sorted(summary.seats.p1.opening)).toEqual(sorted(p1Opening));
    expect(summary.seats.p1.opening).toContain("fx-going-long");
    expect(summary.seats.p1.opening).not.toContain(returned.defId);
    expect(summary.seats.p1.opening).toHaveLength(hand(dealt, "p1").length);
    expect(summary.seats.p2.opening).toEqual(hand(dealt, "p2"));

    expect(summary.seats.p1.played).toEqual([playedDef]);
    expect(summary.seats.p2.played).toEqual([]);
    // Turn 1's draw, then p2's on turn 2 and p1's on turn 3, each drawn by the endTurn before it.
    const [, , , endOne, endTwo] = game.steps;
    if (endOne === undefined || endTwo === undefined) throw new Error("the turns did not end");
    expect(summary.seats.p1.drawn).toEqual([...turnOneDraws.p1, ...drawsKept(endTwo).p1]);
    expect(summary.seats.p2.drawn).toEqual(drawsKept(endOne).p2);
    expect(summary.seats.p1.drawn).toHaveLength(2);
    expect(summary.seats.p2.drawn).toHaveLength(1);
  });

  it("R376 counts a play from hand and never a cast, against the hand card each play names", { timeout: 120_000 }, () => {
    let casts = 0;
    let checked = 0;
    for (let n = 1; n <= 20; n += 1) {
      const seed = `summary-random-${String(n)}`;
      const live = playRandomGame(seed, [P1_DECK, P2_DECK]);
      const summary = summarizeGame({ seed, decks: live.decks, log: live.log });
      if (summary === null) throw new Error(`${seed} has no summary`);

      // The oracles: the hand card each logged `play` named, read off the state it was played from,
      // and each step's draws that its seat's hand holds once the step is done.
      let state = beginGame(newGame(seed, live.decks)).state;
      const plays: Record<PlayerId, string[]> = { p1: [], p2: [] };
      const drawn: Record<PlayerId, string[]> = { p1: [], p2: [] };
      let opening: Record<PlayerId, string[]> | null = null;
      let begun = false;
      for (const action of live.log) {
        if (action.type === "play") {
          const card = state.players[action.playerId].hand.find((instance) => instance.id === action.instanceId);
          if (card !== undefined) plays[action.playerId].push(card.defId);
        }
        const result = reduce(state, action);
        casts += result.events.filter((event) => event.type === "cardPlayed" && event.defId === "fx-hinder").length;
        // The draws of the game start at the first `turnStarted`, in the step that resolves the
        // mulligans. When that step leaves p1 in its first turn, p2's hand is its opening hand, and
        // p1's is its opening hand plus turn 1's draws.
        const from = begun ? 0 : turnStartIn(result.events);
        if (from >= 0) {
          const kept = drawsKept({ events: result.events, after: result.state }, from);
          if (!begun && state.phase === "mulligan" && result.state.phase !== "mulligan" && result.state.turn === 1) {
            const p1 = [...hand(result.state, "p1")];
            for (const card of kept.p1) p1.splice(p1.lastIndexOf(card), 1);
            opening = { p1, p2: hand(result.state, "p2") };
          }
          begun = true;
          drawn.p1.push(...kept.p1);
          drawn.p2.push(...kept.p2);
        }
        state = result.state;
      }

      expect(summary.seats.p1.played, seed).toEqual(plays.p1);
      expect(summary.seats.p2.played, seed).toEqual(plays.p2);
      expect(summary.seats.p1.drawn, seed).toEqual(drawn.p1);
      expect(summary.seats.p2.drawn, seed).toEqual(drawn.p2);
      expect(summary.winner, seed).toBe(live.state.result?.winner);
      if (opening !== null) {
        checked += 1;
        expect(sorted(summary.seats.p1.opening), seed).toEqual(sorted(opening.p1));
        expect(sorted(summary.seats.p2.opening), seed).toEqual(sorted(opening.p2));
      }
      // Every card drawn into a hand came out of that seat's library: its own deck.
      for (const player of ["p1", "p2"] as const) {
        const deck = new Set(live.decks[player === "p1" ? 0 : 1]);
        for (const card of summary.seats[player].drawn) expect(deck.has(card), `${seed} ${player} ${card}`).toBe(true);
      }
    }
    // The property was exercised: Hinder was cast in some game, and most games opened on turn 1.
    expect(casts).toBeGreaterThan(0);
    expect(checked).toBeGreaterThan(10);
  });

  it("R376 leaves out of a seat's draws a card burned on a full hand or cast on its draw", () => {
    // On this seed the deal leaves Hinder in p1's library, so a later draw casts it (§2.4), and both
    // seats keep their hands and only end their turns, so both hands fill to HAND_CAP and every draw
    // past it burns (R4) until the game ends.
    const seed = "summary-burn-and-cast-1";
    const decks: [string[], string[]] = [P1_DECK, P2_DECK];
    const game = stepper(seed, decks);
    expect(game.state.players.p1.library.map((card) => card.defId)).toContain("fx-hinder");
    game.act("p1", { type: "mulligan", keep: game.state.players.p1.hand.map((card) => card.id) });
    game.act("p2", { type: "mulligan", keep: game.state.players.p2.hand.map((card) => card.id) });
    for (let turn = 0; game.state.result === null; turn += 1) {
      if (turn > 2 * TURN_CAP_PLAYER_TURNS) throw new Error("the game did not end");
      game.act(game.state.active, { type: "endTurn" });
    }

    const summary = summarizeGame({ seed, decks, log: game.log });
    if (summary === null) throw new Error("no summary of a finished game");

    const kept: Record<PlayerId, string[]> = { p1: [], p2: [] };
    const draws: Record<PlayerId, number> = { p1: 0, p2: 0 };
    const burned: Record<PlayerId, number> = { p1: 0, p2: 0 };
    let casts = 0;
    let begun = false;
    for (const step of game.steps) {
      const from = begun ? 0 : turnStartIn(step.events);
      if (from < 0) continue;
      begun = true;
      const events = step.events.slice(from);
      const keptHere = drawsKept(step, from);
      kept.p1.push(...keptHere.p1);
      kept.p2.push(...keptHere.p2);
      for (const event of events) {
        if (event.type === "drawn") draws[event.player] += 1;
        if (event.type === "burned") burned[event.owner] += 1;
        if (event.type === "cardPlayed") casts += 1;
      }
    }

    // Both kinds of lost draw happened: a Hinder cast on its draw, and a draw burned in each seat.
    expect(casts).toBeGreaterThan(0);
    expect(burned.p1).toBeGreaterThan(0);
    expect(burned.p2).toBeGreaterThan(0);
    // Nothing was played from a hand, and only what reached a hand is a draw.
    expect(summary.seats.p1.played).toEqual([]);
    expect(summary.seats.p2.played).toEqual([]);
    expect(summary.seats.p1.drawn).toEqual(kept.p1);
    expect(summary.seats.p2.drawn).toEqual(kept.p2);
    expect(summary.seats.p1.drawn).toHaveLength(draws.p1 - burned.p1 - casts);
    expect(summary.seats.p2.drawn).toHaveLength(draws.p2 - burned.p2);
    expect(summary.seats.p1.drawn).not.toContain("fx-hinder");
  });

  it("R376 makes no summary of a game without a result", () => {
    const game = stepper("summary-unfinished", [P1_DECK, P2_DECK]);
    game.act("p1", { type: "mulligan", keep: [] });
    expect(summarizeGame({ seed: "summary-unfinished", decks: [P1_DECK, P2_DECK], log: game.log })).toBeNull();
  });

  it("R376 leaves the opening hands empty when the game ended before a turn began", () => {
    const game = stepper("summary-early-concede", [P1_DECK, P2_DECK]);
    game.act("p2", { type: "concede" });
    const summary = summarizeGame({ seed: "summary-early-concede", decks: [P1_DECK, P2_DECK], log: game.log });
    expect(summary).toEqual({
      first: "p1",
      winner: "p1",
      reason: "concede",
      turns: 0,
      seats: {
        p1: { deck: P1_DECK, opening: [], drawn: [], played: [] },
        p2: { deck: P2_DECK, opening: [], drawn: [], played: [] },
      },
    });
  });

  it("skips a logged action the engine refuses, as the fold does", () => {
    const live = playRandomGame("summary-refused", [P1_DECK, P2_DECK]);
    const refused = { type: "endTurn", playerId: "p2", nonce: "refused-0" } as Action;
    const log = [refused, ...live.log];
    expect(fold({ seed: "summary-refused", decks: live.decks, log }).errors).toHaveLength(1);
    expect(summarizeGame({ seed: "summary-refused", decks: live.decks, log })).toEqual(
      summarizeGame({ seed: "summary-refused", decks: live.decks, log: live.log }),
    );
  });
});
