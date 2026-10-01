// E29 board snapshots (docs/classic-sets.md B5 E29; SPEC §2.2, §10.1; R419, R562, R563, R566, R227): the
// record at each turn's start, the restore's three steps (`restoreBoard`), the R227 rename that keeps the
// history naming a card by its current id, R563's Reborn hold, the views, and a Rollback played through
// `reduce` that folds back from its log. Fixtures: `fixtures/boardHistory.ts`, and the field fixtures'
// Frostspatula and Ivory Tower shapes (`fixtures/field.ts`). The card itself: C+ #35's test file.

import { describe, expect, it } from "vitest";
import type { Action, ActionInput, GameEvent } from "@jackioh/shared";
import { BOARD_HISTORY_DEPTH, DECK_SIZE } from "../src/config";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { stateCheck } from "../src/stateCheck";
import { createGame, findInstance, newInstance, type GameState } from "../src/state";
import { recordBoardSnapshot, restoreBoard, snapshotFor } from "../src/subsystems/boardHistory";
import { viewFor } from "../src/viewFor";
import { cardAt, freshFaceDownId, isLocked, lockZone, moveToZone, placeOnField, zoneContents } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { phoenix, registerBoardHistoryFixtures, rewind } from "./fixtures/boardHistory";
import { act, playing, spatula, tower, watcher } from "./fixtures/field";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";

/** p1's main phase of turn 1 with the field fixtures and this file's registered, the history emptied. */
function board(seed: string): GameState {
  const state = playing(seed);
  registerBoardHistoryFixtures();
  delete state.boardHistory;
  return state;
}

const plainUnit = "fx-1";

describe("E29 the record (R419, R62)", () => {
  it("R419 a turn's start records the field before anything else — an 'Animated on your turn' card is still in its backrow zone — and keeps BOARD_HISTORY_DEPTH", () => {
    let state = playing("bh-record");
    expect(state.boardHistory?.map((snapshot) => snapshot.turn)).toEqual([1]);
    put(state, spatula.id, slot("p1", "backrow", 2));
    const unit = put(state, plainUnit, slot("p1", "units", 1));
    unit.damage = 1;
    state = act(state, { type: "endTurn", playerId: "p1" });
    state = act(state, { type: "endTurn", playerId: "p2" });

    const snapshot = state.boardHistory?.at(-1);
    expect(snapshot?.turn).toBe(3);
    expect(snapshot?.sides.p1.backrow[1]?.defId).toBe(spatula.id);
    expect(cardAt(state, slot("p1", "units", 2))?.defId).toBe(spatula.id);
    expect(snapshot?.sides.p1.units[0]?.[0]).toMatchObject({ id: unit.id, damage: 1 });
    expect(snapshot?.sides.p2.locks).toEqual(state.players.p2.locks);

    for (let n = 0; n < 4; n += 1) state = act(state, { type: "endTurn", playerId: state.active });
    expect(BOARD_HISTORY_DEPTH).toBe(4);
    expect(state.boardHistory?.map((s) => s.turn)).toEqual([4, 5, 6, 7]);
    // §9.3: plain data.
    expect(JSON.parse(JSON.stringify(state.boardHistory))).toEqual(state.boardHistory);
  });

  it("R562 the lookup: the snapshot N turns back, else the oldest held; none before the first turn", () => {
    const state = board("bh-lookup");
    expect(snapshotFor(state, 1)).toBeNull();
    state.turn = 5;
    recordBoardSnapshot(state);
    state.turn = 6;
    recordBoardSnapshot(state);
    expect(snapshotFor(state, 1)?.turn).toBe(5);
    expect(snapshotFor(state, 3)?.turn).toBe(5);
  });
});

