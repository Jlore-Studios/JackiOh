// Setup and the mulligan when a question is asked during it (SPEC §2.1, §2.4, §9.3, §10.1, §10.6, R9,
// R151, R158, R224, R635). Found by the polish-4 edge-case hunt, round 7 (docs/polish/4-edge-cases.md,
// lenses L7, "legality-agreement" and "engine invariants"); every case here failed before its fix.
//
//  - R224: setup used to open the next mulligan over a question a draw's cast had asked, which
//    replaced it and left the cast half-played in its caster's resolving zone for good. A cast-on-draw
//    card is no longer dealt by setup (R635), so what asks during it now is a start-of-game clause run
//    as its card arrives in a hand (R151), the one thing a card setup draws can still do. The rule is
//    the same: setup owes the rest of itself behind the question and the answer finishes it (R113,
//    R122).
//  - §10.6: the mulligan's `promptAnswered` named the word "mulligan", not the prompt.
//  - Round 8 (lens L10). R225: a Quickdraw card is the last of the opening draws it replaces,
//    reported and counted as a draw, so #100's price, the deal's events and the counts while setup
//    waits (R224) do not tell the other seat whether the opening hand holds one.
//  - Round 10 (lens L8, and L7 for the clause that asks). Setup is turn 0, no player's turn (§2.1):
//    a Spell a mulligan's replacement draw cast kept its return flag into its caster's first turn
//    end (R155), and p1's cast armed an end-of-turn clause for turn 1 that p2's did not (R241). A
//    start-of-game clause that asks at §2.1 step 4 now holds the rest of setup, and turn 1, until
//    it is answered (R151, R113). Those two cases keep their casts: R635 sets a cast-on-draw card
//    aside, so the Spell is cast by a start-of-game clause as it arrives in the hand a replacement
//    draw fills, which is the same cast on turn 0.
//
// No Core card has a start-of-game clause that asks, so the asking card is a fixture (a transient def
// and a registered script, the way paused-sequences.test.ts builds its asking cards).

import { describe, expect, it } from "vitest";
import type { Action, ActionBody, ActionInput, CardDef, CardType, PlayerId, PlayerView } from "@jackioh/shared";
import {
  DECK_SIZE,
  OPENING_DRAW,
  RESUME_HOOK,
  SETUP_TURN,
  beginGame,
  createGame,
  mulliganOwed,
  mulliganPromptFor,
  newInstance,
  query,
  reduce,
  registerScripts,
  registeredScripts,
  viewFor,
  wasPlayedThisTurn,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { bounce, cast, chooseMode, damage, delay, exileHand, remember } from "@jackioh/engine/effects";
import { CATALOG } from "../src/index";
// Importing the harness registers the real catalog and every card script (`registerAll()`); these
// cases build their games through `createGame`, since a `scenario()` starts past the mulligan.
import "./_harness";

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function fixtureDef(id: string, type: CardType): CardDef {
  const face = type === "Unit" ? { attack: 2, health: 2, keywords: [], text: id } : { keywords: [], text: id };
  return {
    id,
    index: id,
    name: id,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { ...face },
    radiant: { ...face },
  };
}

/** A fixture card: a transient def in the match state and its script in the registry. */
function fixture(state: GameState, id: string, type: CardType, script: Script): void {
  state.transientDefs[id] = fixtureDef(id, type);
  registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } });
}

/**
 * R151: a start-of-game clause runs as its card arrives in a hand, so a card setup draws can ask a
 * question. It asks once: §2.1 step 4 runs the clause of every card still in a hand or library again
 * (R153).
 */
function askingOnArrival(prompt: string): Script {
  return {
    startOfGame: (ctx) =>
      ctx.self?.memory.asked === true
        ? []
        : [remember({ key: "asked", value: true }), chooseMode({ options: ["ok"], step: "ok", prompt })],
    resume: { ok: () => [] },
  };
}

/** A start-of-game clause that casts its card as it arrives in a hand, which only setup does (turn 0). */
const CAST_ON_SETUP_ARRIVAL: Pick<Script, "startOfGame"> = {
  startOfGame: (ctx) => (ctx.state.turn === SETUP_TURN ? [cast({ target: { of: "self" } })] : []),
};

