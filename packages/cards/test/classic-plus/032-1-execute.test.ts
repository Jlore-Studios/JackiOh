// C+ #32.1 Execute — SPEC §8.7 row 32.1, BUILD M9 Classic+ row C+ 32.1: "Destroys a target damaged Unit
// (damage above 0) on either side, an Indestructible one knocked down instead (R46); an undamaged Unit is
// no legal target, and with none the Spell fizzles and still counts as played (§8's conventions);
// radiant destroys every damaged enemy Unit, no target".

import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/032-1-execute";

const EXECUTE = "classicplus-032-1";
/** #19 Midrange Menace 9/9 and #11 Tempo Timmy 3/3. */
const MENACE = "core-019";
const TIMMY = "core-011";
/** C #41 State of the Game, 3/3 Indestructible. */
const UNBREAKABLE = "classic-041";
const FILLER = "core-005";

const at = (s: Scenario, player: "p1" | "p2", lane: number): Selection[] => {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
};

const destroyed = (s: Scenario): string[] => s.events.flatMap((event) => (event.type === "destroyed" ? [event.defId] : []));

describe("C+ #32.1 Execute", () => {
  it("declares one damaged Unit of either side; the Radiant face declares nothing", () => {
    expect(def.id).toBe(EXECUTE);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"], damaged: true } }]);
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("§6.3 destroys a damaged enemy Unit", () => {
      const s = scenario({ p1: { hand: [EXECUTE, FILLER] }, p2: { field: [{ def: MENACE, damage: 1 }, TIMMY] } });
      s.play(EXECUTE, { targets: at(s, "p2", 1) });
      expect(destroyed(s)).toEqual([MENACE]);
      s.expectInZone(TIMMY, "field");
    });

    it("§8.7 a damaged Unit of your own is a target too", () => {
      const s = scenario({ p1: { hand: [EXECUTE, FILLER], field: [{ def: TIMMY, damage: 2 }] } });
      s.play(EXECUTE, { targets: at(s, "p1", 1) });
      expect(destroyed(s)).toEqual([TIMMY]);
    });

    it("§4 an undamaged Unit is no legal target", () => {
      const s = scenario({ p1: { hand: [EXECUTE, FILLER] }, p2: { field: [MENACE, { def: TIMMY, damage: 1 }] } });
      expect(() => s.play(EXECUTE, { targets: at(s, "p2", 1) })).toThrow();
    });

    it("§8 with no damaged Unit the Spell fizzles and still counts as played", () => {
      const s = scenario({ p1: { hand: [EXECUTE, FILLER] }, p2: { field: [MENACE] } });
      s.play(EXECUTE);
      expect(destroyed(s)).toEqual([]);
      s.expectInZone(EXECUTE, "graveyard");
      s.expectEvents("cardPlayed");
    });

    it("R46 an Indestructible damaged Unit is knocked down instead", () => {
      const s = scenario({
        p1: { hand: [EXECUTE, FILLER] },
        p2: { field: [{ def: UNBREAKABLE, damage: 1, position: "DEF" }] },
      });
      s.play(EXECUTE, { targets: at(s, "p2", 1) });
      s.expectInZone(UNBREAKABLE, "field");
      expect(s.stats(UNBREAKABLE).position).toBe("ATK");
      expect(destroyed(s)).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("§8.7 destroys every damaged enemy Unit, and no other", () => {
      const s = scenario({
        p1: { hand: [{ def: EXECUTE, radiant: true }, FILLER], field: [{ def: TIMMY, damage: 1 }] },
        p2: { field: [{ def: MENACE, damage: 3 }, { def: TIMMY, damage: 1 }, MENACE] },
      });
      s.play(EXECUTE);
      expect(destroyed(s).sort()).toEqual([MENACE, TIMMY].sort());
      // The undamaged enemy Menace and your own damaged Timmy stay.
      expect(s.unit("p2", 3)?.defId).toBe(MENACE);
      expect(s.unit("p1", 1)?.defId).toBe(TIMMY);
    });

    it("R46 an Indestructible one is knocked down; with no damaged enemy nothing happens", () => {
      const s = scenario({
        p1: { hand: [{ def: EXECUTE, radiant: true }, FILLER] },
        p2: { field: [{ def: UNBREAKABLE, damage: 2, position: "DEF" }] },
      });
      s.play(EXECUTE);
      s.expectInZone(UNBREAKABLE, "field");
      expect(s.stats(UNBREAKABLE).position).toBe("ATK");

      const none = scenario({ p1: { hand: [{ def: EXECUTE, radiant: true }, FILLER] }, p2: { field: [MENACE] } });
      none.play(EXECUTE);
      expect(destroyed(none)).toEqual([]);
      none.expectInZone(EXECUTE, "graveyard");
    });
  });
});
