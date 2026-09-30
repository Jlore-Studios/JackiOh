// Cards between the players' piles (docs/classic-sets.md B5 E16, and E2's hand and deck half): a card
// out of the other player's hand or deck becomes the taker's — owner and controller — so every pile
// it reaches later is the taker's, the taker's hand cap burns it into the taker's graveyard, and a
// draw from the other player's deck is a draw of the taker's own (Classic #9, #58, Classic+ #12.3).
// R466 is the hidden-information half: `stolen` names the card only to whoever could read it where it
// was taken or can read it now, and a card taken out of a library turns the rest of that library
// unknown to its owner, whose list (R310) would otherwise name it by what it stopped listing.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HAND_CAP, HERO_HEALTH } from "../src/config";
import { drawFromOpponent, giveFromHand, takeFromLibrary } from "../src/effects";
import { changeOwner, drawFromLibraryOf, takeIntoHand } from "../src/ownership";
import { makeContext } from "../src/resolve";
import type { Effect } from "../src/script";
import { hashState } from "../src/replay";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { draw } from "../src/draw";
import { settle } from "../src/triggers";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { effectiveCost } from "../src/mana";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import {
  castOnDrawMarker,
  commonResources,
  fluffyGrip,
  glitch,
  grunt,
  incomeTax,
  quickdrawOf,
} from "./fixtures/prompts";
import { act, answerKeys, board, castNow, expectReplays, handCard, must, openAs, replayable, roundTrip } from "./fixtures/promptHarness";

/** Apply one effect for `controller` outside any card, and record its events as an applied action. */
function run(state: GameState, effect: Effect, controller: PlayerId = "p1"): GameEvent[] {
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, null, { controller }));
  settle(sink);
  state.rngCursor = sink.rng.cursor;
  state.applied.push({ nonce: `own-${state.applied.length}`, events: sink.events });
  return sink.events;
}

/** The `stolen` events of the newest applied action, as `viewer` reads them. */
function stolenSeenBy(state: GameState, viewer: PlayerId): Extract<GameEvent, { type: "stolen" }>[] {
  return eventsOfType(viewFor(state, viewer).events, "stolen");
}

