// C+ #42.1 KY's Gift — SPEC §8.7 row 42.1, R16, R62, R97, R177, R380, R386, BUILD M9 row C+ 42.1.
// p2 is active in each scenario, so its `endTurn()` starts p1's turn, where the Gift fires.

import { HERO_HEALTH, MAX_MANA, hashState, queryCost, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, GameEvent } from "@jackioh/shared";
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
    it("R62 at the start of your turn: 1 mana above the cap, then your opponent's discard prompt", () => {
      const s = gift().endTurn();
      expect(s.state.active).toBe("p1");
      s.expectMana("p1", MAX_MANA + 1);
      const pending = s.state.pending;
      expect(pending?.kind).toBe("hand");
      expect(pending?.playerId).toBe("p2");
      expect(pending?.min).toBe(1);
      expect(pending?.max).toBe(1);
      expect(pending?.options).toHaveLength(2);
    });

    it("R16 the opponent discards the card they choose; then your hero heals 5, past 30, and the four cards arrive at (0)", () => {
      const s = gift().endTurn();
      s.answer(s.card(TIMMY).id);
      s.expectInZone(TIMMY, "graveyard").expectInZone(MENACE, "hand");
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
      s.answer(s.card(TIMMY).id);
      const health = s.state.players.p1.hero.health;
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(s.state.pending).toBeNull();
      s.expectHealth("p1", health);
    });

    it("R177 the prompt's options reach only the discarding player; the discard is public, the four cards hidden (R97)", () => {
      const s = gift().endTurn();
      expect(s.view("p1").pending).toEqual({ forYou: false, pendingFor: "p2" });
      const theirs = s.view("p2").pending;
      expect(theirs?.forYou === true ? theirs.options.length : 0).toBe(2);
      s.answer(s.card(TIMMY).id);
      const p2Events: GameEvent[] = s.view("p2").events;
      const discarded = p2Events.find((event) => event.type === "discarded");
      expect(discarded?.type === "discarded" ? discarded.defId : "").toBe(TIMMY);
      const p1Discard = s.view("p1").events.find((event) => event.type === "discarded");
      expect(p1Discard?.type === "discarded" ? p1Discard.defId : "").toBe(TIMMY);
      // The four, and the turn's draw: every card reaching p1's hand is the sentinel to p2.
      const adds = p2Events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(adds.length).toBeGreaterThanOrEqual(4);
      for (const event of adds) expect(event.type === "addedToHand" ? event.defId : "").toBe("hidden");
    });

    it("R113 §9.3 paused on the discard prompt, the start of turn resumes where it stopped and survives JSON", () => {
      const s = gift().endTurn();
      expect(s.state.players.p1.hero.health).toBe(HERO_HEALTH);
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(hashState(thawed)).toBe(hashState(s.state));
      const answer = {
        type: "answer",
        playerId: "p2",
        choiceId: s.state.pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: s.card(MENACE).id }],
        nonce: "gift-json",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(hashState(frozen.state)).toBe(hashState(live.state));
      expect(live.state.players.p1.hero.health).toBe(HERO_HEALTH + 5);
      expect(live.state.players.p1.hand.filter((card) => card.costOverride === 0)).toHaveLength(4);
    });

    it("§2.4 a full hand burns what does not fit", () => {
      const s = gift({ p1: { hand: Array.from({ length: 8 }, () => FILLER), library: [FILLER] } }).endTurn();
      s.answer(s.card(TIMMY).id);
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
      expect(s.state.pending?.max).toBe(2);
      s.answer([s.card(TIMMY).id, s.card(MENACE).id]);
      expect(s.hand("p2")).toHaveLength(0);
      s.expectHealth("p1", HERO_HEALTH + 6);
    });
  });

  describe("radiant", () => {
    it("2 mana, 2 discards of the opponent's choice, heal 10, and the four cards are Radiant", () => {
      const s = gift({ radiant: true }).endTurn();
      s.expectMana("p1", MAX_MANA + 2);
      expect(s.state.pending?.min).toBe(2);
      s.answer([s.card(TIMMY).id, s.card(MENACE).id]);
      s.expectInZone(TIMMY, "graveyard").expectInZone(MENACE, "graveyard");
      s.expectHealth("p1", HERO_HEALTH + 10);
      expectTheFour(added(s), true);
    });

    it("R16 fewer cards than asked: the opponent discards all they have", () => {
      const s = gift({ radiant: true, p2: { hand: [TIMMY], library: [FILLER] } }).endTurn();
      expect(s.state.pending?.max).toBe(1);
      s.answer(s.card(TIMMY).id);
      expect(s.hand("p2")).toHaveLength(0);
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
