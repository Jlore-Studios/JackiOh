// C+ #67 Pear — SPEC §8.7 row 67, BUILD M9 Classic+ row C+ 67: "Summons 2 random non-token Units of any
// set printed at (1) Cost and Common rarity (R380; out-of-play cost, R65), no Cry (R1), repeats allowed;
// one on a nearly full board, none on a full one and no random draw (R129); the count reads through
// `param()`; radiant the Units are Radiant".

import { activeUnitsOf, defOf, query, stepParam } from "@jackioh/engine";
import { fillParams } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/067-pear";

const PEAR = "classicplus-067";
const FILLER = "core-005";
const MENACE = "core-019";
const ME_AND_MR_TOKEN = "core-015"; // (1) Common Unit, "Cry: Summon a Rush Token."

function summoned(s: Scenario): string[] {
  return s.lastEvents.flatMap((event) => (event.type === "summoned" ? [event.instanceId] : []));
}

function pear(opts: { radiant?: boolean; seed?: string; field?: number } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "pear",
    p1: {
      hand: [{ def: PEAR, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER],
      field: Array.from({ length: opts.field ?? 0 }, () => MENACE),
    },
    p2: { hand: [FILLER] },
  });
}

describe("C+ #67 Pear", () => {
  it("is a (2) Fruit Spell, both faces one shape of hook", () => {
    expect(def.id).toBe(PEAR);
    expect(def.cost).toBe(2);
    expect(def.tags).toContain("Fruit");
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  it("R482 the text agrees with its count at every value: one Unit, two Units", () => {
    expect(fillParams(def, "base", { units: 1 })).toBe("Summon 1 random (1) Cost Common Unit.");
    expect(fillParams(def, "base")).toBe("Summon 2 random (1) Cost Common Units.");
    expect(fillParams(def, "radiant", { units: 1 })).toBe("Summon 1 random Radiant (1) Cost Common Unit.");
    expect(fillParams(def, "radiant", { units: 3 })).toBe("Summon 3 random Radiant (1) Cost Common Units.");
  });

  describe("base", () => {
    it("R64 summons 2 random (1) Cost Common Units into your leftmost open zones", () => {
      const s = pear();
      s.play(PEAR);
      const ids = summoned(s);
      expect(ids).toHaveLength(2);
      for (const id of ids) {
        const card = defOf(s.state, s.card(id).defId);
        expect(card.type).toBe("Unit");
        expect(card.cost).toBe(1);
        expect(card.rarity).toBe("Common");
        expect(card.token).toBe(false);
        expect(s.card(id).radiant).toBe(false);
      }
      expect([s.unit("p1", 1)?.id, s.unit("p1", 2)?.id]).toEqual(ids);
    });

    it("R380 R65 over many seeds every Unit is in the pool of non-token (1) Cost Commons of every set, and more than one set shows", () => {
      const pool = new Set(query({ type: "Unit", cost: 1, rarity: "Common" }).map((card) => card.id));
      const seen = new Set<string>();
      for (let i = 0; i < 40; i += 1) {
        const s = pear({ seed: `pear-${i}` });
        s.play(PEAR);
        for (const id of summoned(s)) seen.add(s.card(id).defId);
      }
      expect([...seen].every((id) => pool.has(id))).toBe(true);
      expect(new Set([...seen].map((id) => id.split("-")[0])).size).toBeGreaterThan(1);
    });

    it("R1 a summoned Unit fires no Cry: Me and Mr Token arrives without its Rush Token", () => {
      let checked = false;
      for (let i = 0; i < 200 && !checked; i += 1) {
        const s = pear({ seed: `pear-cry-${i}` });
        s.play(PEAR);
        const defs = summoned(s).map((id) => s.card(id).defId);
        if (!defs.includes(ME_AND_MR_TOKEN)) continue;
        checked = true;
        expect(summoned(s)).toHaveLength(2);
        expect(activeUnitsOf(s.state, "p1").some((unit) => unit.defId === "core-t-rush")).toBe(false);
      }
      expect(checked).toBe(true);
    });

    it("R60 repeats are allowed: some seed summons the same Unit twice", () => {
      let twice = false;
      for (let i = 0; i < 200 && !twice; i += 1) {
        const s = pear({ seed: `pear-twice-${i}` });
        s.play(PEAR);
        const defs = summoned(s).map((id) => s.card(id).defId);
        twice = defs.length === 2 && defs[0] === defs[1];
      }
      expect(twice).toBe(true);
    });

    it("R64 a nearly full board takes one", () => {
      const s = pear({ field: 4 });
      s.play(PEAR);
      expect(summoned(s)).toHaveLength(1);
      expect(s.unit("p1", 5)).not.toBeNull();
    });

    it("R129 a full board summons nothing and draws nothing from the rng", () => {
      const s = pear({ field: 5 });
      const cursor = s.state.rngCursor;
      s.play(PEAR);
      expect(summoned(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R386 an Upgrade summons 3; a Degrade 1", () => {
      const up = pear();
      stepParam(up.card(PEAR), "units", 1);
      up.play(PEAR);
      expect(summoned(up)).toHaveLength(3);

      const down = pear();
      stepParam(down.card(PEAR), "units", -1);
      down.play(PEAR);
      expect(summoned(down)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R74 the Units are Radiant", () => {
      const s = pear({ radiant: true });
      s.play(PEAR);
      const ids = summoned(s);
      expect(ids).toHaveLength(2);
      for (const id of ids) {
        const unit = s.card(id);
        expect(unit.radiant).toBe(true);
        const printed = defOf(s.state, unit.defId).radiant;
        s.expectStats(unit, { attack: printed.attack ?? 0, maxHealth: printed.health ?? 0 });
      }
    });

    it("R129 a full board summons nothing on the Radiant face either", () => {
      const s = pear({ radiant: true, field: 5 });
      const cursor = s.state.rngCursor;
      s.play(PEAR);
      expect(summoned(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });
  });
});
