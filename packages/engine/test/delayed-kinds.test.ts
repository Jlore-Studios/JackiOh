// The delayed kinds of patch v0.2.0 (docs/classic-sets.md B5 E27, E28, R458): a destroy at the start
// of your next turn, aimed at a unit (Classic #20) or read then over a scope (its Radiant face); your
// hand discarded at the end of this turn or of your *next* turn (Classic #37); `delay`'s `next`; and
// a start-of-turn effect for the rest of the game (Classic+ #52), which runs in R62's delayed stage
// among the delayed effects in creation order, once per turn, and stacks.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE } from "../src/config";
import { discardHandAtTurnEnd } from "../src/effects";
import { DELAYED_DESTROY_HOOK, DELAYED_DISCARD_HAND_HOOK } from "../src/effects/delay";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { registerScripts, registeredScripts } from "../src/scripts";
import { createGame, type CardInstance, type GameState } from "../src/state";
import { START_OF_TURN_WORK } from "../src/turn";
import { viewFor } from "../src/viewFor";
import { moveToZone, placeOnField } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import {
  LOG_LANE,
  TURN_SCRIPTS,
  contract,
  contractAsk,
  doom,
  doomAll,
  hurrah,
  later,
  logCard,
  notes,
  reminder,
  turnCatalog,
} from "./fixtures/turn";

function register(): void {
  registerCatalog(turnCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...TURN_SCRIPTS });
}

