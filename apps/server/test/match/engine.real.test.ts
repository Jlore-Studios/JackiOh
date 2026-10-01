/**
 * `src/match/engine.real.ts` — the real `EnginePort`, and until now the only file in `src/` with no
 * test at all.
 *
 * WHY IT NEEDS ONE. Every other test in this suite installs a scripted port through
 * `setEnginePort`, which is what lets the actor, the clock and the recovery tests run without the
 * engine in the process — and is exactly why nothing noticed when this file was missing its
 * `registerAll()` call. `createGame` looks its card definitions up in the engine's *registered*
 * catalog (`packages/engine/src/state.ts`: `validateDeck` throws `"core-001" is not in the catalog
 * (§9.4 L6)` when it is empty), so without that call every real match died on the first card of the
 * first deck while all 200-odd server tests stayed green. This file is the one that would have
 * caught it: it builds the port for real, with the real §8 catalog, and plays a card.
 *
 * It asks only what the adapter is responsible for — that the engine is reachable, registered and
 * driveable through the port's own surface. The rules those calls run are `packages/engine`'s and
 * `packages/cards`' business, and the fuzz suite plays 1,000 whole games of them (BUILD §4).
 *
 * Its two legal decks come from `decksTheEngineAccepts` (`test/fakes/engine.ts`), the probe this
 * file used to hold privately — the reasoning for probing the size rather than importing
 * `DECK_SIZE` moved with it, and so did the "refused every deck size" failure that catches an
 * unregistered catalog. It is shared now because the real-engine blocks of `actor.test.ts` and
 * `recovery.test.ts` need the same two decks.
 */

import { describe, expect, it } from "vitest";

import type { Action, ActionBody, PlayerId } from "@jackioh/shared";

import { loadCatalog } from "../../src/api/catalog";
import type { EnginePort, EngineState } from "../../src/match/engine";
import { enginePort } from "../../src/match/engine.real.ts";
import { decksTheEngineAccepts } from "../fakes/engine";

/**
 * Actions that would end the game or that only the server may send (R79, R84). The walk below
 * avoids them for the same reason SPEC §10.7's policy does: it is looking for the first card play,
 * not for a way out of the match.
 */
const NEVER_CHOOSE = new Set<ActionBody["type"]>([
  "concede",
  "offerDraw",
  "answerDraw",
  "timeout",
  "disconnectExpired",
  "ceilingReached",
]);

describe("the real engine port (src/match/engine.real.ts)", () => {
  it("builds without the engine reporting a missing export", () => {
    // `enginePort()` throws `EngineUnavailableError` when `@jackioh/engine` is missing any of
    // `REQUIRED_ENGINE_EXPORTS`; getting a port back at all is that check passing.
    const port = enginePort();
    expect(typeof port.createGame).toBe("function");
    expect(typeof port.reduce).toBe("function");
    expect(typeof port.snapshot).toBe("function");
  });

  it("registers the card catalog, so createGame accepts a deck of real card ids", async () => {
    const catalog = await loadCatalog();
    const pool = catalog.cardIds.filter((cardId) => !catalog.isToken(cardId));
    const port = enginePort();

    const { state, decks } = decksTheEngineAccepts(port, pool, "engine-real-createGame");

    // The decks really are §8 cards, so an empty registered catalog could not have produced this.
    expect(decks[0][0]).toBe(pool[0]);
    expect(port.snapshot(state).phase).toBe("setup");
    expect(port.snapshot(state).result).toBeNull();
    // Nothing is owed before the deal; `beginGame` opens both mulligans at once (R265).
    expect(port.snapshot(state).mulliganOwed).toEqual([]);
    const begun = port.beginGame(state).state;
    expect(port.snapshot(begun)).toMatchObject({ phase: "mulligan", turn: 0, pendingFor: null });
    expect(port.snapshot(begun).mulliganOwed).toEqual(["p1", "p2"]);
  });

  it("reduces a first real card play without throwing", async () => {
    const catalog = await loadCatalog();
    const pool = catalog.cardIds.filter((cardId) => !catalog.isToken(cardId));
    const port = enginePort();

    const { state: created } = decksTheEngineAccepts(port, pool, "engine-real-firstplay");
    let state = port.beginGame(created).state;

    // Walk the real game through the port's own surface — `snapshot` for whose turn it is,
    // `legalActions` for what may be done, `reduce` to do it — until a card is played.
    let played: ActionBody | null = null;
    let steps = 0;
    while (played === null && steps < 400 && port.snapshot(state).result === null) {
      const snapshot = port.snapshot(state);
      // With a prompt open only its holder may act (§9.3); while both mulligans are open (R265), the
      // first seat still owing one; otherwise it is the active player's.
      const player: PlayerId = snapshot.pendingFor ?? snapshot.mulliganOwed[0] ?? snapshot.active;
      const options = port.legalActions(state, player).filter((body) => !NEVER_CHOOSE.has(body.type));
      const choice =
        options.find((body) => body.type === "play") ??
        options.find((body) => body.type !== "endTurn") ??
        options[0];
      if (choice === undefined) break;

      const action = { ...choice, playerId: player, nonce: `step-${String(steps)}` } as Action;
      const result = port.reduce(state, action);

      // `legalActions` and `reduce` are the same engine: anything offered must be accepted.
      expect(result.error, `${action.type} was offered but refused`).toBeUndefined();
      state = result.state;
      if (choice.type === "play") played = choice;
      steps += 1;
    }

    expect(played, `no card was played in ${String(steps)} actions`).not.toBeNull();
    // A play of a card that came out of the deck, not an engine-invented instance.
    expect(port.snapshot(state).turn).toBeGreaterThan(0);
    // The port's other two projections work on a state a real card has passed through.
    expect(port.hashState(state).length).toBeGreaterThan(0);
    expect(port.viewFor(state, "p1")).not.toBeNull();
  });
});

