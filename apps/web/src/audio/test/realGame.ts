// Test helper for the audio suite: real engine games, reached the way the client reaches them
// (`game/engine.ts`'s `enginePort()`, over the WebAssembly engine), so a test can feed the director the very views
// `viewFor` redacts for each seat (R97, R154). It is not a test file.
//
// The driver is deterministic: a seed, two decks and a caller's policy. Nothing here decides a
// rule; every action it sends comes from `legalActions`.

import type { Handicap } from "@jackioh/engine/config";
import type { Action, ActionBody, CardDefs, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { castOnDrawAt } from "../../fx/castOnDraw.ts";
import { resolveDeck } from "../../game/decks.ts";
import { enginePort, type EnginePort, type EngineState } from "../../game/engine.ts";

export type RealGame = {
  port: EnginePort;
  catalog: CardDefs;
  state(): EngineState;
  view(player: PlayerId): PlayerView;
  legal(player: PlayerId): ActionBody[];
  /** The seat that may act now: the one with a legal action other than conceding or offering a draw. */
  actor(): PlayerId | null;
  /** Reduces one action; throws on a refusal. Returns the unredacted events it produced. */
  act(player: PlayerId, body: ActionBody): GameEvent[];
};

let port: EnginePort | null = null;

/** Mulligans and the prompts they open: far more steps than a setup ever takes. */
const SETUP_STEPS_MAX = 8;
/** How far `castOnDrawViews` looks: seeds, and actions in each. */
const CAST_SEARCH_SEEDS = 400;
const CAST_SEARCH_STEPS = 12;

export function realPort(): EnginePort {
  port ??= enginePort();
  return port;
}

export function devDeck(id: "first20" | "cheap20"): string[] {
  const resolved = resolveDeck(id, realPort().catalog?.() ?? {});
  if (!("deck" in resolved)) throw new Error(resolved.error);
  return resolved.deck;
}

/**
 * A game after `beginGame`, with both mulligans answered by keeping every card. `handicaps` gives a
 * seat resources other than SPEC's own (R180): a short deck that fatigues soon, extra draws that fill
 * a hand, a deck at R80's cap. The engine validates them (R184) and throws on a bad one.
 */
export function realGame(
  seed: string,
  decks: [string[], string[]],
  handicaps?: Partial<Record<PlayerId, Handicap>>,
): RealGame {
  const p = realPort();
  let state = p.beginGame(p.createGame({ seed, decks, ...(handicaps === undefined ? {} : { handicaps }) })).state;
  let nonce = 0;
  const game: RealGame = {
    port: p,
    catalog: p.catalog?.() ?? {},
    state: () => state,
    view: (player) => p.viewFor(state, player),
    legal: (player) => p.legalActions(state, player),
    actor: () =>
      (["p1", "p2"] as const).find((player) =>
        p.legalActions(state, player).some((a) => a.type !== "concede" && a.type !== "offerDraw"),
      ) ?? null,
    act: (player, body) => {
      nonce += 1;
      const result = p.reduce(state, { ...body, playerId: player, nonce: `audio-${String(nonce)}` } as Action);
      if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
      state = result.state;
      return result.events;
    },
  };
  // Both mulligans, and any prompt they open on the way (a replacement draw that casts #21 Hinder asks
  // its caster to discard, so the window stays open until that is answered): the first listed answer.
  for (let i = 0; i < SETUP_STEPS_MAX; i += 1) {
    const who = game.actor();
    if (who === null) break;
    const legal = game.legal(who);
    if (legal.some((a) => a.type === "mulligan")) {
      const hand = game.view(who).you.hand;
      game.act(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
      continue;
    }
    const phase = game.view(who).phase;
    const answer = phase === "mulligan" || phase === "setup" ? legal.find((a) => a.type === "answer") : undefined;
    if (answer === undefined) break;
    game.act(who, answer);
  }
  return game;
}

/** The hand card's defId for an instance in `player`'s own hand, or null. */
export function handDefId(game: RealGame, player: PlayerId, instanceId: string): string | null {
  const hand = game.view(player).you.hand;
  if (!Array.isArray(hand)) return null;
  return hand.find((c) => c.instanceId === instanceId)?.defId ?? null;
}

/** A legal `play` of that instance, preferring one with no prompt-bearing extras, or undefined. */
export function playOf(game: RealGame, player: PlayerId, instanceId: string): ActionBody | undefined {
  return game.legal(player).find((a) => a.type === "play" && a.instanceId === instanceId);
}

/** Answers any open prompt with its first legal answer, until none is open. */
export function answerPrompts(game: RealGame): void {
  for (let i = 0; i < 10; i += 1) {
    const who = game.actor();
    if (who === null) return;
    const answer = game.legal(who).find((a) => a.type === "answer");
    if (answer === undefined) return;
    game.act(who, answer);
  }
}

/** Both seats' views of one moment of a real game. */
export type SeatViews = { p1: PlayerView; p2: PlayerView };

/**
 * The first seed (`<prefix>-0`, `-1`, …) where p2 casts `defId` as it draws it, with both seats'
 * views right after the cast has resolved (any prompt it asked answered with the first listed answer)
 * and `settled` holding of them; null when none of `seeds` does within `steps` actions. Every action
 * is an End turn or the first listed answer, so the search is deterministic.
 */
export function castOnDrawViews(
  prefix: string,
  decks: [string[], string[]],
  defId: string,
  settled: (seats: SeatViews) => boolean,
  seeds = CAST_SEARCH_SEEDS,
  steps = CAST_SEARCH_STEPS,
): SeatViews | null {
  for (let seed = 0; seed < seeds; seed += 1) {
    const game = realGame(`${prefix}-${String(seed)}`, decks);
    for (let step = 0; step <= steps; step += 1) {
      const seats = { p1: game.view("p1"), p2: game.view("p2") };
      const at = seats.p1.events.findIndex((e) => e.type === "cardPlayed" && e.defId === defId && e.player === "p2");
      if (at >= 0 && castOnDrawAt(seats.p1.events, at) && settled(seats)) return seats;
      const who = game.actor();
      if (who === null) break;
      game.act(who, game.legal(who).find((a) => a.type === "answer") ?? { type: "endTurn" });
    }
  }
  return null;
}