/** Two legal decks straight from the catalog: the first 40 non-token cards in id order. */
function catalogDecks(): [string[], string[]] {
  const pool = Object.entries(CATALOG)
    .filter(([, def]) => def.token !== true && !def.tags.includes("Token"))
    .map(([id]) => id)
    .sort();
  return [pool.slice(0, DECK_SIZE), pool.slice(DECK_SIZE, DECK_SIZE * 2)];
}

let nonce = 0;
function act(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `setup-mulligan-${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

const P1_DECK = Array.from({ length: 20 }, (_, at) => `core-${String(at + 1).padStart(3, "0")}`);
const P2_DECK = Array.from({ length: 20 }, (_, at) => `core-${String(at + 30).padStart(3, "0")}`);

describe("R224: setup waits for a question", () => {
  it("R224 the question a replacement draw's arrival clause asks p1 is not overwritten by p2's mulligan (§10.1, R9, R151, R158, R265)", () => {
    let state = beginGame(createGame({ seed: "edge-r7-l7-mulligan", decks: [P1_DECK, P2_DECK] })).state;
    expect(mulliganPromptFor(state, "p1")?.kind).toBe("mulligan");
    expect(mulliganOwed(state)).toEqual(["p1", "p2"]);

    // A card whose start-of-game clause asks, on top of p1's library: the replacement draw reaches it.
    fixture(state, "edge-r7-l7-asks", "Spell", askingOnArrival("the clause's question"));
    const asks = newInstance(state, "edge-r7-l7-asks", "p1", { z: "library", player: "p1" });
    state.players.p1.library.unshift(asks);

    // p1 returns one card: R9 draws the replacement first, and it is the asking card (§2.4).
    // The answer is sealed until p2 answers too (R266); p2 returns one card as well.
    const hand = state.players.p1.hand.map((card) => card.id);
    const returned = must(hand[0], "a card to return");
    state = act(state, { type: "mulligan", keep: hand.slice(1), playerId: "p1" }).state;
    expect(state.players.p1.hand.map((card) => card.id)).toEqual(hand);
    const p2Hand = state.players.p2.hand.map((card) => card.id);
    const p2Returned = must(p2Hand[0], "a card p2 returns");
    const result = act(state, { type: "mulligan", keep: p2Hand.slice(1), playerId: "p2" });
    state = result.state;
    // Both are in, so both resolve in seat order (R265): p1's replacement arrived, and its clause
    // asked p1 (R151).
    expect(result.events.some((event) => event.type === "promptOpened" && event.kind === "mode")).toBe(true);

    // §9.3, §10.1: one prompt at a time, and the question is state until p1 answers it. p2's
    // resolution waits behind it: p2's hand is as it answered, and p1's returned card waits to go back.
    const pending = must(state.pending, "an open prompt");
    expect(pending.playerId, `open prompt: ${pending.kind} "${pending.prompt}"`).toBe("p1");
    expect(pending.kind).toBe("mode");
    expect(state.players.p1.library.some((card) => card.id === returned)).toBe(false);
    expect(state.players.p2.hand.map((card) => card.id)).toEqual(p2Hand);
    expect(state.mulliganed).toEqual([]);

    // The answer finishes the rest of p1's mulligan (R122): the returned card is shuffled back, then
    // p2's sealed answer resolves, and then the game begins.
    state = act(state, { type: "answer", choiceId: pending.id, selection: [{ pick: "mode", option: "ok" }], playerId: "p1" }).state;
    expect(state.players.p1.library.some((card) => card.id === returned)).toBe(true);
    expect(state.players.p2.hand.map((card) => card.id)).not.toContain(p2Returned);
    expect(state.players.p2.library.some((card) => card.id === p2Returned)).toBe(true);
    expect(state.mulliganed).toEqual(["p1", "p2"]);
    expect(state.turn).toBe(1);
  });

  it("R224 a question from the opening draw's arrival clauses is not written over by the mulligan (§2.1, R151, R158)", () => {
    const deck = query({})
      .map((def) => def.id)
      .slice(0, 20);
    const game = createGame({ seed: "edge-r7-setup", decks: [deck, deck] });
    // A card whose start-of-game clause asks, as it arrives in a hand (R151).
    const id = "edge-r7-asking";
    fixture(game, id, "Spell", askingOnArrival("the clause's question"));
    game.players.p1.library = game.players.p1.library.map(() =>
      newInstance(game, id, "p1", { z: "library", player: "p1" }),
    );

    const started = beginGame(game);
    const asked = started.events.find((event) => event.type === "promptOpened" && event.kind === "mode");
    expect(asked, "p1's opening draw reaches the card, whose clause asks").toBeDefined();

    // R158: a draw a prompt interrupts stops there and owes the rest, and §2.1's mulligan follows
    // the opening draw. A second prompt never overwrites an unanswered one (R156), so the question
    // stays open until it is answered.
    const answered = started.events.some(
      (event) => event.type === "promptAnswered" && asked !== undefined && event.choiceId === (asked as { choiceId: string }).choiceId,
    );
    const stillOpen = asked !== undefined && started.state.pending?.id === (asked as { choiceId: string }).choiceId;
    expect(answered || stillOpen, `the question was replaced by a ${started.state.pending?.kind ?? "no"} prompt`).toBe(true);

    // Every card in p1's library is one of these, so each of the three opening draws asks in turn.
    // Answering each goes on with the opening deal, and the first mulligan opens only once nothing is
    // asking.
    let state = started.state;
    let questions = 0;
    for (let guard = 0; guard < 40 && state.pending?.kind === "mode"; guard += 1) {
      const open = must(state.pending, "a clause's question");
      state = act(state, { type: "answer", choiceId: open.id, selection: [{ pick: "mode", option: "ok" }], playerId: "p1" }).state;
      questions += 1;
    }
    expect(questions).toBe(OPENING_DRAW[0]);
    expect(state.pending).toBeNull();
    expect(mulliganOwed(state)).toEqual(["p1", "p2"]);
    expect(state.players.p1.hand).toHaveLength(OPENING_DRAW[0] as number);
    expect(state.players.p2.hand).toHaveLength(OPENING_DRAW[1] as number);
  });

  it("R224 the mulligan's promptAnswered names the prompt that promptOpened named (§10.3, §10.6)", () => {
    const begun = beginGame(createGame({ seed: "r7-mulligan-ids", decks: catalogDecks() }));
    const opened = begun.events.find((event) => event.type === "promptOpened");
    const openedId = opened?.type === "promptOpened" ? opened.choiceId : null;
    expect(openedId).not.toBeNull();
    expect(mulliganPromptFor(begun.state, "p1")?.id).toBe(openedId);

    const answered = reduce(begun.state, { type: "mulligan", keep: [], playerId: "p1", nonce: "m1" } as Action);
    expect(answered.error).toBeUndefined();
    const closed = answered.events.find((event) => event.type === "promptAnswered");
    // Every other prompt's answer names its PendingChoice id (`prompts.ts`); the mulligan's named
    // the word "mulligan", which no prompt ever had.
    expect(closed?.type === "promptAnswered" ? closed.choiceId : null).toBe(openedId);
  });
});

const HEROIC_POWER = "core-098";
const CRAFT = "core-099";
const VOID = "core-100";

function sameView(a: PlayerView, b: PlayerView): void {
  expect(b.events).toEqual(a.events);
  expect(b).toEqual(a);
}

// ---------------------------------------------------------------------------
// Quickdraw and the game's draw counter (#100 Ceaseless Void, R55)
// ---------------------------------------------------------------------------

const QD_P1_DECK = [
  "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
  "core-018", "core-023", "core-031", "core-035", "core-036", "core-039", "core-041", "core-044",
  "core-048", "core-050", "core-060", VOID,
];
/** p2's deck less one card: the twentieth is #98 Heroic Power (Quickdraw) or #99 Craft a Card. */
const QD_P2_SHARED = [
  "core-003", "core-004", "core-005", "core-007", "core-008", "core-011", "core-015", "core-023",
  "core-031", "core-035", "core-036", "core-044", "core-050", "core-062", "core-063", "core-081",
  "core-082", "core-086", "core-012",
];

function actAs(state: GameState, player: PlayerId, body: ActionBody): GameState {
  return act(state, { ...body, playerId: player } as ActionInput).state;
}

/** Both players keep their opening hands; p1's first turn has begun. */
function keepBoth(state: GameState): GameState {
  let next = actAs(state, "p1", { type: "mulligan", keep: state.players.p1.hand.map((card) => card.id) });
  next = actAs(next, "p2", { type: "mulligan", keep: next.players.p2.hand.map((card) => card.id) });
  return next;
}

describe("R225: a Quickdraw card is counted as the draw it replaces", () => {
  /** "r8-l10-void-3" puts #100 in p1's opening hand. */
  function voidGame(p2Twentieth: string): GameState {
    const decks: [string[], string[]] = [[...QD_P1_DECK], [...QD_P2_SHARED, p2Twentieth]];
    return keepBoth(beginGame(createGame({ seed: "r8-l10-void-3", decks })).state);
  }

  it("R225 p1's Ceaseless Void does not price in whether p2's opening hand holds a Quickdraw card (§2.1, §9.1, R55)", () => {
    const withPower = voidGame(HEROIC_POWER);
    const without = voidGame(CRAFT);

    // Same public course: p1's turn 1 has begun, p2 holds its 4 opening cards and The Coin (R244),
    // and p1 holds its Void.
    for (const state of [withPower, without]) {
      expect(state.turn).toBe(1);
      expect(state.active).toBe("p1");
      expect(viewFor(state, "p1").opponent.hand).toEqual({ count: 5 });
      const hand = viewFor(state, "p1").you.hand;
      expect(Array.isArray(hand) && hand.some((card) => card.defId === VOID)).toBe(true);
    }
    // The Heroic Power started in p2's hand "instead of a draw" (§6.2 Quickdraw).
    expect(withPower.players.p2.hand.some((card) => card.defId === HEROIC_POWER)).toBe(true);

    // §2.1: p2's opening hand is 4 cards either way, and whether one of them is a Quickdraw card is
    // p2's hand (§9.1). The Void's cost counts draws (R55). Were a Quickdraw card no draw, the cost p1
    // reads off its own hand would tell p1 how many Quickdraw cards p2 started with; R225 counts it
    // as the draw it replaces, so the cost is the same in both games.
    sameView(viewFor(withPower, "p1"), viewFor(without, "p1"));
  });
});

// ---------------------------------------------------------------------------
// Quickdraw and setup resumed after a clause's question (R224)
// ---------------------------------------------------------------------------

const ASKING = "r8-l10-asks";

function asking(state: GameState): void {
  state.transientDefs[ASKING] = fixtureDef(ASKING, "Spell");
  const script = askingOnArrival("the clause's question");
  registerScripts({ ...registeredScripts(), [ASKING]: { base: script, radiant: script } });
}

describe("R225, R224: a Quickdraw card is dealt as the last opening draw", () => {
  /**
   * p1's library holds one card whose start-of-game clause asks as it arrives in a hand (a fixture: no
   * Core card asks), and with this seed p1's opening draw reaches it, so setup waits for p1's answer
   * before it deals to p2 (R224, R151). The answer's action then carries p2's opening deal, which
   * p1's view shows.
   */
  function pausedDeal(seed: string, p2Twentieth: string): GameState {
    const game = createGame({ seed, decks: [[...QD_P1_DECK], [...QD_P2_SHARED, p2Twentieth]] });
    asking(game);
    game.players.p1.library[0] = newInstance(game, ASKING, "p1", { z: "library", player: "p1" });
    return beginGame(game).state;
  }

  function firstPausingSeed(): string {
    for (let at = 0; at < 200; at += 1) {
      const seed = `r8-l10-deal-${at}`;
      const state = pausedDeal(seed, CRAFT);
      if (state.pending?.kind === "mode" && state.pending.playerId === "p1") return seed;
    }
    throw new Error("no seed deals the asking card to p1");
  }

  it("R225 the events of p2's opening deal do not tell p1 whether p2's hand holds a Quickdraw card (§2.1, §9.1, R97, R224)", () => {
    const seed = firstPausingSeed();
    const answered = (p2Twentieth: string): GameState => {
      const state = pausedDeal(seed, p2Twentieth);
      const pending = must(state.pending, "p1's cast question");
      expect(pending.kind).toBe("mode");
      return actAs(state, "p1", { type: "answer", choiceId: pending.id, selection: [{ pick: "mode", option: "ok" }] });
    };
    const withPower = answered(HEROIC_POWER);
    const without = answered(CRAFT);

    // Setup went on to the mulligans, and p2 holds its 4 opening cards in both games.
    for (const state of [withPower, without]) {
      expect(mulliganOwed(state)).toEqual(["p1", "p2"]);
      expect(viewFor(state, "p1").opponent.hand).toEqual({ count: 4 });
    }
    expect(withPower.players.p2.hand.some((card) => card.defId === HEROIC_POWER)).toBe(true);

    // The deal's events reach p1's view (R168), redacted (R97). Were a Quickdraw card dealt with an
    // `addedToHand` alone, it would stand out among p2's `drawn` events and count p2's Quickdraw
    // cards, which are p2's hand (§9.1); R225 reports it as a draw, `drawn` then `addedToHand`.
    const typesOf = (state: GameState): string[] =>
      viewFor(state, "p1").events.map((event) => `${event.type}${"player" in event ? `:${event.player}` : ""}`);
    expect(typesOf(without)).toEqual(typesOf(withPower));
    sameView(viewFor(withPower, "p1"), viewFor(without, "p1"));
  });

  /** p1's deck is 19 cards plus #98 Heroic Power (Quickdraw) or #99; the asking card is p1's. */
  function pausedOwnDeal(seed: string, p1Twentieth: string): GameState {
    const game = createGame({ seed, decks: [[...QD_P1_DECK.slice(0, 19), p1Twentieth], [...QD_P2_SHARED, CRAFT]] });
    asking(game);
    game.players.p1.library[0] = newInstance(game, ASKING, "p1", { z: "library", player: "p1" });
    return beginGame(game).state;
  }

  it("R225 p1's hand and library counts while setup waits do not tell p2 whether p1's opening hand holds a Quickdraw card (§2.1, §9.1, R224)", () => {
    // A seed whose shuffle puts the asking card on top of p1's library, so it is p1's first draw: the
    // only card in p1's hand, where its clause arrived and asked.
    let seed: string | undefined;
    for (let at = 0; at < 400 && seed === undefined; at += 1) {
      const candidate = `r8-l10-own-deal-${at}`;
      const state = pausedOwnDeal(candidate, CRAFT);
      if (state.pending?.kind === "mode" && state.players.p1.hand.map((card) => card.defId).join() === ASKING) {
        seed = candidate;
      }
    }
    const found = must(seed, "a seed that draws the asking card first");
    const withPower = pausedOwnDeal(found, HEROIC_POWER);
    const without = pausedOwnDeal(found, CRAFT);

    // Both games wait on p1's question, the asking card in p1's hand, p2 not yet dealt.
    for (const state of [withPower, without]) {
      expect(state.pending?.kind).toBe("mode");
      expect(state.pending?.playerId).toBe("p1");
      expect(state.players.p2.hand).toHaveLength(0);
    }
    // R225: the Heroic Power replaces the last of p1's opening draws, so it is still in the library
    // while the first draw's clause asks.
    expect(withPower.players.p1.hand.map((card) => card.defId)).toEqual([ASKING]);
    expect(withPower.players.p1.library.some((card) => card.defId === HEROIC_POWER)).toBe(true);

    // p2 may count p1's hand and library (§10.8), but whether p1's deck holds a Quickdraw card is
    // p1's to keep (§9.1). Had the Heroic Power left the library for the hand before the other
    // draws, "instead of" a draw that has not happened yet, the counts would say so; it waits for
    // the last opening draw (R225), so both games count the same.
    sameView(viewFor(withPower, "p2"), viewFor(without, "p2"));
  });
});

// ---------------------------------------------------------------------------
// Round 10: setup is no player's turn (§2.1, §2.2, R155, R241), and a clause that asks (R151)
// ---------------------------------------------------------------------------

/** `player` returns its first opening card, so R9's replacement draw takes the top card (§2.1). */
function mulliganOne(state: GameState, player: PlayerId): GameState {
  const hand = state.players[player].hand.map((card) => card.id);
  return act(state, { type: "mulligan", keep: hand.slice(1), playerId: player }).state;
}

function keepAll(state: GameState, player: PlayerId): GameState {
  return act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
}

/** Play the game on, ending each turn at once (R82 may end one first), until `turn` has ended. */
function endTurnsThrough(state: GameState, turn: number): GameState {
  let next = state;
  for (let guard = 0; guard < 10 && next.result === null && next.turn <= turn; guard += 1) {
    if (next.pending !== null) throw new Error(`unexpected ${next.pending.kind} prompt on turn ${next.turn}`);
    next = act(next, { type: "endTurn", playerId: next.active }).state;
  }
  return next;
}

/** #23's return, verbatim in shape, on a Spell setup casts: the flag step 7 writes, or a play this turn. */
const SETUP_BOOMERANG: Script = {
  ...CAST_ON_SETUP_ARRIVAL,
  cry: () => [],
  endOfTurn: (ctx) => {
    const self = ctx.self;
    if (self === null) return [];
    const returns = self.returnToHandAtEndOfTurn === true || wasPlayedThisTurn(ctx.state, self.controller, self);
    return returns ? [bounce({ target: { of: "self" } })] : [];
  },
};

describe("R155, R241: a card setup casts belongs to no turn of its caster's (§2.1, §6.2)", () => {
  // No Core card casts in setup or carries an end-of-turn clause of a Spell it casts, so the card that
  // makes each case observable is a fixture; the rest of each game is real Core cards. Setup does not
  // draw a cast-on-draw card (R635), so the fixture casts itself as a replacement draw puts it in the
  // hand (`CAST_ON_SETUP_ARRIVAL`).
  it("R155 a return Spell cast by a mulligan's replacement draw stays in the graveyard at its caster's first turn end, a turn it was not played on", () => {
    // Once for each seat: p1's first turn end is turn 1's, p2's is turn 2's.
    for (const seat of ["p1", "p2"] as const) {
      let state = beginGame(createGame({ seed: `edge-r10-setup-return-${seat}`, decks: [P1_DECK, P2_DECK] })).state;
      expect(mulliganOwed(state)).toEqual(["p1", "p2"]);
      const id = `edge-r10-setup-boomerang-${seat}`;
      fixture(state, id, "Spell", SETUP_BOOMERANG);

      if (seat === "p2") state = keepAll(state, "p1");
      const boomerang = newInstance(state, id, seat, { z: "library", player: seat });
      state.players[seat].library.unshift(boomerang);
      state = mulliganOne(state, seat);
      if (seat === "p1") state = keepAll(state, "p2");
      // Both answers are in, so both resolved (R265): R9's replacement was cast during setup (§2.4,
      // R70) and landed in its caster's graveyard.
      expect(state.players[seat].graveyard.some((card) => card.id === boomerang.id)).toBe(true);
      expect(state.turn).toBe(1);

      // Through the caster's first turn end.
      state = endTurnsThrough(state, seat === "p1" ? 1 : 2);
      expect(state.result).toBeNull();

      // R155: the return belongs to the turn the Spell was played on, and a Spell played outside its
      // controller's turn "stays in the graveyard rather than coming back at the end of a later turn
      // it was not played on". Setup is turn 0 and nobody's turn (§2.1, §2.2): the turn-scoped
      // riders a setup cast makes are already dead on turn 1 (`thisTurn` of turn 0), and its return
      // is over too.
      const inHand = state.players[seat].hand.some((card) => card.id === boomerang.id);
      expect(inHand, `${seat}'s Spell cast during its mulligan came back to hand at the end of its first turn`).toBe(false);
    }
  });

  it("R241 an end-of-turn clause armed by a Spell p1's mulligan casts does not exile p1's hand at the end of turn 1", () => {
    let state = beginGame(createGame({ seed: "edge-r10-setup-exile", decks: [P1_DECK, P2_DECK] })).state;
    expect(mulliganOwed(state)).toEqual(["p1", "p2"]);

    // /fullsend's end-of-turn clause, verbatim in shape, on a Spell setup casts.
    const id = "edge-r10-setup-late-exile";
    fixture(state, id, "Spell", {
      ...CAST_ON_SETUP_ARRIVAL,
      cry: () => [delay({ at: { phase: "end", player: "self" }, step: "exile", hook: RESUME_HOOK })],
      resume: { exile: () => [exileHand({ player: "self" })] },
    });
    const cod = newInstance(state, id, "p1", { z: "library", player: "p1" });
    state.players.p1.library.unshift(cod);

    // p1's answer is sealed until p2's is in (R265); then both resolve, p1's first.
    state = mulliganOne(state, "p1");
    state = keepAll(state, "p2");
    expect(state.players.p1.graveyard.some((card) => card.id === cod.id)).toBe(true);
    // Setup's `active` names p1 only as a placeholder (§2.1): the cast was on no turn of p1's.
    expect(state.delayed).toEqual([]);
    expect(state.turn).toBe(1);
    const handOnTurnOne = state.players.p1.hand.map((card) => card.id);
    expect(handOnTurnOne.length).toBeGreaterThan(0);

    state = endTurnsThrough(state, 1);
    expect(state.turn).toBe(2);

    // R241: an end-of-turn clause is "the end of the turn the card was played on", and one made
    // outside its controller's turn "has no end of its controller's turn to wait for, so it is not
    // armed at all". The same card cast by p2's mulligan was never armed, and p1's was not played on
    // turn 1 either: its hand stays.
    const exiled = handOnTurnOne.filter((cardId) => state.players.p1.exile.some((card) => card.id === cardId));
    expect(exiled, "the clause of a Spell p1 cast during setup exiled p1's hand at the end of turn 1").toEqual([]);
  });
});

