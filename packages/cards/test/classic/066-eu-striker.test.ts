// C #66 EU Striker — SPEC §8.6 row 66, BUILD M9 Classic row C 66: "From your hand: after you play a Unit
// and it resolves, it is summoned into your leftmost open zone (no Cry, summoning sick; a full board →
// it stays in hand); on the field: after you play any card, it returns to your hand (R78 reset);
// neither answers the play that moved it (R401, R119): the Unit that summons it doesn't bounce it, the
// card that bounces it doesn't summon it back, and its own play doesn't bounce it; a cast is a play
// (R70) and a countered card is not; the opponent's plays do nothing; the opponent's view never names
// it in your hand, and the summon is the first they see of it; radiant 10/8 Rush; no tuned numbers".
//
// R548: both triggers answer a play of yours once it has resolved (§10.5 step 7's `cardResolved`).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/066-eu-striker";
import { scenario, type Scenario } from "../_harness";

const STRIKER = "classic-066";
const VANILLA = "core-008"; // (1) Unit 4/4
const BIG_FELINOR = "core-043"; // (4) Unit: Cry: Destroy all non-Felinor Units.
const STOCKPILE = "core-005"; // (1) Spell
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw
const GRAND = "classic-072"; // Trap: counters the opponent's non-Unit plays.
const FILLER = "core-010"; // (0) Spell
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const LOCKDOWN = "classic-084"; // After a permanent is played, Lock its zone.

function summoned(s: Scenario, id: string): GameEvent[] {
  return s.lastEvents.filter((event) => event.type === "summoned" && event.instanceId === id);
}

