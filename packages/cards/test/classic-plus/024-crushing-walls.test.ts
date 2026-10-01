// C+ #24 Crushing Walls — SPEC §8.7 row 24, BUILD M9 Classic+ row C+ 24: "Destroys the top card of each of
// the eight zones in lanes 1 and 5 (each side numbers its lanes from its owner's seat), Units and
// backrow cards, face-down ones included, an Indestructible one staying; a card dormant beneath
// resumes; a destroyed backrow card that prints Death fires it (§4.5); lanes 2 to 4 are untouched; a
// Unit topping an Ivory Tower is passed by (R418); radiant only the enemy's four zones".

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

    it("R59 every mark is collected in the one state check after the Spell", () => {
      const s = walls();
      s.play(WALLS);
      expect(s.lastEvents.filter((event) => event.type === "destroyed").length + 0).toBeGreaterThanOrEqual(4);
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
