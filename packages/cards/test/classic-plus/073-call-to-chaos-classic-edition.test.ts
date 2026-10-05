// C+ #73 Call to Chaos (Classic+ Edition) — SPEC §8.7 row 73, BUILD M9 Classic+ row C+ 73: "One random
// effect of ten, each with its own test: 5 random Fruit-pool cards (R382) that cost (0); 3 random Books
// that cost (0); destroy every enemy permanent (face-down included, Indestructible staying); 3 random
// non-token Classic cards that cost (0); Upgrade every card in your hand and deck twice (R386); fuse a
// random non-token card of any set but this one into each card of your deck, each keeping its cost
// (`costOverride`; Immutable cards skipped, R23); Degrade every card on the opponent's field and in their
// hand 3 times; summon a Classic Golem (C+ #73.1); replace your deck with random Call to Chaos cards (Core
// #95 or this, the named pool R387 allows) that cost (0), the deck keeping its size; cast a random Call
// to Chaos of either edition; the chain counts casts of both editions against `CALL_TO_CHAOS_CHAIN_CAP`
// (20, R28), and a recursion roll at the cap resolves into nothing (R87); both players read which
// effects rolled (R436) while deck and hand changes stay hidden (R97, R177, R311); fused deck cards
// survive JSON (R179); radiant three different effects, resolved in the list's order with the recursion
// where it falls (R423)".
//
// HOW AN EFFECT IS FORCED, as `../095-call-to-chaos.test.ts` does it: the roll is the play's first rng
// draw, so setting `state.rngCursor` before the play pins it, and `cursorFor` finds a cursor for each
// entry by asking the engine's own roll with this card's table. The machinery (each entry against
// fixture pools, Immutable deck cards skipped, the chain cap) is proved in
// `packages/engine/test/callToChaosPlus.test.ts`; this file proves the card against the real catalog.

