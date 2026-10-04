// C+ #7 The House — SPEC §8.7 row 7, BUILD M9 Classic+ row C+ 7: "Field Spell: its Cry and each start of
// your turn summon a Right-house defender (Core #3) with chance 2 in 3, else a Wrong-House Attacker (C+
// #6), generated from the catalog (cards, not tokens, so they go to your graveyard when they die),
// yours, no Cry (R1); 'Right-House Protector' is Core #3 (R406); a seeded roll gives a fixed unit;
// nothing at the opponent's start of turn; a full board summons nothing and rolls nothing (R129);
// radiant summons one of each, the second skipped when only one zone is open".

import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { expectAnimated } from "../_animated";
import { base, def, radiant } from "../../src/scripts/classic-plus/007-the-house";

const HOUSE = "classicplus-007";
const RIGHT = "core-003"; // Right-house defender
const WRONG = "classicplus-006"; // Wrong-House Attacker
const VANILLA = "core-008"; // (1) 4/4.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(p1: SideSetup = {}, radiantFace = false, seed?: string): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: { hand: [{ def: HOUSE, radiant: radiantFace }, FILLER], library: [STOCKPILE, STOCKPILE, STOCKPILE], mana: 8, ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE, STOCKPILE] },
  });
}

// Animated since patch v0.2.10 (R383): played into backrow zone 5 it animates into unit lane 5 before
// its Cry, leaving lanes 1 to 4 to the Units it summons.

/** The Units summoned, in order — not The House's own arrival in its backrow zone. */
function summoned(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "summoned" && event.defId !== HOUSE ? [event.defId] : []));
}

/** The defs The House summoned with its Cry, under a seed. */
function cryRoll(seed: string): string[] {
  const s = setup({}, false, seed);
  s.play(HOUSE, { zone: 5 });
  return summoned(s);
}

describe("C+ #7 The House", () => {
  it("is a (3) Field Spell naming Core #3 and C+ #6; each face has a Cry and a start-of-turn hook", () => {
    expect([def.type, def.cost]).toEqual(["Field Spell", 3]);
    expect(def.refs).toEqual([RIGHT, WRONG]);
    for (const face of [base, radiant]) {
      expect(face.cry).toBeTypeOf("function");
      expect(face.startOfTurn).toBeTypeOf("function");
    }
  });

  describe("base", () => {
    it("R406 its Cry summons one of the twins, yours, with no Cry; a seeded roll gives a fixed unit", () => {
      const s = setup();
      s.play(HOUSE, { zone: 5 });

      const defs = summoned(s);
      expect(defs).toHaveLength(1);
      expect([RIGHT, WRONG]).toContain(defs[0]);
      const unit = s.unit("p1", 1)!;
      expect([unit.defId, unit.owner, unit.controller, unit.radiant]).toEqual([defs[0], "p1", "p1", false]);
      expect(s.events.filter((event) => event.type === "cardPlayed").map((event) => event.instanceId)).toEqual([s.card(HOUSE).id]);
      expect(cryRoll("jackioh-harness")).toEqual(defs);
    });

    it("R406 2 in 3 is the Right-house defender's chance: both come up, the defender about twice as often", () => {
      let right = 0;
      let wrong = 0;
      for (let seed = 1; seed <= 90; seed += 1) {
        const [made] = cryRoll(`house-${seed}`);
        if (made === RIGHT) right += 1;
        if (made === WRONG) wrong += 1;
      }
      expect(right + wrong).toBe(90);
      expect(right).toBeGreaterThan(45);
      expect(right).toBeLessThan(75);
    });

    it("R11 the twins are cards, not tokens: a Wrong-House Attacker it made goes to your graveyard when it dies", () => {
      const seed = Array.from({ length: 30 }, (_, i) => `house-${i + 1}`).find((each) => cryRoll(each)[0] === WRONG);
      if (seed === undefined) throw new Error("no seed rolled the Wrong-House Attacker");
      const s = setup({ hand: [{ def: HOUSE }, HIT_JOB, FILLER] }, false, seed);
      s.play(HOUSE, { zone: 5 });
      const made = s.unit("p1", 1)!;
      expect(made.defId).toBe(WRONG);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: made.id }] });

      s.expectInZone(made, "graveyard");
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(WRONG);
    });

    it("§2.2 summons again at the start of your turn, and nothing at the opponent's", () => {
      const s = setup();
      s.play(HOUSE, { zone: 5 });
      expect(summoned(s)).toHaveLength(1);

      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(summoned(s)).toHaveLength(1);

      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(summoned(s)).toHaveLength(2);
      expect(s.unit("p1", 2)?.defId).toMatch(/^(core-003|classicplus-006)$/);
    });

    it("R129 a full board summons nothing and rolls nothing", () => {
      const s = setup({ field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      const cursor = s.state.rngCursor;

      s.play(HOUSE, { zone: 5 });

      expect(summoned(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R64 lands in the leftmost open unit zone", () => {
      const s = setup({ field: [VANILLA, { def: VANILLA, lane: 3 }] });

      s.play(HOUSE, { zone: 5 });

      expect([RIGHT, WRONG]).toContain(s.unit("p1", 2)?.defId);
    });
  });

  describe("radiant", () => {
    it("its Cry summons a Right-house defender and then a Wrong-House Attacker, rolling nothing", () => {
      const s = setup({}, true);
      const cursor = s.state.rngCursor;

      s.play(HOUSE, { zone: 5 });

      expect(summoned(s)).toEqual([RIGHT, WRONG]);
      expect([s.unit("p1", 1)?.defId, s.unit("p1", 2)?.defId]).toEqual([RIGHT, WRONG]);
      // Generated on their base faces, yours.
      expect([1, 2].map((lane) => [s.unit("p1", lane)?.radiant, s.unit("p1", lane)?.owner])).toEqual([
        [false, "p1"],
        [false, "p1"],
      ]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("with one zone open the second is skipped", () => {
      const s = setup({ field: [VANILLA, VANILLA, VANILLA] }, true);

      s.play(HOUSE, { zone: 5 });

      expect(s.unit("p1", 5)?.defId).toBe(HOUSE);
      expect(summoned(s)).toEqual([RIGHT]);
      expect(s.unit("p1", 4)?.defId).toBe(RIGHT);
    });

    it("summons both again at the start of your turn", () => {
      const s = setup({}, true);
      s.play(HOUSE, { zone: 5 });
      s.endTurn();
      s.endTurn();

      expect(summoned(s)).toEqual([RIGHT, WRONG, RIGHT, WRONG]);
    });
  });
});

describe("C+ #7 The House: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 0/6 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classicplus-007", stats: { attack: 0, health: 6 } });
  });

  it("R383 radiant: a 0/12 Unit", () => {
    expectAnimated({ def: "classicplus-007", radiant: true, stats: { attack: 0, health: 12 } });
  });
});
