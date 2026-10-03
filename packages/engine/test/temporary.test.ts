// Temporary (SPEC §6.1, R637): a card that is discarded from its owner's hand at the end of their turn.
// The card keyword, not temporary mana (§2.3).

import type { Action, ActionInput, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { isTemporaryCard } from "../src/temporary";
import { beginGame, reduce } from "../src/reduce";
import type { GameState } from "../src/state";
import { plain, temporaryBody } from "./fixtures/combat";
import { eventsOfType, inHand, newGame, setLibrary } from "./fixtures/harness";

let nonce = 0;
function attempt(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `tp${nonce}` } as Action);
}

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  const result = attempt(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past the mulligans, in the main phase of turn 1. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" }).state;
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" }).state;
  return state;
}

describe("Temporary (R637)", () => {
  it("R637 each Temporary card in the ending player's hand is discarded at the end of their turn, in hand order", () => {
    const state = playing("temporary-end");
    const [first, second] = inHand(state, temporaryBody.id, "p1", 2);
    const [kept] = inHand(state, plain.id, "p1");
    if (first === undefined || second === undefined || kept === undefined) throw new Error("no cards");
    const handBefore = state.players.p1.hand.length;

    const { state: after, events } = act(state, { type: "endTurn", playerId: "p1" });

    expect(after.players.p1.graveyard.map((card) => card.id)).toEqual(expect.arrayContaining([first.id, second.id]));
    expect(after.players.p1.hand.map((card) => card.id)).toContain(kept.id);
    expect(after.players.p1.hand).toHaveLength(handBefore - 2);
    // A discard (§6.3): one `discarded` event each, in hand order, so "whenever you discard" sees them.
    expect(eventsOfType(events, "discarded").map((event) => event.instanceId)).toEqual([first.id, second.id]);
  });

  it("R637 it waits for its owner's own turn end: a Temporary card in the other player's hand stays", () => {
    const state = playing("temporary-other-hand");
    const [theirs] = inHand(state, temporaryBody.id, "p2");
    if (theirs === undefined) throw new Error("no card");

    const afterMine = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(afterMine.players.p2.hand.map((card) => card.id)).toContain(theirs.id);

    const afterTheirs = act(afterMine, { type: "endTurn", playerId: "p2" });
    expect(afterTheirs.state.players.p2.graveyard.map((card) => card.id)).toContain(theirs.id);
    expect(eventsOfType(afterTheirs.events, "discarded").map((event) => event.instanceId)).toEqual([theirs.id]);
  });

  it("R637 a Temporary card played before the end of the turn escapes, and it does nothing on the field", () => {
    const state = playing("temporary-played");
    const [card] = inHand(state, temporaryBody.id, "p1");
    if (card === undefined) throw new Error("no card");

    let next = act(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" }).state;
    expect(next.players.p1.units[0]?.[0]?.id).toBe(card.id);
    next = act(next, { type: "endTurn", playerId: "p1" }).state;
    next = act(next, { type: "endTurn", playerId: "p2" }).state;
    expect(next.players.p1.units[0]?.[0]?.id).toBe(card.id);
    expect(next.players.p1.graveyard.map((c) => c.id)).not.toContain(card.id);
  });

  it("R637 it does nothing in a deck: a Temporary card is drawn like any other", () => {
    const state = playing("temporary-deck");
    const [deckCard] = setLibrary(state, "p1", [temporaryBody.id, plain.id]);
    if (deckCard === undefined) throw new Error("no card");
    expect(isTemporaryCard(state, deckCard)).toBe(true);
    const after = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(after.players.p1.library.map((card) => card.id)).toContain(deckCard.id);
    expect(after.players.p1.graveyard.map((card) => card.id)).not.toContain(deckCard.id);
  });

  it("R637 a granted Temporary counts, and a Vanilla card loses a printed one but keeps a given one (§10.4)", () => {
    const state = playing("temporary-granted");
    const [granted] = inHand(state, plain.id, "p1");
    const [printedVanilla] = inHand(state, temporaryBody.id, "p1");
    const [givenVanilla] = inHand(state, temporaryBody.id, "p1");
    if (granted === undefined || printedVanilla === undefined || givenVanilla === undefined) throw new Error("no cards");
    expect(isTemporaryCard(state, granted)).toBe(false);
    granted.grantedKeywords.push({ kind: "Temporary" });
    printedVanilla.vanilla = true;
    givenVanilla.vanilla = true;
    givenVanilla.grantedKeywords.push({ kind: "Temporary" });

    expect(isTemporaryCard(state, granted)).toBe(true);
    expect(isTemporaryCard(state, printedVanilla)).toBe(false);
    expect(isTemporaryCard(state, givenVanilla)).toBe(true);

    const after = act(state, { type: "endTurn", playerId: "p1" }).state;
    const inGraveyard = after.players.p1.graveyard.map((card) => card.id);
    expect(inGraveyard).toEqual(expect.arrayContaining([granted.id, givenVanilla.id]));
    expect(inGraveyard).not.toContain(printedVanilla.id);
    expect(after.players.p1.hand.map((card) => card.id)).toContain(printedVanilla.id);
  });
});
