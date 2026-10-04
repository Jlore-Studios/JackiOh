// C+ #18 Gullible Treatler — SPEC §8.7 row 18, BUILD M9 Classic+ row C+ 18: "At its controller's start
// of turn, if they control no Field Spell, Trap or Field Trap (face-down cards count; an animated one
// standing in a unit zone is a Unit and doesn't, §6.1), it Tributes itself: a Sacrifice that bypasses
// Indestructible and counts as a death; with one it stays; nothing at the opponent's start of turn;
// `conditionMet` on the field answers whether it would be Tributed now (R195); radiant only when no
// player controls one".
//
// R195's proofs for this card are in its own file (the `describe("R195 conditionMet …")` block).

import type { PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/018-gullible-treatler";

const TREATLER = "classicplus-018";
const FIELD_SPELL = "core-064"; // Gifted Program
const TRAP = "core-060"; // Bear Honeypot
const FIELD_TRAP = "core-018"; // Bread and Butter
const FROSTSPATULA = "classicplus-012-8"; // a Field Spell, "Animated on your turn"
const TOWER = "classicplus-033"; // Ivory Tower, a Field Spell that fuses the first Unit stacked onto it
const VANILLA = "core-008";
const FILLER = "core-005";
const DECK = ["core-005", "core-005", "core-005", "core-005"];

function board(p1: SideSetup, p2: SideSetup = {}, active: PlayerId = "p1"): Scenario {
  return scenario({
    active,
    p1: { hand: [FILLER], library: DECK, ...p1 },
    p2: { hand: [FILLER], library: DECK, ...p2 },
  });
}

function treatlerAt(s: Scenario): ReturnType<Scenario["card"]> | null {
  const unit = s.unit("p1", 1);
  return unit !== null && unit.defId === TREATLER ? unit : null;
}

/** p1's Treatler in lane 1, face as given, plus p1's other field and p2's. */
function withTreatler(radiantFace: boolean, p1Field: SideSetup["field"] = [], p2: SideSetup = {}, active: PlayerId = "p1"): Scenario {
  return board({ field: [{ def: TREATLER, lane: 1, radiant: radiantFace }, ...(p1Field ?? [])] }, p2, active);
}

function glows(s: Scenario): boolean {
  return s.view("p1").you.units[0]?.conditionActive === true;
}

describe("C+ #18 Gullible Treatler", () => {
  it("is a (2) 8/9 Human Unit with a start-of-turn hook and no play-time choice", () => {
    expect(def.id).toBe(TREATLER);
    expect(def.cost).toBe(2);
    expect(def.tags).toContain("Human");
    expect(base.startOfTurn).toBeDefined();
    expect(radiant.startOfTurn).toBeDefined();
    expect(base.targets).toBeUndefined();
  });

  describe("base", () => {
    it("with no Field Spell or Trap of yours, it Tributes itself at your start of turn: a death with no killer", () => {
      const s = withTreatler(false, [], { backrow: [{ def: FIELD_SPELL }] });
      const treatler = treatlerAt(s);
      if (treatler === null) throw new Error("no Treatler");
      const destroyedBefore = s.state.counters.destroyed;
      s.startTurn();
      s.expectInZone(treatler, "graveyard");
      const death = s.lastEvents.find((event) => event.type === "destroyed" && event.instanceId === treatler.id);
      expect(death).toMatchObject({ type: "destroyed", killerId: null });
      expect(s.state.counters.destroyed).toBe(destroyedBefore + 1);
    });

    it("a Field Spell of yours keeps it", () => {
      const s = withTreatler(false, [{ def: FIELD_SPELL, row: "backrow" }]);
      s.startTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });

    it("a face-down Trap of yours keeps it", () => {
      const s = withTreatler(false, [{ def: TRAP, row: "backrow", faceUp: false }]);
      s.startTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });

    it("a Field Trap counts as a Trap", () => {
      const s = withTreatler(false, [{ def: FIELD_TRAP, row: "backrow", faceUp: false }]);
      s.startTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });

    it("R383 an animated Field Spell standing in a unit zone is a Unit and does not count", () => {
      // Frostspatula animates at its controller's start of turn, before the start-of-turn triggers.
      const s = withTreatler(false, [{ def: FROSTSPATULA, row: "backrow", lane: 3 }]);
      const treatler = treatlerAt(s);
      if (treatler === null) throw new Error("no Treatler");
      s.startTurn();
      const animated = [1, 2, 3, 4, 5].some((lane) => s.unit("p1", lane)?.defId === FROSTSPATULA);
      expect(animated).toBe(true);
      s.expectInZone(treatler, "graveyard");
    });

    it("R418 an Ivory Tower that has fused a Unit in is still a Field Spell you control: it keeps it", () => {
      const s = board({ hand: [VANILLA, FILLER], field: [{ def: TREATLER, lane: 1 }], backrow: [{ def: TOWER, lane: 2 }] });
      const tower = s.card(TOWER).id;
      const rider = s.card(VANILLA).id;
      s.play(rider, { zone: 2, row: "backrow" });
      // R651: the Vanilla is fused into the Tower, which stays a Field Spell in its backrow zone.
      s.expectInZone(rider, "gone");
      expect(s.backrow("p1", 2)?.id).toBe(tower);
      expect(glows(s)).toBe(false);
      s.endTurn().endTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });

    it("§6.3 the Tribute is a Sacrifice: Indestructible does not save it", () => {
      const s = withTreatler(false);
      const treatler = treatlerAt(s);
      if (treatler === null) throw new Error("no Treatler");
      s.card(treatler).grantedKeywords.push({ kind: "Indestructible" });
      expect(s.stats(treatler).keywords.some((keyword) => keyword.kind === "Indestructible")).toBe(true);
      s.startTurn();
      s.expectInZone(treatler, "graveyard");
    });

    it("an enemy Field Spell does not keep the base face", () => {
      const s = withTreatler(false, [], { backrow: [{ def: FIELD_SPELL }, { def: TRAP, faceUp: false }] });
      const treatler = treatlerAt(s);
      if (treatler === null) throw new Error("no Treatler");
      s.startTurn();
      s.expectInZone(treatler, "graveyard");
    });

    it("nothing happens at the opponent's start of turn", () => {
      const s = withTreatler(false, [], {}, "p2");
      s.startTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });
  });

  describe("radiant", () => {
    it("an enemy Field Spell or face-down Trap keeps it", () => {
      const spell = withTreatler(true, [], { backrow: [{ def: FIELD_SPELL }] });
      spell.startTurn();
      expect(treatlerAt(spell)).not.toBeNull();

      const trap = withTreatler(true, [], { backrow: [{ def: TRAP, faceUp: false }] });
      trap.startTurn();
      expect(treatlerAt(trap)).not.toBeNull();
    });

    it("with no player controlling one, it Tributes itself", () => {
      const s = withTreatler(true);
      const treatler = treatlerAt(s);
      if (treatler === null) throw new Error("no Treatler");
      s.expectStats(treatler, { attack: 16, health: 18 });
      s.startTurn();
      s.expectInZone(treatler, "graveyard");
    });

    it("a Field Spell of its own keeps it too", () => {
      const s = withTreatler(true, [{ def: FIELD_SPELL, row: "backrow" }]);
      s.startTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });
  });

  describe("R195 conditionMet on the field: whether it would be Tributed now", () => {
    it("base: lit with no Field Spell or Trap of yours, and the start of turn Tributes it", () => {
      const s = withTreatler(false, [], { backrow: [{ def: FIELD_SPELL }] });
      expect(base.conditionMet).toBeDefined();
      expect(glows(s)).toBe(true);
      s.endTurn().endTurn();
      expect(treatlerAt(s)).toBeNull();
    });

    it("base: unlit with a Trap of yours, and the start of turn keeps it", () => {
      const s = withTreatler(false, [{ def: TRAP, row: "backrow", faceUp: false }]);
      expect(glows(s)).toBe(false);
      s.endTurn().endTurn();
      expect(treatlerAt(s)).not.toBeNull();
    });

    it("radiant: unlit while the opponent controls one, lit when nobody does", () => {
      const kept = withTreatler(true, [], { backrow: [{ def: TRAP, faceUp: false }] });
      expect(glows(kept)).toBe(false);
      kept.endTurn().endTurn();
      expect(treatlerAt(kept)).not.toBeNull();

      const gone = withTreatler(true);
      expect(glows(gone)).toBe(true);
      gone.endTurn().endTurn();
      expect(treatlerAt(gone)).toBeNull();
    });

    it("never lit on the opponent's view, nor in the hand", () => {
      const s = board({ hand: [TREATLER, FILLER] }, { field: [{ def: TREATLER, lane: 1 }] });
      expect(s.view("p1").opponent.units[0]?.conditionActive).toBeUndefined();
      const inHand = s.view("p1").you.hand;
      if (!Array.isArray(inHand)) throw new Error("own hand is a list");
      expect(inHand.find((card) => card.defId === TREATLER)?.conditionActive).toBeUndefined();
    });
  });
});
