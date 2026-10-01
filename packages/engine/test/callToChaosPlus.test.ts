// C+ #73 Call to Chaos (Classic+ Edition)'s table (subsystems/callToChaosPlus.ts), rolled by Core #95's
// subsystem (R28, R87, R423, R436): each of the ten entries against fixture pools, the Radiant's three
// different entries in list order, the recursion counting casts of either edition and resolving into
// nothing at the cap, and fused deck cards through JSON (R179, R468). The real card's test covers the
// same cases against the real catalog.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defOf, registerCatalog, registeredCatalog } from "../src/catalog";
import {
  CALL_TO_CHAOS_CHAIN_CAP,
  CALL_TO_CHAOS_RADIANT_EFFECTS,
  CHAOS_PLUS_BOOKS,
  CHAOS_PLUS_CLASSIC_CARDS,
  CHAOS_PLUS_DEGRADES,
  CHAOS_PLUS_FRUITS,
  CHAOS_PLUS_UPGRADES,
  HAND_CAP,
} from "../src/config";
import { effectiveCost } from "../src/mana";
import { hashState } from "../src/replay";
import { applyEffects, makeContext, type EngineSink } from "../src/resolve";
import { createRng } from "../src/rng";
import type { CardScripts, Effect, Hook } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { stateCheck } from "../src/stateCheck";
import { CHAOS_CHAIN_KEY, callToChaos, castRandomCallToChaos, chaosChainOf, rollChaosEffects } from "../src/subsystems/callToChaos";
import { CHAOS_PLUS_EFFECTS } from "../src/subsystems/callToChaosPlus";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import {
  book,
  bookToken,
  chaosPlusCatalog,
  classic,
  core95,
  fruit,
  golem,
  grape,
  hardField,
  immutable,
  plus,
  trap,
} from "./fixtures/callToChaosPlus";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

function game(seed: string, cries: { plus?: Hook; core?: Hook } = {}): GameState {
  const state = newGame(seed);
  registerCatalog(chaosPlusCatalog(registeredCatalog()));
  const scripts = (cry: Hook): CardScripts => ({ base: { cry }, radiant: { cry } });
  registerScripts({
    ...registeredScripts(),
    [plus.id]: scripts(cries.plus ?? (() => [callToChaos({ table: CHAOS_PLUS_EFFECTS })])),
    [core95.id]: scripts(cries.core ?? (() => [])),
  });
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  state.players.p1.hand = [];
  state.players.p2.hand = [];
  setLibrary(state, "p1", []);
  return state;
}

/** A C+ #73 mid-resolution, as `self` is while its Spell script runs (§10.5). */
function plusCard(state: GameState, options: { radiant?: boolean; chain?: number } = {}): CardInstance {
  const card = newInstance(state, plus.id, "p1", { z: "resolving", player: "p1" });
  if (options.radiant === true) card.radiant = true;
  if (options.chain !== undefined) card.memory[CHAOS_CHAIN_KEY] = options.chain;
  return card;
}

function entry(name: string): Effect {
  const found = CHAOS_PLUS_EFFECTS.find((effect) => effect.name === name);
  if (found === undefined) throw new Error(`no entry ${name}`);
  return found.build();
}

function run(state: GameState, effect: Effect, self: CardInstance | null = plusCard(state), controller: PlayerId = "p1"): GameEvent[] {
  const events: GameEvent[] = [];
  const sink: EngineSink = sinkFor(state, events);
  applyEffects([effect], makeContext(sink, self, { controller }));
  stateCheck(sink);
  state.rngCursor = sink.rng.cursor;
  return events;
}

/** A seed whose roll (base or Radiant) the predicate accepts. */
function seedWhere(radiant: boolean, accept: (names: string[]) => boolean, tag: string): string {
  for (let i = 0; i < 4000; i += 1) {
    const seed = `${tag}-${i}`;
    if (accept(rollChaosEffects(createRng(seed), radiant, CHAOS_PLUS_EFFECTS).map((effect) => effect.name))) return seed;
  }
  throw new Error(`no seed for ${tag}`);
}

