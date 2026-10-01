// C+ #24 Crushing Walls — SPEC §8.7 row 24, BUILD M9 Classic+ row C+ 24: "Destroys the top card of each of
// the eight zones in lanes 1 and 5 (each side numbers its lanes from its owner's seat), Units and
// backrow cards, face-down ones included, an Indestructible one staying; a card dormant beneath
// resumes; a destroyed backrow card that prints Death fires it (§4.5); lanes 2 to 4 are untouched; a
// Unit topping an Ivory Tower is passed by (R418); radiant only the enemy's four zones".

import { newInstance, placeOnField, registerScripts, registeredScripts } from "@jackioh/engine";
import { draw } from "@jackioh/engine/effects";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/024-crushing-walls";

const WALLS = "classicplus-024";
const BODY = "core-008"; // 4/4
const ROCK = "core-066"; // Indestructible
const FIELD_SPELL = "core-064";
const TRAP = "core-060";
const FIENDER = "core-092"; // Stack
const TOP = "classicplus-019-1"; // Radiant: Immune to Spells
const FROST = "classicplus-012-8"; // Field Spell, Animated on your turn
const TOWER = "classicplus-033"; // Ivory Tower: a Unit may be played on top of it
const FILLER = "core-005";

function walls(radiantFace = false): Scenario {
  return scenario({
    p1: {
      hand: [{ def: WALLS, radiant: radiantFace }, FILLER],
      field: [{ def: BODY, lane: 1 }, { def: BODY, lane: 3 }, { def: BODY, lane: 5 }],
      backrow: [{ def: FIELD_SPELL, lane: 1 }, { def: TRAP, lane: 5, faceUp: false }, { def: FIELD_SPELL, lane: 2 }],
    },
    p2: {
      hand: [FILLER],
      field: [{ def: BODY, lane: 1 }, { def: BODY, lane: 2 }, { def: BODY, lane: 5 }],
      backrow: [{ def: TRAP, lane: 1, faceUp: false }, { def: FIELD_SPELL, lane: 5 }, { def: TRAP, lane: 4, faceUp: false }],
    },
  });
}

function occupied(s: Scenario, player: "p1" | "p2"): { units: boolean[]; backrow: boolean[] } {
  return {
    units: [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane) !== null),
    backrow: [1, 2, 3, 4, 5].map((lane) => s.backrow(player, lane) !== null),
  };
}

