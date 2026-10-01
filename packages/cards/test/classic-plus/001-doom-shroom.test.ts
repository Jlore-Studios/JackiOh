// C+ #1 Doom Shroom — SPEC §8.7 row 1, BUILD M9 Classic+ row C+ 1: "Face-down Trap that fires in §4.2
// step 4's trap window when an enemy Unit declares an attack on your hero, never on an attack on a
// unit; exiles every Unit on both sides (the tops of piles, a card dormant beneath resuming), so the
// attacker is gone and no combat resolves (R44's cancel); no Death fires and exiled tokens cease to
// exist; it goes to your graveyard as it fires and its backrow zone is Locked, so a later play into
// that zone is refused; the opponent sees only a face-down card until `trapFired` (R33, R97); radiant
// exiles enemy Units only, yours stay".
//
// Doom Shroom sits face-down in p1's backrow lane 2; p2 is active and attacks.

import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/001-doom-shroom";

const DOOM = "classicplus-001";
const TIMMY = "core-011"; // (1) 3/3 Rush, First Strike.
const VANILLA = "core-008"; // (1) 4/4.
const FIENDER = "core-092"; // (2) 5/7 Stack.
const DEFENDER = "core-003"; // (1) Taunt, Divine Shield, Reborn; Radiant: Death summons a base one.
const TOKEN = "core-t-rush"; // a unit token
const SHEEPISH = "core-041"; // (1) Trap
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], backrow: [{ def: DOOM, radiant: radiantFace, faceUp: false, lane: 2 }], ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], field: [TIMMY], ...p2 },
  });
}

function count(s: Scenario, type: string): number {
  return s.events.filter((event) => event.type === type).length;
}

describe("C+ #1 Doom Shroom", () => {
  it("is a (3) Trap; each face declares one trap trigger on `attackDeclared`", () => {
    expect(def.type).toBe("Trap");
    expect(def.cost).toBe(3);
    expect(base.triggers?.map((t) => t.on)).toEqual([["attackDeclared"]]);
    expect(radiant.triggers?.map((t) => t.on)).toEqual([["attackDeclared"]]);
  });

  describe("base", () => {
    it("R44 an enemy Unit's attack on your hero sets it off: every Unit on both sides is exiled and no combat resolves", () => {
      const s = setup({ field: [VANILLA, TIMMY] }, { field: [TIMMY, VANILLA] });
      const attacker = s.unit("p2", 1)!;
      const doom = s.card(DOOM);

      s.attack(attacker, "hero");

      s.expectEvents("attackDeclared", "trapFired", "exiled");
      expect(count(s, "exiled")).toBe(4);
      for (const lane of [1, 2]) {
        expect(s.unit("p1", lane)).toBeNull();
        expect(s.unit("p2", lane)).toBeNull();
      }
      s.expectInZone(attacker, "exile");
      s.expectHealth("p1", 30);
      expect(count(s, "damage")).toBe(0);
      s.expectInZone(doom, "graveyard");
    });

    it("never fires on an attack on a unit: the combat resolves and the trap stays set", () => {
      const s = setup({ field: [VANILLA] });
      const doom = s.card(DOOM);

      s.attack(s.unit("p2", 1)!, s.unit("p1", 1)!);

      expect(count(s, "trapFired")).toBe(0);
      expect(count(s, "damage")).toBeGreaterThan(0);
      expect(s.card(doom).zone).toMatchObject({ z: "field", row: "backrow", lane: 2 });
      expect(s.card(doom).faceUp).toBe(false);
    });

    it("never fires on its own controller's attack on the enemy hero", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [TIMMY], backrow: [{ def: DOOM, faceUp: false, lane: 2 }] },
        p2: { hand: [FILLER] },
      });

      s.attack(s.unit("p1", 1)!, "hero");

      expect(count(s, "trapFired")).toBe(0);
      s.expectHealth("p2", 27);
      expect(s.card(DOOM).zone).toMatchObject({ z: "field", row: "backrow" });
    });

    it("§3.2 R13 it exiles the tops of piles only: a card dormant beneath resumes and stays", () => {
      const s = setup({ field: [VANILLA, { def: FIENDER, stack: true }] });
      const vanilla = s.card(VANILLA);
      const fiender = s.card(FIENDER);

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectInZone(fiender, "exile");
      expect(s.unit("p1", 1)?.id).toBe(vanilla.id);
    });

    it("R11 §6.3 no Death fires, Reborn does not return, and an exiled token ceases to exist", () => {
      // The Radiant defender (Reborn; Death: summon a base one) stands on the attacker's side, where
      // its Taunt binds nobody.
      const s = setup({ field: [TOKEN] }, { field: [TIMMY, { def: DEFENDER, radiant: true }] });
      const defender = s.card(DEFENDER);
      const token = s.unit("p1", 1)!;

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectInZone(defender, "exile");
      s.expectInZone(token, "gone");
      expect(count(s, "summoned")).toBe(0);
      expect(count(s, "destroyed")).toBe(0);
      expect(s.unit("p1", 1)).toBeNull();
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("E20 its backrow zone is Locked as it fires, and a later play into that zone is refused", () => {
      const s = setup({ hand: [SHEEPISH, SHEEPISH, FILLER] });

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectEvents("trapFired", "locked");
      expect(s.view("p1").you.locks.backrow).toEqual([false, true, false, false, false]);
      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(() => s.play(SHEEPISH, { zone: 2 })).toThrow();
      s.play(SHEEPISH, { zone: 3 });
      expect(s.backrow("p1", 2)).toBeNull();
    });

    it("R33 R97 the opponent reads only a face-down card until it fires; then its graveyard names it", () => {
      const s = setup();
      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(DOOM);
      expect(JSON.stringify(s.view("p2"))).not.toContain(DOOM);

      s.attack(s.unit("p2", 1)!, "hero");

      expect(s.view("p2").opponent.graveyard.map((card) => card.defId)).toContain(DOOM);
    });
  });

  describe("radiant", () => {
    it("exiles enemy Units only: the attacker is gone, no combat, and your Units stay", () => {
      const s = setup({ field: [VANILLA, TIMMY] }, { field: [TIMMY, VANILLA] }, true);
      const attacker = s.unit("p2", 1)!;
      const mine = [s.unit("p1", 1)!, s.unit("p1", 2)!];

      s.attack(attacker, "hero");

      s.expectInZone(attacker, "exile");
      expect(s.unit("p2", 2)).toBeNull();
      expect(mine.map((card) => s.card(card).zone.z)).toEqual(["field", "field"]);
      s.expectHealth("p1", 30);
      expect(count(s, "damage")).toBe(0);
      expect(s.view("p1").you.locks.backrow[1]).toBe(true);
      s.expectInZone(DOOM, "graveyard");
    });
  });
});
