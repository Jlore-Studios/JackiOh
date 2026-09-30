// Deck and graveyard triggers, and "summon this from your hand or deck" (docs/classic-sets.md B5 E26):
// a card in a library answers its `deckTriggers` (Classic+ #37 Wardrum), a card in a graveyard its
// `graveyardTriggers` (Classic #47 Recurring Felinor), and a hand or deck trigger may summon its own
// card — no Cry, summoning sick, the leftmost open zone (Classic #66 EU Striker). R464 is their place
// in R68's order: after the hand's triggers and before the graveyard's, in the order the instances
// were created and never by library position, which is hidden (§9.1); and a library card's queue
// entry, like a hand card's, takes no number from the counter both seats read (R177).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HERO_HEALTH } from "../src/config";
import { hashState } from "../src/replay";
import type { Script } from "../src/script";
import { scriptsFor } from "../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { fuse } from "../src/subsystems/fuse";
import { cardsInTriggerOrder, dispatchEvent, triggerHoldersForEvent } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";
import {
  deckAsker,
  deckWatcher,
  graveWatcher,
  grunt,
  quickdrawOf,
  recurring,
  snare,
  spark,
  striker,
  wardrum,
} from "./fixtures/prompts";
import { act, answerKeys, board, expectReplays, handCard, openAs, replayable, roundTrip } from "./fixtures/promptHarness";

function played(state: GameState, player: "p1" | "p2", defId: string): GameEvent {
  return { type: "cardPlayed", player, instanceId: `c${state.nextId + 1000}`, defId, costPaid: 0 };
}

function resolved(state: GameState, player: "p1" | "p2", defId: string): GameEvent {
  return { type: "cardResolved", player, instanceId: `c${state.nextId + 1000}`, defId, costPaid: 0, permanent: false };
}

function graveCard(state: GameState, player: "p1" | "p2", defId: string): CardInstance {
  const card = newInstance(state, defId, player, { z: "graveyard", player });
  state.players[player].graveyard.push(card);
  return card;
}

describe("E26: deck and graveyard triggers in R68's order (R464)", () => {
  it("R464 deck triggers come after the hand's and before the graveyard's, in creation order, never library order", () => {
    const state = board("r464-order");
    const hand = inHand(state, striker.id, "p1")[0] as CardInstance;
    // Created first, then second — and laid in the library the other way round.
    const older = newInstance(state, deckWatcher.id, "p1", { z: "library", player: "p1" });
    const newer = newInstance(state, wardrum.id, "p1", { z: "library", player: "p1" });
    const filler = newInstance(state, plain.id, "p1", { z: "library", player: "p1" });
    state.players.p1.library = [newer, filler, older];
    const grave = graveCard(state, "p1", graveWatcher.id);
    const enemyDeck = newInstance(state, deckWatcher.id, "p2", { z: "library", player: "p2" });
    state.players.p2.library = [enemyDeck];

    // The registry: a library card is a holder only when it declares deck triggers.
    const order = cardsInTriggerOrder(state).map((holder) => holder.card.id);
    expect(order.indexOf(hand.id)).toBeLessThan(order.indexOf(older.id));
    expect(order.indexOf(older.id)).toBeLessThan(order.indexOf(newer.id));
    expect(order.indexOf(newer.id)).toBeLessThan(order.indexOf(grave.id));
    expect(order).not.toContain(filler.id);
    // Active side first: the other player's deck card comes after all of p1's.
    expect(order.indexOf(enemyDeck.id)).toBeGreaterThan(order.indexOf(grave.id));
    expect(triggerHoldersForEvent(state, "cardResolved").map((holder) => holder.zone)).toEqual([
      "hand",
      "library",
      "library",
      "graveyard",
      "library",
    ]);

    // The queue follows it.
    const sink = sinkFor(state);
    dispatchEvent(sink, resolved(state, "p1", grunt.id));
    expect(state.triggerQueue.map((entry) => entry.instanceId)).toEqual([hand.id, older.id, newer.id, grave.id, enemyDeck.id]);
  });

  it("R464 R177 a library card's queue entry takes no number from the counter both seats read", () => {
    const state = board("r464-hidden");
    const hidden = newInstance(state, deckWatcher.id, "p1", { z: "library", player: "p1" });
    state.players.p1.library = [hidden];
    const grave = graveCard(state, "p1", graveWatcher.id);
    const before = state.nextSeq;
    const sink = sinkFor(state);
    dispatchEvent(sink, played(state, "p2", grunt.id));
    const [deckEntry, graveEntry] = state.triggerQueue;
    expect(deckEntry?.instanceId).toBe(hidden.id);
    expect(deckEntry?.id.startsWith("h")).toBe(true);
    // The public graveyard card's entry is numbered; the hidden one's took none.
    expect(graveEntry?.instanceId).toBe(grave.id);
    expect(state.nextSeq).toBe(before + 1);
  });
});

