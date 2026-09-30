// What every play leaves to be counted (docs/classic-sets.md B5 E4; R451): per player per turn, the
// plays by type (Classic+ #37 Wardrum), on both players' turns and cleared at every start of turn;
// per player per game, the plays by tag (Classic+ #64's Fruit, AI Scaling Law's AI); game-wide, the
// last Spell anyone played (Classic #57 Echo, which records the Spell it copied), and the last face-up
// card each player played (AI Autocomplete: Traps are set face-down and never count, AI generated cards
// are passed over). Casts count (R70); countered plays never (R448).

import type { Action, ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { beginGame, reduce } from "../src/reduce";
import {
  lastFaceUpPlayed,
  lastSpellPlayed,
  playedThisGameWithTag,
  playedThisTurnOfType,
} from "../src/query";
import { castCard } from "../src/resolve";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";
import { PA, withPlayA } from "./fixtures/playPipelineA";

let nonce = 0;

function game(seed: string): GameState {
  let ready = beginGame(withPlayA(newGame(seed))).state;
  for (const player of ["p1", "p2"] as const) {
    ready = must(ready, player, { type: "mulligan", keep: ready.players[player].hand.map((card) => card.id) }).state;
  }
  for (const player of ["p1", "p2"] as const) {
    ready.players[player].mana.current = 9;
    ready.players[player].mana.max = 9;
  }
  return ready;
}

function must(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, playerId, nonce: `pa-pc-${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function hand(state: GameState, player: PlayerId, defId: string, radiant = false): CardInstance {
  const [card] = inHand(state, defId, player);
  if (card === undefined) throw new Error("no card");
  card.radiant = radiant;
  return card;
}

/** Play a card that needs no choices (a hero target for the bolts), and return the state after. */
function play(state: GameState, player: PlayerId, defId: string, radiant = false): GameState {
  const card = hand(state, player, defId, radiant);
  const targets = defId === PA.bolt.id ? [{ pick: "hero" as const, player: player === "p1" ? ("p2" as const) : ("p1" as const) }] : [];
  return must(state, player, { type: "play", instanceId: card.id, ...(targets.length === 0 ? {} : { targets }) }).state;
}

const NON_UNIT = ["Spell", "Field Spell", "Trap"] as const;

describe("R451 plays by type, this turn (Classic+ #37)", () => {
  it("R451 counts each play by the type it was played as, and a Field Trap is a Trap", () => {
    let state = game("r451-types");
    state = play(state, "p1", PA.field.id);
    state = play(state, "p1", PA.ping.id);
    state = play(state, "p1", PA.crier.id);
    state = play(state, "p1", PA.hiddenTrap.id);
    state = play(state, "p1", PA.hiddenFieldTrap.id);
    expect(playedThisTurnOfType(state, "p1", "Unit")).toBe(1);
    expect(playedThisTurnOfType(state, "p1", "Trap")).toBe(1);
    expect(playedThisTurnOfType(state, "p1", ["Trap", "Field Trap"])).toBe(2);
    expect(playedThisTurnOfType(state, "p1", [...NON_UNIT, "Field Trap"])).toBe(4);
    expect(playedThisTurnOfType(state, "p2", "Spell")).toBe(0);
  });

  it("R451 clears at every start of turn, for both players", () => {
    let state = play(game("r451-reset"), "p1", PA.ping.id);
    expect(playedThisTurnOfType(state, "p1", "Spell")).toBe(1);
    state = must(state, "p1", { type: "endTurn" }).state;
    expect(state.active).toBe("p2");
    expect(playedThisTurnOfType(state, "p1", "Spell")).toBe(0);
    expect(state.players.p1.turnLog.playedByType).toBeUndefined();
  });

  it("R451 a cast counts (R70), on the turn it happens, whoever's turn that is", () => {
    const state = game("r451-cast");
    const ping = newInstance(state, PA.ping.id, "p2", { z: "hand", player: "p2" });
    const sink = sinkFor(state);
    castCard(sink, ping);
    settle(sink);
    expect(state.active).toBe("p1");
    expect(playedThisTurnOfType(state, "p2", "Spell")).toBe(1);
    expect(playedThisGameWithTag(state, "p2", "Book")).toBe(0);
    expect(lastSpellPlayed(state)).toEqual({ defId: PA.ping.id, radiant: false });
  });

  it("R451 a countered play counts nowhere (R448)", () => {
    const state = game("r451-countered");
    put(state, PA.counterTrap.id, slot("p2", "backrow", 1));
    const after = play(state, "p1", PA.apple.id);
    expect(playedThisTurnOfType(after, "p1", "Spell")).toBe(0);
    expect(playedThisGameWithTag(after, "p1", "Fruit")).toBe(0);
    expect(lastSpellPlayed(after)).toBeNull();
    expect(lastFaceUpPlayed(after, "p1")).toBeNull();
  });
});

describe("R451 plays by tag, this game (Classic+ #64, AI Scaling Law)", () => {
  it("R451 counts each tag of each play, and never resets", () => {
    let state = game("r451-tags");
    state = play(state, "p1", PA.apple.id);
    state = play(state, "p1", PA.pear.id);
    state = play(state, "p1", PA.aiCard.id);
    expect(playedThisGameWithTag(state, "p1", "Fruit")).toBe(2);
    expect(playedThisGameWithTag(state, "p1", "AI")).toBe(1);
    expect(playedThisGameWithTag(state, "p1", "Token")).toBe(1);
    state = must(state, "p1", { type: "endTurn" }).state;
    state = must(state, "p2", { type: "endTurn" }).state;
    expect(state.active).toBe("p1");
    expect(playedThisGameWithTag(state, "p1", "Fruit")).toBe(2);
    expect(playedThisGameWithTag(state, "p2", "Fruit")).toBe(0);
  });
});

describe("R451 the last Spell played, game-wide (Classic #57)", () => {
  it("R451 records the last Spell either player played, with its face, and no other type overwrites it", () => {
    let state = game("r451-last-spell");
    expect(lastSpellPlayed(state)).toBeNull();
    state = play(state, "p1", PA.bolt.id, true);
    expect(lastSpellPlayed(state)).toEqual({ defId: PA.bolt.id, radiant: true });
    state = play(state, "p1", PA.crier.id);
    state = play(state, "p1", PA.field.id);
    expect(lastSpellPlayed(state)).toEqual({ defId: PA.bolt.id, radiant: true });
    state = must(state, "p1", { type: "endTurn" }).state;
    state = play(state, "p2", PA.ping.id);
    expect(lastSpellPlayed(state)).toEqual({ defId: PA.ping.id, radiant: false });
  });

  it("R451 a played Echo records the Spell it copied, never itself, so it can't copy itself into a loop", () => {
    let state = game("r451-echo");
    state = play(state, "p1", PA.bolt.id, true);
    state = play(state, "p1", PA.echoCopy.id);
    expect(lastSpellPlayed(state)).toEqual({ defId: PA.bolt.id, radiant: true });
    expect(lastFaceUpPlayed(state, "p1")).toEqual({ defId: PA.bolt.id, radiant: true, type: "Spell" });
    // With nothing to copy, it records nothing.
    const fresh = play(game("r451-echo-empty"), "p1", PA.echoCopy.id);
    expect(lastSpellPlayed(fresh)).toBeNull();
  });
});

describe("R451 the last face-up card each player played (AI Autocomplete)", () => {
  it("R451 records per player; Traps set face-down and AI generated cards never count", () => {
    let state = game("r451-face-up");
    state = play(state, "p1", PA.crier.id);
    expect(lastFaceUpPlayed(state, "p1")).toEqual({ defId: PA.crier.id, radiant: false, type: "Unit" });
    state = play(state, "p1", PA.hiddenTrap.id);
    state = play(state, "p1", PA.hiddenFieldTrap.id);
    state = play(state, "p1", PA.aiCard.id);
    expect(lastFaceUpPlayed(state, "p1")).toEqual({ defId: PA.crier.id, radiant: false, type: "Unit" });
    state = play(state, "p1", PA.field.id);
    expect(lastFaceUpPlayed(state, "p1")).toEqual({ defId: PA.field.id, radiant: false, type: "Field Spell" });
    expect(lastFaceUpPlayed(state, "p2")).toBeNull();
    // The records are copies: writing through one changes nothing.
    const read = lastFaceUpPlayed(state, "p1");
    if (read !== null) read.defId = "nothing";
    expect(lastFaceUpPlayed(state, "p1")?.defId).toBe(PA.field.id);
  });

  it("R451 every record is plain data: it survives a JSON round trip and hashes the same", () => {
    let state = game("r451-json");
    state = play(state, "p1", PA.apple.id);
    state = play(state, "p1", PA.crier.id);
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(round).toEqual(state);
    expect(lastFaceUpPlayed(round, "p1")).toEqual(lastFaceUpPlayed(state, "p1"));
    expect(playedThisGameWithTag(round, "p1", "Fruit")).toBe(1);
  });
});