describe("E16, E2: a card out of the other player's deck (Classic+ #12.3)", () => {
  it("R466 a card taken out of the other player's deck is the taker's, reads to the taker alone, and leaves the rest of that deck unknown to its owner", () => {
    const state = board("fluffy");
    // A copy: the harness hands back the library array itself, which the steal splices.
    const library = [...setLibrary(state, "p2", [glitch.id, plain.id, glitch.id])];
    setLibrary(state, "p1", [plain.id]);
    expect(viewFor(state, "p2").you.ownLibrary?.unknown).toBe(0);
    const sink = castNow(state, fluffyGrip.id);
    state.applied.push({ nonce: "fluffy", events: sink.events });

    // The one Unit of their deck, now p1's in every sense, costing (0) in p1's hand.
    const taken = library[1] as CardInstance;
    expect(state.players.p1.hand.map((card) => card.id)).toEqual([taken.id]);
    expect(taken.owner).toBe("p1");
    expect(taken.controller).toBe("p1");
    expect(effectiveCost(state, taken)).toBe(0);
    expect(taken.knownAs).toBeUndefined();
    expect(state.players.p2.library.map((card) => card.id)).toEqual([library[0]?.id, library[2]?.id]);

    // `stolen`: the taker reads it; its old owner, who never saw their deck, reads only that a card left.
    expect(stolenSeenBy(state, "p1")).toEqual([
      { type: "stolen", instanceId: taken.id, defId: plain.id, from: "p2", to: "p1", zone: "library" },
    ]);
    expect(stolenSeenBy(state, "p2")).toEqual([
      { type: "stolen", instanceId: HIDDEN_ID, defId: HIDDEN_ID, from: "p2", to: "p1", zone: "library" },
    ]);
    expect(JSON.stringify(viewFor(state, "p2").events)).not.toContain(taken.id);
    // R466: p2's list would have named the card by what it stopped listing; the rest is unknown now.
    expect(viewFor(state, "p2").you.ownLibrary).toEqual({ cards: [], unknown: 2 });
    // The taker's own library is untouched.
    expect(viewFor(state, "p1").you.ownLibrary?.unknown).toBe(0);
    expect(viewFor(state, "p1").you.ownLibrary?.cards).toHaveLength(1);

    // Played later, it is public, and the old event reads for both (R97: judged by where it is now).
    taken.zone = { z: "graveyard", player: "p1" };
    state.players.p1.hand = [];
    state.players.p1.graveyard.push(taken);
    expect(stolenSeenBy(state, "p2")[0]?.instanceId).toBe(taken.id);
  });

  it("E16 Radiant makes the taken card Radiant; with no Unit in the deck nothing is taken", () => {
    const state = board("fluffy-radiant");
    setLibrary(state, "p2", [plain.id]);
    castNow(state, fluffyGrip.id, "p1", true);
    expect(state.players.p1.hand[0]?.radiant).toBe(true);
    expect(state.players.p1.hand[0]?.costOverride).toBe(0);

    const empty = board("fluffy-none");
    setLibrary(empty, "p2", [glitch.id, glitch.id]);
    const sink = castNow(empty, fluffyGrip.id);
    expect(empty.players.p1.hand).toEqual([]);
    expect(eventsOfType(sink.events, "stolen")).toEqual([]);
    expect(viewFor(empty, "p2").you.ownLibrary?.unknown).toBe(0);
  });

  it("E16 R218 a unit-token card never leaves a library for a hand this way", () => {
    const state = board("fluffy-token");
    setLibrary(state, "p2", ["fx-token-rush"]);
    castNow(state, fluffyGrip.id);
    expect(state.players.p1.hand).toEqual([]);
    expect(state.players.p2.library).toHaveLength(1);
  });

  it("E2 the taker's hand cap burns a taken card into the taker's graveyard, and no price rides it there", () => {
    const state = board("fluffy-burn");
    inHand(state, plain.id, "p1", HAND_CAP);
    const [card] = setLibrary(state, "p2", [plain.id]);
    const sink = castNow(state, fluffyGrip.id);
    expect(state.players.p1.graveyard.map((c) => c.id)).toEqual([card?.id]);
    expect(card?.owner).toBe("p1");
    expect(card?.costOverride).toBeUndefined();
    expect(eventsOfType(sink.events, "burned").map((event) => event.owner)).toEqual(["p1"]);
  });

  it("E16 a Fluffy Grip game replays from its log", () => {
    const qd = quickdrawOf(fluffyGrip).id;
    let { state, log, decks } = replayable("fluffy-replay", [qd]);
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", qd).id }, log);
    expect(state.players.p1.hand.some((card) => card.owner === "p1" && card.costOverride === 0)).toBe(true);
    expectReplays("fluffy-replay", decks, log, state);
  });
});

