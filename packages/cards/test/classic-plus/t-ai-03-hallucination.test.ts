// T-AI-3 Hallucination — SPEC §8.7 row T-AI-3, BUILD M9 row T-AI-3: adds a copy of a random card of the
// opponent's deck to your hand, a new card you own (the original stays), carrying the original's radiant
// flag, `statsOverride` and `tuning` (R57), and gives it Brittle 2: held in your hand without ticking
// (R638), then on the field, entered on your turn t, it ticks to 1 at the start of your turn t + 2 and
// crumbles at the start of t + 4 (R385); an empty deck, nothing and no
// random draw (R129); only you learn what it copied — the opponent's view shows a card added under the
// sentinel and their library list does not change (R97, R310); radiant copies of 2 different random
// cards, both Brittle 2.

import { describe, expect, it } from "vitest";
import { HAND_CAP, HIDDEN_ID } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-03-hallucination";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const HALLUCINATION = "classicplus-t-ai-03";
const UNIT = "core-008"; // Mr. Vanilla 4/4, (1).
const OTHERS = ["core-001", "core-002", "core-011", "core-019", "core-043"];
const FILLER = "core-005";

function cast(p2: SideSetup, opts: { radiant?: boolean; seed?: string } = {}): Scenario {
  const s = scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: [{ def: HALLUCINATION, radiant: opts.radiant === true }, FILLER] },
    p2: { hand: [FILLER], ...p2 },
  });
  return s;
}

function copies(s: Scenario, before: readonly string[]): ReturnType<Scenario["hand"]> {
  return s.hand("p1").filter((card) => !before.includes(card.id));
}

function played(s: Scenario): ReturnType<Scenario["hand"]> {
  const before = s.hand("p1").map((card) => card.id);
  s.play(HALLUCINATION);
  return copies(s, before);
}

describe("T-AI-3 Hallucination", () => {
  it("is a (0) Spell, AI, Token", () => {
    expect(def).toMatchObject({ cost: 0, type: "Spell", tags: ["AI", "Token"], token: true });
    expect(base.cry).toBeDefined();
    expect(radiant.cry).toBeDefined();
  });

  describe("base", () => {
    it("R57 adds a copy of a card of the opponent's deck: a new card you own, with its radiant flag, statsOverride and tuning; the original stays", () => {
      const s = cast({ library: [{ def: UNIT, radiant: true }] });
      const [original] = s.pile("p2", "library");
      original!.tuning = { attack: 2 };
      original!.statsOverride = { attack: 1, health: 5 };
      original!.costMod = 1;
      const [copy] = played(s);
      expect(copy).toMatchObject({ defId: UNIT, owner: "p1", controller: "p1", radiant: true, costMod: 0 });
      expect(copy?.id).not.toBe(original!.id);
      expect(copy?.tuning).toEqual({ attack: 2 });
      expect(copy?.statsOverride).toEqual({ attack: 1, health: 5 });
      expect(s.pile("p2", "library").map((card) => card.id)).toEqual([original!.id]);
    });

    it("R60 the copy is of a random card of their deck, the seed's", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 16; n += 1) {
        const s = cast({ library: OTHERS }, { seed: `hallucination-${n}` });
        const [copy] = played(s);
        expect(OTHERS).toContain(copy?.defId);
        seen.add(copy!.defId);
      }
      expect(seen.size).toBeGreaterThan(2);
    });

    it("R385 R638 Brittle 2: held in the hand without ticking; on the field it ticks to 1 at t + 2 and crumbles at t + 4, not a discard", () => {
      const s = cast({ library: [UNIT], hand: [FILLER, FILLER, FILLER] });
      const t = s.state.turn;
      const [copy] = played(s);
      expect(copy?.brittle).toEqual({ count: 2, since: t });
      // R638: two whole rounds in the hand and the count has not moved, nothing crumbled.
      s.endTurn().endTurn().endTurn().endTurn();
      expect(s.state.turn).toBe(t + 4);
      expect(s.card(copy!).brittle).toEqual({ count: 2, since: t });
      s.expectInZone(copy!, "hand");
      expect(s.events.some((event) => event.type === "crumbled")).toBe(false);

      // Entering the field starts its cycle: a count of 2 at turn t + 4 ticks at t + 6 and crumbles at t + 8.
      s.play(copy!);
      expect(s.card(copy!).brittle).toEqual({ count: 2, since: t + 4 });
      s.endTurn().endTurn();
      expect(s.state.turn).toBe(t + 6);
      expect(s.card(copy!).brittle?.count).toBe(1);
      s.endTurn().endTurn();
      expect(s.state.turn).toBe(t + 8);
      s.expectInZone(copy!, "graveyard");
      expect(s.events.some((event) => event.type === "crumbled" && event.instanceId === copy!.id)).toBe(true);
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
    });

    it("R129 an empty deck gives nothing and draws nothing", () => {
      const s = cast({ library: [] });
      const cursor = s.state.rngCursor;
      expect(played(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
      s.expectInZone(HALLUCINATION, "graveyard");
    });

    it("R97 R310 only you learn what it copied: their view shows a card added under the sentinel, their library list unchanged", () => {
      const s = cast({ library: OTHERS });
      const listBefore = s.view("p2").you.ownLibrary;
      const [copy] = played(s);
      const theirs = s.view("p2").events.filter((event): event is Extract<GameEvent, { type: "addedToHand" }> => event.type === "addedToHand");
      expect(theirs).toHaveLength(1);
      expect(theirs[0]).toMatchObject({ player: "p1", instanceId: HIDDEN_ID, defId: HIDDEN_ID });
      expect(JSON.stringify(s.view("p2").events)).not.toContain(copy!.id);
      expect(s.view("p2").you.ownLibrary).toEqual(listBefore);
      const mine = s.view("p1").events.find((event) => event.type === "addedToHand");
      expect(mine).toMatchObject({ instanceId: copy!.id, defId: copy!.defId });
    });

  });

  describe("radiant", () => {
    it("R60 copies of 2 different random cards, both Brittle 2", () => {
      for (let n = 0; n < 8; n += 1) {
        const s = cast({ library: OTHERS }, { radiant: true, seed: `hallucination-r-${n}` });
        const made = played(s);
        expect(made).toHaveLength(2);
        expect(new Set(made.map((card) => card.defId)).size).toBe(2);
        expect(made.every((card) => card.brittle?.count === 2)).toBe(true);
        expect(s.pile("p2", "library")).toHaveLength(OTHERS.length);
      }
    });

    it("R586 §2.4 with room for one, the second copy is burned to your graveyard and takes no Brittle", () => {
      const s = scenario({
        p1: { hand: [{ def: HALLUCINATION, radiant: true }, ...Array.from({ length: HAND_CAP - 1 }, () => FILLER)] },
        p2: { hand: [FILLER], library: OTHERS },
      });
      const before = s.hand("p1").map((card) => card.id);
      s.play(HALLUCINATION);
      const landed = copies(s, before);
      expect(landed).toHaveLength(1);
      expect(landed[0]?.brittle?.count).toBe(2);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      const burned = s.pile("p1", "graveyard").filter((card) => OTHERS.includes(card.defId));
      expect(burned).toHaveLength(1);
      expect(burned[0]?.brittle).toBeUndefined();
      s.expectEvents("burned");
    });

    it("R129 a deck of one card gives one copy, with no draw", () => {
      const s = cast({ library: [UNIT] }, { radiant: true });
      const cursor = s.state.rngCursor;
      expect(played(s).map((card) => card.defId)).toEqual([UNIT]);
      expect(s.state.rngCursor).toBe(cursor);
    });
  });
});
