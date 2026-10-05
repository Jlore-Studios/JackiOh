// C+ #42.1 KY's Gift — SPEC §8.7 row 42.1, R662, R62, R97, R177, R380, R386, BUILD M9 row C+ 42.1.
// p2 is active in each scenario, so its `endTurn()` starts p1's turn, where the Gift fires.

import { HERO_HEALTH, MAX_MANA, queryCost, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/index";
import { base, def, radiant } from "../../src/scripts/classic-plus/042-1-kys-gift";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const GIFT = "classicplus-042-1";
const FILLER = "core-005";
const MENACE = "core-019";
const TIMMY = "core-011";

/** p1's Gift in the backrow, p2 to move; `endTurn()` hands p1 its turn. */
function gift(opts: { radiant?: boolean; p1?: SideSetup; p2?: SideSetup } = {}): Scenario {
  return scenario({
    active: "p2",
    p1: { backrow: [{ def: GIFT, radiant: opts.radiant === true }], library: [FILLER, FILLER], hand: [FILLER], ...opts.p1 },
    p2: { hand: [MENACE, TIMMY], library: [FILLER, FILLER], ...opts.p2 },
  });
}

/** The cards the Gift added: p1's hand cards carrying `costOverride` 0. */
function added(s: Scenario): { defId: string; radiant: boolean }[] {
  return s
    .hand("p1")
    .filter((card) => card.costOverride === 0)
    .map((card) => ({ defId: card.defId, radiant: card.radiant }));
}

function expectTheFour(cards: { defId: string; radiant: boolean }[], radiantFace: boolean): void {
  expect(cards).toHaveLength(4);
  const [book, ky, legendary, four] = cards.map((card) => cardDef(card.defId));
  expect(book?.tags).toContain("Book");
  expect(ky?.tags).toContain("KY");
  expect(legendary?.rarity).toBe("Legendary");
  expect(queryCost(four ?? cardDef(FILLER))).toBe(4);
  for (const card of cards) {
    expect(cardDef(card.defId).token).toBe(false);
    expect(card.radiant).toBe(radiantFace);
  }
}

describe("C+ #42.1 KY's Gift", () => {
  it("is a start-of-turn script on both faces, its numbers the declared mana, discards and heal", () => {
    expect(def.id).toBe(GIFT);
    expect(def.token).toBe(true);
    expect(def.printedRarity).toBe("Legendary");
    expect(radiant).toBe(base);
    expect(def.params?.map((entry) => [entry.key, entry.base, entry.radiant])).toEqual([
      ["mana", 1, 2],
      ["discards", 1, 2],
      ["heal", 5, 10],
    ]);
  });

  describe("base", () => {
    it("R62 at the start of your turn: 1 mana above the cap, then the random discard lands with no prompt", () => {
      const s = gift().endTurn();
      expect(s.state.active).toBe("p1");
      s.expectMana("p1", MAX_MANA + 1);
      // R662: no prompt opens — the random discard landed and the rest followed.
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toHaveLength(1);
      expect(s.pile("p2", "graveyard")).toHaveLength(1);
      s.expectHealth("p1", HERO_HEALTH + 5);
      expectTheFour(added(s), false);
    });

    it("R662 the opponent discards 1 at random; then your hero heals 5, past 30, and the four cards arrive at (0)", () => {
      const s = gift().endTurn();
      const grave = s.pile("p2", "graveyard").map((card) => card.defId);
      expect(grave).toHaveLength(1);
      expect([MENACE, TIMMY]).toContain(grave[0]);
      expect(s.hand("p2")).toHaveLength(1);
      s.expectHealth("p1", HERO_HEALTH + 5);
      expectTheFour(added(s), false);
      s.expectEvents("discarded", "healed", "addedToHand");
    });

    it("R380 the four come from every set's non-token cards, and over seeds reach beyond Core", () => {
      const sets = new Set<string>();
      for (let n = 0; n < 12; n += 1) {
        const s = scenario({
          seed: `kys-gift-sets-${n}`,
          active: "p2",
          p1: { backrow: [GIFT], library: [FILLER], hand: [FILLER] },
          p2: { hand: [], library: [FILLER] },
        }).endTurn();
        for (const card of added(s)) sets.add(cardDef(card.defId).set);
      }
      expect(sets.has("Classic") || sets.has("Classic+")).toBe(true);
    });

    it("R16 an empty opponent hand asks nothing, and the rest still happens", () => {
      const s = gift({ p2: { hand: [], library: [FILLER] } }).endTurn();
      expect(s.state.pending).toBeNull();
      s.expectHealth("p1", HERO_HEALTH + 5);
      expectTheFour(added(s), false);
    });

    it("R62 it does nothing at the opponent's start of turn", () => {
      const s = gift().endTurn();
      const health = s.state.players.p1.hero.health;
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(s.state.pending).toBeNull();
      s.expectHealth("p1", health);
    });

    it("R177 R662 no prompt opens: the opponent's remaining hand stays hidden, the discard is public, the four cards hidden (R97)", () => {
      const s = gift().endTurn();
      expect(s.state.pending).toBeNull();
      expect(s.view("p1").pending).toBeNull();
      const mine = JSON.stringify(s.view("p1"));
      for (const card of s.hand("p2")) {
        expect(mine).not.toContain(`"${card.id}"`);
        expect(mine).not.toContain(card.defId);
      }
      const p2Events: GameEvent[] = s.view("p2").events;
      const discarded = p2Events.find((event) => event.type === "discarded");
      expect(discarded?.type === "discarded" && [MENACE, TIMMY].includes(discarded.defId)).toBe(true);
      const p1Discard = s.view("p1").events.find((event) => event.type === "discarded");
      expect(p1Discard?.type === "discarded" && [MENACE, TIMMY].includes(p1Discard.defId)).toBe(true);
      // The four, and the turn's draw: every card reaching p1's hand is the sentinel to p2.
      const adds = p2Events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(adds.length).toBeGreaterThanOrEqual(4);
      for (const event of adds) expect(event.type === "addedToHand" ? event.defId : "").toBe("hidden");
    });

    it("R662 the random discard comes from the match rng: the same game discards the same card", () => {
      const first = gift().endTurn();
      const second = gift().endTurn();
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) => (event.type === "discarded" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("§2.4 a full hand burns what does not fit", () => {
      const s = gift({ p1: { hand: Array.from({ length: 8 }, () => FILLER), library: [FILLER] } }).endTurn();
      // Start-of-turn triggers come before the draw (§2): two of the four fit beside the 8, two are
      // burned, and then the turn's draw is burned too.
      expect(s.hand("p1")).toHaveLength(10);
      expect(added(s)).toHaveLength(2);
      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(3);
    });

    it("R386 its numbers read through param(): an Upgrade of each", () => {
      const s = gift();
      const card = s.card(GIFT);
      stepParam(card, "mana", 1);
      stepParam(card, "discards", 1);
      stepParam(card, "heal", 1);
      s.endTurn();
      s.expectMana("p1", MAX_MANA + 2);
      expect(s.state.pending).toBeNull();
      expect(s.pile("p2", "graveyard")).toHaveLength(2);
      expect(s.hand("p2")).toHaveLength(0);
      s.expectHealth("p1", HERO_HEALTH + 6);
    });
  });

  describe("radiant", () => {
    it("2 mana, 2 random discards, heal 10, and the four cards are Radiant", () => {
      const s = gift({ radiant: true }).endTurn();
      s.expectMana("p1", MAX_MANA + 2);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toHaveLength(0);
      expect(s.pile("p2", "graveyard")).toHaveLength(2);
      s.expectHealth("p1", HERO_HEALTH + 10);
      expectTheFour(added(s), true);
    });

    it("R662 fewer cards than asked: the opponent discards all they have", () => {
      const s = gift({ radiant: true, p2: { hand: [TIMMY], library: [FILLER] } }).endTurn();
      expect(s.state.pending).toBeNull();
      expect(s.hand("p2")).toHaveLength(0);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([TIMMY]);
      s.expectHealth("p1", HERO_HEALTH + 10);
    });

    it("R386 a Degrade steps the Radiant heal from 10 to 9", () => {
      const s = gift({ radiant: true, p2: { hand: [], library: [FILLER] } });
      stepParam(s.card(GIFT), "heal", -1);
      s.endTurn();
      s.expectHealth("p1", HERO_HEALTH + 9);
    });
  });
});
