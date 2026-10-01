// C #86 Genn (SPEC §8.6 row 86; BUILD M9 row C 86). (4) Unit, Common, 14/14 → 42/42: no text, a vanilla
// Unit like #8 Mr. Vanilla; the Radiant face is its tripled stats (R276).

import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/086-genn";
import { scenario, type SideSetup } from "../_harness";

const GENN = "classic-086";
const SAINTESS = "core-081"; // Death: make your other Units Radiant
const HIT_JOB = "core-016";
const STOCKPILE = "core-005";
const VANILLA = "core-008";

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] };

describe("C #86 Genn", () => {
  describe("base", () => {
    it("§8.6 a 14/14 with no text, no keywords and no script, as #8 Mr. Vanilla", () => {
      const s = scenario({ p1: { hand: [GENN, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(GENN);
      s.expectStats(GENN, { attack: 14, health: 14 });
      expect(s.stats(GENN).keywords).toEqual([]);
      expect([def.base.text, def.base.keywords]).toEqual(["", []]);
      expect(base).toEqual({});
    });
  });

  describe("radiant", () => {
    it("R276 42/42 with no text: tripled as #8's, so the faces differ", () => {
      const s = scenario({ p1: { field: [{ def: GENN, radiant: true }] } });
      s.expectStats(GENN, { attack: 42, health: 42 });
      expect([def.radiant.text, def.radiant.keywords]).toEqual(["", []]);
      expect(radiant).toBe(base);
    });

    it("§5.2 made Radiant on the field, it takes that face at once", () => {
      const s = scenario({ p1: { field: [GENN, SAINTESS], hand: [HIT_JOB, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(SAINTESS).id }] });
      expect(s.card(GENN).radiant).toBe(true);
      s.expectStats(GENN, { attack: 42, health: 42 });
    });
  });
});