describe("C+ #24 Crushing Walls", () => {
  it("is a (3) Spell with no play-time choice", () => {
    expect(def.cost).toBe(3);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("§3.1 destroys the top card of all eight zones in lanes 1 and 5, face-down ones included; lanes 2 to 4 untouched", () => {
      const s = walls();
      s.play(WALLS);
      expect(occupied(s, "p1")).toEqual({ units: [false, false, true, false, false], backrow: [false, true, false, false, false] });
      expect(occupied(s, "p2")).toEqual({ units: [false, true, false, false, false], backrow: [false, false, false, true, false] });
    });

    it("R59 all eight cards are destroyed at once, in one pass", () => {
      const s = walls();
      s.play(WALLS);
      const types = s.lastEvents.map((event) => event.type);
      expect(types.filter((type) => type === "destroyed")).toHaveLength(8);
      // One pass: the eight deaths are reported together, before anything else happens.
      const first = types.indexOf("destroyed");
      expect(types.slice(first, first + 8)).toEqual(Array.from({ length: 8 }, () => "destroyed"));
    });

    it("§4.5 a destroyed backrow card that prints Death fires it", () => {
      const s = scenario({ p1: { hand: [WALLS, FILLER], library: [FILLER, FILLER] }, p2: { hand: [FILLER] } });
      // A test-only Field Spell whose Death draws its controller a card (the real ones, C+ #12.8 and #61,
      // are other units' cards).
      const id = "test-backrow-death";
      const face = { keywords: [], text: "Death: Draw 1." };
      s.state.transientDefs[id] = { id, index: id, name: id, set: "Core", type: "Field Spell", tags: [], rarity: "Common", token: false, cost: 0, base: face, radiant: face };
      registerScripts({ ...registeredScripts(), [id]: { base: { death: () => [draw({ count: 1 })] }, radiant: { death: () => [draw({ count: 1 })] } } });
      const card = newInstance(s.state, id, "p1", { z: "hand", player: "p1" });
      if (!placeOnField(s.state, card, { player: "p1", row: "backrow", lane: 5 })) throw new Error("setup");
      const handBefore = s.hand("p1").length;
      s.play(WALLS);
      s.expectInZone(card, "graveyard");
      // Walls left the hand, the Death drew one.
      expect(s.hand("p1")).toHaveLength(handBefore);
      expect(s.lastEvents.some((event) => event.type === "drawn")).toBe(true);
    });

    it("R418 a Unit topping an Ivory Tower is passed by, while the Tower beneath is destroyed and the Unit steps down", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [WALLS, FILLER], library: [FILLER, FILLER] },
        p2: { hand: [BODY, FILLER], backrow: [{ def: TOWER, lane: 1 }], library: [FILLER, FILLER] },
      });
      const tower = s.backrow("p2", 1);
      const rider = s.card(BODY);
      if (tower === null) throw new Error("setup");
      s.play(BODY, { zone: 1, row: "backrow" });
      expect(s.card(rider).zone).toMatchObject({ z: "field", row: "backrow", lane: 1 });
      s.endTurn();
      s.play(WALLS);
      s.expectInZone(tower, "graveyard");
      expect(s.lastEvents.some((event) => event.type === "destroyed" && event.instanceId === rider.id)).toBe(false);
      expect(s.card(rider).zone).toMatchObject({ z: "field", player: "p2", row: "units", lane: 1 });
    });

    it("R46 an Indestructible one stays, knocked to Attack Position", () => {
      const s = scenario({ p1: { hand: [WALLS, FILLER] }, p2: { hand: [FILLER], field: [{ def: ROCK, lane: 5, position: "DEF" }] } });
      s.play(WALLS);
      expect(s.unit("p2", 5)?.defId).toBe(ROCK);
      expect(s.stats(s.unit("p2", 5) ?? "").position).toBe("ATK");
    });

    it("§3.2 a card dormant beneath resumes", () => {
      const s = scenario({
        p1: { hand: [WALLS, FILLER] },
        p2: { hand: [FILLER], field: [{ def: BODY, lane: 1 }, { def: FIENDER, lane: 1, stack: true }] },
      });
      const beneath = s.state.players.p2.units[0]?.[1];
      s.play(WALLS);
      expect(s.unit("p2", 1)?.id).toBe(beneath?.id);
    });

    it("§6.1 a Unit immune to Spells is passed by", () => {
      const s = scenario({ p1: { hand: [WALLS, FILLER] }, p2: { hand: [FILLER], field: [{ def: TOP, lane: 1, radiant: true }] } });
      s.play(WALLS);
      expect(s.unit("p2", 1)?.defId).toBe(TOP);
    });

    it("R383 an Animated card is hit where it stands: in its backrow zone on the opponent's turn", () => {
      const s = scenario({ p1: { hand: [WALLS, FILLER] }, p2: { hand: [FILLER], backrow: [{ def: FROST, lane: 5 }] } });
      const frost = s.backrow("p2", 5);
      s.play(WALLS);
      // A Field Spell in its backrow zone, not a Unit token: R11 sends it to the graveyard.
      s.expectInZone(frost ?? "", "graveyard");
    });
  });

  describe("radiant", () => {
    it("destroys only the enemy's four zones in lanes 1 and 5", () => {
      const s = walls(true);
      s.play(WALLS);
      expect(occupied(s, "p1")).toEqual({ units: [true, false, true, false, true], backrow: [true, true, false, false, true] });
      expect(occupied(s, "p2")).toEqual({ units: [false, true, false, false, false], backrow: [false, false, false, true, false] });
    });
  });
});