describe("C #66 EU Striker", () => {
  it("is a (2) 5/4 Human Unit (10/8 Rush Radiant), a hand trigger and a field trigger on `cardResolved`", () => {
    expect(def.cost).toBe(2);
    expect(def.tags).toEqual(["Human"]);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([5, 4, 10, 8]);
    expect(def.base.keywords).toEqual([]);
    expect(def.radiant.keywords).toEqual([{ kind: "Rush" }]);
    expect(def.params).toBeUndefined();
    expect(base.handTriggers?.map((trigger) => trigger.on)).toEqual([["cardResolved"]]);
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["cardResolved"]]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R548 from your hand: after you play a Unit, it is summoned into your leftmost open zone, no Cry", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER] } });
      const striker = s.card(STRIKER);
      s.play(VANILLA);
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
      expect(s.unit("p1", 2)?.id).toBe(striker.id);
      expect(summoned(s, striker.id)).toHaveLength(1);
      // A summon is no play.
      expect(s.lastEvents.some((event) => event.type === "cardPlayed" && event.instanceId === striker.id)).toBe(false);
    });

    it("R64 the leftmost open zone, a gap to the left of the played Unit included", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER], field: [{ def: VANILLA, lane: 2 }] } });
      const striker = s.card(STRIKER);
      const played = s.hand("p1").find((card) => card.defId === VANILLA);
      if (played === undefined) throw new Error("setup");
      s.play(played, { zone: 4 });
      expect(s.unit("p1", 1)?.id).toBe(striker.id);
      expect(s.unit("p1", 4)?.id).toBe(played.id);
    });

    it("R64 a Locked zone is passed over: C #84 Lockdown's lock on an emptied lane 1 sends it to lane 2", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, HIT_JOB, VANILLA], backrow: [LOCKDOWN], mana: 9 } });
      const striker = s.card(STRIKER);
      const [first, second] = s.hand("p1").filter((card) => card.defId === VANILLA);
      if (first === undefined || second === undefined) throw new Error("setup");
      // The first Vanilla's lane Locks; the Striker it summons is no play and takes lane 2, unlocked.
      s.play(first, { zone: 1 });
      expect(s.unit("p1", 2)?.id).toBe(striker.id);
      // Hit Job clears lane 1 (still Locked) and bounces the Striker; the next Unit summons it again.
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: first.id }] });
      s.expectInZone(striker, "hand");
      s.play(second, { zone: 4 });
      expect(s.unit("p1", 1)).toBeNull();
      expect(s.unit("p1", 2)?.id).toBe(striker.id);
    });

    it("§4.1 it is summoning sick: it cannot attack the turn it arrives", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER] }, p2: { field: [VANILLA], hand: [FILLER] } });
      s.play(VANILLA);
      const striker = s.card(STRIKER);
      expect(() => s.attack(striker, s.unit("p2", 1) ?? "hero")).toThrow(/summoning sick/);
    });

    it("R548 after the Unit resolves: its Cry happens first, so Big Felinor's sweep does not reach it", () => {
      const s = scenario({ p1: { hand: [STRIKER, BIG_FELINOR, FILLER] } });
      s.play(BIG_FELINOR);
      s.expectInZone(STRIKER, "field");
      expect(s.unit("p1", 2)?.defId).toBe(STRIKER);
    });

    it("R64 with no open unit zone it stays in your hand", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER], field: [VANILLA, VANILLA, VANILLA, VANILLA] } });
      s.play(VANILLA);
      s.expectInZone(STRIKER, "hand");
    });

    it("R548 on the field: after you play any card, it returns to your hand, reset (R78)", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [{ def: STRIKER, damage: 2 }], library: [VANILLA, VANILLA] } });
      s.expectStats(STRIKER, { health: 2 });
      s.play(STOCKPILE);
      const striker = s.card(STRIKER);
      s.expectInZone(striker, "hand");
      expect(striker.damage).toBe(0);
      s.expectEvents("cardResolved", "bounced");
    });

    it("R401 R548 the Unit that summons it doesn't bounce it", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER] } });
      s.play(VANILLA);
      s.expectInZone(STRIKER, "field");
    });

    it("R401 R548 the card that bounces it doesn't summon it back: a Unit play returns it, and it stays in hand", () => {
      const s = scenario({ p1: { hand: [VANILLA, FILLER, VANILLA], field: [STRIKER] } });
      const [first, second] = s.hand("p1").filter((card) => card.defId === VANILLA);
      if (first === undefined || second === undefined) throw new Error("setup");
      s.play(first);
      s.expectInZone(STRIKER, "hand");
      // A Spell played next is no Unit: it stays in hand.
      s.play(FILLER);
      s.expectInZone(STRIKER, "hand");
      // The next Unit you play summons it again.
      s.play(second);
      s.expectInZone(STRIKER, "field");
    });

    it("R401 R548 a Unit play bounces the one on the field and summons the one in hand, and neither comes back", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER], field: [STRIKER] } });
      const onField = s.unit("p1", 1);
      const inHand = s.hand("p1").find((card) => card.defId === STRIKER);
      if (onField === null || inHand === undefined) throw new Error("setup");
      s.play(VANILLA);
      s.expectInZone(onField, "hand");
      s.expectInZone(inHand, "field");
    });

    it("R119 R401 its own play doesn't bounce it", () => {
      const s = scenario({ p1: { hand: [STRIKER, FILLER] } });
      s.play(STRIKER);
      s.expectInZone(STRIKER, "field");
    });

    it("R70 a cast is a play: a Spell cast on draw returns it to your hand", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [STRIKER], library: [CN_VIRUS, VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      s.expectInZone(STRIKER, "hand");
      expect(s.lastEvents.some((event) => event.type === "cardResolved" && event.defId === CN_VIRUS)).toBe(true);
    });

    it("a countered card is no play: under the opponent's Grand Counterspell it stays on the field", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], field: [STRIKER] },
        p2: { backrow: [GRAND], hand: [FILLER] },
      });
      s.play(STOCKPILE);
      expect(s.lastEvents.some((event) => event.type === "countered")).toBe(true);
      s.expectInZone(STRIKER, "field");
    });

    it("the opponent's plays do nothing: their Unit leaves yours in hand, their card leaves yours on the field", () => {
      const s = scenario({
        p1: { hand: [STRIKER, FILLER], field: [STRIKER] },
        p2: { hand: [VANILLA, STOCKPILE, FILLER] },
        active: "p2",
      });
      s.play(VANILLA);
      s.play(STOCKPILE);
      expect(s.hand("p1").filter((card) => card.defId === STRIKER)).toHaveLength(1);
      expect(s.unit("p1", 1)?.defId).toBe(STRIKER);
    });

    it("§2.4 the hand cap: returned to a full hand, it burns into your graveyard", () => {
      const nine = Array.from({ length: 9 }, () => FILLER);
      const s = scenario({ p1: { hand: [STOCKPILE, ...nine], field: [STRIKER], library: [VANILLA, VANILLA] } });
      // Stockpile leaves the hand (9 left) and draws 2 (10, then a burn); the return finds the hand full.
      s.play(STOCKPILE);
      s.expectInZone(STRIKER, "graveyard");
    });

    it("R97 in your hand the opponent's view never names it; the summon is the first they see of it", () => {
      const s = scenario({ p1: { hand: [STRIKER, VANILLA, FILLER] } });
      const striker = s.card(STRIKER);
      expect(JSON.stringify(s.view("p2"))).not.toContain(STRIKER);
      s.play(VANILLA);
      const theirs = s.view("p2");
      expect(theirs.opponent.units[1]?.defId).toBe(STRIKER);
      const summonEvent = theirs.events.find((event) => event.type === "summoned" && event.instanceId === striker.id);
      expect(summonEvent).toBeDefined();
    });
  });

  describe("radiant", () => {
    it("is a 10/8 with Rush, summoned from your hand after a Unit, and may attack a Unit at once", () => {
      const s = scenario({ p1: { hand: [{ def: STRIKER, radiant: true }, VANILLA, FILLER] }, p2: { field: [VANILLA], hand: [FILLER] } });
      s.play(VANILLA);
      const striker = s.card(STRIKER);
      s.expectStats(striker, { attack: 10, health: 8 });
      expect(s.stats(striker).keywords.map((keyword) => keyword.kind)).toEqual(["Rush"]);
      const enemy = s.unit("p2", 1);
      if (enemy === null) throw new Error("no enemy");
      s.attack(striker, enemy);
      s.expectInZone(enemy, "graveyard");
    });

    it("R401 R548 the same two triggers: after you play a card it returns to your hand, and its own play doesn't", () => {
      const s = scenario({ p1: { hand: [{ def: STRIKER, radiant: true }, STOCKPILE, FILLER], library: [VANILLA, VANILLA] } });
      s.play(STRIKER);
      s.expectInZone(STRIKER, "field");
      s.play(STOCKPILE);
      s.expectInZone(STRIKER, "hand");
    });
  });
});