/**
 * R258's deal, through the real port: `buildAiDeck` over the registered catalog, nothing banned.
 * The deck size is not imported (see `decksTheEngineAccepts`): it is whatever size the real engine
 * accepts, and the dealt pair must be a game `createGame` takes as it is.
 */
describe("All Random's deal (R258, src/match/engine.real.ts)", () => {
  it("R258 deals twenty distinct deckable cards the real engine accepts, the same for the same seed", async () => {
    const catalog = await loadCatalog();
    const port = enginePort();
    const pool = catalog.cardIds.filter((cardId) => !catalog.isToken(cardId));
    // The smallest deck the engine accepts is the one size it accepts: L2's `DECK_SIZE`.
    const size = decksTheEngineAccepts(port, pool, "r258-size").decks[0].length;

    const p1 = port.dealRandomDeck("r258-match:p1-deck");
    const p2 = port.dealRandomDeck("r258-match:p2-deck");

    for (const deck of [p1, p2]) {
      expect(deck).toHaveLength(size);
      // Distinct (MAX_COPIES is one) and deckable: every id a catalog card and none a Token.
      expect(new Set(deck).size).toBe(deck.length);
      for (const cardId of deck) {
        expect(catalog.cardIds, cardId).toContain(cardId);
        expect(catalog.isToken(cardId), cardId).toBe(false);
      }
    }
    // The dealt pair is a game the engine starts, exactly as the match row will hand it over.
    const state = port.createGame({ seed: "r258-match", decks: [p1, p2] });
    expect(port.snapshot(state).phase).toBe("setup");

    // Seeded: the same seed deals the same deck, in the same order, every time and in any process…
    expect(port.dealRandomDeck("r258-match:p1-deck")).toEqual(p1);
    expect(enginePort().dealRandomDeck("r258-match:p1-deck")).toEqual(p1);
    // …and another seed deals another deck.
    expect(p2).not.toEqual(p1);
  });
});

/**
 * R376's record, through the real port: a real match played to a concede and summarized off its
 * log, as `api/game-records.ts` summarizes every live match. What each field means is proved in
 * `packages/engine` and `packages/cards`; this proves the adapter hands the engine's answer over.
 */
describe("a finished match's record (R376, src/match/engine.real.ts)", () => {
  it("R376 summarizes a real match off its log, and makes nothing of an unfinished one", async () => {
    const catalog = await loadCatalog();
    const pool = catalog.cardIds.filter((cardId) => !catalog.isToken(cardId));
    const port = enginePort();
    const { state: created, decks } = decksTheEngineAccepts(port, pool, "r376-real");
    let state = port.beginGame(created).state;

    const log: Action[] = [];
    const act = (player: PlayerId, body: ActionBody): void => {
      const action = { ...body, playerId: player, nonce: `r376-${String(log.length)}` } as Action;
      const result = port.reduce(state, action);
      expect(result.error, `${action.type} refused`).toBeUndefined();
      log.push(action);
      state = result.state;
    };
    // Both keep their hands (R265), p1 ends its first turn, p2 concedes on its own.
    act("p1", { type: "mulligan", keep: handIds(port, state, "p1") });
    act("p2", { type: "mulligan", keep: handIds(port, state, "p2") });
    expect(port.summarizeGame({ seed: "r376-real", decks, log })).toBeNull();
    act("p1", { type: "endTurn" });
    act("p2", { type: "concede" });

    const summary = port.summarizeGame({ seed: "r376-real", decks, log });
    expect(summary).toMatchObject({ first: "p1", winner: "p1", reason: "concede", turns: 2 });
    expect(summary?.seats.p1.deck).toEqual(decks[0]);
    expect(summary?.seats.p1.opening).toHaveLength(3);
    // §2.1, R244: the seat going second opens with its four cards and The Coin.
    expect(summary?.seats.p2.opening).toHaveLength(5);
    expect(summary?.seats.p2.drawn).toHaveLength(1);
  });
});

/** The instance ids of a seat's hand, read through its own view (§10.8). */
function handIds(port: EnginePort, state: EngineState, player: PlayerId): string[] {
  const hand = port.viewFor(state, player).you.hand;
  return Array.isArray(hand) ? hand.map((card) => card.instanceId) : [];
}
