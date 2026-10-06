// C #55 Book of Wildfire — SPEC §8.6 row 55, BUILD M9 Classic row C 55: "`classic-055`, a Book: one
// targeted hit of 4 on any Unit or hero, either side; nothing names it, so C #23 and C #29 never make
// it (R381); C #4's base face answers it as a Book; radiant 8; its tuned number (damage) reads through
// `param()` (R386)".
//
// The hit is one §4.4 damage instance from the Spell, so Divine Shield, Armor and Spell Damage meet it
// as they meet any Spell's. C #4 Palantir's base face answers it as a Book (its steal prompt), and
// C #29's Radiant face makes a Book of Flame, never this card (R381).
//
// Patch v0.2.X (#271, R671): at the end of its owner's turn, while it is in their hand, it becomes a
// different Book — every non-token Book but Wildfire, Book of Flame included, drawn with the match rng
// — on its own face, in its place in the hand; the Book it becomes has that Book's own text and keeps
// the swap (the `swapsBook` enchantment), so it changes again at each end of its owner's turn.

import {
  BOOK_SWAP_TRIGGER,
  hashState,
  heroOf,
  legalActions,
  playedThisGameWithTag,
  query,
  reduce,
  stepParam,
  type CardInstance,
  type GameState,
} from "@jackioh/engine";
import type { Action, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { base, def, radiant } from "../../src/scripts/classic/055-book-of-wildfire";
import { scenario, type Scenario } from "../_harness";

const WILDFIRE = "classic-055";
const VANILLA = "core-008"; // 4/4, no text
const MENACE = "core-019"; // 9/9 Taunt
const DEFENDER = "core-003"; // 1/1 Taunt, Divine Shield, Reborn
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const FILLER = "core-005"; // a hand card, so a turn never auto-ends (§2.5)
const PALANTIR = "classic-004"; // Base: when your opponent plays a Book, you may Tribute this to steal it.
const VITAL_KILL = "classic-029"; // Radiant: … Add a Book of Flame to your hand.
const BOOK_OF_FLAME = "classic-016";

function hero(player: PlayerId): Selection {
  return { pick: "hero", player };
}

function at(s: Scenario, ref: string): Selection {
  return { pick: "instance", instanceId: s.card(ref).id };
}

function wildfirePlays(s: Scenario, player: PlayerId = "p1"): Selection[][] {
  const id = s.card(WILDFIRE).id;
  return legalActions(s.state, player).flatMap((action) =>
    action.type === "play" && action.instanceId === id ? [action.targets ?? []] : [],
  );
}

/** Every non-token Book of every set but Wildfire: the swap's pool (R380, R671). */
const OTHER_BOOKS = query({ tags: ["Book"] })
  .map((book) => book.id)
  .filter((id) => id !== WILDFIRE);

/** p1 holds this card (Radiant or not) and a filler, and p2 a filler, so no turn ends on its own. */
function holding(seed: string, radiant = false): Scenario {
  return scenario({ seed, p1: { hand: [{ def: WILDFIRE, radiant }, FILLER] }, p2: { hand: [FILLER] } });
}

/** p1's first hand card: where the Wildfire sits, and where what it became sits (R671). */
function firstCard(s: Scenario): CardInstance {
  const card = s.hand("p1")[0];
  if (card === undefined) throw new Error("p1's hand is empty");
  return card;
}

/** The Book p1's Wildfire is after p1's end of turn, from a game on `seed`. */
function swappedOn(seed: string, radiant = false): CardInstance {
  return firstCard(holding(seed, radiant).endTurn());
}

function hitsOn(s: Scenario, targetId: string): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === targetId ? [event.amount] : []));
}