describe("E16: cards handed from one hand to the other (Classic #9)", () => {
  function taxed(seed: string, radiant = false): GameState {
    const state = board(seed);
    const trap = put(state, incomeTax.id, slot("p1", "backrow", 2));
    trap.radiant = radiant;
    inHand(state, plain.id, "p2", 2);
    inHand(state, grunt.id, "p2", 1);
    setLibrary(state, "p2", [glitch.id, plain.id]);
    state.active = "p2";
    const sink = sinkFor(state);
    draw(sink, "p2", 1);
    settle(sink);
    state.rngCursor = sink.rng.cursor;
    state.applied.push({ nonce: `${seed}-draw`, events: sink.events });
    return state;
  }

  it("E16 the other player keeps one card of their choice and the rest become the taker's", () => {
    const state = taxed("tax");
    const pending = openAs(state, "hand", "p2");
    const hand = [...state.players.p2.hand];
    expect(pending.options.map((option) => option.key)).toEqual(hand.map((card) => `instance:${card.id}`));
    expect(pending.min).toBe(1);
    expect(pending.max).toBe(1);
    // The trap's controller sees that the other player is choosing, and nothing of their hand.
    expect(viewFor(state, "p1").pending).toEqual({ forYou: false, pendingFor: "p2" });
    const kept = hand[3] as CardInstance;

    const copy = roundTrip(state);
    const { sink } = answerKeys(state, `instance:${kept.id}`);
    answerKeys(copy, `instance:${kept.id}`);
    expect(hashState(copy)).toBe(hashState(state));
    state.applied.push({ nonce: "tax-answer", events: sink.events });

    expect(state.players.p2.hand.map((card) => card.id)).toEqual([kept.id]);
    const given = hand.filter((card) => card.id !== kept.id);
    expect(state.players.p1.hand.map((card) => card.id)).toEqual(given.map((card) => card.id));
    for (const card of given) {
      expect(card.owner).toBe("p1");
      expect(card.controller).toBe("p1");
    }
    // A card out of a hand reads to the hand's holder, who held it, and to its new holder (R466).
    for (const viewer of ["p1", "p2"] as const) {
      expect(stolenSeenBy(state, viewer).map((event) => event.instanceId)).toEqual(given.map((card) => card.id));
    }
    // The spent trap reached its owner's graveyard.
    expect(state.players.p1.graveyard.map((card) => card.defId)).toEqual([incomeTax.id]);
  });

  it("E16 Radiant: the cards handed over cost (1) less in the taker's hand", () => {
    const state = taxed("tax-radiant", true);
    const kept = state.players.p2.hand[0] as CardInstance;
    answerKeys(state, `instance:${kept.id}`);
    expect(state.players.p1.hand.map((card) => card.costMod)).toEqual([-1, -1, -1]);
    expect(kept.costMod).toBe(0);
  });

  it("E16 cards may be given the other way, and at random", () => {
    const state = board("give-away");
    const mine = inHand(state, plain.id, "p1", 3);
    run(state, giveFromHand({ from: "self", cards: "random", count: 2 }));
    expect(state.players.p1.hand).toHaveLength(1);
    expect(state.players.p2.hand).toHaveLength(2);
    for (const card of state.players.p2.hand) {
      expect(mine.map((c) => c.id)).toContain(card.id);
      expect(card.owner).toBe("p2");
    }
    // p1 held them: p1 reads the `stolen` events; p2 holds them now and reads them too.
    expect(stolenSeenBy(state, "p1").every((event) => event.instanceId !== HIDDEN_ID)).toBe(true);
  });
});

