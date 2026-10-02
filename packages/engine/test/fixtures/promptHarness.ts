// Helpers the prompts-and-movement tests share (docs/classic-sets.md B5 E13, E16–E18, E26): a board
// with the fixtures registered, a Spell resolving, an answer by option key, a round trip, and a short
// replayable game whose opening hands hold the fixture under test (Quickdraw, R225).

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { expect } from "vitest";
import { DECK_SIZE } from "../../src/config";
import { answerPrompt, runHookResumable } from "../../src/prompts";
import { beginGame, reduce } from "../../src/reduce";
import { fold, hashState } from "../../src/replay";
import type { EngineSink } from "../../src/resolve";
import { createGame, newInstance, type CardInstance, type GameState, type PendingChoice } from "../../src/state";
import { settle } from "../../src/triggers";
import { vanillaDeck } from "./catalog";
import { newGame, setupCatalog, sinkFor } from "./harness";
import { registerPromptFixtures } from "./prompts";

/** p1's main phase on turn 3 with 4 mana, the fixtures registered (after `newGame`'s own). */
export function board(seed: string): GameState {
  const state = newGame(seed);
  registerPromptFixtures();
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  state.players.p1.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  state.players.p2.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  return state;
}

/** A card of `defId` resolving for `player` (§10.5 step 4), where a Spell's Cry runs. */
export function resolvingCard(state: GameState, defId: string, player: PlayerId = "p1", radiant = false): CardInstance {
  const card = newInstance(state, defId, player, { z: "resolving", player });
  card.radiant = radiant;
  state.players[player].resolving.push(card);
  return card;
}

/** Run a resolving card's Cry the resumable way and settle, as the pipeline does; returns the sink. */
export function castNow(state: GameState, defId: string, player: PlayerId = "p1", radiant = false): EngineSink {
  const sink = sinkFor(state);
  const card = resolvingCard(state, defId, player, radiant);
  runHookResumable(sink, card, "cry", { controller: player });
  settle(sink);
  state.rngCursor = sink.rng.cursor;
  return sink;
}

export function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

/** Answer the open prompt by option key, as a client sends back what it was offered. */
export function answerKeys(state: GameState, ...keys: string[]): { sink: EngineSink; error: string | null } {
  const pending = must(state.pending, "an open prompt");
  const selection = keys.map((key) => must(pending.options.find((option) => option.key === key), `option ${key}`).selection);
  const sink = sinkFor(state);
  const error = answerPrompt(sink, { playerId: pending.playerId, choiceId: pending.id, selection });
  if (error === null) settle(sink);
  state.rngCursor = sink.rng.cursor;
  return { sink, error };
}

/** The open prompt, asserted to be of this kind and held by this player. */
export function openAs(state: GameState, kind: PendingChoice["kind"], player: PlayerId): PendingChoice {
  const pending = must(state.pending, `a ${kind} prompt`);
  expect(pending.kind).toBe(kind);
  expect(pending.playerId).toBe(player);
  return pending;
}

/** §9.3: a paused state survives JSON, and the copy answers exactly as the original does. */
export function roundTrip(state: GameState): GameState {
  return JSON.parse(JSON.stringify(state)) as GameState;
}

export function eventTypes(events: readonly GameEvent[]): string[] {
  return events.map((event) => event.type);
}

let nonce = 0;
/** One action through `reduce`, logged, erroring loudly. */
export function act(state: GameState, body: ActionInput, log?: Action[]): GameState {
  nonce += 1;
  const action = { ...body, nonce: `pm${nonce}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(`${action.type} refused: ${result.error}`);
  log?.push(action);
  return result.state;
}

/**
 * A replayable game (§9.2, §9.3): each deck is vanilla fixtures plus the named Quickdraw cards, so the
 * opening hands hold them whatever the shuffle; both mulligans keep everything, and it is p1's first
 * main phase. The log and decks go to `fold`, which must rebuild the very same state.
 */
export function replayable(
  seed: string,
  p1Cards: readonly string[],
  p2Cards: readonly string[] = [],
): { state: GameState; log: Action[]; decks: [string[], string[]] } {
  setupCatalog();
  registerPromptFixtures();
  const decks: [string[], string[]] = [
    [...vanillaDeck(DECK_SIZE - p1Cards.length, 1), ...p1Cards],
    [...vanillaDeck(DECK_SIZE - p2Cards.length, 21), ...p2Cards],
  ];
  const log: Action[] = [];
  let state = beginGame(createGame({ seed, decks })).state;
  state = act(state, { type: "mulligan", playerId: "p1", keep: state.players.p1.hand.map((card) => card.id) }, log);
  state = act(state, { type: "mulligan", playerId: "p2", keep: state.players.p2.hand.map((card) => card.id) }, log);
  return { state, log, decks };
}

/** §9.2: fold the log from scratch and compare hashes with the live state. */
export function expectReplays(seed: string, decks: [string[], string[]], log: readonly Action[], live: GameState): void {
  const folded = fold({ seed, decks, log });
  expect(folded.errors).toEqual([]);
  expect(hashState(folded.state)).toBe(hashState(live));
}

/** The first card in `player`'s hand of this definition. */
export function handCard(state: GameState, player: PlayerId, defId: string): CardInstance {
  return must(state.players[player].hand.find((card) => card.defId === defId), `${defId} in ${player}'s hand`);
}
