// Draws counted per turn and limited (docs/classic-sets.md B5 E3, E4, R457), and the cast-on-draw
// enchantment and a Unit cast on draw with nowhere to stand (B5 E39, Classic+ #26, R459).
//
// What is pinned: every draw that happens is counted for its player on the turn it happens, whoever's
// turn that is — the start-of-turn draw and a fatigue draw included — and the `drawn` event carries the
// draw's number, which a trap reads however much later the loop hands it the event (Classic #9); a draw
// past a limit does not happen at all (no card moves, no fatigue, nothing is cast) and says so with a
// public `drawLimited`; the lowest limit holds and "both" binds its own controller; a named draw is a
// draw; and both survive a JSON round trip and a replay.

import type { Action, ActionInput, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE, UNIT_ZONES } from "../src/config";
import { drawBlocked, drawLimitOf, drawOne, drawsThisTurn } from "../src/draw";
import { drawFromLibrary } from "../src/effects";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { registerScripts, registeredScripts } from "../src/scripts";
import { drawFromLibraryOf } from "../src/ownership";
import { createGame, type CardInstance, type GameState } from "../src/state";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, setLibrary, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import {
  LOG_LANE,
  TURN_SCRIPTS,
  antiGreed,
  castSpell,
  castUnit,
  drawTwo,
  logCard,
  notes,
  palantir,
  plain,
  taxman,
  turnCatalog,
} from "./fixtures/turn";

function register(): void {
  registerCatalog(turnCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...TURN_SCRIPTS });
}