describe("E29 the restore (R419)", () => {
  it("R419 puts back cards from a library, a hand, a graveyard and exile, rebuilds a Stack pile, a backrow pile and a carried Unit, and restores the Locks", () => {
    const state = board("bh-restore");
    const fromLibrary = put(state, plainUnit, slot("p1", "units", 1));
    const fromHand = put(state, "fx-2", slot("p1", "units", 2));
    const fromGraveyard = put(state, "fx-3", slot("p2", "units", 1));
    const fromExile = put(state, "fx-4", slot("p2", "units", 2));
    const top = newInstance(state, "fx-5", "p1", { z: "hand", player: "p1" });
    placeOnField(state, top, slot("p1", "units", 1), { stack: true });
    const carrier = put(state, tower.id, slot("p1", "backrow", 1));
    const carried = newInstance(state, "fx-6", "p1", { z: "hand", player: "p1" });
    placeOnField(state, carried, slot("p1", "backrow", 1));
    const trap = put(state, watcher.id, slot("p2", "backrow", 3));
    lockZone(state, slot("p2", "units", 5));
    recordBoardSnapshot(state);

    moveToZone(state, fromLibrary, "library");
    moveToZone(state, fromHand, "hand");
    moveToZone(state, fromGraveyard, "graveyard");
    moveToZone(state, fromExile, "exile");
    moveToZone(state, carried, "hand");
    const newcomer = put(state, "fx-7", slot("p2", "units", 4));
    lockZone(state, slot("p1", "units", 3));
    state.players.p2.locks.units[4] = false;

    const sink = sinkFor(state);
    expect(restoreBoard(sink, "p1", 1, ["p1", "p2"])).toBe(0);
    expect(zoneContents(state, slot("p1", "units", 1)).map((card) => card.id)).toEqual([top.id, fromLibrary.id]);
    expect(cardAt(state, slot("p1", "units", 2))?.id).toBe(fromHand.id);
    expect(cardAt(state, slot("p2", "units", 1))?.id).toBe(fromGraveyard.id);
    expect(cardAt(state, slot("p2", "units", 2))?.id).toBe(fromExile.id);
    expect(zoneContents(state, slot("p1", "backrow", 1)).map((card) => card.id)).toEqual([carried.id, carrier.id]);
    expect(cardAt(state, slot("p2", "backrow", 3))?.id).toBe(trap.id);
    expect(findInstance(state, newcomer.id)?.zone.z).toBe("hand");
    expect(isLocked(state, slot("p1", "units", 3))).toBe(false);
    expect(isLocked(state, slot("p2", "units", 5))).toBe(true);
    for (const pile of ["hand", "library", "graveyard", "exile"] as const) {
      expect(state.players.p1[pile].map((card) => card.id)).not.toContain(fromLibrary.id);
    }
    const types = sink.events.map((event) => event.type);
    expect(types[0]).toBe("rolledBack");
    expect(eventsOfType(sink.events, "locked")).toEqual([{ type: "locked", player: "p2", row: "units", lane: 5 }]);
    expect(eventsOfType(sink.events, "unlocked")).toEqual([{ type: "unlocked", player: "p1", row: "units", lane: 3 }]);
    // The face-down trap never moved: no fresh id and no event names it.
    expect(eventsOfType(sink.events, "controlChanged").map((event) => event.instanceId)).not.toContain(trap.id);
  });

  it("R227 a card that takes a fresh id is renamed in the history, so a re-set trap goes back as itself, not as a second copy", () => {
    const state = board("bh-rename");
    const trap = put(state, watcher.id, slot("p1", "backrow", 2));
    const first = trap.id;
    recordBoardSnapshot(state);
    moveToZone(state, trap, "hand");
    freshFaceDownId(state, trap);
    placeOnField(state, trap, slot("p1", "backrow", 4));
    expect(state.boardHistory?.[0]?.sides.p1.backrow[1]?.id).toBe(trap.id);
    expect(trap.id).not.toBe(first);

    restoreBoard(sinkFor(state), "p1", 1, ["p1"]);
    expect(cardAt(state, slot("p1", "backrow", 2))?.id).toBe(trap.id);
    expect(cardAt(state, slot("p1", "backrow", 4))).toBeNull();
    expect(findInstance(state, first)).toBeUndefined();
  });

  it("R227 a card going back face-down from a public zone takes a fresh id; its views follow it by formerId (R97)", () => {
    const state = board("bh-fresh");
    const trap = put(state, watcher.id, slot("p2", "backrow", 1));
    recordBoardSnapshot(state);
    moveToZone(state, trap, "graveyard");
    const old = trap.id;
    const events: GameEvent[] = [];
    restoreBoard(sinkFor(state, events), "p1", 1, ["p2"]);
    const back = cardAt(state, slot("p2", "backrow", 1));
    expect(back?.id).not.toBe(old);
    expect(eventsOfType(events, "controlChanged")).toEqual([
      { type: "controlChanged", instanceId: back?.id, controller: "p2", row: "backrow", lane: 1, formerId: old },
    ]);
    state.applied.push({ nonce: "bh-fresh", events });
    const seen = eventsOfType(viewFor(state, "p1").events, "controlChanged");
    expect(seen).toEqual([{ type: "controlChanged", instanceId: "hidden", controller: "p2", row: "backrow", lane: 1 }]);
    expect(eventsOfType(viewFor(state, "p2").events, "controlChanged")[0]?.formerId).toBe(old);
  });

  it("R566 a card that stayed on its side keeps its exertion and sickness; one put back from elsewhere entered on this turn", () => {
    const state = board("bh-turn-state");
    const stayed = put(state, plainUnit, slot("p1", "units", 1));
    const away = put(state, "fx-2", slot("p1", "units", 2));
    recordBoardSnapshot(state);
    stayed.exertion = { attacked: true, switched: false };
    moveToZone(state, away, "hand");
    const events: GameEvent[] = [];
    restoreBoard(sinkFor(state, events), "p1", 1, ["p1"]);
    expect(stayed.exertion.attacked).toBe(true);
    const back = findInstance(state, away.id);
    expect(back?.summonedTurn).toBe(state.turn);
    expect(eventsOfType(events, "controlChanged").map((event) => event.instanceId)).toEqual([away.id]);
  });
});

