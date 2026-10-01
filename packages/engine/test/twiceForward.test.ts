// C+ #74 Twice Forward One Step Backwards' Field Trap (subsystems/twiceForward.ts, R425): it counts the
// opponent's plays from when it is set, and on every second one, once that card has resolved, fuses it
// (Radiant: a Radiant copy of it) into itself (R77, R102) and gains Brittle (R385); with nothing left to
// fuse it gains the Brittle face-down; it turns face-up at its first fuse (R33). Through fixture scripts
// (fixtures/twiceForward.ts); the real card's test covers the same cases again.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defOf, registerCatalog, registeredCatalog } from "../src/catalog";
import { activeBrittleCount } from "../src/brittleCount";
import { beginGame, reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import { registerScripts, registeredScripts } from "../src/scripts";
import { findInstance, type CardInstance, type GameState } from "../src/state";
import { TWICE_FORWARD_PLAYS_KEY, twiceForwardPlays } from "../src/subsystems/twiceForward";
import { stepParam } from "../src/params";
import { viewFor } from "../src/viewFor";
import {
  CRIER_DAMAGE,
  TURNER_DAMAGE,
  TWICE_FORWARD_SCRIPTS,
  crier,
  forward,
  selfExiler,
  spell,
  trap,
  turner,
  twiceForwardCatalog,
} from "./fixtures/twiceForward";
import { eventsOfType, inHand, newGame, put, slot } from "./fixtures/harness";

let nonce = 0;

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `tf${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** p1 has the trap set face-down (lane 2) on its turn; then p2's turn begins, with mana to spare. */
function opponentsTurn(seed: string, radiant = false): { state: GameState; trapId: string } {
  let state = beginGame(newGame(`twice-forward-${seed}`)).state;
  registerCatalog(twiceForwardCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...TWICE_FORWARD_SCRIPTS });
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  const [held] = inHand(state, forward.id, "p1");
  if (held === undefined) throw new Error("no trap");
  held.radiant = radiant;
  state.players.p1.mana.current = 4;
  state = act(state, { type: "play", instanceId: held.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" }).state;
  // R227: a card set face-down takes a fresh id.
  const trapId = state.players.p1.backrow[1]?.id ?? "";
  state = act(state, { type: "endTurn", playerId: "p1" }).state;
  state.players.p2.mana.current = 10;
  return { state, trapId };
}

function play(state: GameState, player: PlayerId, defId: string, zone?: { row: "units" | "backrow"; lane: number }): {
  state: GameState;
  events: GameEvent[];
  card: CardInstance;
} {
  const card = inHand(state, defId, player)[0] as CardInstance;
  return { ...act(state, { type: "play", instanceId: card.id, ...(zone === undefined ? {} : { zone }), playerId: player }), card };
}

function trapOf(state: GameState, trapId: string): CardInstance {
  const found = findInstance(state, trapId);
  if (found === undefined) throw new Error("the trap is gone");
  return found;
}

describe("C+ #74's Field Trap (R425)", () => {
  it("R385 its printed Brittle starts when it is set", () => {
    const { state, trapId } = opponentsTurn("brittle");
    const card = trapOf(state, trapId);
    expect(card.faceUp === true).toBe(false);
    expect(activeBrittleCount(card)).toBe(4);
    expect(card.brittle?.since).toBe(1);
  });

  it("R425 R99 counts the opponent's 1st play and stays face-down, unfired", () => {
    const { state, trapId } = opponentsTurn("first");
    const after = play(state, "p2", spell.id);
    expect(twiceForwardPlays(trapOf(after.state, trapId))).toBe(1);
    expect(trapOf(after.state, trapId).faceUp === true).toBe(false);
    expect(eventsOfType(after.events, "trapFired")).toEqual([]);
  });

  it("R425 R77 on the 2nd, after it resolves, a Unit on the field is fused into this: still a Field Trap, +1 Brittle, face-up", () => {
    const { state, trapId } = opponentsTurn("unit");
    const first = play(state, "p2", spell.id);
    const second = play(first.state, "p2", "fx-5", { row: "units", lane: 1 });
    const kept = trapOf(second.state, trapId);
    expect(findInstance(second.state, second.card.id)).toBeUndefined();
    expect(second.state.players.p2.units[0]).toBeNull();
    expect(defOf(second.state, kept.defId).type).toBe("Field Trap");
    expect(kept.zone).toMatchObject({ z: "field", row: "backrow", lane: 2, player: "p1" });
    expect(activeBrittleCount(kept)).toBe(5);
    expect(kept.faceUp).toBe(true);
    const types = second.events.map((event) => event.type);
    expect(types.indexOf("cardResolved")).toBeLessThan(types.indexOf("trapFired"));
    expect(types.indexOf("trapFired")).toBeLessThan(types.indexOf("fused"));
  });

  it("R425 a Spell is fused from its owner's graveyard", () => {
    const { state, trapId } = opponentsTurn("spell");
    const second = play(play(state, "p2", spell.id).state, "p2", spell.id);
    expect(second.state.players.p2.graveyard.some((card) => card.id === second.card.id)).toBe(false);
    expect(eventsOfType(second.events, "fused")[0]?.instanceIds).toContain(second.card.id);
    expect(trapOf(second.state, trapId).faceUp).toBe(true);
  });

  it("R425 a trap is fused from its owner's backrow", () => {
    const { state, trapId } = opponentsTurn("trap");
    const second = play(play(state, "p2", spell.id).state, "p2", trap.id, { row: "backrow", lane: 1 });
    expect(second.state.players.p2.backrow[0]).toBeNull();
    // R227: the trap was set under a fresh id, the one its play announced.
    const setId = eventsOfType(second.events, "cardPlayed")[0]?.instanceId;
    expect(eventsOfType(second.events, "fused")[0]?.instanceIds).toContain(setId);
    expect(activeBrittleCount(trapOf(second.state, trapId))).toBe(5);
  });

  it("R589 R425 with nothing left to fuse (an exiled Spell) it still gains +1 Brittle, unfired and face-down (R33)", () => {
    const { state, trapId } = opponentsTurn("gone");
    const second = play(play(state, "p2", spell.id).state, "p2", selfExiler.id);
    const kept = trapOf(second.state, trapId);
    expect(second.card.id).toBeDefined();
    expect(second.state.players.p2.exile.some((card) => card.id === second.card.id)).toBe(true);
    expect(eventsOfType(second.events, "trapFired")).toEqual([]);
    expect(kept.faceUp === true).toBe(false);
    expect(kept.defId).toBe(forward.id);
    expect(activeBrittleCount(kept)).toBe(5);
    // The opponent reads neither the count nor the card.
    const theirs = JSON.stringify(viewFor(second.state, "p2"));
    expect(theirs).not.toContain(trapId);
    expect(theirs).not.toContain(forward.id);
  });

  it("R425 its own controller's plays never count", () => {
    const { state, trapId } = opponentsTurn("own");
    let next = act(state, { type: "endTurn", playerId: "p2" }).state;
    next.players.p1.mana.current = 10;
    next = play(next, "p1", spell.id).state;
    next = play(next, "p1", spell.id).state;
    expect(twiceForwardPlays(trapOf(next, trapId))).toBe(0);
    expect(trapOf(next, trapId).faceUp === true).toBe(false);
  });

  it("R102 a fused end-of-turn line runs for its controller; a fused Cry never runs", () => {
    const { state, trapId } = opponentsTurn("texts");
    let next = play(state, "p2", turner.id, { row: "units", lane: 1 }).state;
    next = play(next, "p2", turner.id, { row: "units", lane: 2 }).state;
    // The second Turner is fused into p1's trap; the first stays p2's.
    expect(trapOf(next, trapId).defId).not.toBe(forward.id);
    const p2Before = next.players.p2.hero.health;
    const p1Before = next.players.p1.hero.health;
    next = act(next, { type: "endTurn", playerId: "p2" }).state;
    // p2's own Turner hit p1 at p2's end of turn; the fused one waits for p1's end of turn.
    expect(next.players.p1.hero.health).toBe(p1Before - TURNER_DAMAGE);
    next = act(next, { type: "endTurn", playerId: "p1" }).state;
    expect(next.players.p2.hero.health).toBe(p2Before - TURNER_DAMAGE);

    const cry = opponentsTurn("cry");
    const crierFused = play(play(cry.state, "p2", spell.id).state, "p2", crier.id, { row: "units", lane: 1 });
    // The Crier's own Cry hit p1 as it was played; fused into p1's trap, its Cry never runs again.
    expect(eventsOfType(crierFused.events, "damage").filter((event) => event.amount === CRIER_DAMAGE)).toHaveLength(1);
    expect(crierFused.state.players.p2.hero.health).toBe(30);
  });

  it("R425 it goes on counting: every second play fuses again", () => {
    const { state, trapId } = opponentsTurn("again");
    let next = state;
    for (let i = 0; i < 4; i += 1) next = play(next, "p2", spell.id).state;
    expect(twiceForwardPlays(trapOf(next, trapId))).toBe(4);
    expect(activeBrittleCount(trapOf(next, trapId))).toBe(6);
  });

  it("R386 the every-N step reads through param(): an Upgrade's 'plays' never goes below 2, a Degrade makes it 3", () => {
    const { state, trapId } = opponentsTurn("params");
    stepParam(trapOf(state, trapId), "plays", -1);
    let next = play(play(state, "p2", spell.id).state, "p2", spell.id).state;
    expect(trapOf(next, trapId).faceUp).toBe(true);

    const slow = opponentsTurn("params-slow");
    stepParam(trapOf(slow.state, slow.trapId), "plays", 1);
    stepParam(trapOf(slow.state, slow.trapId), "brittleGain", 1);
    next = play(play(slow.state, "p2", spell.id).state, "p2", spell.id).state;
    expect(trapOf(next, slow.trapId).faceUp === true).toBe(false);
    next = play(next, "p2", spell.id).state;
    expect(trapOf(next, slow.trapId).faceUp).toBe(true);
    expect(activeBrittleCount(trapOf(next, slow.trapId))).toBe(6);
  });

  it("R179 its count and fused definition survive JSON, and the round trip plays on exactly as the live game", () => {
    const { state, trapId } = opponentsTurn("json");
    const fused = play(play(state, "p2", spell.id).state, "p2", "fx-5", { row: "units", lane: 1 }).state;
    const round = JSON.parse(JSON.stringify(fused)) as GameState;
    expect(round).toEqual(fused);
    expect(trapOf(round, trapId).memory[TWICE_FORWARD_PLAYS_KEY]).toBe(2);
    const [a] = inHand(fused, spell.id, "p2");
    const [b] = inHand(round, spell.id, "p2");
    const live = act(fused, { type: "play", instanceId: a?.id ?? "", playerId: "p2" }).state;
    const again = act(round, { type: "play", instanceId: b?.id ?? "", playerId: "p2" }).state;
    expect(hashState(again)).toBe(hashState(live));
    expect(twiceForwardPlays(trapOf(again, trapId))).toBe(3);
  });

  describe("radiant", () => {
    it("R469 a Radiant copy of the played card is fused in, and the card stays where it is", () => {
      const { state, trapId } = opponentsTurn("radiant", true);
      const second = play(play(state, "p2", spell.id).state, "p2", "fx-5", { row: "units", lane: 1 });
      expect(second.state.players.p2.units[0]?.[0]?.id).toBe(second.card.id);
      const kept = trapOf(second.state, trapId);
      expect(kept.defId).not.toBe(forward.id);
      expect(defOf(second.state, kept.defId).type).toBe("Field Trap");
      expect(activeBrittleCount(kept)).toBe(11);
      expect(kept.faceUp).toBe(true);
    });

    it("R425 a card that left still has its copy fused: the Radiant face always fuses", () => {
      const { state, trapId } = opponentsTurn("radiant-gone", true);
      const second = play(play(state, "p2", spell.id).state, "p2", selfExiler.id);
      expect(eventsOfType(second.events, "fused")).toHaveLength(1);
      expect(trapOf(second.state, trapId).faceUp).toBe(true);
      expect(second.state.players.p2.exile.some((card) => card.id === second.card.id)).toBe(true);
    });
  });

  it("put places it face-down too: a set trap counts from when it arrived", () => {
    const { state } = opponentsTurn("placed");
    const placed = put(state, forward.id, slot("p1", "backrow", 4));
    placed.faceUp = false;
    const next = play(state, "p2", spell.id).state;
    expect(twiceForwardPlays(trapOf(next, placed.id))).toBe(1);
  });
});