let nonce = 0;

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `dl${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(`draw-limit-${seed}`)).state;
  register();
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

const ids = (cards: readonly CardInstance[]): string[] => cards.map((card) => card.id);

describe("B5 E4: draws counted per player per turn (R457)", () => {
  it("R457 every draw that happens is counted for its player this turn, on either player's turn, fatigue included", () => {
    const state = playing("count");
    const sink = sinkFor(state);
    // Whatever p1 has drawn this turn so far, the next draw is one more.
    const already = drawsThisTurn(state, "p1");
    setLibrary(state, "p1", ["fx-5", "fx-6"]);
    setLibrary(state, "p2", ["fx-25"]);
    drawOne(sink, "p1");
    drawOne(sink, "p2");
    drawOne(sink, "p2");
    expect(drawsThisTurn(state, "p1")).toBe(already + 1);
    // p2's second draw found an empty library: a fatigue draw, and a draw that happened.
    expect(state.players.p2.fatigueCount).toBe(1);
    expect(drawsThisTurn(state, "p2")).toBe(2);
    expect(eventsOfType(sink.events, "drawn").map((event) => [event.player, event.turnDraw])).toEqual([
      ["p1", already + 1],
      ["p2", 1],
    ]);
  });

  it("R457 setup is no player's turn: the opening deal and the mulligan count nothing and number no draw (R225)", () => {
    const opened = beginGame(newGame("draw-limit-setup"));
    expect(eventsOfType(opened.events, "drawn").length).toBeGreaterThan(0);
    expect(eventsOfType(opened.events, "drawn").every((event) => event.turnDraw === undefined)).toBe(true);
    expect(opened.state.players.p1.draws).toBeUndefined();
    expect(opened.state.players.p2.draws).toBeUndefined();
  });

  it("R457 a drawn event's number is public, though the card it names is not (R97)", () => {
    const state = playing("public-count");
    setLibrary(state, "p2", ["fx-25", "fx-26"]);
    const after = act(state, { type: "endTurn", playerId: "p1" }).state;
    const theirDraw = viewFor(after, "p1").events.find((event) => event.type === "drawn" && event.player === "p2");
    expect(theirDraw).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID, turnDraw: 1 });
    const ownDraw = viewFor(after, "p2").events.find((event) => event.type === "drawn" && event.player === "p2");
    expect(ownDraw).toMatchObject({ defId: "fx-25", turnDraw: 1 });
  });

  it("R457 the count resets with the turn, as the turn log does, and the turn's own draw is the first of the new one", () => {
    const state = playing("reset");
    setLibrary(state, "p2", ["fx-25", "fx-26", "fx-27"]);
    drawOne(sinkFor(state), "p2");
    expect(drawsThisTurn(state, "p2")).toBe(1);
    const { state: after, events } = act(state, { type: "endTurn", playerId: "p1" });
    expect(after.active).toBe("p2");
    expect(drawsThisTurn(after, "p2")).toBe(1);
    expect(eventsOfType(events, "drawn").filter((event) => event.player === "p2").map((event) => event.turnDraw)).toEqual([1]);
    expect(drawsThisTurn(after, "p1")).toBe(0);
  });

  it("R457 a trap on the opponent's 2nd draw reads the draw's own number, however late the loop hands it the event", () => {
    const state = playing("taxman");
    put(state, taxman.id, slot("p1", "backrow", 1));
    setLibrary(state, "p2", ["fx-25", "fx-26", "fx-27", "fx-28"]);
    // p2's turn: the start-of-turn draw is the 1st, and sets nothing off.
    const started = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(notes(started)).toEqual([]);
    // One effect draws two more: both `drawn` events reach the trigger after the whole effect, when
    // the count already reads 3, and only the one that was the 2nd sets it off.
    const two = inHand(started, drawTwo.id, "p2")[0] as CardInstance;
    const { state: after, events } = act(started, { type: "play", instanceId: two.id, playerId: "p2" });
    expect(eventsOfType(events, "drawn").map((event) => event.turnDraw)).toEqual([2, 3]);
    expect(notes(after)).toEqual(["taxed"]);
    expect(drawsThisTurn(after, "p2")).toBe(3);
  });
});

describe("B5 E3: draw limits (R457)", () => {
  it("R457 a draw from the other player's deck is the drawer's draw, so the drawer's limit stops it before any card moves (E16)", () => {
    const state = playing("limit-opponent-deck");
    // Palantir in p2's backrow limits p2's opponent, p1, to one draw a turn.
    put(state, palantir.id, slot("p2", "backrow", 1));
    expect(drawLimitOf(state, "p1")).toBe(1);
    // p1's turn's own draw has been made: the one draw the limit allows.
    expect(drawsThisTurn(state, "p1")).toBe(1);
    const theirs = setLibrary(state, "p2", ["fx-26", "fx-27"]);
    const sink = sinkFor(state);
    expect(drawFromLibraryOf(sink, "p1", "p2")).toBe("limited");
    expect(ids(state.players.p2.library)).toEqual(ids(theirs));
    expect(eventsOfType(sink.events, "stolen")).toEqual([]);
    expect(eventsOfType(sink.events, "drawLimited")).toEqual([{ type: "drawLimited", player: "p1" }]);
  });

  it("R457 a draw past the limit does not happen: no card moves, nothing is cast, no fatigue, and drawLimited says so", () => {
    const state = playing("limit");
    put(state, palantir.id, slot("p1", "backrow", 1));
    expect(drawLimitOf(state, "p2")).toBe(1);
    expect(drawLimitOf(state, "p1")).toBeNull();

    const second = setLibrary(state, "p2", ["fx-25", castSpell.id])[1] as CardInstance;
    const sink = sinkFor(state);
    expect(drawOne(sink, "p2")).toBe("drawn");
    expect(drawOne(sink, "p2")).toBe("limited");
    // The cast-on-draw card stays where it was, uncast.
    expect(ids(state.players.p2.library)).toEqual([second.id]);
    expect(notes(state)).toEqual([]);
    expect(drawsThisTurn(state, "p2")).toBe(1);
    expect(eventsOfType(sink.events, "drawLimited")).toEqual([{ type: "drawLimited", player: "p2" }]);

    // An empty library behind a limit fatigues no one.
    state.players.p2.library = [];
    expect(drawOne(sink, "p2")).toBe("limited");
    expect(state.players.p2.fatigueCount).toBe(0);
    expect(eventsOfType(sink.events, "fatigue")).toEqual([]);
  });

  it("R457 the start-of-turn draw counts toward the limit, so a Draw 2 on that turn draws nothing", () => {
    const state = playing("start-draw");
    put(state, palantir.id, slot("p1", "backrow", 1));
    setLibrary(state, "p2", ["fx-25", "fx-26", "fx-27"]);
    const started = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(drawsThisTurn(started, "p2")).toBe(1);
    const card = started.players.p2.hand.find((held) => held.defId === "fx-25");
    expect(card).toBeDefined();

    const two = inHand(started, drawTwo.id, "p2")[0] as CardInstance;
    const { state: after, events } = act(started, { type: "play", instanceId: two.id, playerId: "p2" });
    expect(eventsOfType(events, "drawLimited")).toEqual([
      { type: "drawLimited", player: "p2" },
      { type: "drawLimited", player: "p2" },
    ]);
    expect(after.players.p2.library.map((held) => held.defId)).toEqual(["fx-26", "fx-27"]);
  });

  it("R457 with several limits the lowest holds, and a limit on both players binds its own controller too", () => {
    const state = playing("lowest");
    put(state, antiGreed.id, slot("p2", "units", 1));
    expect(drawLimitOf(state, "p1")).toBe(2);
    expect(drawLimitOf(state, "p2")).toBe(2);
    put(state, palantir.id, slot("p1", "backrow", 1));
    expect(drawLimitOf(state, "p2")).toBe(1);
    expect(drawLimitOf(state, "p1")).toBe(2);
    // §6.3 Vanilla: a card with no text sets no limit.
    const greed = state.players.p2.units[0]?.[0] as CardInstance;
    greed.vanilla = true;
    expect(drawLimitOf(state, "p1")).toBeNull();
  });

  it("R457 a named draw is a draw: the limit stops it and the card stays in the library", () => {
    const state = playing("named");
    put(state, palantir.id, slot("p1", "backrow", 1));
    const [first, second] = ids(setLibrary(state, "p2", ["fx-25", "fx-26"]));
    const ctx = makeContext(sinkFor(state), null, { controller: "p2" });
    drawFromLibrary({ instanceId: second }).apply(ctx);
    drawFromLibrary({ instanceId: first }).apply(ctx);
    expect(ids(state.players.p2.library)).toEqual([first]);
    expect(eventsOfType(ctx.events, "drawLimited")).toHaveLength(1);
    expect(drawBlocked(sinkFor(state), "p2")).toBe(true);
  });

  it("R457 a cast-on-draw chain stops at the limit: the next draw of the chain does not happen", () => {
    const state = playing("chain");
    put(state, palantir.id, slot("p2", "backrow", 1));
    setLibrary(state, "p1", [castSpell.id, plain.id]);
    // No draw of p1's yet this turn, so the chain's first draw is the one the limit allows.
    state.players.p1.draws = { turn: state.turn, count: 0 };
    const sink = sinkFor(state);
    expect(drawOne(sink, "p1")).toBe("cast");
    expect(notes(state)).toEqual(["cast-spell"]);
    expect(state.players.p1.library.map((held) => held.defId)).toEqual([plain.id]);
    expect(eventsOfType(sink.events, "drawLimited")).toEqual([{ type: "drawLimited", player: "p1" }]);
  });

  it("R457 drawLimited is public in both views: it names a player and no card", () => {
    const state = playing("public");
    put(state, palantir.id, slot("p1", "backrow", 1));
    setLibrary(state, "p2", ["fx-25", "fx-26"]);
    const started = act(state, { type: "endTurn", playerId: "p1" }).state;
    const two = inHand(started, drawTwo.id, "p2")[0] as CardInstance;
    const after = act(started, { type: "play", instanceId: two.id, playerId: "p2" }).state;
    for (const viewer of ["p1", "p2"] as const) {
      expect(viewFor(after, viewer).events.filter((event) => event.type === "drawLimited")).toEqual([
        { type: "drawLimited", player: "p2" },
        { type: "drawLimited", player: "p2" },
      ]);
    }
  });

  it("R457 a limited game survives JSON and replays from its log", () => {
    const seed = "draw-limit-replay";
    const decks: [string[], string[]] = [
      [palantir.id, ...vanillaDeck(DECK_SIZE - 1, 1)],
      [drawTwo.id, ...vanillaDeck(DECK_SIZE - 1, 21)],
    ];
    setupCatalog();
    register();
    let state = beginGame(createGame({ seed, decks })).state;
    const log: Action[] = [];
    const step = (body: ActionInput): void => {
      const action = { ...body, nonce: `rp${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
    };
    for (const player of ["p1", "p2"] as const) {
      step({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    }
    const limitCard = state.players.p1.hand.find((card) => card.defId === palantir.id) as CardInstance;
    step({ type: "play", instanceId: limitCard.id, playerId: "p1" });
    step({ type: "endTurn", playerId: "p1" });
    const library = state.players.p2.library.length;
    const two = state.players.p2.hand.find((card) => card.defId === drawTwo.id) as CardInstance;
    step({ type: "play", instanceId: two.id, playerId: "p2" });
    expect(state.players.p2.library).toHaveLength(library);
    expect(JSON.parse(JSON.stringify(state))).toEqual(state);

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});

