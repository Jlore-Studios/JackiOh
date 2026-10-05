// C+ #10 New Wraps — SPEC §8.7 row 10, BUILD M9 Classic+ row C+ 10: "A target Unit gains Reborn as a
// granted keyword (kept by a Vanilla, lost on leaving the field, R78), so its next death returns it at
// 1 health without Reborn; a Unit that has Reborn gains nothing; radiant also makes it Radiant, its
// Radiant face applying at once with damage and buffs kept and no Cry (§5.2)".

import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/010-new-wraps";

const WRAPS = "classicplus-010";
const VANILLA = "core-008"; // (1) 4/4.
const DEFENDER = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
const MR_TOKEN = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const FLOOD = "core-017"; // (4) Bounce all Units.
const SILENCE = "classicplus-009"; // (0) Vanilla a Unit.
const SURGERY = "core-063"; // (1) +3/+3 and 1 random keyword.
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(p1: SideSetup, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { library: [STOCKPILE, STOCKPILE], mana: 10, ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], ...p2 },
  });
}

function at(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

function rebornCount(s: Scenario, id: string): number {
  return s.stats(id).keywords.filter((k) => k.kind === "Reborn").length;
}

describe("C+ #10 New Wraps", () => {
  it("is a (0) Spell targeting a Unit on either side, on both faces", () => {
    expect([def.type, def.cost]).toEqual(["Spell", 0]);
    for (const face of [base, radiant]) {
      expect(face.targets).toEqual([
        { kind: "target", min: 1, max: 1, aim: "help", filter: { side: "any", of: ["unit"] } },
      ]);
    }
  });

  describe("base", () => {
    it("§10.4 R64 the Unit gains Reborn as a granted keyword; its next death returns it at 1 health without Reborn", () => {
      const s = setup({ hand: [WRAPS, HIT_JOB, HIT_JOB, FILLER], field: [VANILLA] });
      const vanilla = s.card(VANILLA);

      s.play(WRAPS, { targets: at(vanilla.id) });
      expect(s.card(vanilla).grantedKeywords).toContainEqual({ kind: "Reborn" });
      expect(rebornCount(s, vanilla.id)).toBe(1);

      s.play(HIT_JOB, { targets: at(vanilla.id) });
      const back = s.unit("p1", 1)!;
      expect(back.defId).toBe(VANILLA);
      s.expectStats(back, { attack: 4, health: 1 });
      expect(rebornCount(s, back.id)).toBe(0);

      s.play(HIT_JOB, { targets: at(back.id) });
      expect(s.unit("p1", 1)).toBeNull();
      s.expectInZone(vanilla, "graveyard");
    });

    it("reaches an enemy Unit too", () => {
      const s = setup({ hand: [WRAPS, FILLER] }, { field: [VANILLA] });

      s.play(WRAPS, { targets: at(s.card(VANILLA).id) });

      expect(rebornCount(s, VANILLA)).toBe(1);
    });

    it("§10.4 a Vanilla keeps the granted Reborn", () => {
      const s = setup({ hand: [WRAPS, SILENCE, HIT_JOB, FILLER], field: [VANILLA] });
      const vanilla = s.card(VANILLA);

      s.play(WRAPS, { targets: at(vanilla.id) });
      s.play(SILENCE, { targets: at(vanilla.id) });
      expect(rebornCount(s, vanilla.id)).toBe(1);

      s.play(HIT_JOB, { targets: at(vanilla.id) });
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
    });

    it("R78 leaving the field clears it: a bounced Unit comes back to hand without it", () => {
      const s = setup({ hand: [WRAPS, FLOOD, FILLER], field: [VANILLA] });
      const vanilla = s.card(VANILLA);

      s.play(WRAPS, { targets: at(vanilla.id) });
      s.play(FLOOD);

      expect(s.card(vanilla).zone.z).toBe("hand");
      expect(s.card(vanilla).grantedKeywords).toEqual([]);
    });

    it("a Unit that has Reborn gains no second one: it still returns once", () => {
      const s = setup({ hand: [WRAPS, HIT_JOB, HIT_JOB, FILLER], field: [DEFENDER] });
      const defender = s.card(DEFENDER);

      s.play(WRAPS, { targets: at(defender.id) });
      expect(rebornCount(s, defender.id)).toBe(1);

      s.play(HIT_JOB, { targets: at(defender.id) });
      const back = s.unit("p1", 1)!;
      expect(rebornCount(s, back.id)).toBe(0);
      s.play(HIT_JOB, { targets: at(back.id) });
      expect(s.unit("p1", 1)).toBeNull();
    });

    it("R570 on a Unit that prints Reborn the grant is still recorded: a later Vanilla leaves it the granted Reborn", () => {
      const s = setup({ hand: [WRAPS, SILENCE, HIT_JOB, FILLER], field: [DEFENDER] });
      const defender = s.card(DEFENDER);
      s.play(WRAPS, { targets: at(defender.id) });
      expect(s.card(defender).grantedKeywords).toContainEqual({ kind: "Reborn" });

      s.play(SILENCE, { targets: at(defender.id) });
      expect(rebornCount(s, defender.id)).toBe(1);
      s.play(HIT_JOB, { targets: at(defender.id) });
      expect(s.unit("p1", 1)?.defId).toBe(DEFENDER);

      // Without New Wraps a Vanilla Right-house defender has no Reborn left, and stays dead.
      const bare = setup({ hand: [SILENCE, HIT_JOB, FILLER], field: [DEFENDER] });
      bare.play(SILENCE, { targets: at(bare.card(DEFENDER).id) });
      bare.play(HIT_JOB, { targets: at(bare.card(DEFENDER).id) });
      expect(bare.unit("p1", 1)).toBeNull();
    });

    it("a Reborn body that spent its Reborn gains it again, and returns once more", () => {
      const s = setup({ hand: [WRAPS, HIT_JOB, HIT_JOB, FILLER], field: [DEFENDER] });
      s.play(HIT_JOB, { targets: at(s.card(DEFENDER).id) });
      const body = s.unit("p1", 1)!;
      expect(rebornCount(s, body.id)).toBe(0);

      s.play(WRAPS, { targets: at(body.id) });
      expect(rebornCount(s, body.id)).toBe(1);
      s.play(HIT_JOB, { targets: at(body.id) });

      expect(s.unit("p1", 1)?.defId).toBe(DEFENDER);
    });
  });

  describe("radiant", () => {
    it("§5.2 also makes it Radiant at once: the Radiant stats apply with its damage and buffs kept, and no Cry", () => {
      const s = setup({ hand: [{ def: WRAPS, radiant: true }, SURGERY, FILLER], field: [{ def: MR_TOKEN, damage: 0 }] });
      const unit = s.card(MR_TOKEN);
      s.play(SURGERY, { targets: at(unit.id) }); // +3/+3: a 4/4 Mr Token

      s.play(WRAPS, { targets: at(unit.id) });

      expect(s.card(unit).radiant).toBe(true);
      s.expectStats(unit, { attack: 2 + 3, maxHealth: 2 + 3 });
      expect(rebornCount(s, unit.id)).toBe(1);
      s.expectEvents("keywordGranted", "radiantSet");
      expect(s.events.filter((event) => event.type === "summoned")).toHaveLength(0);
    });

    it("§5.2 damage is kept through the face swap", () => {
      const s = setup({ hand: [{ def: WRAPS, radiant: true }, FILLER], field: [{ def: VANILLA, damage: 3 }] });

      s.play(WRAPS, { targets: at(s.card(VANILLA).id) });

      s.expectStats(VANILLA, { attack: 12, maxHealth: 12, health: 9 }); // Radiant Mr. Vanilla is 12/12
    });
  });
});