describe("E26: summon this from your hand or deck", () => {
  it("E26 a deck trigger summons its card: no Cry, summoning sick, the leftmost open zone", () => {
    let state = board("deck-summon");
    put(state, plain.id, slot("p1", "units", 1));
    const drum = newInstance(state, wardrum.id, "p1", { z: "library", player: "p1" });
    state.players.p1.library = [newInstance(state, plain.id, "p1", { z: "library", player: "p1" }), drum];
    const card = inHand(state, spark.id, "p1")[0] as CardInstance;
    state = act(state, { type: "play", playerId: "p1", instanceId: card.id });
    const found = state.players.p1.units[1]?.[0];
    expect(found?.id).toBe(drum.id);
    expect(found?.summonedTurn).toBe(state.turn);
    expect(state.players.p1.library).toHaveLength(1);
    const events = state.applied.at(-1)?.events ?? [];
    expect(eventsOfType(events, "summoned").map((event) => event.instanceId)).toEqual([drum.id]);
    // A summon, not a play: no cardPlayed of its own.
    expect(eventsOfType(events, "cardPlayed").map((event) => event.instanceId)).toEqual([card.id]);
    // The deck card is public once it is on the field, and not before.
    expect(eventsOfType(viewFor(state, "p2").events, "summoned")[0]?.defId).toBe(wardrum.id);
  });

  it("E26 a hand trigger summons its card after a Unit is played, never on its own play, and fires no Cry", () => {
    let state = board("hand-summon");
    const eu = inHand(state, striker.id, "p1")[0] as CardInstance;
    const body = inHand(state, grunt.id, "p1")[0] as CardInstance;
    const zap = inHand(state, spark.id, "p1")[0] as CardInstance;
    // A Spell sets nothing off.
    state = act(state, { type: "play", playerId: "p1", instanceId: zap.id });
    expect(state.players.p1.hand.map((card) => card.id)).toContain(eu.id);
    state = act(state, { type: "play", playerId: "p1", instanceId: body.id, zone: { row: "units", lane: 1 } });
    expect(state.players.p1.units[1]?.[0]?.id).toBe(eu.id);
    // Its Cry (9 to the enemy hero) did not fire: only the spark's 1 landed.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
  });

  it("E26 with no open zone the card stays where it is", () => {
    let state = board("deck-full");
    for (let lane = 1; lane <= 5; lane += 1) put(state, plain.id, slot("p1", "units", lane));
    const drum = newInstance(state, wardrum.id, "p1", { z: "library", player: "p1" });
    state.players.p1.library = [drum];
    const card = inHand(state, spark.id, "p1")[0] as CardInstance;
    state = act(state, { type: "play", playerId: "p1", instanceId: card.id });
    expect(state.players.p1.library.map((c) => c.id)).toEqual([drum.id]);
    expect(eventsOfType(state.applied.at(-1)?.events ?? [], "summoned")).toEqual([]);
  });

  it("E26 R113 a deck trigger that asks parks its tail under its id, and the answer finishes it", () => {
    let state = board("deck-asks");
    const asker = newInstance(state, deckAsker.id, "p1", { z: "library", player: "p1" });
    state.players.p1.library = [asker];
    const card = inHand(state, spark.id, "p1")[0] as CardInstance;
    state = act(state, { type: "play", playerId: "p1", instanceId: card.id });
    openAs(state, "mode", "p1");
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    // The Spell resolved (its 1 landed) before the trigger answered it; the tail's 1 waits.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    const copy = roundTrip(state);
    answerKeys(state, "mode:come");
    answerKeys(copy, "mode:come");
    expect(hashState(copy)).toBe(hashState(state));
    expect(state.players.p1.units[0]?.[0]?.id).toBe(asker.id);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
  });

  it("E26 a Wardrum game replays from its log", () => {
    const zap = quickdrawOf(spark).id;
    const { state: dealt, log, decks } = replayable("wardrum-replay", [zap, wardrum.id]);
    let state = dealt;
    const drum = [...state.players.p1.hand, ...state.players.p1.library].find((card) => card.defId === wardrum.id);
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", zap).id }, log);
    // From the hand or the deck, wherever the deal left it, it answered the Spell.
    expect(state.players.p1.units[0]?.[0]?.id).toBe(drum?.id);
    expectReplays("wardrum-replay", decks, log, state);
  });
});