describe("B5 E39 and Classic+ #26: cast on draw (R459)", () => {
  it("R459 the castOnDraw enchantment makes a drawn card cast on draw", () => {
    const state = playing("enchanted");
    const [card] = setLibrary(state, "p1", [plain.id, "fx-6"]);
    if (card === undefined) throw new Error("no card");
    const sink = sinkFor(state);
    expect(drawOne(sink, "p1")).toBe("drawn");
    expect(notes(state)).toEqual([]);

    const [enchanted] = setLibrary(state, "p1", [plain.id, "fx-6"]);
    if (enchanted === undefined) throw new Error("no card");
    enchanted.enchantments = [{ kind: "castOnDraw" }];
    expect(drawOne(sink, "p1")).toBe("cast");
    expect(notes(state)).toEqual(["plain"]);
    expect(state.players.p1.graveyard.map((held) => held.id)).toContain(enchanted.id);
    // The chain drew on: the card beneath came to hand.
    expect(state.players.p1.hand.map((held) => held.defId)).toContain("fx-6");
  });

  it("R459 a Unit cast on draw with no open unit zone goes to the hand uncast, and with one it is cast into it", () => {
    const state = playing("unit-room");
    for (let lane = 1; lane <= UNIT_ZONES; lane += 1) put(state, "fx-2", slot("p1", "units", lane));
    const [unit] = setLibrary(state, "p1", [castUnit.id]);
    if (unit === undefined) throw new Error("no card");
    const sink = sinkFor(state);
    expect(drawOne(sink, "p1")).toBe("drawn");
    expect(state.players.p1.hand.map((held) => held.id)).toContain(unit.id);
    expect(notes(state)).toEqual([]);
    expect(eventsOfType(sink.events, "cardPlayed")).toEqual([]);

    const open = playing("unit-open");
    put(open, "fx-2", slot("p1", "units", 1));
    const [second] = setLibrary(open, "p1", [castUnit.id]);
    const openSink = sinkFor(open);
    expect(drawOne(openSink, "p1")).toBe("cast");
    expect(open.players.p1.units[1]?.[0]?.id).toBe(second?.id);
    expect(notes(open)).toEqual(["cast-unit"]);
  });
});