import { CALL_TO_CHAOS_CHAIN_CAP, HAND_CAP, createRng, effectiveCost, hashState, reduce, subsystems, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { scenario, type PileSetup, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/073-call-to-chaos-classic-edition";

const CHAOS = "classicplus-073";
const CORE_CHAOS = "core-095";
const GOLEM = "classicplus-073-1";
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const TIMMY = "core-011"; // (1) Unit
const STOCKPILE = "core-005"; // (1) Spell
const BEAR = "core-060"; // (1) Trap
const TWINSPELL = "core-079"; // (2) Field Spell
const HEROIC_POWER = "core-098"; // Field Spell, Indestructible
const BIG_FELINOR = "core-043"; // (4) Unit
const FIENDER = "core-092"; // (2) Unit, Stack
const HINDER = "core-021"; // (0) Spell, cast on draw; base face: discard 1 (R431)

const SEED = "chaos-plus";
const TABLE = subsystems.CHAOS_PLUS_EFFECTS;
type Entry = (typeof TABLE)[number]["name"];
const CURSOR_SEARCH = 800;

function cursorFor(name: Entry): number {
  for (let cursor = 0; cursor < CURSOR_SEARCH; cursor += 1) {
    if (subsystems.rollChaosEffects(createRng(SEED, cursor), false, TABLE)[0]?.name === name) return cursor;
  }
  throw new Error(`no cursor rolls ${name}`);
}

function radiantCursorWhere(accept: (names: readonly string[]) => boolean): number {
  for (let cursor = 0; cursor < CURSOR_SEARCH * 4; cursor += 1) {
    if (accept(subsystems.rollChaosEffects(createRng(SEED, cursor), true, TABLE).map((effect) => effect.name))) return cursor;
  }
  throw new Error("no radiant cursor");
}

/** p1 plays C+ #73 with its roll pinned; a spare card in hand keeps §2.5's auto-end away. */
function chaos(
  entry: Entry | null,
  opts: { p1?: SideSetup; p2?: SideSetup; radiantCursor?: number; chain?: number } = {},
): Scenario {
  const radiantFace = opts.radiantCursor !== undefined;
  const s = scenario({
    seed: SEED,
    p1: { hand: [{ def: CHAOS, radiant: radiantFace }, VANILLA], mana: 8, ...opts.p1 },
    p2: { hand: [VANILLA], ...opts.p2 },
  });
  if (opts.chain !== undefined) s.card(CHAOS).memory[subsystems.CHAOS_CHAIN_KEY] = opts.chain;
  s.state.rngCursor = opts.radiantCursor ?? cursorFor(entry ?? "fruits");
  return s.play(CHAOS);
}

function eventsOf<T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

function addedTo(s: Scenario): ReturnType<Scenario["hand"]> {
  const ids = new Set(eventsOf(s, "addedToHand").map((event) => event.instanceId));
  return s.hand("p1").filter((card) => ids.has(card.id));
}

const deck = (cards: readonly PileSetup[]): SideSetup => ({ hand: [{ def: CHAOS }, VANILLA], library: cards, mana: 8 });

describe("C+ #73 Call to Chaos (Classic+ Edition)", () => {
  it("is a (4) Legendary Spell of the Call to Chaos tag, and both faces hang the card off `cry`", () => {
    expect(def.cost).toBe(4);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["Call to Chaos"]);
    expect(def.rarity).toBe("Legendary");
    expect(Object.keys(base)).toEqual(["cry"]);
    expect(Object.keys(radiant)).toEqual(["cry"]);
  });

  it("R436 each entry's label is a clause of the card's printed list, in the printed order", () => {
    const text = def.base.text.toLowerCase();
    let from = 0;
    for (const entry of TABLE) {
      const at = text.indexOf(entry.label.toLowerCase(), from);
      expect(at, entry.label).toBeGreaterThanOrEqual(0);
      from = at;
    }
    expect(def.radiant.text.toLowerCase()).toContain(TABLE[TABLE.length - 1]?.label.toLowerCase());
  });

  it("§9.3 the roll is the play's first rng draw: a pinned cursor rolls the Classic Golem, the one entry nothing else makes", () => {
    const s = chaos("golem");
    expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
    s.expectInZone(CHAOS, "graveyard");
    s.expectMana("p1", 4);
  });

  describe("base, the ten entries", () => {
    it("R382 1/10 adds 5 random Fruit-pool cards that cost (0), the Grapes in the pool", () => {
      const s = chaos("fruits");
      const added = addedTo(s);
      expect(added).toHaveLength(5);
      for (const card of added) {
        expect(cardDef(card.defId).tags).toContain("Fruit");
        expect(card.costOverride).toBe(0);
        expect(effectiveCost(s.state, card)).toBe(0);
      }
    });

    it("R380 2/10 adds 3 random non-token Books that cost (0)", () => {
      const s = chaos("books");
      const added = addedTo(s);
      expect(added).toHaveLength(3);
      for (const card of added) {
        expect(cardDef(card.defId).tags).toContain("Book");
        expect(cardDef(card.defId).token).toBe(false);
        expect(card.costOverride).toBe(0);
      }
    });

    it("R46 3/10 destroys every enemy permanent, face-down ones too; an Indestructible one stays; yours stay", () => {
      const s = chaos("destroy", {
        p1: { hand: [{ def: CHAOS }, VANILLA], field: [TIMMY], mana: 8 },
        p2: {
          field: [MENACE, VANILLA],
          backrow: [{ def: BEAR, faceUp: false }, { def: TWINSPELL, faceUp: true }, { def: HEROIC_POWER, faceUp: true }],
        },
      });
      expect([1, 2].map((lane) => s.unit("p2", lane))).toEqual([null, null]);
      expect([1, 2].map((lane) => s.backrow("p2", lane))).toEqual([null, null]);
      expect(s.backrow("p2", 3)?.defId).toBe(HEROIC_POWER);
      expect(s.unit("p1", 1)?.defId).toBe(TIMMY);
    });

    it("§2.4 R4 1/10 into a nearly full hand: what does not fit is burned", () => {
      const filler = Array.from({ length: HAND_CAP - 3 }, () => VANILLA);
      const s = chaos("fruits", { p1: { hand: [{ def: CHAOS }, ...filler], mana: 8 } });
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(addedTo(s)).toHaveLength(3);
      expect(eventsOf(s, "burned")).toHaveLength(2);
    });

    it("§3.2 R13 3/10 takes the top of an enemy Stack pile; the card dormant beneath it acts again", () => {
      const s = chaos("destroy", { p2: { field: [BIG_FELINOR, { def: FIENDER, stack: true }] } });
      s.expectInZone(FIENDER, "graveyard");
      expect(s.unit("p2", 1)?.defId).toBe(BIG_FELINOR);
    });

    it("R380 4/10 adds 3 random non-token Classic cards that cost (0)", () => {
      const s = chaos("classic");
      const added = addedTo(s);
      expect(added).toHaveLength(3);
      for (const card of added) {
        expect(cardDef(card.defId).set).toBe("Classic");
        expect(cardDef(card.defId).token).toBe(false);
        expect(card.costOverride).toBe(0);
      }
    });

    it("R386 5/10 Upgrades every card in your hand and deck twice, none of the opponent's", () => {
      const s = chaos("upgrade", { p1: deck([MENACE, TIMMY]), p2: { hand: [MENACE] } });
      const upgraded = eventsOf(s, "upgraded");
      // The Vanilla in hand and the two deck cards: two Upgrades each, every one of them a change.
      expect(upgraded).toHaveLength(6);
      expect(s.hand("p2")[0]?.tuning).toBeUndefined();
    });

    it("R311 R177 5/10's deck Upgrades are hidden from both players, its hand Upgrades from the opponent", () => {
      const s = chaos("upgrade", { p1: deck([MENACE]) });
      const deckCard = s.pile("p1", "library")[0];
      for (const viewer of ["p1", "p2"] as const) {
        expect(JSON.stringify(s.view(viewer).events)).not.toContain(deckCard?.id ?? "?");
      }
      const handCard = s.hand("p1").find((card) => card.defId === VANILLA);
      expect(JSON.stringify(s.view("p2").events)).not.toContain(handCard?.id ?? "?");
      expect(JSON.stringify(s.view("p1").events)).toContain(handCard?.id ?? "?");
    });

    it("R77 R470 R387 6/10 fuses a random card into each deck card, which keeps its cost and type; never this card", () => {
      const s = chaos("fuse", { p1: deck([MENACE, TIMMY, STOCKPILE]) });
      const library = s.pile("p1", "library");
      expect(eventsOf(s, "fused")).toHaveLength(3);
      expect(library).toHaveLength(3);
      const costs = library.map((card) => effectiveCost(s.state, card));
      expect(costs).toEqual([3, 1, 1]);
      for (const card of library) {
        const fused = s.state.transientDefs[card.defId];
        expect(fused).toBeDefined();
        expect(fused?.ingredients?.some((part) => part.defId === CHAOS)).toBe(false);
      }
      expect(library.map((card) => s.state.transientDefs[card.defId]?.type)).toEqual(["Unit", "Unit", "Spell"]);
    });

    it("R311 R177 6/10's deck fusions are hidden from both players", () => {
      const s = chaos("fuse", { p1: deck([MENACE, TIMMY]) });
      const library = s.pile("p1", "library");
      for (const viewer of ["p1", "p2"] as const) {
        const fused = s.view(viewer).events.filter((event) => event.type === "fused");
        expect(fused).toHaveLength(2);
        const text = JSON.stringify(fused);
        for (const card of library) {
          expect(text).not.toContain(card.id);
          expect(text).not.toContain(card.defId);
        }
      }
    });

    it("R179 6/10's fused deck cards survive JSON, and the game plays on from the round trip exactly as live", () => {
      const s = chaos("fuse", { p1: deck([MENACE, TIMMY]) });
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const spare = s.hand("p1").find((card) => card.defId === VANILLA);
      const resumed = reduce(revived, { type: "play", playerId: "p1", instanceId: spare?.id ?? "", zone: { row: "units", lane: 1 }, nonce: "plus-json" });
      expect(resumed.error).toBeUndefined();
      s.play(spare ?? VANILLA, { zone: 1 });
      expect(hashState(resumed.state)).toBe(hashState(s.state));
    });

    it("R386 7/10 Degrades every card on the opponent's field and in their hand three times, none of yours", () => {
      const s = chaos("degrade", { p1: { hand: [{ def: CHAOS }, VANILLA], field: [TIMMY], mana: 8 }, p2: { hand: [MENACE], field: [MENACE] } });
      const degraded = eventsOf(s, "degraded").map((event) => event.instanceId);
      const theirs = [s.unit("p2", 1)?.id, s.hand("p2")[0]?.id];
      expect(new Set(degraded)).toEqual(new Set(theirs));
      for (const id of theirs) expect(degraded.filter((each) => each === id)).toHaveLength(3);
      expect(s.unit("p1", 1)?.tuning).toBeUndefined();
    });

    it("R177 7/10's Degrades in their hand are hidden from you", () => {
      const s = chaos("degrade", { p2: { hand: [MENACE] } });
      const theirs = s.hand("p2")[0];
      expect(JSON.stringify(s.view("p1").events)).not.toContain(theirs?.id ?? "?");
      expect(JSON.stringify(s.view("p2").events)).toContain(theirs?.id ?? "?");
    });

    it("R64 8/10 summons a Classic Golem into the leftmost free zone", () => {
      const s = chaos("golem", { p1: { hand: [{ def: CHAOS }, VANILLA], field: [TIMMY], mana: 8 } });
      expect(s.unit("p1", 2)?.defId).toBe(GOLEM);
      s.expectStats(s.unit("p1", 2) ?? "", { attack: 10, health: 10 });
    });

    it("R35 R387 9/10 replaces your deck one for one with Call to Chaos cards of either edition that cost (0)", () => {
      const s = chaos("replace", { p1: deck([MENACE, TIMMY, STOCKPILE, VANILLA, MENACE, TIMMY]) });
      const library = s.pile("p1", "library");
      expect(library).toHaveLength(6);
      for (const card of library) {
        expect([CORE_CHAOS, CHAOS]).toContain(card.defId);
        expect(card.costOverride).toBe(0);
      }
    });

    it("R311 R97 9/10's new deck cards are unknown to their owner and unnamed to both players", () => {
      const s = chaos("replace", { p1: deck([MENACE, TIMMY]) });
      const library = s.pile("p1", "library");
      expect(library.every((card) => card.knownAs === undefined)).toBe(true);
      for (const viewer of ["p1", "p2"] as const) {
        const text = JSON.stringify(s.view(viewer).events.filter((event) => event.type === "transformed"));
        for (const card of library) expect(text).not.toContain(card.id);
      }
    });

    it("R70 R28 10/10 casts a random Call to Chaos of either edition, free and counted as a play", () => {
      const editions = new Set<string>();
      for (const seed of ["a", "b", "c", "d", "e", "f"]) {
        const s = scenario({ seed: `${SEED}-${seed}`, p1: { hand: [{ def: CHAOS }, VANILLA], mana: 8 }, p2: { hand: [VANILLA] } });
        s.card(CHAOS).memory[subsystems.CHAOS_CHAIN_KEY] = CALL_TO_CHAOS_CHAIN_CAP - 1;
        let cursor = 0;
        while (subsystems.rollChaosEffects(createRng(`${SEED}-${seed}`, cursor), false, TABLE)[0]?.name !== "recast") cursor += 1;
        s.state.rngCursor = cursor;
        s.play(CHAOS);
        const cast = eventsOf(s, "cardPlayed").slice(1);
        expect(cast).toHaveLength(1);
        expect(cast[0]?.costPaid).toBe(0);
        editions.add(cast[0]?.defId ?? "");
      }
      expect([...editions].every((id) => id === CHAOS || id === CORE_CHAOS)).toBe(true);
      expect(editions.size).toBe(2);
    });
  });

  describe("the chain", () => {
    it("R87 a recursion rolled at CALL_TO_CHAOS_CHAIN_CAP resolves into nothing", () => {
      const s = chaos("recast", { chain: CALL_TO_CHAOS_CHAIN_CAP });
      expect(eventsOf(s, "cardPlayed")).toHaveLength(1);
      expect(eventsOf(s, "chaosRolled")[0]?.effects).toEqual(["Cast a random Call to Chaos"]);
    });

    it("R28 the chain counts casts of either edition: one link short of the cap, a cast happens and its own recursion can't", () => {
      const s = chaos("recast", { chain: CALL_TO_CHAOS_CHAIN_CAP - 1 });
      const played = eventsOf(s, "cardPlayed");
      // The played card, then one cast (link 20). Whatever that cast rolled, a further cast would be link 21.
      expect(played.length).toBe(2);
      expect([CHAOS, CORE_CHAOS]).toContain(played[1]?.defId);
    });
  });

  describe("R436 what was rolled is public", () => {
    it("both players read the rolled clause by this card, while the cards it added stay hidden from the opponent", () => {
      const s = chaos("fruits");
      for (const viewer of ["p1", "p2"] as const) {
        const rolled = s.view(viewer).events.filter((event) => event.type === "chaosRolled");
        expect(rolled).toEqual([expect.objectContaining({ defId: CHAOS, effects: ["Add 5 random Fruits to your hand, which cost (0)"] })]);
      }
      const added = addedTo(s);
      const theirs = JSON.stringify(s.view("p2"));
      for (const card of added) expect(theirs).not.toContain(card.id);
      expect(s.view("p2").opponent.hand).toEqual({ count: s.hand("p1").length });
    });
  });

  describe("radiant", () => {
    it("R423 rolls three different entries and resolves them in the list's order", () => {
      const cursor = radiantCursorWhere((names) => names.join() === ["fruits", "golem", "replace"].join());
      const s = chaos(null, { radiantCursor: cursor, p1: { hand: [{ def: CHAOS, radiant: true }, VANILLA], library: [MENACE, TIMMY], mana: 8 } });
      expect(eventsOf(s, "chaosRolled")[0]?.effects).toEqual([
        "Add 5 random Fruits to your hand, which cost (0)",
        "Summon a Classic Golem",
        "Replace your deck with random Call to Chaos cards, which cost (0)",
      ]);
      const types = s.events.map((event) => event.type);
      expect(types.indexOf("addedToHand")).toBeLessThan(types.indexOf("summoned"));
      expect(types.indexOf("summoned")).toBeLessThan(types.indexOf("transformed"));
      expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
      expect(s.pile("p1", "library").every((card) => [CHAOS, CORE_CHAOS].includes(card.defId))).toBe(true);
    });

    it("R423 R87 the recursion resolves where it falls: last, after the two entries before it", () => {
      const cursor = radiantCursorWhere((names) => names.length === 3 && names[2] === "recast" && names.includes("golem"));
      const s = chaos(null, { radiantCursor: cursor, chain: CALL_TO_CHAOS_CHAIN_CAP - 1 });
      const types = s.events.map((event) => event.type);
      const firstSummon = types.indexOf("summoned");
      const cast = types.indexOf("cardPlayed", types.indexOf("chaosRolled"));
      expect(firstSummon).toBeGreaterThan(-1);
      expect(firstSummon).toBeLessThan(cast);
    });

    it("R436 R87 the recursion finishes in one pass: Hinder's random discard pauses nothing, and nothing runs twice", () => {
      // The recursion, one link short of the cap, casts a base Core #95 that rolls "draw your whole
      // deck": the deck's Hinder is cast and discards at random (R662, R431: no prompt), so the draw
      // and the Golem run through with no prompt open (R113) and the chain lands once.
      let found: Scenario | null = null;
      for (let cursor = 0; cursor < CURSOR_SEARCH * 4 && found === null; cursor += 1) {
        const names = subsystems.rollChaosEffects(createRng(SEED, cursor), true, TABLE).map((effect) => effect.name);
        if (!names.includes("recast") || !names.includes("golem")) continue;
        const s = chaos(null, {
          radiantCursor: cursor,
          chain: CALL_TO_CHAOS_CHAIN_CAP - 1,
          p1: { hand: [{ def: CHAOS, radiant: true }, VANILLA], library: [HINDER, TIMMY], mana: 8 },
        });
        const hinderCast = s.pile("p1", "graveyard").some((card) => card.defId === HINDER);
        const golem = eventsOf(s, "summoned").some((event) => event.defId === GOLEM);
        if (s.state.pending === null && hinderCast && golem) found = s;
      }
      const s = found;
      if (s === null) throw new Error("no roll casts the Hinder through the recursion");
      // Nothing of the Radiant's own roll ran twice, and it landed in the graveyard once the chain was done.
      expect(eventsOf(s, "chaosRolled").filter((event) => event.defId === CHAOS)).toHaveLength(1);
      expect(eventsOf(s, "summoned").filter((event) => event.defId === GOLEM)).toHaveLength(1);
      expect(s.pile("p1", "graveyard").filter((card) => card.defId === CHAOS)).toHaveLength(1);
    });

    it("R87 at the cap a Radiant runs only its other two", () => {
      const cursor = radiantCursorWhere((names) => names.includes("recast") && names.includes("golem"));
      const s = chaos(null, { radiantCursor: cursor, chain: CALL_TO_CHAOS_CHAIN_CAP });
      expect(eventsOf(s, "cardPlayed")).toHaveLength(1);
      expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
    });
  });
});