describe("C #55 Book of Wildfire", () => {
  it("is a (1) Spell with the Book tag, declaring one target, its damage a declared number on both faces", () => {
    expect(def.id).toBe(WILDFIRE);
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["Book"]);
    expect(def.params).toEqual([{ key: "damage", base: 4, radiant: 8, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
    expect(base.handTriggers).toEqual([BOOK_SWAP_TRIGGER]);
  });

  it("R671 prints the swap on both faces, the Radiant one naming a Radiant Book", () => {
    expect(def.base.text).toBe("Deal {damage} damage.\nEnd of turn: Become a different Book.");
    expect(def.radiant.text).toBe("Deal {damage} damage.\nEnd of turn: Become a different Radiant Book.");
    expect(def.refs).toBeUndefined();
  });

  it("R381 nothing names it: C #23 and C #29 name Book of Flame (C #16), never Book of Wildfire", () => {
    expect(cardDef("classic-023").refs).toEqual(["classic-016"]);
    expect(cardDef("classic-029").refs).toEqual(["classic-016"]);
    expect(cardDef("classic-016").name).toBe("Book of Flame");
    expect(def.name).toBe("Book of Wildfire");
  });

  describe("base", () => {
    it("deals 4 damage to a target enemy Unit", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [MENACE] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 5 });
      s.expectInZone(WILDFIRE, "graveyard");
    });

    it("deals 4 damage to the enemy hero, or to your own hero: any target, either side", () => {
      const enemy = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      enemy.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 26);
      const own = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      own.play(WILDFIRE, { targets: [hero("p1")] }).expectHealth("p1", 26);
    });

    it("may hit one of your own Units, and kills a 4-health one", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [VANILLA] } });
      s.play(WILDFIRE, { targets: [at(s, VANILLA)] });
      s.expectInZone(VANILLA, "graveyard");
    });

    it("legalActions offers every Unit and both heroes, one target each, and a play with none is refused", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [VANILLA] }, p2: { field: [MENACE] } });
      const offered = wildfirePlays(s);
      expect(offered).toHaveLength(4);
      expect(offered.every((targets) => targets.length === 1)).toBe(true);
      expect(offered).toContainEqual([hero("p1")]);
      expect(offered).toContainEqual([hero("p2")]);
      expect(() => s.play(WILDFIRE)).toThrow(/target/);
      expect(() => s.play(WILDFIRE, { targets: [hero("p1"), hero("p2")] })).toThrow(/at most/);
    });

    it("E35 a Unit Immune to Spells is never offered, and a play naming it is refused", () => {
      const s = scenario({
        p1: { hand: [WILDFIRE, FILLER] },
        p2: { field: [{ def: TOP_LOSER, radiant: true }] },
      });
      const loser = s.card(TOP_LOSER);
      expect(wildfirePlays(s)).not.toContainEqual([{ pick: "instance", instanceId: loser.id }]);
      expect(() => s.play(WILDFIRE, { targets: [{ pick: "instance", instanceId: loser.id }] })).toThrow(/not a legal target/);
    });

    it("§4.4 step 1: Divine Shield takes the whole hit, and the unit stays", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [DEFENDER] } });
      s.play(WILDFIRE, { targets: [at(s, DEFENDER)] });
      s.expectInZone(DEFENDER, "field").expectStats(DEFENDER, { health: 1 });
      s.expectEvents("divineShieldLost");
    });

    it("§4.4 step 2: Armor reduces it — a Unit in Defense Position takes 3", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [{ def: MENACE, position: "DEF" }] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 6 });
    });

    it("§4.4 step 0: Spell Damage on your side raises the hit — Solarius's +2 makes it 6", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [SOLARIUS] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(hitsOn(s, "hero-p2")).toEqual([6]);
    });

    it("is a Book play: the game counts its Book tag for its player (what C #4 Palantir answers)", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(playedThisGameWithTag(s.state, "p1", "Book")).toBe(1);
    });

    it("C #4 Palantir's base face answers it as a Book: Tributed at once with no prompt, and steals this", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { backrow: [PALANTIR], hand: [FILLER] } });
      const wildfire = s.card(WILDFIRE);
      s.play(wildfire, { targets: [hero("p2")] });
      expect(s.state.pending).toBeNull();
      s.expectInZone(PALANTIR, "graveyard");
      expect(s.card(wildfire.id)).toMatchObject({ owner: "p2", zone: { z: "hand", player: "p2" } });
      s.expectHealth("p2", 30);
    });

    it("R381 C #29's Radiant face adds a Book of Flame, never this card", () => {
      const s = scenario({ p1: { hand: [{ def: VITAL_KILL, radiant: true }, FILLER] } });
      s.play(VITAL_KILL, { targets: [hero("p2")] });
      const added = s.hand("p1").map((card) => card.defId).filter((defId) => defId !== FILLER);
      expect(added).toEqual([BOOK_OF_FLAME]);
    });

    it("R386 its damage is the declared number: an Upgrade's step makes it 5, a Degrade's 3", () => {
      const up = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      stepParam(up.card(WILDFIRE), "damage", 1);
      up.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 25);
      const down = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      stepParam(down.card(WILDFIRE), "damage", -1);
      down.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 27);
      expect(down.view("p1").you.graveyard.find((card) => card.defId === WILDFIRE)?.params).toEqual({ damage: 3 });
    });

    it("R97 in its owner's hand the opponent's view never names it", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 2 });
      expect(JSON.stringify(theirs)).not.toContain(WILDFIRE);
      const own = s.view("p1");
      expect(Array.isArray(own.you.hand) && own.you.hand.some((card) => card.defId === WILDFIRE)).toBe(true);
    });

    it("its play replays exactly from the log: a JSON round trip of the state plays the same", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [MENACE] } });
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "play",
        playerId: "p1",
        nonce: "c55-json",
        instanceId: s.card(WILDFIRE).id,
        targets: [at(s, MENACE)],
      };
      const a = reduce(s.state, action);
      const b = reduce(round, action);
      expect(a.error).toBeUndefined();
      expect(b.state).toEqual(a.state);
    });
  });

  describe("base: the swap (R671)", () => {
    it("R671 at the end of its owner's turn, in hand, it becomes a different Book in its place in the hand", () => {
      const s = holding("swap-1");
      const wildfire = s.card(WILDFIRE);
      s.endTurn();
      const book = firstCard(s);
      expect(s.hand("p1").some((card) => card.id === wildfire.id)).toBe(false);
      expect(OTHER_BOOKS).toContain(book.defId);
      expect(book.radiant).toBe(false);
      expect(book.enchantments).toEqual([{ kind: "swapsBook", from: WILDFIRE }]);
      expect(s.hand("p1")[1]?.defId).toBe(FILLER);
      const swapped = s.events.find((event) => event.type === "transformed");
      expect(swapped).toMatchObject({ instanceId: wildfire.id, fromDefId: WILDFIRE, toDefId: book.defId, hiddenFrom: ["p2"] });
    });

    it("R671 the pool is every other Book: never Wildfire, Book of Flame among them, the pick the match rng's", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 60; n += 1) seen.add(swappedOn(`pool-${String(n)}`).defId);
      expect(seen.has(WILDFIRE)).toBe(false);
      expect([...seen].every((id) => OTHER_BOOKS.includes(id))).toBe(true);
      expect(seen.has(BOOK_OF_FLAME)).toBe(true);
      expect(seen.size).toBeGreaterThan(5);
      // The same game picks the same Book.
      expect(swappedOn("pool-7").defId).toBe(swappedOn("pool-7").defId);
    });

    it("R671 not at the end of the opponent's turn: in the other player's hand it stays Wildfire", () => {
      const s = scenario({ p1: { hand: [FILLER] }, p2: { hand: [WILDFIRE, FILLER] } });
      s.endTurn();
      expect(s.hand("p2").map((card) => card.defId)).toContain(WILDFIRE);
      s.expectInZone(WILDFIRE, "hand");
      expect(s.events.some((event) => event.type === "transformed")).toBe(false);
    });

    it("R671 only in hand: in the library or the graveyard it stays Wildfire", () => {
      const s = scenario({ p1: { hand: [FILLER], library: [FILLER, WILDFIRE], graveyard: [WILDFIRE] }, p2: { hand: [FILLER] } });
      s.endTurn();
      expect(s.pile("p1", "library").map((card) => card.defId)).toContain(WILDFIRE);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([WILDFIRE]);
      expect(s.events.some((event) => event.type === "transformed")).toBe(false);
    });

    it("R671 the Book it became keeps swapping: unchanged on the opponent's turn, a different Book at its owner's next", () => {
      const s = holding("swap-2");
      s.endTurn();
      const first = firstCard(s);
      s.endTurn(); // p2's turn ends: p1's Book is not theirs to swap.
      expect(firstCard(s).id).toBe(first.id);
      s.endTurn();
      const second = firstCard(s);
      expect(second.id).not.toBe(first.id);
      expect(second.defId).not.toBe(first.defId);
      expect(OTHER_BOOKS).toContain(second.defId);
      expect(second.enchantments).toEqual([{ kind: "swapsBook", from: WILDFIRE }]);
    });

    it("R671 the Book it became has that Book's own text: Book of Flame deals its 4 to a target", () => {
      let seed = 0;
      while (seed < 200 && swappedOn(`flame-${String(seed)}`).defId !== BOOK_OF_FLAME) seed += 1;
      const s = holding(`flame-${String(seed)}`).endTurn().endTurn();
      const flame = firstCard(s);
      expect(flame.defId).toBe(BOOK_OF_FLAME);
      const before = heroOf(s.state, "p2").health;
      s.play(flame, { targets: [hero("p2")] }).expectHealth("p2", before - 4);
    });

    it("R671 its owner's view shows the Book it became with the swap riding it; the opponent's names neither", () => {
      const s = holding("swap-3").endTurn();
      const book = firstCard(s);
      const own = s.view("p1");
      const shown = Array.isArray(own.you.hand) ? own.you.hand.find((card) => card.instanceId === book.id) : undefined;
      expect(shown).toMatchObject({ defId: book.defId, enchantments: [{ kind: "swapsBook", from: WILDFIRE }] });
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(book.defId);
      expect(theirs).not.toContain("swapsBook");
    });

    it("R671, R637 a Temporary Wildfire becomes a Temporary Book, which cleanup still discards", () => {
      const s = holding("swap-4");
      s.card(WILDFIRE).grantedKeywords.push({ kind: "Temporary" });
      s.endTurn();
      expect(s.hand("p1").map((card) => card.defId)).not.toContain(WILDFIRE);
      const discarded = s.pile("p1", "graveyard");
      expect(discarded).toHaveLength(1);
      expect(OTHER_BOOKS).toContain(discarded[0]?.defId);
    });

    it("R671 its end of turn replays exactly: a JSON round trip of the state swaps to the same Book", () => {
      const s = holding("swap-5");
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = { type: "endTurn", playerId: "p1", nonce: "c55-swap" };
      const a = reduce(s.state, action);
      const b = reduce(round, action);
      expect(a.error).toBeUndefined();
      expect(hashState(b.state)).toBe(hashState(a.state));
      expect(b.state).toEqual(a.state);
    });
  });

  describe("radiant", () => {
    it("deals 8 damage to a target", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] }, p2: { field: [MENACE] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 1 });
    });

    it("deals 8 to a hero, either side", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] } });
      s.play(WILDFIRE, { targets: [hero("p1")] }).expectHealth("p1", 22);
    });

    it("R386 its declared damage steps from 8: an Upgrade makes it 9, enough to kill the 9/9", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] }, p2: { field: [MENACE] } });
      stepParam(s.card(WILDFIRE), "damage", 1);
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectInZone(MENACE, "graveyard");
    });

    it("§4.4: the Spell Damage of your side raises the Radiant hit too (8 + 2)", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER], field: [SOLARIUS] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(hitsOn(s, "hero-p2")).toEqual([10]);
    });

    it("R671 at the end of its owner's turn it becomes the Radiant face of a different Book, which keeps swapping Radiant", () => {
      const s = holding("radiant-swap-1", true);
      s.endTurn();
      const first = firstCard(s);
      expect(OTHER_BOOKS).toContain(first.defId);
      expect(first.radiant).toBe(true);
      expect(first.enchantments).toEqual([{ kind: "swapsBook", from: WILDFIRE }]);
      s.endTurn().endTurn();
      const second = firstCard(s);
      expect(second.defId).not.toBe(first.defId);
      expect(second.radiant).toBe(true);
    });

    it("R671 every Radiant pick is a Radiant Book other than Wildfire", () => {
      for (let n = 0; n < 20; n += 1) {
        const book = swappedOn(`radiant-pool-${String(n)}`, true);
        expect(OTHER_BOOKS).toContain(book.defId);
        expect(book.radiant).toBe(true);
      }
    });

    it("R671 not at the opponent's end of turn, nor outside the hand", () => {
      const s = scenario({
        p1: { hand: [FILLER], library: [FILLER, { def: WILDFIRE, radiant: true }] },
        p2: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] },
      });
      s.endTurn();
      expect(s.events.some((event) => event.type === "transformed")).toBe(false);
    });

    it("R671 its swap replays exactly from a JSON round trip", () => {
      const s = holding("radiant-swap-2", true);
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = { type: "endTurn", playerId: "p1", nonce: "c55-swap" };
      expect(reduce(round, action).state).toEqual(reduce(s.state, action).state);
    });
  });
});