describe("R151, R113: a start-of-game clause that asks at §2.1 step 4", () => {
  it("R151 a start-of-game clause that asks holds the rest of setup, and turn 1, until it is answered (§2.1, §9.3, R113)", () => {
    let state = beginGame(createGame({ seed: "edge-r10-setup-start-asks", decks: [P1_DECK, P2_DECK] })).state;
    // "Start of game: choose one; then deal 3 damage to the enemy hero", at the bottom of p1's
    // library, where no opening draw reaches it: §2.1 step 4 runs it over the library too (R153).
    const id = "edge-r10-setup-start-asks";
    fixture(state, id, "Unit", {
      startOfGame: () => [
        chooseMode({ options: ["a", "b"], step: "picked", prompt: "the clause's question" }),
        damage({ to: { of: "enemyHero" }, amount: 3 }),
      ],
      resume: { picked: () => [] },
    });
    state.players.p1.library.push(newInstance(state, id, "p1", { z: "library", player: "p1" }));

    state = keepAll(state, "p1");
    state = keepAll(state, "p2");
    // §9.3: the question is state, and what comes after it — the rest of the clause and turn 1 —
    // waits for its answer rather than running over it.
    const pending = must(state.pending, "the clause's question");
    expect(pending.kind).toBe("mode");
    expect(pending.playerId).toBe("p1");
    expect(state.turn).toBe(0);
    expect(state.players.p2.hero.health).toBe(30);

    const answered = act(state, { type: "answer", choiceId: pending.id, selection: [{ pick: "mode", option: "a" }], playerId: "p1" });
    state = answered.state;
    // R122: the answer finishes the clause, then the setup it held: turn 1 is p1's (§2.1 step 5),
    // and it begins only after the clause's last effect.
    expect(state.players.p2.hero.health).toBe(27);
    expect(state.turn).toBe(1);
    expect(state.active).toBe("p1");
    const hitAt = answered.events.findIndex((event) => event.type === "damage" && event.amount === 3);
    const turnAt = answered.events.findIndex((event) => event.type === "turnStarted");
    expect(hitAt).toBeGreaterThanOrEqual(0);
    expect(turnAt).toBeGreaterThan(hitAt);
  });
});