let nonce = 0;

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `dk${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

function playing(seed: string): GameState {
  let state = beginGame(newGame(`delayed-kinds-${seed}`)).state;
  register();
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

function play(state: GameState, player: PlayerId, defId: string, extra: Partial<Extract<ActionInput, { type: "play" }>> = {}) {
  const card = inHand(state, defId, player)[0] as CardInstance;
  return { ...act(state, { type: "play", instanceId: card.id, playerId: player, ...extra }), card };
}

/** End the active player's turn. */
function pass(state: GameState): { state: GameState; events: GameEvent[] } {
  return act(state, { type: "endTurn", playerId: state.active });
}

const onField = (state: GameState, id: string): boolean =>
  (["p1", "p2"] as const).some((player) => state.players[player].units.some((pile) => pile?.[0]?.id === id));

describe("B5 E27: a destroy at the start of your next turn (R458)", () => {
  it("R458 the chosen unit is destroyed at the start of its maker's next turn, not the opponent's", () => {
    const state = playing("doom");
    const victim = put(state, "fx-25", slot("p2", "units", 1));
    const cast = play(state, "p1", doom.id, { targets: [{ pick: "instance", instanceId: victim.id }] }).state;
    expect(cast.delayed.map((entry) => [entry.resume.hook, entry.at, entry.watch])).toEqual([
      [DELAYED_DESTROY_HOOK, { phase: "start", player: "p1" }, victim.id],
    ]);

    const theirs = pass(cast).state;
    expect(theirs.active).toBe("p2");
    expect(onField(theirs, victim.id)).toBe(true);

    const { state: mine, events } = pass(theirs);
    expect(mine.active).toBe("p1");
    expect(onField(mine, victim.id)).toBe(false);
    expect(mine.players.p2.graveyard.map((card) => card.id)).toContain(victim.id);
    expect(eventsOfType(events, "destroyed").map((event) => event.instanceId)).toEqual([victim.id]);
    expect(mine.delayed).toEqual([]);
  });

  it("R458 it is a destroy: Indestructible ignores it (R46)", () => {
    const state = playing("doom-indestructible");
    const victim = put(state, "fx-25", slot("p2", "units", 1));
    victim.grantedKeywords = [{ kind: "Indestructible" }];
    const cast = play(state, "p1", doom.id, { targets: [{ pick: "instance", instanceId: victim.id }] }).state;
    const mine = pass(pass(cast).state).state;
    expect(onField(mine, victim.id)).toBe(true);
  });

  it("R458 (R174) it fizzles once the unit leaves the field, even when the unit comes back", () => {
    const state = playing("doom-fizzle");
    const victim = put(state, "fx-25", slot("p2", "units", 1));
    const cast = play(state, "p1", doom.id, { targets: [{ pick: "instance", instanceId: victim.id }] }).state;
    const card = cast.players.p2.units[0]?.[0] as CardInstance;
    moveToZone(cast, card, "hand");
    expect(cast.delayed).toEqual([]);
    expect(placeOnField(cast, card, slot("p2", "units", 1))).toBe(true);
    const mine = pass(pass(cast).state).state;
    expect(onField(mine, victim.id)).toBe(true);
  });

  it("R458 all enemy Units then: the ones standing as it resolves, not a list fixed when it was made", () => {
    const state = playing("doom-all");
    const early = put(state, "fx-25", slot("p2", "units", 1));
    const mine = put(state, "fx-5", slot("p1", "units", 1));
    const cast = play(state, "p1", doomAll.id).state;
    expect(cast.delayed.map((entry) => [entry.resume.hook, entry.watch])).toEqual([[DELAYED_DESTROY_HOOK, undefined]]);

    const theirs = pass(cast).state;
    const late = put(theirs, "fx-26", slot("p2", "units", 2));
    const back = pass(theirs).state;
    expect(onField(back, early.id)).toBe(false);
    expect(onField(back, late.id)).toBe(false);
    expect(onField(back, mine.id)).toBe(true);
  });
});

describe("B5 E27: your hand discarded at the end of this turn or your next (R458)", () => {
  it("R458 this turn: at the end of the turn it was made on, the maker's hand is discarded, the other hand kept", () => {
    const state = playing("hurrah");
    const cast = play(state, "p1", hurrah.id).state;
    const held = cast.players.p1.hand.length;
    expect(held).toBeGreaterThan(0);
    const other = cast.players.p2.hand.length;
    const { state: after, events } = pass(cast);
    const discarded = eventsOfType(events, "discarded").filter((event) => event.owner === "p1");
    expect(discarded).toHaveLength(held);
    expect(after.players.p1.hand).toEqual([]);
    // p2 drew for their turn; nothing of theirs was discarded.
    expect(after.players.p2.hand.length).toBeGreaterThanOrEqual(other);
  });

  it("R458 your next turn: the end of the turn it was made on passes it by, the end of your next one discards", () => {
    const state = playing("hurrah-next");
    const card = inHand(state, hurrah.id, "p1")[0] as CardInstance;
    card.radiant = true;
    const cast = act(state, { type: "play", instanceId: card.id, playerId: "p1" }).state;
    expect(cast.delayed.map((entry) => [entry.resume.hook, entry.notBefore])).toEqual([[DELAYED_DISCARD_HAND_HOOK, state.turn + 1]]);

    const afterFirst = pass(cast).state;
    expect(afterFirst.players.p1.hand.length).toBe(cast.players.p1.hand.length);
    const mine = pass(afterFirst).state;
    expect(mine.players.p1.hand.length).toBeGreaterThan(0);
    const { state: after, events } = pass(mine);
    expect(after.players.p1.hand).toEqual([]);
    expect(eventsOfType(events, "discarded").filter((event) => event.owner === "p1").length).toBeGreaterThan(0);
  });

  it("R458 made on the opponent's turn: this turn is theirs, and your next turn is the next one", () => {
    const state = playing("hurrah-theirs");
    const theirs = pass(state).state;
    expect(theirs.active).toBe("p2");
    const ctx = makeContext(sinkFor(theirs), null, { controller: "p1" });
    discardHandAtTurnEnd({ turn: "this" }).apply(ctx);
    discardHandAtTurnEnd({ turn: "next" }).apply(ctx);
    expect(theirs.delayed.map((entry) => [entry.owner, entry.at, entry.notBefore])).toEqual([
      ["p1", { phase: "end", player: "p2" }, undefined],
      ["p1", { phase: "end", player: "p1" }, theirs.turn + 1],
    ]);
    const held = theirs.players.p1.hand.length;
    const { state: mine, events } = pass(theirs);
    // Discarded at the end of p2's turn; the one card left is the draw of p1's own turn.
    expect(eventsOfType(events, "discarded").filter((event) => event.owner === "p1")).toHaveLength(held);
    expect(mine.players.p1.hand.map((card) => card.id)).toEqual(
      eventsOfType(events, "drawn").filter((event) => event.player === "p1").map((event) => event.instanceId),
    );
    expect(mine.delayed.map((entry) => entry.resume.hook)).toEqual([DELAYED_DISCARD_HAND_HOOK]);
  });

  it("R458 delay's next: a card step at the end of the maker's next turn, not this one", () => {
    const state = playing("later");
    const cast = play(state, "p1", later.id).state;
    const three = pass(pass(cast).state).state;
    expect(notes(three)).toEqual([]);
    const four = pass(three).state;
    expect(notes(four)).toEqual([`later:${state.turn + 2}`]);
  });
});

describe("B5 E28: for the rest of the game, at the start of your turn (R458)", () => {
  it("R458 it runs at each start of its player's turn from the next one, as the player's effect, and shows as a badge", () => {
    const state = playing("contract");
    const cast = play(state, "p1", contract.id).state;
    const label = "At the start of your turn, take a note";
    expect(viewFor(cast, "p1").you.modifiers).toContainEqual(expect.objectContaining({ label }));
    expect(viewFor(cast, "p2").opponent.modifiers).toContainEqual(expect.objectContaining({ label }));
    expect(notes(cast)).toEqual([]);

    const theirs = pass(cast).state;
    expect(notes(theirs)).toEqual([]);
    const three = pass(theirs).state;
    const five = pass(pass(three).state).state;
    expect(notes(five)).toEqual([`tick:p1:${state.turn + 2}:1:no-self`, `tick:p1:${state.turn + 4}:1:no-self`]);
  });

  it("R458 several stack, and they run with the delayed effects in creation order (R68)", () => {
    const state = playing("stack");
    let next = play(state, "p1", reminder.id).state;
    next = play(next, "p1", contract.id).state;
    next = play(next, "p1", contract.id).state;
    next = play(next, "p1", reminder.id).state;
    const turn = pass(pass(next).state).state.turn;
    const back = pass(pass(next).state).state;
    expect(notes(back)).toEqual(["delayed", `tick:p1:${turn}:1:no-self`, `tick:p1:${turn}:1:no-self`, "delayed"]);
  });

  it("R458 one that asks pauses the start of the turn, runs once that turn, and the pause survives JSON and replay", () => {
    const seed = "delayed-kinds-replay";
    const decks: [string[], string[]] = [[contractAsk.id, ...vanillaDeck(DECK_SIZE - 1, 1)], vanillaDeck(DECK_SIZE, 21)];
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
    // The note log is not part of an action-built game, so this one reads the prompt and the work.
    const held = state.players.p1.hand.find((card) => card.defId === contractAsk.id) as CardInstance;
    step({ type: "play", instanceId: held.id, playerId: "p1" });
    step({ type: "endTurn", playerId: "p1" });
    step({ type: "endTurn", playerId: "p2" });

    // p1's start of turn 3 is asking, before its draw and main phase.
    expect(state.active).toBe("p1");
    expect(state.phase).toBe("start");
    expect(state.pending?.playerId).toBe("p1");
    expect(state.work.map((item) => item.resume.hook)).toEqual(["delayed", START_OF_TURN_WORK]);
    const mod = state.players.p1.mods.find((entry) => entry.kind === "startOfTurnEffect");
    expect(mod).toMatchObject({ ranTurn: state.turn });
    const copy = JSON.parse(JSON.stringify(state)) as GameState;
    expect(copy).toEqual(state);

    const pending = state.pending;
    if (pending === null) throw new Error("expected a prompt");
    const hand = state.players.p1.hand.length;
    step({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: "p1" });
    expect(state.phase).toBe("main");
    expect(state.pending).toBeNull();
    // It ran once: the answer finished it and the turn went on to its draw, not back to the effect.
    expect(state.players.p1.hand.length).toBe(hand + 1);

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});
