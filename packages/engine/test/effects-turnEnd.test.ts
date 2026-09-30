// Ending a turn from an effect (docs/classic-sets.md B5 E10, R456): `endTurn` and `endTurnAfterActions`
// (`src/effects/turnEnd.ts`), the "your turn ends" rider they leave (`modifiers.cutTurnShort`), and the
// reducer ending the turn once it is due (`reduce.endDueTurns`).
//
// What is pinned: the rest of the effect list and of the action resolve first, then the turn ends as
// if End turn were pressed, every end-of-turn step included, after `turnCutShort`; a prompt the action
// opened is answered first; Classic+ #26 drawn at the start of its controller's turn ends that turn
// before its main phase; "one more action" counts main-phase actions only and not the one that set
// it; ending the turn yourself uses it; on the other player's turn nothing happens; the AI card Rate
// Limit ends the opponent's turn after the play that set it off. And the pauses survive JSON and
// replay.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE } from "../src/config";
import { endTurn, endTurnAfterActions } from "../src/effects";
import { turnEndsOf } from "../src/modifiers";
import { beginGame, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { registerScripts, registeredScripts } from "../src/scripts";
import { createGame, type CardInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, setLibrary, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import {
  LOG_LANE,
  TURN_SCRIPTS,
  cutAsker,
  cutter,
  logCard,
  marker,
  notes,
  oneMore,
  questioner,
  rateLimit,
  tempo,
  turnCatalog,
  twoMore,
} from "./fixtures/turn";

function register(): void {
  registerCatalog(turnCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...TURN_SCRIPTS });
}

function game(seed: string): GameState {
  const state = newGame(`turn-end-${seed}`);
  register();
  return state;
}

let nonce = 0;

function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `te${nonce}` } as Action);
}

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  const result = actResult(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow. */
function playing(seed: string): GameState {
  let state = beginGame(game(seed)).state;
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  // R345: nothing but the rule under test ends a turn behind the test's back (R82).
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

function play(state: GameState, player: PlayerId, defId: string): { state: GameState; events: GameEvent[]; card: CardInstance } {
  const card = inHand(state, defId, player)[0] as CardInstance;
  return { ...act(state, { type: "play", instanceId: card.id, playerId: player }), card };
}

function answer(state: GameState): { state: GameState; events: GameEvent[] } {
  const pending = state.pending;
  if (pending === null) throw new Error("expected a prompt");
  return act(state, { type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: pending.playerId });
}

const roundTrip = (state: GameState): GameState => JSON.parse(JSON.stringify(state)) as GameState;
const typesOf = (events: readonly GameEvent[]): string[] => events.map((event) => event.type);

describe("B5 E10: End your turn (R456)", () => {
  it("R456 the rest of the effect list and of the action resolve, then the turn ends as if End turn were pressed", () => {
    const state = playing("cut");
    const { state: after, events, card } = play(state, "p1", cutter.id);

    expect(notes(after)).toEqual(["after the cut"]);
    expect(after.active).toBe("p2");
    expect(after.phase).toBe("main");
    expect(eventsOfType(events, "turnCutShort")).toEqual([{ type: "turnCutShort", player: "p1", byInstanceId: card.id }]);
    const types = typesOf(events);
    // The play resolved whole, then the cut, then every end-of-turn step, then the next turn.
    expect(types.indexOf("cardResolved")).toBeLessThan(types.indexOf("turnCutShort"));
    expect(types.indexOf("turnCutShort")).toBeLessThan(types.indexOf("turnEnded"));
    expect(types.indexOf("turnEnded")).toBeLessThan(types.lastIndexOf("turnStarted"));
    // Cleanup ran: the turn log was closed and the rider went with the turn.
    expect(after.players.p1.turnLog.unspentAtEnd).toBe(state.players.p1.mana.current);
    expect(after.players.p1.mods.filter((mod) => mod.kind === "turnEnds")).toEqual([]);
  });

  it("R456 a prompt the action opened is answered first; the paused game survives JSON and replays", () => {
    // Quickdraw puts the card in p1's opening hand, so the whole game is actions and a replay folds it.
    const seed = "turn-end-replay";
    const decks: [string[], string[]] = [[cutAsker.id, ...vanillaDeck(DECK_SIZE - 1, 1)], vanillaDeck(DECK_SIZE, 21)];
    setupCatalog();
    register();
    let state = beginGame(createGame({ seed, decks })).state;
    const log: Action[] = [];
    const step = (body: ActionInput): GameEvent[] => {
      const action = { ...body, nonce: `rp${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
      return result.events;
    };
    for (const player of ["p1", "p2"] as const) {
      step({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    }
    const held = state.players.p1.hand.find((card) => card.defId === cutAsker.id) as CardInstance;
    step({ type: "play", instanceId: held.id, playerId: "p1" });

    // Asked, so the turn has not ended: the rider waits with no actions left.
    expect(state.pending?.playerId).toBe("p1");
    expect(state.active).toBe("p1");
    expect(turnEndsOf(state, "p1")).toMatchObject({ actionsLeft: 0, byInstanceId: held.id });
    const copy = roundTrip(state);
    expect(copy).toEqual(state);

    const pending = state.pending;
    if (pending === null) throw new Error("expected a prompt");
    const events = step({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: seatToAct(state) });
    expect(eventsOfType(events, "turnCutShort")).toHaveLength(1);
    expect(state.active).toBe("p2");

    expect(hashState(answer(copy).state)).toBe(hashState(state));
    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });

  it("R456 on the other player's turn there is no turn of yours to end", () => {
    const state = playing("off-turn");
    const ctx = makeContext(sinkFor(state), null, { controller: "p2" });
    endTurn().apply(ctx);
    endTurnAfterActions({ actions: 1 }).apply(ctx);
    expect(state.players.p2.mods).toEqual([]);
    // "enemy" is the active player's turn, which exists.
    endTurn({ player: "enemy" }).apply(ctx);
    expect(turnEndsOf(state, "p1")).toMatchObject({ actionsLeft: 0 });
  });

  it("R456 Cast on draw: End your turn, drawn at the start of that turn, ends it before its main phase", () => {
    const state = playing("tempo");
    setLibrary(state, "p2", [tempo.id, "fx-30"]);
    const [drawn] = state.players.p2.library;
    const { state: after, events } = act(state, { type: "endTurn", playerId: "p1" });

    // p2's turn began, cast the unit it drew, and ended there: it is p1's turn again.
    expect(after.turn).toBe(state.turn + 2);
    expect(after.active).toBe("p1");
    expect(after.players.p2.units.some((pile) => pile?.[0]?.id === drawn?.id)).toBe(true);
    expect(eventsOfType(events, "turnCutShort")).toEqual([{ type: "turnCutShort", player: "p2", byInstanceId: drawn?.id ?? null }]);
    const p2Ended = events.findIndex((event) => event.type === "turnEnded" && event.player === "p2");
    expect(events.findIndex((event) => event.type === "turnCutShort")).toBeLessThan(p2Ended);
  });
});

describe("B5 E10: one more action, then your turn ends (R456)", () => {
  it("R456 Radiant: drawn at the start of the turn, the player takes one action, and the turn ends once it resolves", () => {
    const state = playing("one-more-drawn");
    setLibrary(state, "p2", [tempo.id, "fx-30"]);
    const radiantTempo = state.players.p2.library[0] as CardInstance;
    radiantTempo.radiant = true;

    const started = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(started.active).toBe("p2");
    expect(started.phase).toBe("main");
    expect(turnEndsOf(started, "p2")).toMatchObject({ actionsLeft: 1, byInstanceId: radiantTempo.id });
    // R169: the rider is a badge on both seats.
    const badge = { label: "Your turn ends after 1 more action" };
    expect(viewFor(started, "p2").you.modifiers).toContainEqual(expect.objectContaining(badge));
    expect(viewFor(started, "p1").opponent.modifiers).toContainEqual(expect.objectContaining(badge));

    const { state: after, events } = play(started, "p2", marker.id);
    expect(notes(after)).toEqual(["marker:p2"]);
    expect(after.active).toBe("p1");
    expect(eventsOfType(events, "turnCutShort")).toEqual([{ type: "turnCutShort", player: "p2", byInstanceId: radiantTempo.id }]);
  });

  it("R456 the action that sets the rider is not counted, a draw offer is not an action, and a position switch is", () => {
    const state = playing("count");
    const unit = put(state, "fx-2", slot("p1", "units", 1));
    unit.summonedTurn = state.turn;
    const set = play(state, "p1", oneMore.id).state;
    expect(set.active).toBe("p1");
    expect(turnEndsOf(set, "p1")?.actionsLeft).toBe(1);

    const offered = act(set, { type: "offerDraw", playerId: "p1" }).state;
    expect(offered.active).toBe("p1");
    expect(turnEndsOf(offered, "p1")?.actionsLeft).toBe(1);

    const { state: after, events } = act(offered, { type: "switchPosition", instanceId: unit.id, playerId: "p1" });
    expect(after.active).toBe("p2");
    expect(eventsOfType(events, "turnCutShort")).toHaveLength(1);
  });

  it("R456 the last action ends the turn once it has resolved, its prompt included, and an answer is no action of its own", () => {
    const state = playing("last-asks");
    const set = play(state, "p1", twoMore.id).state;
    expect(turnEndsOf(set, "p1")?.actionsLeft).toBe(2);
    const first = play(set, "p1", marker.id).state;
    expect(first.active).toBe("p1");
    expect(turnEndsOf(first, "p1")?.actionsLeft).toBe(1);

    const asking = play(first, "p1", questioner.id).state;
    expect(asking.pending?.playerId).toBe("p1");
    expect(asking.active).toBe("p1");
    expect(turnEndsOf(asking, "p1")?.actionsLeft).toBe(0);

    const { state: after, events } = answer(asking);
    expect(notes(after)).toEqual(["marker:p1", "questioner:answered"]);
    expect(after.active).toBe("p2");
    expect(eventsOfType(events, "turnCutShort")).toHaveLength(1);
  });

  it("R456 ending the turn yourself uses the actions up: no cut, and the rider ends with the turn", () => {
    const state = playing("self-end");
    const set = play(state, "p1", oneMore.id).state;
    const { state: after, events } = act(set, { type: "endTurn", playerId: "p1" });
    expect(eventsOfType(events, "turnCutShort")).toEqual([]);
    expect(after.active).toBe("p2");
    expect(after.players.p1.mods.filter((mod) => mod.kind === "turnEnds")).toEqual([]);
    // p2's own actions are p2's: nothing of p1's rider reaches them.
    expect(play(after, "p2", marker.id).state.active).toBe("p2");
  });

  it("R456 with two riders on one turn the sooner end holds, and names the card that set it", () => {
    const state = playing("two-riders");
    const sink = sinkFor(state);
    const first = put(state, "fx-3", slot("p1", "units", 1));
    const second = put(state, "fx-4", slot("p1", "units", 2));
    endTurnAfterActions({ actions: 2 }).apply(makeContext(sink, first));
    endTurn().apply(makeContext(sink, second));
    expect(turnEndsOf(state, "p1")).toMatchObject({ actionsLeft: 0, byInstanceId: second.id });
    endTurnAfterActions({ actions: 1 }).apply(makeContext(sink, first));
    expect(turnEndsOf(state, "p1")).toMatchObject({ actionsLeft: 0, byInstanceId: second.id });
    expect(state.players.p1.mods.filter((mod) => mod.kind === "turnEnds")).toHaveLength(1);
  });
});

describe("B5 E10: the opponent's trap ends the turn (R456, the AI card Rate Limit)", () => {
  it("R456 the play that set it off resolves first, then the active player's turn ends, named by the trap", () => {
    const state = playing("rate-limit");
    const trap = put(state, rateLimit.id, slot("p2", "backrow", 1));
    const { state: after, events } = play(state, "p1", marker.id);

    expect(notes(after)).toEqual(["marker:p1"]);
    expect(after.active).toBe("p2");
    expect(eventsOfType(events, "turnCutShort")).toEqual([{ type: "turnCutShort", player: "p1", byInstanceId: trap.id }]);
    const types = typesOf(events);
    expect(types.indexOf("cardResolved")).toBeLessThan(types.indexOf("turnCutShort"));
    // The trap fired face-up and went to its graveyard: both players may read what cut the turn.
    for (const viewer of ["p1", "p2"] as const) {
      expect(viewFor(after, viewer).events.find((event) => event.type === "turnCutShort")).toMatchObject({ byInstanceId: trap.id });
    }
  });
});
