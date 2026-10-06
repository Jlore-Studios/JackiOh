// C #47 Recurring Felinor — SPEC §8.6 row 47, BUILD M9 Classic row C 47: "Cry: cast a generated Ancient
// Acquisition (C #34, base face), free and counted as played (R70), returning 2 at random (R684), the
// Spell going to your graveyard afterwards (R87); while this card is in your graveyard, whenever one
// of your Traps or Field Traps fires (`trapFired`), return this to hand (a graveyard trigger, R68); an
// opponent's trap doesn't, and in a hand or on the field it doesn't; the returned card follows R97 in
// the opponent's view; radiant 6/4: it returns and costs (0) (`costOverride`); its tuned number
// (radiant cost) reads through `param()` (R386)".
//
// C #34 Ancient Acquisition ("Return 2 random cards from your graveyard to hand") has its
// own tests. The traps that fire are Core's Sheepish (a Trap answering a played Unit) and Bread and
// Butter (a Field Trap answering a turn's end with mana unspent), and C #52 Final Gambit (a Trap that
// fires as it replaces a lethal hit).

import { cardsPlayedThisTurn, effectiveCost, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/047-recurring-felinor";

const FELINOR = "classic-047";
const ACQUISITION = "classic-034";
const GAMBIT = "classic-052";
const SHEEPISH = "core-041"; // Trap: when your opponent plays a Unit, after its Cry: transform it into a Sheep.
const BREAD = "core-018"; // Field Trap: when a turn ends with mana unspent, summon a Bread Token.
const VANILLA = "core-008";
const GARY = "core-004";
const LUNAR = "core-035";
const FILLER = "core-005";

/** p2 plays Mr. Vanilla into p1's face-down Sheepish, with p1's Recurring Felinor in its graveyard. */
function sheepishFires(felinor: { def: string; radiant?: boolean } = { def: FELINOR }, p1Hand: readonly string[] = [FILLER]): Scenario {
  const s = scenario({
    p1: { hand: p1Hand, backrow: [{ def: SHEEPISH, faceUp: false }], graveyard: [felinor] },
    p2: { hand: [VANILLA, FILLER] },
    active: "p2",
  });
  s.play(VANILLA);
  return s;
}

describe("C #47 Recurring Felinor", () => {
  it("names C #34 in its refs; a Cry and one graveyard trigger on `trapFired` on each face", () => {
    expect(def.id).toBe(FELINOR);
    expect(def.refs).toEqual([ACQUISITION]);
    expect(def.params).toEqual([{ key: "returnCost", base: 0, radiant: 0, better: "down", step: 1, min: 0 }]);
    for (const script of [base, radiant]) {
      expect(script.graveyardTriggers?.map((trigger) => trigger.on)).toEqual([["trapFired"]]);
    }
  });

  describe("base", () => {
    it("R70 Cry: casts a generated Ancient Acquisition, free and counted as played, returning 2 at random; R87 it then lands in your graveyard", () => {
      const s = scenario({
        p1: { hand: [FELINOR, FILLER], graveyard: [LUNAR, GARY, VANILLA] },
        p2: { hand: [FILLER] },
      });
      s.play(FELINOR);
      s.expectMana("p1", 2);
      expect(cardsPlayedThisTurn(s.state, "p1")).toBe(2);
      const played = s.events.flatMap((event) => (event.type === "cardPlayed" ? [event.defId] : []));
      expect(played).toEqual([FELINOR, ACQUISITION]);

      // R684: no prompt — two random cards return, one stays.
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(3);
      expect(s.pile("p1", "graveyard")).toHaveLength(2);

      const acquisition = s.pile("p1", "graveyard").find((card) => card.defId === ACQUISITION);
      expect(acquisition?.radiant).toBe(false);
      expect(acquisition?.owner).toBe("p1");
      s.expectInZone(FELINOR, "field");
    });

    it("Cry: with an empty graveyard the cast asks nothing and still lands in your graveyard", () => {
      const s = scenario({ p1: { hand: [FELINOR, FILLER] }, p2: { hand: [FILLER] } });
      s.play(FELINOR);
      expect(s.state.pending).toBeNull();
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ACQUISITION]);
    });

    it("R684 the generated cast returns at random: the same game returns the same cards", () => {
      const mk = (): Scenario =>
        scenario({ p1: { hand: [FELINOR, FILLER], graveyard: [LUNAR, GARY, VANILLA] }, p2: { hand: [FILLER] } });
      const first = mk();
      first.play(FELINOR);
      const second = mk();
      second.play(FELINOR);
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) => (event.type === "addedToHand" && event.player === "p1" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("R68 in your graveyard: when one of your Traps fires, it returns to your hand", () => {
      const s = sheepishFires();
      s.expectEvents("trapFired");
      s.expectInZone(FELINOR, "hand");
      expect(s.card(FELINOR).costOverride).toBeUndefined();
    });

    it("a Field Trap's firing counts: Bread and Butter at your turn's end returns it", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [BREAD], graveyard: [FELINOR] },
        p2: { hand: [FILLER], library: [FILLER] },
      });
      s.endTurn();
      s.expectEvents("turnEnded", "trapFired");
      s.expectInZone(FELINOR, "hand");
    });

    it("a Trap that fires as it replaces counts: Final Gambit re-aiming a lethal hit returns it", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], graveyard: [FELINOR], library: [FILLER, FILLER, FILLER], health: 4 },
        p2: { hand: [FILLER], field: [VANILLA] },
        active: "p2",
      });
      s.attack(VANILLA, "hero");
      s.expectInZone(FELINOR, "hand");
    });

    it("your opponent's Trap does not return it", () => {
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], graveyard: [FELINOR] },
        p2: { hand: [FILLER], backrow: [{ def: SHEEPISH, faceUp: false }] },
      });
      s.play(VANILLA);
      s.expectEvents("trapFired");
      s.expectInZone(FELINOR, "graveyard");
    });

    it("on the field or in a hand it does nothing when your Trap fires", () => {
      const s = scenario({
        p1: { hand: [FELINOR, FILLER], field: [{ def: FELINOR }], backrow: [{ def: SHEEPISH, faceUp: false }] },
        p2: { hand: [VANILLA, FILLER] },
        active: "p2",
      });
      const onField = s.unit("p1", 1);
      const inHand = s.hand("p1").find((card) => card.defId === FELINOR);
      s.play(VANILLA);
      s.expectEvents("trapFired");
      if (onField === null || inHand === undefined) throw new Error("both Felinors should be in place");
      s.expectInZone(onField, "field").expectInZone(inHand, "hand");
      expect(s.hand("p1")).toHaveLength(2);
    });

    it("R4 the hand cap applies: with a full hand it is burned back into your graveyard", () => {
      const ten = Array.from({ length: 10 }, () => FILLER);
      const s = sheepishFires({ def: FELINOR }, ten);
      expect(s.events.some((event) => event.type === "burned")).toBe(true);
      s.expectInZone(FELINOR, "graveyard");
      expect(s.hand("p1")).toHaveLength(10);
    });

    it("R97 once back in your hand the opponent no longer reads it", () => {
      const s = sheepishFires();
      expect(JSON.stringify(s.view("p1"))).toContain(FELINOR);
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).not.toContain(FELINOR);
      expect(theirs.opponent.graveyard.map((card) => card.defId)).not.toContain(FELINOR);
    });
  });

  describe("radiant", () => {
    it("R70 Cry: casts Ancient Acquisition on its base face — 2 random returns, not the Radiant's 4", () => {
      const s = scenario({
        p1: { hand: [{ def: FELINOR, radiant: true }, FILLER], graveyard: [LUNAR, GARY, VANILLA] },
        p2: { hand: [FILLER] },
      });
      s.play(FELINOR);
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(3);
      expect(s.pile("p1", "graveyard").find((card) => card.defId === ACQUISITION)?.radiant).toBe(false);
      s.expectStats(FELINOR, { attack: 6, health: 4 });
    });

    it("returns when one of your Traps fires, and costs (0)", () => {
      const s = sheepishFires({ def: FELINOR, radiant: true });
      const felinor = s.card(FELINOR);
      s.expectInZone(felinor, "hand");
      expect(felinor.costOverride).toBe(0);
      expect(effectiveCost(s.state, felinor)).toBe(0);
    });

    it("R97 R177 back in your hand at (0), the opponent reads neither the card nor its new cost", () => {
      const s = sheepishFires({ def: FELINOR, radiant: true });
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(FELINOR);
      expect(theirs).not.toContain(s.card(FELINOR).id);
    });

    it("R78 the (0) persists: it is played for nothing and keeps its (0) on the field", () => {
      const s = sheepishFires({ def: FELINOR, radiant: true }, [FILLER]);
      const felinor = s.card(FELINOR);
      s.endTurn(); // p2's turn ends; p1's begins.
      const mana = s.state.players.p1.mana.current;
      s.play(felinor);
      s.expectMana("p1", mana);
      expect(s.card(felinor).costOverride).toBe(0);
    });

    it("your opponent's Trap does not return it", () => {
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], graveyard: [{ def: FELINOR, radiant: true }] },
        p2: { hand: [FILLER], backrow: [{ def: SHEEPISH, faceUp: false }] },
      });
      s.play(VANILLA);
      s.expectInZone(FELINOR, "graveyard");
    });

    it("R386 a Degrade makes it return costing (1)", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: SHEEPISH, faceUp: false }], graveyard: [{ def: FELINOR, radiant: true }] },
        p2: { hand: [VANILLA, FILLER] },
        active: "p2",
      });
      stepParam(s.card(FELINOR), "returnCost", 1);
      s.play(VANILLA);
      const felinor = s.card(FELINOR);
      s.expectInZone(felinor, "hand");
      expect(effectiveCost(s.state, felinor)).toBe(1);
    });
  });
});