describe("E26: graveyard triggers (Classic #47)", () => {
  function trapFires(seed: string, radiant: boolean): { state: GameState; card: CardInstance } {
    let state = board(seed);
    const card = graveCard(state, "p1", recurring.id);
    card.radiant = radiant;
    put(state, snare.id, slot("p1", "backrow", 1));
    state.active = "p2";
    const zap = inHand(state, spark.id, "p2")[0] as CardInstance;
    state = act(state, { type: "play", playerId: "p2", instanceId: zap.id });
    const moved = state.players.p1.hand.find((c) => c.id === card.id) ?? card;
    return { state, card: moved };
  }

  it("E26 when one of your Traps fires, the card in your graveyard returns to your hand", () => {
    const { state, card } = trapFires("recur", false);
    // (p2's play ended their turn by itself, R82, so p1 has drawn for the new turn as well.)
    expect(state.players.p1.hand.map((c) => c.id)).toContain(card.id);
    expect(card.zone.z).toBe("hand");
    expect(state.players.p1.graveyard.map((c) => c.defId)).toEqual([snare.id]);
    expect(card.costOverride).toBeUndefined();
  });

  it("E26 Radiant: it returns costing (0)", () => {
    const { state, card } = trapFires("recur-radiant", true);
    expect(state.players.p1.hand.map((c) => c.id)).toContain(card.id);
    expect(card.costOverride).toBe(0);
  });

  it("E26 the other player's Trap does not return it, and a card elsewhere has no graveyard trigger", () => {
    let state = board("recur-theirs");
    const card = graveCard(state, "p1", recurring.id);
    put(state, snare.id, slot("p2", "backrow", 1));
    const zap = inHand(state, spark.id, "p1")[0] as CardInstance;
    state = act(state, { type: "play", playerId: "p1", instanceId: zap.id });
    expect(eventsOfType(state.applied.at(-1)?.events ?? [], "trapFired")).toHaveLength(1);
    expect(state.players.p1.graveyard.map((c) => c.id)).toContain(card.id);

    // In a hand the graveyard trigger is not registered.
    const held = board("recur-hand");
    const inHandCard = inHand(held, recurring.id, "p1")[0] as CardInstance;
    expect(triggerHoldersForEvent(held, "trapFired").map((holder) => holder.card.id)).not.toContain(inHandCard.id);
  });
});

describe("E26: a fused card keeps its deck and graveyard triggers (R102)", () => {
  it("R102 a fusion's script carries each ingredient's deck and graveyard triggers, namespaced", () => {
    const state = board("fuse-triggers");
    const a = inHand(state, wardrum.id, "p1")[0] as CardInstance;
    const b = inHand(state, recurring.id, "p1")[0] as CardInstance;
    const sink = sinkFor(state);
    const fused = fuse(sink, { ingredients: [a, b], toHand: "p1" });
    const script: Script = scriptsFor(fused?.defId ?? "").base;
    expect(script.deckTriggers?.map((trigger) => trigger.id)).toEqual([`${wardrum.id}:wardrum-arrive`]);
    expect(script.graveyardTriggers?.map((trigger) => trigger.id)).toEqual([`${recurring.id}:recur`]);
  });
});
