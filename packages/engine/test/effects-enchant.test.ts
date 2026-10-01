// Enchantments that ride a card (docs/classic-sets.md B5 E39; R443): `enchant` on a named card or a
// scope, never twice the same, kept in every zone and through leaving the field (R78's reset leaves
// them alone), carried by a copy and a Fuse, gone with a Transform, and shown where the card is read.

import type { Enchantment, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { enchant, summonCopy, shuffleCopiesOfSelf, transform } from "../src/effects";
import { addEnchantment, enchantmentsOf, enchantmentsOfKind, hasEnchantment, unitedEnchantments } from "../src/enchantments";
import { makeContext, type HookOptions } from "../src/resolve";
import type { Effect } from "../src/script";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { fuse } from "../src/subsystems/fuse";
import { viewFor } from "../src/viewFor";
import { moveToZone, placeOnField } from "../src/zones";
import { plain } from "./fixtures/combat";
import { inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { body, echoBolt, instanceGame } from "./fixtures/instanceData";

const RETURN: Enchantment = { kind: "returnAfterResolve", floor: 2 };
const CAST: Enchantment = { kind: "castOnDraw" };
const ENEMIES: Enchantment = { kind: "targetEnemies" };

function game(): GameState {
  const state = instanceGame("enchant");
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

function run(state: GameState, effect: Effect, options: HookOptions & { self?: CardInstance | null } = {}): GameEvent[] {
  const { self = null, ...hook } = options;
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, self, { controller: "p1", ...hook }));
  return sink.events;
}

describe("B5 E39 enchant (R443)", () => {
  it("R443 enchant puts an enchantment on a named card anywhere, never twice the same one, and reports nothing", () => {
    const state = game();
    const [deckCard] = setLibrary(state, "p1", [echoBolt.id]);
    if (deckCard === undefined) throw new Error("no card");
    expect(run(state, enchant({ instanceId: deckCard.id, enchantment: CAST }))).toEqual([]);
    run(state, enchant({ instanceId: deckCard.id, enchantment: CAST }));
    run(state, enchant({ instanceId: deckCard.id, enchantment: ENEMIES }));
    expect(enchantmentsOf(deckCard)).toEqual([CAST, ENEMIES]);
    expect(hasEnchantment(deckCard, "targetEnemies")).toBe(true);
    // Two of a kind that differ are both kept, for their reader to combine.
    expect(addEnchantment(deckCard, RETURN)).toBe(true);
    expect(addEnchantment(deckCard, { kind: "returnAfterResolve", floor: 1 })).toBe(true);
    expect(addEnchantment(deckCard, RETURN)).toBe(false);
    expect(enchantmentsOfKind(deckCard, "returnAfterResolve")).toEqual([RETURN, { kind: "returnAfterResolve", floor: 1 }]);
  });

  it("R443 enchant over a scope reaches every card of it, hand and deck included", () => {
    const state = game();
    const hand = inHand(state, echoBolt.id, "p1", 2);
    const deck = setLibrary(state, "p1", [echoBolt.id, plain.id]);
    run(state, enchant({ scope: { zones: ["hand", "library"], types: ["Spell"] }, enchantment: CAST }));
    expect(hand.map((card) => enchantmentsOf(card))).toEqual([[CAST], [CAST]]);
    expect(deck.map((card) => enchantmentsOf(card))).toEqual([[CAST], []]);
  });

  it("R443 an enchantment rides the card through every zone and through leaving the field (R78, R215)", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    run(state, enchant({ instanceId: unit.id, enchantment: RETURN }));
    moveToZone(state, unit, "hand");
    expect(enchantmentsOf(unit)).toEqual([RETURN]);
    moveToZone(state, unit, "graveyard");
    moveToZone(state, unit, "library");
    moveToZone(state, unit, "exile");
    expect(enchantmentsOf(unit)).toEqual([RETURN]);
    state.players.p1.exile = [];
    expect(placeOnField(state, unit, slot("p1", "units", 2))).toBe(true);
    expect(enchantmentsOf(unit)).toEqual([RETURN]);
  });

  it("R443 a copy carries its source's enchantments, a Fuse unites its ingredients', a Transform makes a card without them", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    run(state, enchant({ instanceId: unit.id, enchantment: ENEMIES }));
    run(state, summonCopy({ of: { of: "instance", instanceId: unit.id } }));
    expect(enchantmentsOf(state.players.p1.units[1]?.[0] ?? unit)).toEqual([ENEMIES]);

    const spell = newInstance(state, echoBolt.id, "p1", { z: "resolving", player: "p1" });
    state.players.p1.resolving.push(spell);
    run(state, enchant({ instanceId: spell.id, enchantment: CAST }));
    run(state, shuffleCopiesOfSelf({ count: 1 }), { self: spell });
    expect(state.players.p1.library.filter((c) => c.defId === echoBolt.id).map(enchantmentsOf)).toEqual([[CAST]]);

    const other = put(state, plain.id, slot("p1", "units", 3));
    run(state, enchant({ instanceId: other.id, enchantment: CAST }));
    run(state, enchant({ instanceId: other.id, enchantment: ENEMIES }));
    fuse(sinkFor(state), { ingredients: [other], target: unit });
    // In ingredient order, the target last (R77), each once.
    expect(enchantmentsOf(unit)).toEqual([CAST, ENEMIES]);
    expect(unitedEnchantments([{ enchantments: [CAST] }, {}])).toEqual([CAST]);
    expect(unitedEnchantments([{}, {}])).toBeUndefined();

    run(state, transform({ instanceId: unit.id, defId: plain.id }));
    expect(enchantmentsOf(state.players.p1.units[0]?.[0] ?? unit)).toEqual([]);
  });

  it("R443 the view shows the enchantments where the card is read, and the other player never reads a hand card's", () => {
    const state = game();
    const [held] = inHand(state, echoBolt.id, "p1");
    if (held === undefined) throw new Error("no card");
    run(state, enchant({ instanceId: held.id, enchantment: RETURN }));
    const hand = viewFor(state, "p1").you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand.find((c) => c.instanceId === held.id)?.enchantments).toEqual([RETURN]);
    expect(JSON.stringify(viewFor(state, "p2"))).not.toContain("returnAfterResolve");
  });
});