describe("E29 held zones (R563)", () => {
  it("R563 a Reborn unit whose zone a Rollback let go does not return: the snapshot's occupant stands there", () => {
    const state = board("bh-reborn");
    state.turn = 2;
    const occupant = put(state, plainUnit, slot("p1", "units", 1));
    recordBoardSnapshot(state);
    moveToZone(state, occupant, "hand");
    const bird = put(state, phoenix.id, slot("p1", "units", 1));
    bird.damage = 2;

    const events: GameEvent[] = [];
    stateCheck(sinkFor(state, events));
    expect(eventsOfType(events, "rolledBack")).toHaveLength(1);
    expect(zoneContents(state, slot("p1", "units", 1)).map((card) => card.id)).toEqual([occupant.id]);
    expect(findInstance(state, bird.id)?.zone.z).toBe("graveyard");
    expect(state.reserved).toEqual([]);
  });
});

describe("E29 through reduce (§9.3)", () => {
  it("R419 a game that rolls back survives JSON and folds to the same hash from its log", () => {
    registerBoardHistoryFixtures();
    const decks: [string[], string[]] = [[rewind.id, ...vanillaDeck(DECK_SIZE - 1, 1)], vanillaDeck(DECK_SIZE, 21)];
    let state = beginGame(createGame({ seed: "bh-fold", decks })).state;
    const log: Action[] = [];
    const step = (body: ActionInput): void => {
      const action = { ...body, nonce: `bh-${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
    };
    step({ type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
    step({ type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
    while (state.turn < 5 || state.active !== "p1" || !state.players.p1.hand.some((c) => c.defId === rewind.id)) {
      const player = state.active;
      const unit = state.players[player].hand.find((c) => c.defId !== rewind.id);
      if (unit !== undefined && state.players[player].mana.current >= 1) {
        step({ type: "play", playerId: player, instanceId: unit.id, zone: { row: "units", lane: 1 + state.players[player].units.filter((p) => p !== null).length } } as ActionInput);
      }
      step({ type: "endTurn", playerId: player });
      if (state.turn > 20) throw new Error("p1 never held the fixture");
    }
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(hashState(round)).toBe(hashState(state));
    const card = state.players.p1.hand.find((c) => c.defId === rewind.id)?.id ?? "";
    step({ type: "play", playerId: "p1", instanceId: card, modes: ["2"] } as ActionInput);
    expect(state.players.p1.graveyard.some((c) => c.defId === rewind.id)).toBe(true);

    const replayed = fold({ seed: "bh-fold", decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
    expect(JSON.stringify(viewFor(state, "p2"))).not.toContain("boardHistory");
  });
});