describe("C+ #73's table (R423)", () => {
  it("is the ten entries of §8.7 row 73, in the card's order, each with its printed clause", () => {
    expect(CHAOS_PLUS_EFFECTS.map((effect) => effect.name)).toEqual([
      "fruits",
      "books",
      "destroy",
      "classic",
      "upgrade",
      "fuse",
      "degrade",
      "golem",
      "replace",
      "recast",
    ]);
    expect(CHAOS_PLUS_EFFECTS.map((effect) => effect.label)).toEqual([
      "Add 5 random Fruits to your hand, which cost (0)",
      "Add 3 random Books to your hand, which cost (0)",
      "Destroy all enemy permanents",
      "Add 3 random Classic cards to your hand, which cost (0)",
      "Upgrade every card in your hand and deck twice",
      "Fuse a random card into each card in your deck, each keeping its cost",
      "Degrade every card on your opponent's field and in their hand three times",
      "Summon a Classic Golem",
      "Replace your deck with random Call to Chaos cards, which cost (0)",
      "Cast a random Call to Chaos",
    ]);
  });

  it("R382 entry 1: 5 Fruits that cost (0), the Grapes in the pool, repeats allowed (R60)", () => {
    const seen = new Set<string>();
    for (let i = 0; i < 6; i += 1) {
      const state = game(`fruits-${i}`);
      run(state, entry("fruits"));
      const hand = state.players.p1.hand;
      expect(hand).toHaveLength(CHAOS_PLUS_FRUITS);
      for (const card of hand) {
        expect([fruit.id, grape.id]).toContain(card.defId);
        expect(card.costOverride).toBe(0);
        seen.add(card.defId);
      }
    }
    expect(seen).toEqual(new Set([fruit.id, grape.id]));
  });

  it("§2.4 R4 a full hand burns what entry 1 cannot fit", () => {
    const state = game("fruits-full");
    inHand(state, "fx-1", "p1", HAND_CAP - 2);
    const events = run(state, entry("fruits"));
    expect(state.players.p1.hand).toHaveLength(HAND_CAP);
    expect(eventsOfType(events, "burned")).toHaveLength(CHAOS_PLUS_FRUITS - 2);
  });

  it("R380 entry 2: 3 non-token Books of any set, which cost (0)", () => {
    const state = game("books");
    run(state, entry("books"));
    expect(state.players.p1.hand.map((card) => card.defId)).toEqual(Array.from({ length: CHAOS_PLUS_BOOKS }, () => book.id));
    expect(state.players.p1.hand.some((card) => card.defId === bookToken.id)).toBe(false);
    expect(state.players.p1.hand.every((card) => effectiveCost(state, card) === 0)).toBe(true);
  });

  it("R46 R59 entry 3: every enemy permanent is destroyed — the top of each pile, face-down cards too — Indestructible ones staying", () => {
    const state = game("destroy");
    const unit = put(state, "fx-2", slot("p2", "units", 1));
    const faceDown = put(state, trap.id, slot("p2", "backrow", 1));
    const hard = put(state, hardField.id, slot("p2", "backrow", 2));
    const mine = put(state, "fx-3", slot("p1", "units", 1));
    run(state, entry("destroy"));
    expect(faceDown.faceUp === true).toBe(false);
    expect([unit.zone.z, faceDown.zone.z, hard.zone.z, mine.zone.z]).toEqual(["graveyard", "graveyard", "field", "field"]);
  });

  it("R380 entry 4: 3 non-token cards of the Classic set only, which cost (0)", () => {
    const state = game("classic");
    run(state, entry("classic"));
    const hand = state.players.p1.hand;
    expect(hand).toHaveLength(CHAOS_PLUS_CLASSIC_CARDS);
    for (const card of hand) {
      expect(defOf(state, card.defId).set).toBe("Classic");
      expect(defOf(state, card.defId).token).toBe(false);
      expect(card.costOverride).toBe(0);
    }
  });

  it("R386 entry 5: two separate Upgrades of each card in your hand and your deck, none of the opponent's", () => {
    const state = game("upgrade");
    const hand = inHand(state, classic.id, "p1", 2);
    const deck = setLibrary(state, "p1", [classic.id, classic.id]);
    const theirs = inHand(state, classic.id, "p2", 1);
    const events = run(state, entry("upgrade"));
    expect(eventsOfType(events, "upgraded")).toHaveLength((hand.length + deck.length) * CHAOS_PLUS_UPGRADES);
    expect(theirs[0]?.tuning).toBeUndefined();
    expect([...hand, ...deck].every((card) => card.tuning !== undefined || card.costMod !== 0)).toBe(true);
  });

  it("R311 R177 entry 5's deck Upgrades stay unread by the deck's owner too", () => {
    const state = game("upgrade-hidden");
    setLibrary(state, "p1", [classic.id]);
    const events = run(state, entry("upgrade"));
    state.applied = [{ nonce: "upgrade-hidden", events }];
    for (const viewer of ["p1", "p2"] as const) {
      const upgrades = viewFor(state, viewer).events.filter((event) => event.type === "upgraded");
      expect(upgrades.length).toBeGreaterThan(0);
      expect(upgrades.every((event) => event.type === "upgraded" && event.instanceId === HIDDEN_ID)).toBe(true);
    }
  });

  it("R77 R470 R387 entry 6: a random card fused into each deck card, which keeps its cost; never this card; Immutable skipped (R23)", () => {
    const state = game("fuse");
    const deck = [...setLibrary(state, "p1", [classic.id, "fx-1", immutable.id])];
    const events = run(state, entry("fuse"));
    const fused = eventsOfType(events, "fused");
    expect(fused).toHaveLength(2);
    const [first, second, third] = state.players.p1.library;
    expect(first?.id).toBe(deck[0]?.id);
    expect(defOf(state, first?.defId ?? "").type).toBe("Unit");
    expect(effectiveCost(state, first as CardInstance)).toBe(3);
    expect(effectiveCost(state, second as CardInstance)).toBe(1);
    expect(third?.defId).toBe(immutable.id);
    for (const card of [first, second]) {
      expect(state.transientDefs[card?.defId ?? ""]).toBeDefined();
      expect(defOf(state, card?.defId ?? "").ingredients?.some((part) => part.defId === plus.id)).toBe(false);
    }
  });

  it("R179 R468 entry 6's fused deck cards survive JSON", () => {
    const state = game("fuse-json");
    setLibrary(state, "p1", [classic.id, "fx-1", "fx-2"]);
    run(state, entry("fuse"));
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(round).toEqual(state);
    expect(hashState(round)).toBe(hashState(state));
    expect(round.players.p1.library.every((card) => round.transientDefs[card.defId] !== undefined)).toBe(true);
  });

  it("R386 entry 7: three separate Degrades of each card on the opponent's field and in their hand, none of yours", () => {
    const state = game("degrade");
    const field = put(state, classic.id, slot("p2", "units", 1));
    const hand = inHand(state, classic.id, "p2", 1);
    const mine = put(state, classic.id, slot("p1", "units", 1));
    const deck = setLibrary(state, "p2", [classic.id]);
    const events = run(state, entry("degrade"));
    // Up to three each: a Degrade with nothing left it can change is no Degrade (B3.4 rule 1).
    const degraded = eventsOfType(events, "degraded").map((event) => event.instanceId);
    for (const id of [field.id, hand[0]?.id]) {
      const times = degraded.filter((each) => each === id).length;
      expect(times).toBeGreaterThan(0);
      expect(times).toBeLessThanOrEqual(CHAOS_PLUS_DEGRADES);
    }
    expect(new Set(degraded)).toEqual(new Set([field.id, hand[0]?.id]));
    expect(mine.tuning).toBeUndefined();
    expect(deck[0]?.tuning).toBeUndefined();
    expect(field.tuning !== undefined || field.costMod !== 0).toBe(true);
    expect(hand[0]?.tuning !== undefined || hand[0]?.costMod !== 0).toBe(true);
  });

  it("R64 entry 8: a Classic Golem, summoned into the leftmost free zone", () => {
    const state = game("golem");
    put(state, "fx-1", slot("p1", "units", 1));
    const events = run(state, entry("golem"));
    expect(eventsOfType(events, "summoned")).toMatchObject([{ defId: golem.id, lane: 2 }]);
  });

  it("R35 R311 R387 entry 9: each deck card replaced one for one by a Call to Chaos of either edition, which costs (0)", () => {
    const seen = new Set<string>();
    for (let i = 0; i < 4; i += 1) {
      const state = game(`replace-${i}`);
      // A copy: `setLibrary` hands back the library array itself.
      const old = [...setLibrary(state, "p1", ["fx-1", "fx-2", "fx-3", "fx-4", "fx-5"])];
      const events = run(state, entry("replace"));
      expect(eventsOfType(events, "transformed")).toHaveLength(old.length);
      const deck = state.players.p1.library;
      expect(deck).toHaveLength(old.length);
      for (const card of deck) {
        expect([plus.id, core95.id]).toContain(card.defId);
        expect(card.costOverride).toBe(0);
        expect(card.knownAs).toBeUndefined();
        seen.add(card.defId);
      }
      expect(old.every((card) => card.zone.z === "gone" && findInstance(state, card.id) === undefined)).toBe(true);
    }
    expect(seen).toEqual(new Set([plus.id, core95.id]));
  });

  it("R129 entry 9 on an empty deck changes nothing and draws nothing", () => {
    const state = game("replace-empty");
    const self = plusCard(state);
    const events = run(state, entry("replace"), self);
    expect(events).toEqual([]);
    expect(state.rngCursor).toBe(0);
  });

  it("R28 R87 entry 10: the chain counts casts of either edition against the cap", () => {
    const state = game("chain", { plus: () => [castRandomCallToChaos()], core: () => [castRandomCallToChaos()] });
    const events = run(state, entry("recast"));
    const played = eventsOfType(events, "cardPlayed");
    expect(played).toHaveLength(CALL_TO_CHAOS_CHAIN_CAP);
    expect(new Set(played.map((event) => event.defId))).toEqual(new Set([plus.id, core95.id]));
  });

  it("R87 a recursion rolled at the cap resolves into nothing; the Radiant's other two still run", () => {
    const seed = seedWhere(true, (names) => names.includes("recast") && names.includes("golem"), "cap");
    const state = game(seed);
    const self = plusCard(state, { radiant: true, chain: CALL_TO_CHAOS_CHAIN_CAP });
    const events = run(state, callToChaos({ table: CHAOS_PLUS_EFFECTS }), self);
    expect(eventsOfType(events, "cardPlayed")).toEqual([]);
    expect(eventsOfType(events, "summoned").map((event) => event.defId)).toEqual([golem.id]);
    expect(chaosChainOf(self)).toBe(CALL_TO_CHAOS_CHAIN_CAP);
  });

  it("R423 the base face rolls one entry, and all ten are reachable", () => {
    const reached = new Set<string>();
    for (let i = 0; i < 200; i += 1) {
      const rolled = rollChaosEffects(createRng(`plus-base-${i}`), false, CHAOS_PLUS_EFFECTS);
      expect(rolled).toHaveLength(1);
      reached.add(rolled[0]?.name ?? "");
    }
    expect(reached.size).toBe(CHAOS_PLUS_EFFECTS.length);
  });

  it("R423 the Radiant face rolls three different entries, resolved in the list's order", () => {
    for (let i = 0; i < 200; i += 1) {
      const names = rollChaosEffects(createRng(`plus-radiant-${i}`), true, CHAOS_PLUS_EFFECTS).map((effect) => effect.name);
      expect(new Set(names).size).toBe(CALL_TO_CHAOS_RADIANT_EFFECTS);
      const order = names.map((name) => CHAOS_PLUS_EFFECTS.findIndex((effect) => effect.name === name));
      expect(order).toEqual([...order].sort((a, b) => a - b));
    }
    const seed = seedWhere(true, (names) => names.join() === ["fruits", "golem", "recast"].join(), "order");
    const state = game(seed);
    const events = run(state, callToChaos({ table: CHAOS_PLUS_EFFECTS }), plusCard(state, { radiant: true }));
    const types = events.map((event) => event.type);
    expect(types.indexOf("addedToHand")).toBeLessThan(types.indexOf("summoned"));
    expect(types.indexOf("summoned")).toBeLessThan(types.indexOf("cardPlayed"));
  });

  it("R436 both players are told the rolled clauses, by this card, before any of it resolves", () => {
    const seed = seedWhere(true, (names) => !names.includes("recast"), "announce");
    const state = game(seed);
    const self = plusCard(state, { radiant: true });
    const events = run(state, callToChaos({ table: CHAOS_PLUS_EFFECTS }), self);
    const expected = rollChaosEffects(createRng(seed), true, CHAOS_PLUS_EFFECTS).map((effect) => effect.label);
    expect(events[0]).toEqual({ type: "chaosRolled", player: "p1", instanceId: self.id, defId: plus.id, effects: expected });
  });
});