describe("E16: a draw from the other player's deck (Classic #58)", () => {
  it("E16 it is a draw of the drawer's own, from the bottom of their deck", () => {
    const state = board("resources");
    const library = [...setLibrary(state, "p2", [glitch.id, grunt.id, plain.id])];
    const drawnBefore = state.counters.drawn;
    const events = run(state, drawFromOpponent());
    const bottom = library[2] as CardInstance;
    expect(state.players.p1.hand.map((card) => card.id)).toEqual([bottom.id]);
    expect(bottom.owner).toBe("p1");
    expect(state.players.p2.library.map((card) => card.id)).toEqual([library[0]?.id, library[1]?.id]);
    expect(state.counters.drawn).toBe(drawnBefore + 1);
    expect(eventsOfType(events, "drawn")).toEqual([
      { type: "drawn", player: "p1", instanceId: bottom.id, defId: plain.id },
    ]);
    // The drawer reads it; the deck's owner reads that p1 drew, never what (R466).
    expect(eventsOfType(viewFor(state, "p2").events, "drawn")[0]?.instanceId).toBe(HIDDEN_ID);
    expect(stolenSeenBy(state, "p2")[0]?.instanceId).toBe(HIDDEN_ID);
    expect(viewFor(state, "p2").you.ownLibrary).toEqual({ cards: [], unknown: 2 });
  });

  it("E16 an empty enemy deck gives nothing, and fatigue to nobody", () => {
    const state = board("resources-empty");
    state.players.p2.library = [];
    const events = run(state, drawFromOpponent());
    expect(events).toEqual([]);
    expect(state.players.p1.fatigueCount).toBe(0);
    expect(state.players.p2.fatigueCount).toBe(0);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH);
    expect(drawFromLibraryOf(sinkFor(state), "p1", "p2")).toBeNull();
  });

  it("E16 R58 a cast-on-draw card drawn from their deck is cast for the drawer", () => {
    const state = board("resources-cast");
    setLibrary(state, "p2", [plain.id, castOnDrawMarker.id]);
    run(state, drawFromOpponent());
    // The marker's Cry hits its caster's enemy: p2, whose deck it came from.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH);
    expect(state.players.p1.graveyard.map((card) => card.defId)).toEqual([castOnDrawMarker.id]);
  });

  it("E16 R4 the drawer's hand cap burns a card drawn from their deck into the drawer's graveyard", () => {
    const state = board("resources-burn");
    inHand(state, plain.id, "p1", HAND_CAP);
    const [card] = setLibrary(state, "p2", [grunt.id]);
    run(state, drawFromOpponent({ end: "top" }));
    expect(state.players.p1.graveyard.map((c) => c.id)).toEqual([card?.id]);
    expect(state.players.p2.graveyard).toEqual([]);
  });

  it("E16 Common Resources draws at its controller's start of turn", () => {
    let state = board("resources-turn");
    put(state, commonResources.id, slot("p1", "backrow", 1));
    setLibrary(state, "p1", [plain.id, plain.id]);
    const theirs = [...setLibrary(state, "p2", [plain.id, grunt.id, glitch.id])];
    state.active = "p2";
    state = act(state, { type: "endTurn", playerId: "p2" });
    expect(state.active).toBe("p1");
    // Their bottom card, and p1's own draw of the turn.
    expect(state.players.p1.hand.map((card) => card.id)).toContain(theirs[2]?.id);
    expect(state.players.p1.hand).toHaveLength(2);
    expect(state.players.p2.library).toHaveLength(2);
  });
});

describe("E2, E16: the change of owner itself", () => {
  it("E2 a card that has ceased to exist is not taken", () => {
    const state = board("gone");
    const card = newInstance(state, plain.id, "p2", { z: "gone", player: "p2" });
    const sink = sinkFor(state);
    expect(takeIntoHand(sink, card, "p1")).toBeNull();
    expect(changeOwner(sink, card, "p1")).toBe(false);
    expect(card.owner).toBe("p2");
    expect(sink.events).toEqual([]);
  });

  it("R466 a card taken off the stack or out of a public pile reads to both players", () => {
    const state = board("public");
    const resolving = newInstance(state, glitch.id, "p2", { z: "resolving", player: "p2" });
    state.players.p2.resolving.push(resolving);
    const dead = newInstance(state, plain.id, "p2", { z: "graveyard", player: "p2" });
    state.players.p2.graveyard.push(dead);
    const sink = sinkFor(state);
    takeIntoHand(sink, resolving, "p1");
    takeIntoHand(sink, dead, "p1");
    state.applied.push({ nonce: "public", events: sink.events });
    expect(state.players.p1.hand.map((card) => card.id)).toEqual([resolving.id, dead.id]);
    for (const viewer of ["p1", "p2"] as const) {
      expect(stolenSeenBy(state, viewer).map((event) => event.instanceId)).toEqual([resolving.id, dead.id]);
    }
    // And the other way round: p2 takes the top of p1's deck, and the rest of it turns unknown to p1.
    setLibrary(state, "p1", [plain.id, grunt.id]);
    const own = state.players.p1.library[0] as CardInstance;
    run(state, takeFromLibrary({ from: "enemy", pick: "top" }), "p2");
    expect(own.owner).toBe("p2");
    expect(viewFor(state, "p1").you.ownLibrary?.unknown).toBe(1);
  });
});
