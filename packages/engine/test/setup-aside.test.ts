// What setup deals and what it sets aside (SPEC §2.1, §2.4, R9, R225, R635, R640, R745; issues #152, #355).
//
//  - R635: a card that casts on draw is not dealt by setup while another card is left. The opening
//    draw and the mulligan's replacement draws skip it, it stays in its owner's library, and once both
//    mulligans are resolved it is shuffled in. Setup never deals a fatigue draw.
//  - R745: a hand the other cards cannot fill takes cast-on-draw cards, uncast, and each one still in
//    a hand is cast at the start of the game, before turn 1.
//  - R640: a Quickdraw card replaces one of the opening draws, so a seat is dealt at most as many as
//    its opening hand holds, and the others are ordinary cards in the library.
//
// Decks hold a card once (§2.6 L3), so the cards below are fixtures, one def each: `fx-cod-N` casts on
// draw, `fx-qd-N` is a Quickdraw Field Spell, `fx-dual` is both.

import type { Action, ActionInput, CardDef, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { AI_DIFFICULTY, CAST_ON_DRAW_CHAIN_CAP, DECK_SIZE, HAND_CAP, OPENING_DRAW, type Handicap } from "../src/config";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { chooseMode } from "../src/effects";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { mulliganOwed, mulliganPromptFor } from "../src/setup";
import { createGame, newInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { vanillaDeck } from "./fixtures/catalog";
import { setupCatalog } from "./fixtures/harness";

const POOL = 30;
const DUAL = "fx-dual";
/** A cast-on-draw Spell whose cast asks its caster something (R745's casts at the start of the game). */
const ASKS = "fx-cod-ask";

const cod = (n: number): string => `fx-cod-${n}`;
const qd = (n: number): string => `fx-qd-${n}`;
const isCod = (defId: string): boolean => defId.startsWith("fx-cod-") || defId === DUAL;
const isQd = (defId: string): boolean => defId.startsWith("fx-qd-") || defId === DUAL;

function spell(id: string, tags: CardDef["tags"], type: CardDef["type"]): CardDef {
  return {
    id,
    index: id,
    name: id,
    set: "Core",
    type,
    tags,
    rarity: "Common",
    token: false,
    cost: 0,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
  };
}

/** The shared fixture catalog, plus POOL cast-on-draw Spells, POOL Quickdraw Field Spells and the dual card. */
function register(): void {
  setupCatalog();
  const defs: Record<string, CardDef> = {};
  const scripts: Record<string, CardScripts> = {};
  const add = (def: CardDef, flags: { castOnDraw?: true; quickdraw?: true }): void => {
    defs[def.id] = def;
    const script = { staticFlags: flags, cry: () => [] };
    scripts[def.id] = { base: script, radiant: script };
  };
  for (let n = 1; n <= POOL; n += 1) {
    add(spell(cod(n), [], "Spell"), { castOnDraw: true });
    add(spell(qd(n), ["Quickdraw"], "Field Spell"), { quickdraw: true });
  }
  add(spell(DUAL, ["Quickdraw"], "Spell"), { castOnDraw: true, quickdraw: true });
  defs[ASKS] = spell(ASKS, [], "Spell");
  const asks = {
    staticFlags: { castOnDraw: true },
    cry: () => [chooseMode({ options: ["ok", "fine"], step: "ok", prompt: "R745" })],
    resume: { ok: () => [] },
  };
  scripts[ASKS] = { base: asks, radiant: asks };
  registerCatalog({ ...registeredCatalog(), ...defs });
  registerScripts({ ...registeredScripts(), ...scripts });
}

/** `size` cards: `quick` Quickdraw cards, `cast` cast-on-draw ones, the rest vanilla Units. */
function deckOf(quick: number, cast: number, size: number = DECK_SIZE): string[] {
  const rest = size - quick - cast;
  return [
    ...Array.from({ length: quick }, (_, at) => qd(at + 1)),
    ...Array.from({ length: cast }, (_, at) => cod(at + 1)),
    ...vanillaDeck(rest, 1),
  ];
}

const OTHER = (): string[] => vanillaDeck(DECK_SIZE, 21);

function start(seed: string, decks: [string[], string[]], handicaps?: Partial<Record<PlayerId, Handicap>>) {
  register();
  const game = createGame({ seed, decks, ...(handicaps === undefined ? {} : { handicaps }) });
  return { game, begun: beginGame(game) };
}

let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[]; action: Action } {
  nonce += 1;
  const action = { ...body, nonce: `aside-${nonce}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(`${body.type} refused: ${result.error}`);
  return { state: result.state, events: result.events, action };
}

/** Both seats answer: `keepP1`/`keepP2` are the ids kept, every card of the hand by default. */
function answerBoth(
  state: GameState,
  keep: Partial<Record<PlayerId, string[]>> = {},
): { state: GameState; events: GameEvent[]; log: Action[] } {
  const events: GameEvent[] = [];
  const log: Action[] = [];
  let next = state;
  for (const player of ["p1", "p2"] as const) {
    const answered = act(next, {
      type: "mulligan",
      keep: keep[player] ?? next.players[player].hand.map((card) => card.id),
      playerId: player,
    });
    next = answered.state;
    events.push(...answered.events);
    log.push(answered.action);
  }
  return { state: next, events, log };
}

/** What happened in setup: the events up to, and not including, turn 1's start. */
function beforeTurnOne(events: readonly GameEvent[]): GameEvent[] {
  const at = events.findIndex((event) => event.type === "turnStarted");
  return at < 0 ? [...events] : events.slice(0, at);
}

const count = (events: readonly GameEvent[], type: GameEvent["type"]): number =>
  events.filter((event) => event.type === type).length;

const defsOf = (cards: readonly { defId: string }[]): string[] => cards.map((card) => card.defId);

describe("R635: cast on draw cards sit out the deal and are shuffled in after the mulligan", () => {
  it("R635 the opening draw and the mulligan's replacements cast nothing: no cast-on-draw card is dealt", () => {
    for (let n = 0; n < 40; n += 1) {
      const { begun } = start(`r635-deal-${n}`, [deckOf(0, 8), OTHER()]);
      const state = begun.state;
      const side = state.players.p1;

      // Three of the twelve other cards are dealt, and nothing was cast to get there.
      expect(side.hand).toHaveLength(OPENING_DRAW[0] as number);
      expect(defsOf(side.hand).some(isCod)).toBe(false);
      expect(side.graveyard).toEqual([]);
      expect(count(begun.events, "cardPlayed")).toBe(0);
      // They are still in the library, behind every card the draws can reach: the library count is
      // the deck less the hand, as for any deck.
      expect(side.library).toHaveLength(DECK_SIZE - side.hand.length);
      expect(defsOf(side.library.slice(-8)).every(isCod)).toBe(true);

      // The mulligan returns the whole hand: three replacements, none of them a cast-on-draw card, and
      // nothing cast on the way (turn 1's own draw comes after setup's last event).
      const returned = side.hand.map((card) => card.id);
      const answered = answerBoth(state, { p1: [] });
      const dealt = beforeTurnOne(answered.events);
      const replacements = dealt.filter((event) => event.type === "drawn" && event.player === "p1");
      expect(replacements).toHaveLength(returned.length);
      expect(replacements.every((event) => event.type === "drawn" && !isCod(event.defId))).toBe(true);
      expect(count(dealt, "cardPlayed")).toBe(0);
      expect(count(dealt, "fatigue")).toBe(0);
      expect(count(dealt, "shuffledIn")).toBe(returned.length);
    }
  });

  it("R635 shuffles them into the library once both mulligans are resolved, at random places", () => {
    // p2 draws first on turn 2, so its library is as setup left it. Left at the bottom, the six sit at
    // indices 10 to 15 whatever the seed; shuffled in, their mean place in 16 cards is 7.5.
    let places = 0;
    let seen = 0;
    let atTop = 0;
    for (let n = 0; n < 150; n += 1) {
      const { begun } = start(`r635-shuffle-${n}`, [OTHER(), deckOf(0, 6)]);
      const settled = answerBoth(begun.state).state;
      const library = settled.players.p2.library;
      expect(library).toHaveLength(DECK_SIZE - OPENING_DRAW[1]!);
      library.forEach((card, at) => {
        if (!isCod(card.defId)) return;
        places += at;
        seen += 1;
        if (at === 0) atTop += 1;
      });
    }
    expect(seen).toBe(150 * 6);
    expect(places / seen).toBeGreaterThan(7.0);
    expect(places / seen).toBeLessThan(8.0);
    expect(atTop).toBeGreaterThan(0);
  });

  it("R635 says nothing to the other seat: its view is the same whether the deck holds cast-on-draw cards", () => {
    const typesOf = (state: GameState): string[] =>
      viewFor(state, "p1").events.map((event) => `${event.type}${"player" in event ? `:${event.player}` : ""}`);

    for (let n = 0; n < 20; n += 1) {
      const seed = `r635-view-${n}`;
      const withCast = start(seed, [OTHER(), deckOf(0, 7)]);
      const without = start(seed, [OTHER(), deckOf(0, 0)]);

      // The deal: the same events, the same counts. The seat that holds the cards is dealt four
      // others either way, and nothing is announced about the ones set aside.
      expect(typesOf(withCast.begun.state)).toEqual(typesOf(without.begun.state));
      expect(viewFor(withCast.begun.state, "p1").opponent.hand).toEqual({ count: 4 });
      expect(viewFor(withCast.begun.state, "p1").opponent.libraryCount).toEqual(
        viewFor(without.begun.state, "p1").opponent.libraryCount,
      );

      // After the mulligans, where the shuffle-in happens: no `shuffledIn` per card, which would count them.
      const settledWith = answerBoth(withCast.begun.state);
      const settledWithout = answerBoth(without.begun.state);
      expect(count(settledWith.events, "shuffledIn")).toBe(0);
      expect(typesOf(settledWith.state)).toEqual(typesOf(settledWithout.state));
      expect(viewFor(settledWith.state, "p1").opponent.libraryCount).toEqual(
        viewFor(settledWithout.state, "p1").opponent.libraryCount,
      );
    }
  });

  it("R635 a card that is both Quickdraw and cast on draw is dealt as a Quickdraw card, never cast", () => {
    for (let n = 0; n < 20; n += 1) {
      const deck = [DUAL, ...vanillaDeck(DECK_SIZE - 1, 1)];
      const { begun } = start(`r635-dual-${n}`, [OTHER(), deck]);
      const hand = begun.state.players.p2.hand;
      expect(defsOf(hand)).toContain(DUAL);
      expect(begun.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(begun.state.players.p2.graveyard).toEqual([]);

      // Returned by the mulligan, it is a cast-on-draw card in the library like the rest of them: it
      // goes back among them and is shuffled in, not cast.
      const dual = hand.find((card) => card.defId === DUAL)!;
      const kept = hand.filter((card) => card.id !== dual.id).map((card) => card.id);
      const answered = answerBoth(begun.state, { p2: kept });
      expect(count(beforeTurnOne(answered.events), "cardPlayed")).toBe(0);
      expect(answered.state.players.p2.graveyard).toEqual([]);
      expect(answered.state.players.p2.library.some((card) => card.id === dual.id)).toBe(true);
    }
  });

  it("R635 a game with cards set aside folds from its seed and log, whatever the mulligans returned", () => {
    for (let n = 0; n < 15; n += 1) {
      const seed = `r635-fold-${n}`;
      const decks: [string[], string[]] = [deckOf(1, 6), deckOf(2, 9)];
      const { begun } = start(seed, decks);
      const p1 = begun.state.players.p1.hand.map((card) => card.id);
      const p2 = begun.state.players.p2.hand.map((card) => card.id);
      const played = answerBoth(begun.state, { p1: p1.slice(1), p2: p2.slice(2) });

      const replayed = fold({ seed, decks, log: played.log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(played.state));
    }
  });
});

describe("R745: a hand the other cards cannot fill takes cast-on-draw cards, cast at the start of the game", () => {
  /** The `cardPlayed` events, by instance id, in order. */
  const playedIn = (events: readonly GameEvent[]): string[] =>
    events.flatMap((event) => (event.type === "cardPlayed" ? [event.instanceId] : []));
  const drawnBy = (events: readonly GameEvent[], player: PlayerId): GameEvent[] =>
    events.filter((event) => event.type === "drawn" && event.player === player);

  it("R745 an all-cast-on-draw deck deals a full hand of them, uncast, and casts them at the start of the game", () => {
    const { begun } = start("r745-all", [deckOf(0, DECK_SIZE), OTHER()]);
    const state = begun.state;
    const hand = state.players.p1.hand;

    // Three of them, each reported as a draw (R225), and none of them cast in the deal.
    expect(hand).toHaveLength(OPENING_DRAW[0] as number);
    expect(defsOf(hand).every(isCod)).toBe(true);
    expect(drawnBy(begun.events, "p1")).toHaveLength(OPENING_DRAW[0] as number);
    expect(state.players.p1.library).toHaveLength(DECK_SIZE - hand.length);
    expect(state.players.p1.fatigueCount).toBe(0);
    expect(count(begun.events, "fatigue")).toBe(0);
    expect(count(begun.events, "cardPlayed")).toBe(0);
    // The mulligan offers them like any other card.
    expect(mulliganPromptFor(state, "p1")?.options.map((option) => option.key)).toEqual(hand.map((card) => card.id));

    const answered = answerBoth(state);
    // Once both mulligans are in, the three are cast before turn 1, in hand order, no draw repeated.
    expect(playedIn(beforeTurnOne(answered.events))).toEqual(hand.map((card) => card.id));
    expect(count(beforeTurnOne(answered.events), "fatigue")).toBe(0);
    expect(drawnBy(beforeTurnOne(answered.events), "p1")).toEqual([]);
    // Turn 1's draw meets the other seventeen: it casts them all, and the draw after the last one
    // finds the library empty.
    const turnOne = answered.events.slice(answered.events.findIndex((event) => event.type === "turnStarted"));
    expect(answered.state.turn).toBeGreaterThanOrEqual(1);
    expect(count(turnOne, "cardPlayed")).toBe(DECK_SIZE - hand.length);
    expect(answered.state.players.p1.graveyard).toHaveLength(DECK_SIZE);
    expect(answered.state.players.p1.library).toEqual([]);
    expect(answered.state.players.p1.fatigueCount).toBe(1);
    expect(answered.state.result).toBeNull();
  });

  it("R745 turn 1 of an all-cast-on-draw library is still bounded by R58's cap, as a chain mid-game is", () => {
    register();
    const game = createGame({ seed: "r745-cap", decks: [deckOf(0, DECK_SIZE), OTHER()] });
    // Five more, as a Unstable Clone Machine's or a CN-Virus's copies would add: 25 cards, all of them
    // cast on draw.
    for (const defId of [cod(1), cod(2), cod(3), cod(4), cod(5)]) {
      game.players.p1.library.push(newInstance(game, defId, "p1", { z: "library", player: "p1" }));
    }
    const begun = beginGame(game);
    expect(begun.state.players.p1.hand).toHaveLength(OPENING_DRAW[0] as number);
    expect(begun.state.players.p1.library).toHaveLength(DECK_SIZE + 5 - (OPENING_DRAW[0] as number));

    const settled = answerBoth(begun.state).state;
    // The hand's three at the start of the game, then turn 1's twenty casts; the next card goes to the
    // hand uncast, which ends the chain.
    expect(settled.players.p1.graveyard).toHaveLength((OPENING_DRAW[0] as number) + CAST_ON_DRAW_CHAIN_CAP);
    expect(settled.players.p1.hand).toHaveLength(1);
    expect(settled.players.p1.library).toHaveLength(1);
    expect(settled.players.p1.fatigueCount).toBe(0);
  });

  it("R745 one other card and two cast-on-draw cards fill the hand, and each seat's are cast, Player 1's first", () => {
    // p1: one other card and a hand of three. p2: two other cards and a hand of four.
    const { begun } = start("r745-one", [deckOf(0, DECK_SIZE - 1), deckOf(0, DECK_SIZE - 2)]);
    const state = begun.state;
    const p1 = state.players.p1.hand;
    const p2 = state.players.p2.hand;
    expect(defsOf(p1).filter((defId) => !isCod(defId))).toEqual(["fx-1"]);
    expect(defsOf(p1).filter(isCod)).toHaveLength(2);
    expect(defsOf(p2).filter((defId) => !isCod(defId))).toHaveLength(2);
    expect(defsOf(p2).filter(isCod)).toHaveLength(2);
    expect(state.players.p1.library).toHaveLength(DECK_SIZE - 3);
    expect(count(begun.events, "fatigue")).toBe(0);
    expect(count(begun.events, "cardPlayed")).toBe(0);

    const answered = answerBoth(state);
    const cast = [...p1, ...p2].filter((card) => isCod(card.defId)).map((card) => card.id);
    expect(playedIn(beforeTurnOne(answered.events))).toEqual(cast);
    // The other cards stay in the hands.
    expect(answered.state.players.p2.hand.map((card) => card.id)).toEqual(
      p2.filter((card) => !isCod(card.defId)).map((card) => card.id),
    );
    expect(answered.state.players.p1.hand.some((card) => card.defId === "fx-1")).toBe(true);
  });

  it("R745 a mulligan the other cards cannot replace is dealt cast-on-draw cards for the rest", () => {
    // Five other cards: three in the hand and two left in the library for the three replacements.
    const { begun } = start("r745-short", [deckOf(0, DECK_SIZE - 5), OTHER()]);
    const state = begun.state;
    expect(state.players.p1.hand).toHaveLength(3);
    expect(defsOf(state.players.p1.hand).some(isCod)).toBe(false);
    expect(state.players.p1.library.filter((card) => !isCod(card.defId))).toHaveLength(2);

    const answered = answerBoth(state, { p1: [] });
    const dealt = beforeTurnOne(answered.events);
    const replacements = drawnBy(dealt, "p1");
    // Three back: the two other cards, and one cast-on-draw card dealt uncast, cast at the start.
    expect(replacements).toHaveLength(3);
    const filled = replacements.filter((event) => event.type === "drawn" && isCod(event.defId));
    expect(filled).toHaveLength(1);
    expect(playedIn(dealt)).toEqual(filled.map((event) => (event.type === "drawn" ? event.instanceId : "")));
    expect(count(dealt, "fatigue")).toBe(0);
    expect(answered.state.players.p1.fatigueCount).toBe(0);
    expect(answered.state.players.p1.hero.health).toBe(30);
  });

  it("R745 a cast-on-draw card dealt uncast that the mulligan returns goes back uncast", () => {
    // p2 draws nothing on turn 1, so its library is as setup left it.
    const { begun } = start("r745-returned", [OTHER(), deckOf(0, DECK_SIZE - 2)]);
    const hand = begun.state.players.p2.hand;
    const waiting = hand.filter((card) => isCod(card.defId));
    expect(waiting).toHaveLength(2);
    const [returned, kept] = waiting as [(typeof hand)[number], (typeof hand)[number]];

    const answered = answerBoth(begun.state, {
      p2: hand.filter((card) => card.id !== returned.id).map((card) => card.id),
    });
    const dealt = beforeTurnOne(answered.events);
    // The replacement is another of them, uncast; the returned one is shuffled back and not cast.
    const replacement = drawnBy(dealt, "p2");
    expect(replacement).toHaveLength(1);
    expect(replacement[0]?.type === "drawn" && isCod(replacement[0].defId)).toBe(true);
    expect(dealt.some((event) => event.type === "shuffledIn" && event.instanceId === returned.id)).toBe(true);
    const played = playedIn(dealt);
    expect(played).not.toContain(returned.id);
    expect(played).toEqual([kept.id, replacement[0]?.type === "drawn" ? replacement[0].instanceId : ""]);
    const back = answered.state.players.p2.library.find((card) => card.id === returned.id);
    expect(back?.memory).toEqual({});
  });

  it("R745 a cast at the start of the game that asks holds turn 1 until it is answered, and the game folds from its log", () => {
    const decks: [string[], string[]] = [[ASKS, ...Array.from({ length: DECK_SIZE - 1 }, (_, at) => cod(at + 1))], OTHER()];
    let seed = "";
    let begun: ReturnType<typeof beginGame> | undefined;
    for (let n = 0; n < 50 && begun === undefined; n += 1) {
      const tried = start(`r745-asks-${n}`, decks).begun;
      if (tried.state.players.p1.hand.some((card) => card.defId === ASKS)) {
        seed = `r745-asks-${n}`;
        begun = tried;
      }
    }
    if (begun === undefined) throw new Error("no seed dealt the asking card");

    const answered = answerBoth(begun.state);
    const pending = answered.state.pending;
    expect(pending?.kind).toBe("mode");
    expect(pending?.playerId).toBe("p1");
    expect(answered.state.turn).toBe(0);
    expect(count(answered.events, "turnStarted")).toBe(0);
    // A paused setup is plain data (R113).
    expect(JSON.parse(JSON.stringify(answered.state))).toEqual(answered.state);

    const done = act(answered.state, {
      type: "answer",
      choiceId: pending!.id,
      selection: [{ pick: "mode", option: "ok" }],
      playerId: "p1",
    });
    expect(done.state.turn).toBeGreaterThanOrEqual(1);
    expect(done.state.players.p1.graveyard.length).toBeGreaterThanOrEqual(OPENING_DRAW[0] as number);

    const replayed = fold({ seed, decks, log: [...answered.log, done.action] });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(done.state));
  });
});

describe("R640: a Quickdraw card replaces one of the opening draws", () => {
  const quickIn = (cards: readonly { defId: string }[]): number => cards.filter((card) => isQd(card.defId)).length;

  it("R640 a seat holding all five Quickdraw cards is dealt only as many as it has draws, p1's three and p2's four", () => {
    const hands = new Set<string>();
    for (let n = 0; n < 60; n += 1) {
      const { begun } = start(`r636-five-${n}`, [deckOf(5, 0), deckOf(5, 0)]);
      const { p1, p2 } = begun.state.players;

      expect(p1.hand).toHaveLength(OPENING_DRAW[0] as number);
      expect(p2.hand).toHaveLength(OPENING_DRAW[1] as number);
      // All of the hand is Quickdraw cards, and the rest wait in the library as ordinary ones.
      expect(quickIn(p1.hand)).toBe(3);
      expect(quickIn(p2.hand)).toBe(4);
      expect(quickIn(p1.library)).toBe(2);
      expect(quickIn(p2.library)).toBe(1);
      expect(p1.library).toHaveLength(DECK_SIZE - 3);
      expect(p2.library).toHaveLength(DECK_SIZE - 4);
      // R225: each is reported as a draw, `drawn` then `addedToHand`, and only the dealt ones are.
      const drawn = (player: PlayerId): number =>
        begun.events.filter((event) => event.type === "drawn" && event.player === player).length;
      expect(drawn("p1")).toBe(3);
      expect(drawn("p2")).toBe(4);
      expect(count(begun.events, "burned")).toBe(0);
      hands.add(defsOf(p1.hand).sort().join(","));
    }
    // Which of the five fill the hand is the shuffle's choice, not always the same three.
    expect(hands.size).toBeGreaterThan(3);
  });

  it("R640 a handicapped seat is dealt up to its larger hand: four for p1 and five for p2 under Medium", () => {
    const medium = AI_DIFFICULTY.medium;
    const decks: [string[], string[]] = [deckOf(5, 0, medium.deckSize), deckOf(5, 0, medium.deckSize)];
    const { begun } = start("r636-medium", decks, { p1: medium, p2: medium });
    const { p1, p2 } = begun.state.players;

    expect(p1.hand).toHaveLength((OPENING_DRAW[0] as number) + medium.extraOpeningCards);
    expect(p2.hand).toHaveLength((OPENING_DRAW[1] as number) + medium.extraOpeningCards);
    expect(quickIn(p1.hand)).toBe(4);
    expect(quickIn(p1.library)).toBe(1);
    // Five draws and five Quickdraw cards: no draw is left over and none is missing.
    expect(quickIn(p2.hand)).toBe(5);
    expect(quickIn(p2.library)).toBe(0);
  });

  it("R640 the opening hand is the table's size whether or not the deck holds Quickdraw cards", () => {
    const typesOf = (state: GameState): string[] =>
      viewFor(state, "p1").events.map((event) => `${event.type}${"player" in event ? `:${event.player}` : ""}`);

    for (let n = 0; n < 20; n += 1) {
      const seed = `r636-view-${n}`;
      const five = start(seed, [OTHER(), deckOf(5, 0)]);
      const none = start(seed, [OTHER(), deckOf(0, 0)]);
      // Were every Quickdraw card added to the hand, a hand of five would say p2 holds at least two.
      expect(viewFor(five.begun.state, "p1").opponent.hand).toEqual({ count: 4 });
      expect(viewFor(five.begun.state, "p1").opponent.libraryCount).toEqual(viewFor(none.begun.state, "p1").opponent.libraryCount);
      expect(typesOf(five.begun.state)).toEqual(typesOf(none.begun.state));
    }
  });

  it("R640 a deck of nothing but Quickdraw cards deals the hand and burns nothing", () => {
    const { begun } = start("r636-all", [deckOf(DECK_SIZE, 0), OTHER()]);
    const side = begun.state.players.p1;
    expect(side.hand).toHaveLength(OPENING_DRAW[0] as number);
    expect(side.hand.length).toBeLessThanOrEqual(HAND_CAP);
    expect(side.library).toHaveLength(DECK_SIZE - side.hand.length);
    expect(count(begun.events, "burned")).toBe(0);
  });

  it("R640 the Quickdraw cards that were not dealt are ordinary cards: a mulligan's replacements can draw them", () => {
    let drawnSurplus = 0;
    for (let n = 0; n < 60; n += 1) {
      const { begun } = start(`r636-surplus-${n}`, [deckOf(5, 0), OTHER()]);
      const dealt = new Set(begun.state.players.p1.hand.map((card) => card.id));
      const answered = answerBoth(begun.state, { p1: [] });
      const replacements = beforeTurnOne(answered.events).filter(
        (event) => event.type === "drawn" && event.player === "p1",
      );
      expect(replacements).toHaveLength(3);
      for (const event of replacements) {
        if (event.type === "drawn" && isQd(event.defId) && !dealt.has(event.instanceId)) drawnSurplus += 1;
      }
    }
    expect(drawnSurplus).toBeGreaterThan(0);
  });

  it("R640, R745 two Quickdraw cards and one cast-on-draw card fill the hand, uncast", () => {
    // Two Quickdraw cards, eighteen that cast on draw, and three draws: the two, and one of the
    // eighteen for the third, cast at the start of the game.
    const { begun } = start("r636-mixed", [deckOf(2, 18), OTHER()]);
    const side = begun.state.players.p1;
    expect(defsOf(side.hand).filter(isQd)).toHaveLength(2);
    expect(defsOf(side.hand).filter(isCod)).toHaveLength(1);
    expect(side.hand).toHaveLength(OPENING_DRAW[0] as number);
    expect(side.library).toHaveLength(17);
    expect(side.fatigueCount).toBe(0);
    expect(count(begun.events, "fatigue")).toBe(0);
    expect(count(begun.events, "cardPlayed")).toBe(0);
  });
});
