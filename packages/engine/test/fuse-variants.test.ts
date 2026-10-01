// The Fuse variants of patch v0.2.0 (docs/classic-sets.md B5 E23; R77, R102, R179) and the three
// rulings they needed: R468 (a fused id is bounded: past `FUSED_ID_CAP` it is a digest of its
// ingredient list, which the definition keeps), R469 (an ingredient fused "as a Radiant card" puts
// its Radiant face into both fused forms) and R470 (a fusion may keep a hand or deck card, which
// stays where it is, and "its cost doesn't change" keeps the cost it had).

import type { CardDef, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defOf, excludingDefId, fusedIdParts, fusedIdSpecs, isDigestId, selfDefIds } from "../src/catalog";
import { FUSED_ID_CAP } from "../src/config";
import { fuseCards, fuseGenerated, fuseRandomInto } from "../src/effects";
import { unitView } from "../src/layers";
import { effectiveCost } from "../src/mana";
import { plagueMultiplierOf } from "../src/plague";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { makeContext, type EngineSink } from "../src/resolve";
import type { Effect } from "../src/script";
import { registerScripts, registeredScripts, scriptsFor } from "../src/scripts";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { fuse, fusedDigest, fusedIngredients } from "../src/subsystems/fuse";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import {
  GEN_SCRIPTS,
  LAB_POOL,
  act,
  aiSpell,
  aiUnit,
  answer,
  bigUnit,
  body,
  deckFusion,
  felinorA,
  felinorB,
  felinorC,
  fieldTrap,
  frozen,
  fuseA,
  fuseB,
  fuser,
  handCard,
  immutable,
  lab,
  mutate,
  pick,
  plainTrap,
  playing,
  replayed,
  slime,
  slop,
  xUnit,
  type Run,
} from "./fixtures/generation";
import { eventsOfType, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

function run(sink: EngineSink, effect: Effect, self: CardInstance | null = null): void {
  effect.apply(makeContext(sink, self, { controller: "p1" }));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function fusedEvents(events: readonly GameEvent[]): Extract<GameEvent, { type: "fused" }>[] {
  return eventsOfType(events, "fused");
}

/** A phantom ingredient: a definition only, in no pile (R86), as `fuseCards` makes one. */
function phantom(state: GameState, defId: string): CardInstance {
  return newInstance(state, defId, "p1", { z: "gone", player: "p1" });
}

// ---------------------------------------------------------------------------
// R468: bounded fused ids.
// ---------------------------------------------------------------------------

describe("R468 a fused id is bounded: past FUSED_ID_CAP it is a digest of the ingredient list", () => {
  it("R468 a short list is spelled out (R179); a card fused onto again and again switches to a digest once the list outgrows the cap", () => {
    const start = playing("fuse-digest");
    const state = start.state;
    const kept = put(state, slime.id, slot("p1", "units", 1));
    const sink = sinkFor(state);

    const first = must(fuse(sink, { ingredients: [phantom(state, slime.id)], target: kept }), "a fusion");
    expect(first.defId).toBe(`t-1:${slime.id}+${slime.id}`);

    let fusions = 1;
    while (!isDigestId(kept.defId)) {
      fuse(sink, { ingredients: [phantom(state, slime.id)], target: kept });
      fusions += 1;
      if (fusions > 40) throw new Error("the id never switched to a digest");
    }
    const id = kept.defId;
    expect(id).toMatch(/^t-\d+:#[0-9a-f]{16}$/);
    // The fused id before it spelled out everything and sat within the cap; this one would not have.
    const def = defOf(state, id);
    const spelled = must(def.ingredients, "the ingredient list").map((entry) =>
      /^t-\d+:/.test(entry.defId) ? `(${entry.defId})` : entry.defId,
    );
    expect(spelled.join("+").length).toBeGreaterThan(FUSED_ID_CAP);
    expect(id).toBe(`t-${fusions}:#${fusedDigest(spelled.join("+"))}`);

    // The scripts are the list's: every Slime text multiplies (R471), fused onto itself `fusions` times.
    expect(plagueMultiplierOf(state, kept)).toBe(2 ** (fusions + 1));
    // One more fusion onto it names the digest in parentheses, which stays short. The kept card is the
    // last ingredient, as R77's target always is (#85's played card fused onto its target).
    fuse(sink, { ingredients: [phantom(state, slime.id)], target: kept });
    expect(kept.defId).toBe(`t-${fusions + 1}:${slime.id}+(${id})`);
    expect(fusedIdParts(kept.defId)).toEqual([slime.id, id]);
    expect(plagueMultiplierOf(state, kept)).toBe(2 ** (fusions + 2));
  });

  it("R468 the digest is a pure function of the list: the same list gives the same digest, a different one another", () => {
    const a = fusedDigest("gen-a+gen-b");
    expect(fusedDigest("gen-a+gen-b")).toBe(a);
    expect(fusedDigest("gen-b+gen-a")).not.toBe(a);
    expect(fusedDigest("gen-a+gen-b*")).not.toBe(a);
    expect(a).toMatch(/^[0-9a-f]{16}$/);
  });

  it("R468 a state holding a digest id this process never minted rebuilds its scripts from the definition", () => {
    const start = playing("fuse-digest-json");
    const state = start.state;
    const id = `t-1:#${fusedDigest("never-minted-here")}`;
    const fused: CardDef = {
      ...defOf(state, slime.id),
      id,
      index: id,
      name: "Gen slime + Gen slime",
      ingredients: [{ defId: slime.id }, { defId: slime.id }],
    };
    state.transientDefs[id] = fused;
    const card = put(state, id, slot("p1", "units", 1));
    expect(fusedIdSpecs(id)).toBeNull();
    expect(registeredScripts()[id]).toBeUndefined();

    const round = JSON.parse(JSON.stringify(state)) as GameState;
    legalActions(round, "p1");
    expect(fusedIdSpecs(id)).toEqual([{ defId: slime.id }, { defId: slime.id }]);
    expect(fusedIngredients(id)).toEqual([slime.id, slime.id]);
    expect(scriptsFor(id).base.plagueMultiplier).toBeDefined();
    const again = must(findInstance(round, card.id), "the card after JSON");
    expect(plagueMultiplierOf(round, again)).toBe(4);

    // B4.1: a digest-named fused card never generates any of its ingredients either.
    expect(selfDefIds(id)).toEqual([slime.id, slime.id]);
    expect(new Set(excludingDefId({}, id).excludeDefId)).toEqual(new Set([slime.id]));

    // A registry replaced wholesale is repaired on the next entry, digest ids included.
    registerScripts({ ...GEN_SCRIPTS });
    expect(registeredScripts()[id]).toBeUndefined();
    viewFor(round, "p1");
    expect(scriptsFor(id).base.plagueMultiplier).toBeDefined();
  });

  it("R468 every fused definition keeps its ingredient list, and sums its ingredients' lines of code (E36)", () => {
    const start = playing("fuse-loc");
    const state = start.state;
    const kept = put(state, fuseA.id, slot("p1", "units", 1));
    const sink = sinkFor(state);
    const result = must(fuse(sink, { ingredients: [phantom(state, fuseB.id)], target: kept }), "a fusion");
    const def = defOf(state, result.defId);
    expect(def.ingredients).toEqual([{ defId: fuseB.id }, { defId: fuseA.id }]);
    expect(def.loc).toBe(17);

    const other = put(state, body.id, slot("p1", "units", 2));
    const plain = must(fuse(sink, { ingredients: [phantom(state, fuseB.id)], target: other }), "a fusion");
    expect(defOf(state, plain.defId).loc).toBe(7);
    const none = put(state, body.id, slot("p1", "units", 3));
    const bare = must(fuse(sink, { ingredients: [phantom(state, body.id)], target: none }), "a fusion");
    expect(defOf(state, bare.defId).loc).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// R469: an ingredient fused as a Radiant card.
// ---------------------------------------------------------------------------

describe("R469 an ingredient fused on its Radiant face goes into both fused forms", () => {
  it("R469 the id marks it with `*`, and the base form carries its Radiant face and text", () => {
    const start = playing("fuse-radiant-ingredient");
    const state = start.state;
    const kept = put(state, body.id, slot("p1", "units", 1));
    const ingredient = phantom(state, fuseA.id);
    const result = must(
      fuse(sinkFor(state), { ingredients: [ingredient], target: kept, radiantIngredients: [ingredient.id] }),
      "a fusion",
    );
    expect(result.defId).toBe(`t-1:${fuseA.id}*+${body.id}`);
    expect(fusedIdSpecs(result.defId)).toEqual([{ defId: fuseA.id, radiant: true }, { defId: body.id }]);
    expect(fusedIdParts(result.defId)).toEqual([fuseA.id, body.id]);
    const def = defOf(state, result.defId);
    // body 1/1 on the base form, fuseA's Radiant 7/1 Taunt on both.
    expect(def.base).toMatchObject({ attack: 8, health: 2 });
    expect(def.base.keywords).toEqual([{ kind: "Taunt" }]);
    expect(def.base.text).toContain("fuse-a radiant");
    expect(def.radiant).toMatchObject({ attack: 9, health: 3 });
    expect(unitView(state, result)).toMatchObject({ attack: 8, maxHealth: 2 });
  });

  it("R469 its Radiant script runs on the base form: a Radiant Slime triples on a base card", () => {
    const start = playing("fuse-radiant-script");
    const state = start.state;
    const kept = put(state, body.id, slot("p1", "units", 1));
    const ingredient = phantom(state, slime.id);
    fuse(sinkFor(state), { ingredients: [ingredient], target: kept, radiantIngredients: [ingredient.id] });
    expect(kept.radiant).toBe(false);
    expect(plagueMultiplierOf(state, kept)).toBe(3);
  });

  it("R469 fuseCards' radiantIngredients: the opponent's played card fused into your permanent as a Radiant copy (Classic+ #74)", () => {
    const start = playing("fuse-twice-forward");
    const state = start.state;
    const trap = put(state, fieldTrap.id, slot("p1", "backrow", 1));
    const played = put(state, fuseA.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    run(sink, fuseCards({ instanceIds: [played.id], targetInstanceId: trap.id, radiantIngredients: true }), trap);
    const def = defOf(state, trap.defId);
    // The kept instance and its type stay; the opponent's card ceases to exist (R77, R86).
    expect(def.type).toBe("Field Trap");
    expect(trap.zone).toEqual({ z: "field", player: "p1", row: "backrow", lane: 1 });
    expect(played.zone).toEqual({ z: "gone", player: "p2" });
    expect(state.players.p2.units[0]).toBeNull();
    expect(def.base.text).toContain("fuse-a radiant");
    expect(trap.defId).toBe(`t-1:${fuseA.id}*+${fieldTrap.id}`);
  });
});

// ---------------------------------------------------------------------------
// R470: fusing into a hand or deck card, keeping its cost.
// ---------------------------------------------------------------------------

describe("R470 a fusion keeps a hand or deck card where it is, and 'its cost doesn't change'", () => {
  it("R470 Fusion Lab: a random card of the pool (never the Lab, B4.1) into the declared hand card, which keeps its cost", () => {
    const start = playing("fuse-lab");
    const state = start.state;
    const card = handCard(state, fuseB.id);
    card.costMod = 1;
    const labCard = handCard(state, lab.id);
    const costBefore = effectiveCost(state, card);
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: labCard.id, targets: [pick(card)], playerId: "p1" });

    const kept = must(findInstance(run1.state, card.id), "the hand card");
    expect(kept.zone).toEqual({ z: "hand", player: "p1" });
    const def = defOf(run1.state, kept.defId);
    const parts = must(fusedIdParts(kept.defId), "a fused id");
    expect(parts).toHaveLength(2);
    expect(parts[1]).toBe(fuseB.id);
    expect(LAB_POOL.filter((id) => id !== lab.id)).toContain(parts[0]);
    expect(def.type).toBe("Unit");
    // Its own cost as it stood, printed 1, now its costOverride; its costMod is its own and stays.
    expect(kept.costOverride).toBe(1);
    expect(kept.costMod).toBe(1);
    expect(effectiveCost(run1.state, kept)).toBe(costBefore);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));

    // The opponent learns that some card of that hand fused, never which or into what (§10.8).
    const theirs = fusedEvents(viewFor(run1.state, "p2").events).at(-1);
    expect(theirs).toMatchObject({ resultInstanceId: HIDDEN_ID, defId: HIDDEN_ID });
    expect(theirs?.instanceIds.every((id) => id === HIDDEN_ID)).toBe(true);
    expect(viewFor(run1.state, "p2").defs?.[kept.defId]).toBeUndefined();
    const mine = fusedEvents(viewFor(run1.state, "p1").events).at(-1);
    expect(mine).toMatchObject({ resultInstanceId: card.id, defId: kept.defId });
    expect(viewFor(run1.state, "p1").defs?.[kept.defId]?.ingredients).toEqual(def.ingredients);
  });

  it("R470 Fusion Lab's Radiant face fuses a Radiant card (R469)", () => {
    const start = playing("fuse-lab-radiant");
    const state = start.state;
    const card = handCard(state, body.id);
    const labCard = handCard(state, lab.id);
    labCard.radiant = true;
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: labCard.id, targets: [pick(card)], playerId: "p1" });
    const kept = must(findInstance(run1.state, card.id), "the hand card");
    expect(fusedIdSpecs(kept.defId)?.[0]?.radiant).toBe(true);
    expect(fusedIdSpecs(kept.defId)?.[1]).toEqual({ defId: body.id });
  });

  it("R470 an X-cost or embiggen hand card keeps its printed form; a card with an override keeps the override", () => {
    const start = playing("fuse-keep-forms");
    const state = start.state;
    const sink = sinkFor(state);
    const x = handCard(state, xUnit.id);
    const big = handCard(state, bigUnit.id);
    const crafted = handCard(state, fuseA.id);
    crafted.costOverride = 0;

    fuse(sink, { ingredients: [phantom(state, fuseA.id)], into: x, keepCost: true });
    fuse(sink, { ingredients: [phantom(state, fuseA.id)], into: big, keepCost: true });
    fuse(sink, { ingredients: [phantom(state, fuseB.id)], into: crafted, keepCost: true });

    expect(defOf(state, x.defId).cost).toBe("X");
    expect(x.costOverride).toBeUndefined();
    expect(defOf(state, big.defId).cost).toEqual({ base: 2, embiggen: 4 });
    expect(big.costOverride).toBeUndefined();
    expect(crafted.costOverride).toBe(0);
    expect(effectiveCost(state, crafted)).toBe(0);

    // Without keepCost the kept card takes R77's fused cost.
    const plain = handCard(state, fuseA.id);
    fuse(sink, { ingredients: [phantom(state, fuseB.id)], into: plain });
    expect(plain.costOverride).toBeUndefined();
    expect(effectiveCost(state, plain)).toBe(3);
  });

  it("R470 `into` takes only a hand or library card, `target` only a field card, and an Immutable card refuses either", () => {
    const start = playing("fuse-into-refusals");
    const state = start.state;
    const sink = sinkFor(state);
    const onField = put(state, body.id, slot("p1", "units", 1));
    const inHand = handCard(state, body.id);
    const locked = handCard(state, immutable.id);
    const gy = newInstance(state, body.id, "p1", { z: "graveyard", player: "p1" });
    state.players.p1.graveyard.push(gy);

    expect(fuse(sink, { ingredients: [phantom(state, fuseB.id)], into: onField })).toBeNull();
    expect(fuse(sink, { ingredients: [phantom(state, fuseB.id)], target: inHand })).toBeNull();
    expect(fuse(sink, { ingredients: [phantom(state, fuseB.id)], into: gy })).toBeNull();
    expect(fuse(sink, { ingredients: [phantom(state, fuseB.id)], into: locked })).toBeNull();
    expect(sink.events).toEqual([]);
    expect(state.transientDefs).toEqual({});

    // A random fusion into a card the Fuse would refuse draws nothing for it (R129).
    const cursor = sink.rng.cursor;
    const targeted = (card: CardInstance): Effect =>
      fuseRandomInto({ into: { target: { of: "instance", instanceId: card.id } }, query: { defId: LAB_POOL } });
    run(sink, targeted(locked));
    run(sink, targeted(gy));
    expect(sink.rng.cursor).toBe(cursor);
    expect(sink.events).toEqual([]);
  });

  it("R470 the deck fusion: a random card into every deck card, each keeping its cost, hidden from both players (Classic+ #73)", () => {
    const start = playing("fuse-deck");
    const state = start.state;
    const library = setLibrary(state, "p1", [fuseA.id, xUnit.id, body.id]);
    const costs = library.map((card) => effectiveCost(state, card));
    const listBefore = viewFor(state, "p1").you.ownLibrary;
    const spell = handCard(state, deckFusion.id);
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, playerId: "p1" });

    const after = run1.state.players.p1.library;
    expect(after.map((card) => card.id)).toEqual(library.map((card) => card.id));
    expect(after.map((card) => fusedIdParts(card.defId)?.[1])).toEqual(library.map((card) => card.defId));
    expect(new Set(after.map((card) => card.defId)).size).toBe(3);
    expect(after.map((card) => effectiveCost(run1.state, card))).toEqual(costs);
    expect(defOf(run1.state, after[1]?.defId ?? "").cost).toBe("X");
    // Nobody reads a change made inside a library: the owner's list is as it was (R311), and every
    // `fused` event is the sentinel in both views.
    expect(viewFor(run1.state, "p1").you.ownLibrary).toEqual(listBefore);
    for (const viewer of ["p1", "p2"] as const) {
      const events = fusedEvents(viewFor(run1.state, viewer).events);
      expect(events).toHaveLength(3);
      expect(events.every((event) => event.resultInstanceId === HIDDEN_ID && event.defId === HIDDEN_ID)).toBe(true);
    }
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));

    // An empty library fuses nothing and draws nothing (R129).
    const empty = playing("fuse-deck-empty");
    empty.state.players.p1.library = [];
    const sink = sinkFor(empty.state);
    const cursor = sink.rng.cursor;
    run(sink, fuseRandomInto({ into: { pile: "library" }, query: { defId: LAB_POOL } }));
    expect(sink.rng.cursor).toBe(cursor);
    expect(sink.events).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Three generated cards with no target (Classic+ #43 AI Slop).
// ---------------------------------------------------------------------------

describe("E23 fuse three generated cards into the hand (Classic+ #43)", () => {
  it("R77 three random picks of the pool, repeats allowed, fused with no target: a Token at (0) in the hand", () => {
    const start = playing("fuse-slop");
    const spell = handCard(start.state, slop.id);
    const handBefore = start.state.players.p1.hand.length;
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, playerId: "p1" });

    const hand = run1.state.players.p1.hand;
    expect(hand).toHaveLength(handBefore);
    const made = must(hand.at(-1), "the fused card");
    const def = defOf(run1.state, made.defId);
    expect(def.ingredients).toHaveLength(3);
    expect(def.ingredients?.every((entry) => [aiUnit.id, aiSpell.id].includes(entry.defId))).toBe(true);
    expect(def.token).toBe(true);
    const types = def.ingredients?.map((entry) => defOf(run1.state, entry.defId).type) ?? [];
    // R102: the shared type, else the first pick's — the first pick's either way.
    expect(def.type).toBe(types[0]);
    expect(made.costOverride).toBe(0);
    expect(made.radiant).toBe(false);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));
  });

  it("R469 the Radiant face fuses Radiant AI cards; fewer than two picks or an empty pool fuses and draws nothing", () => {
    const start = playing("fuse-slop-radiant");
    const spell = handCard(start.state, slop.id);
    spell.radiant = true;
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, playerId: "p1" });
    const made = must(run1.state.players.p1.hand.at(-1), "the fused card");
    expect(defOf(run1.state, made.defId).ingredients?.every((entry) => entry.radiant === true)).toBe(true);
    // Every ingredient Radiant and no kept card: the fused card is Radiant too.
    expect(made.radiant).toBe(true);

    const sink = sinkFor(start.state);
    const cursor = sink.rng.cursor;
    run(sink, fuseGenerated({ count: 1, query: { defId: [aiUnit.id] } }));
    run(sink, fuseGenerated({ count: 3, query: { tags: ["Pancake"] } }));
    expect(sink.rng.cursor).toBe(cursor);
    expect(sink.events).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// An enemy card onto one of yours of its type (Classic #78 Mutate Spell, Radiant).
// ---------------------------------------------------------------------------

function mutateBoard(seed: string): { run: Run; enemy: CardInstance; field: CardInstance; inHand: CardInstance; deck: CardInstance[] } {
  const start = playing(seed);
  const state = start.state;
  const enemy = put(state, fuseA.id, slot("p2", "units", 1));
  const field = put(state, body.id, slot("p1", "units", 1));
  put(state, immutable.id, slot("p1", "units", 2));
  state.players.p1.hand = [];
  const inHand = handCard(state, fuseB.id);
  handCard(state, slop.id); // a Spell, not of the enemy card's type
  const deck = setLibrary(state, "p1", [felinorC.id, lab.id, felinorA.id]);
  return { run: start, enemy, field, inHand, deck };
}

describe("E23 fuse an enemy card onto one of yours of its type (Classic #78 Radiant)", () => {
  it("R470 the controller picks among their cards of its type on the field, in hand and in deck; the answer fuses it there", () => {
    const { run: start, enemy, field, inHand, deck } = mutateBoard("fuse-mutate");
    const spell = handCard(start.state, mutate.id);
    let run1 = frozen(start);
    const enemyHero = run1.state.players.p2.hero.health;
    run1 = act(run1, { type: "play", instanceId: spell.id, targets: [pick(enemy)], playerId: "p1" });

    const pending = must(run1.state.pending, "the pick");
    expect(pending.playerId).toBe("p1");
    // Field in lane order, hand in hand order, the deck's Units in an order of their own (name).
    const felinorAInDeck = must(deck[2], "felinor-a");
    const felinorCInDeck = must(deck[0], "felinor-c");
    expect(pending.options.map((option) => option.selection)).toEqual([
      pick(field),
      pick(inHand),
      pick(felinorAInDeck),
      pick(felinorCInDeck),
    ]);
    // The rest of the Spell waits for the answer (R113).
    expect(run1.state.players.p2.hero.health).toBe(enemyHero);

    // The other player sees an open prompt, nothing more; the chooser sees its deck cards (§10.8).
    expect(viewFor(run1.state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    expect(JSON.stringify(viewFor(run1.state, "p2"))).not.toContain(felinorAInDeck.id);
    const mine = viewFor(run1.state, "p1").pending;
    if (mine === null || mine.forYou !== true) throw new Error("expected p1's prompt");
    expect(mine.options.find((option) => option.instanceId === felinorAInDeck.id)?.defId).toBe(felinorA.id);

    // JSON mid-prompt, and the answer: onto the deck card, which stays in the deck.
    const round = JSON.parse(JSON.stringify(run1.state)) as GameState;
    expect(round).toEqual(run1.state);
    const live = answer(run1, pick(felinorAInDeck));
    const fromJson = answer({ ...run1, state: round }, pick(felinorAInDeck));
    expect(hashState(fromJson.state)).toBe(hashState(live.state));
    expect(hashState(replayed(live))).toBe(hashState(live.state));

    const kept = must(findInstance(live.state, felinorAInDeck.id), "the deck card");
    expect(kept.zone).toEqual({ z: "library", player: "p1" });
    expect(fusedIdParts(kept.defId)).toEqual([fuseA.id, felinorA.id]);
    expect(findInstance(live.state, enemy.id)).toBeUndefined();
    expect(live.state.players.p2.units[0]).toBeNull();
    expect(live.state.players.p2.hero.health).toBe(enemyHero - 1);
  });

  it("R470 onto a field card it is R77's target; a Trap counts as a Field Trap; with none of its type the card is exiled", () => {
    const start = playing("fuse-mutate-trap");
    const state = start.state;
    const enemyTrap = put(state, plainTrap.id, slot("p2", "backrow", 2));
    const mineTrap = put(state, fieldTrap.id, slot("p1", "backrow", 4));
    const spell = handCard(state, mutate.id);
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, targets: [pick(enemyTrap)], playerId: "p1" });
    expect(run1.state.pending?.options.map((option) => option.selection)).toEqual([pick(mineTrap)]);
    run1 = answer(run1, pick(mineTrap));
    const kept = must(findInstance(run1.state, mineTrap.id), "the Field Trap");
    expect(defOf(run1.state, kept.defId).type).toBe("Field Trap");
    expect(kept.zone).toEqual({ z: "field", player: "p1", row: "backrow", lane: 4 });

    const lonely = playing("fuse-mutate-none");
    const target = put(lonely.state, fuseA.id, slot("p2", "units", 3));
    lonely.state.players.p1.library = [];
    lonely.state.players.p1.hand = [];
    const again = handCard(lonely.state, mutate.id);
    let run2 = frozen(lonely);
    run2 = act(run2, { type: "play", instanceId: again.id, targets: [pick(target)], playerId: "p1" });
    expect(run2.state.pending).toBeNull();
    expect(run2.state.players.p2.exile.map((card) => card.id)).toEqual([target.id]);
  });
});

// ---------------------------------------------------------------------------
// Discover and fuse onto self (Classic+ #30 Felinor Fuser).
// ---------------------------------------------------------------------------

describe("E23 Discover twice and fuse both onto this (Classic+ #30)", () => {
  it("R77 two chained Discovers of Felinor Units — never the Fuser itself (B4.1) — fused onto the Fuser on the field", () => {
    const start = playing("fuse-felinor");
    const card = handCard(start.state, fuser.id);
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: card.id, playerId: "p1" });

    const first = must(run1.state.pending, "the first Discover");
    const offered = first.options.map((option) => (option.selection.pick === "mode" ? option.selection.option : ""));
    expect(offered).not.toContain(fuser.id);
    expect(offered.every((id) => [felinorA.id, felinorB.id, felinorC.id].includes(id))).toBe(true);
    expect(viewFor(run1.state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

    const firstPick = must(first.options[0], "an option").selection;
    run1 = answer(run1, firstPick);
    const second = must(run1.state.pending, "the second Discover");
    const secondPick = must(second.options[1], "an option").selection;
    run1 = answer(run1, secondPick);

    const kept = must(findInstance(run1.state, card.id), "the Fuser");
    expect(kept.zone.z).toBe("field");
    const parts = must(fusedIdParts(kept.defId), "a fused id");
    expect(parts).toHaveLength(3);
    expect(parts[2]).toBe(fuser.id);
    const summed = parts.reduce((sum, id) => sum + (defOf(run1.state, id).base.attack ?? 0), 0);
    expect(unitView(run1.state, kept).attack).toBe(summed);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));
  });
});
