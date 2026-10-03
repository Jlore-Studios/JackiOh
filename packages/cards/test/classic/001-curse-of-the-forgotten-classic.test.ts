// C #1 Curse of the Forgotten Classic — SPEC §8.6 row 1, BUILD M9 Classic row C 1: "One hit of N on
// the enemy hero, N = the size of their exile as it resolves, so Armor applies once; an empty exile
// deals no hit (R63); then draw 1; its preview is N (R280); radiant: the same hit and draw (the
// Radiant keeps "Draw 1"), then Recruit from their exile its most recently exiled permanent card, under
// your control, becoming yours as it reaches your side, so it goes to your piles when it leaves the
// field (R12); a Unit
// recruited that way makes one forced attack on the enemy hero at once, summoning sickness ignored
// (R53); no permanent in their exile, or no open zone, → nothing recruited; a recruited trap is set
// face-down and read by you alone (R33); its tuned numbers (damage per card, draw) read through
// `param()` (R386)".
//
// The preview (R280) is proved in `test/preview.test.ts`, with the other cards that declare one.

import { describe, expect, it } from "vitest";
import { stepParam, type CardInstance } from "@jackioh/engine";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/001-curse-of-the-forgotten-classic";

const CURSE = "classic-001";
const STOCKPILE = "core-005"; // (1) Spell
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const MR_TOKEN = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
const SHEEPISH = "core-041"; // (1) Trap
const MANA_WELL = "core-006"; // (3) Field Spell
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const RUSH_TOKEN = "core-t-rush";

function hitsOn(s: Scenario, player: PlayerId): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === `hero-${player}` ? [event.amount] : []));
}

/** p2's exiled card of this definition (p1's library holds some of the same definitions). */
function theirExiled(s: Scenario, defId: string): CardInstance {
  const card = s.pile("p2", "exile").find((candidate) => candidate.defId === defId);
  if (card === undefined) throw new Error(`p2's exile holds no ${defId}`);
  return card;
}

function curse(
  radiantFace: boolean,
  opts: { theirExile?: readonly string[]; myExile?: readonly string[]; armor?: number; myField?: readonly string[]; theirField?: readonly string[] } = {},
): Scenario {
  return scenario({
    p1: {
      hand: [{ def: CURSE, radiant: radiantFace }, STOCKPILE],
      library: [VANILLA, MENACE, STOCKPILE],
      exile: [...(opts.myExile ?? [])],
      field: [...(opts.myField ?? [])],
    },
    p2: {
      hand: [STOCKPILE, HIT_JOB],
      exile: [...(opts.theirExile ?? [STOCKPILE, STOCKPILE, STOCKPILE])],
      field: [...(opts.theirField ?? [])],
      library: [STOCKPILE, STOCKPILE],
      ...(opts.armor === undefined ? {} : { armor: opts.armor }),
    },
  });
}

describe("C #1 Curse of the Forgotten Classic", () => {
  it("declares its two numbers (R386) and a preview on both faces", () => {
    expect(def.params).toEqual([
      { key: "damage", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
    ]);
    expect(base.preview).toBeTypeOf("function");
    expect(radiant.preview).toBeTypeOf("function");
    expect(base.targets).toBeUndefined();
  });

  describe("base", () => {
    it("one hit of N on the enemy hero, N = the size of their exile as it resolves", () => {
      const s = curse(false);
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([3]);
      s.expectHealth("p2", 27);
    });

    it("R63 one hit, so Armor applies once", () => {
      const s = curse(false, { armor: 2, theirExile: [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE] });
      s.play(CURSE);
      s.expectHealth("p2", 28);
      expect(s.events.filter((event) => event.type === "damage")).toHaveLength(1);
    });

    it("R63 an empty exile deals no hit, and the draw still happens", () => {
      const s = curse(false, { theirExile: [] });
      s.play(CURSE);
      expect(s.events.filter((event) => event.type === "damage")).toEqual([]);
      s.expectHealth("p2", 30);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
    });

    it("then draws 1", () => {
      const s = curse(false);
      s.play(CURSE);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
      s.expectEvents("damage", "drawn");
    });

    it("only their exile counts, not yours", () => {
      const s = curse(false, { theirExile: [STOCKPILE], myExile: [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE] });
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([1]);
    });

    it("the base face recruits nothing", () => {
      const s = curse(false, { theirExile: [VANILLA] });
      const vanilla = theirExiled(s, VANILLA);
      s.play(CURSE);
      expect(s.unit("p1", 1)).toBeNull();
      s.expectInZone(vanilla, "exile");
    });

    it("R386 an Upgrade of damage per card deals 2 for each", () => {
      const s = curse(false);
      stepParam(s.card(CURSE), "damage", 1);
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([6]);
    });

    it("R386 an Upgrade of draw draws 2", () => {
      const s = curse(false);
      stepParam(s.card(CURSE), "draw", 1);
      s.play(CURSE);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA, MENACE]);
    });
  });

  describe("radiant", () => {
    it("keeps the same hit and the draw", () => {
      const s = curse(true);
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([3]);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
    });

    it("E25 recruits the most recently exiled permanent of theirs, under your control and current ownership", () => {
      const s = curse(true, { theirExile: [MENACE, VANILLA, STOCKPILE] });
      const menace = theirExiled(s, MENACE);
      const vanilla = theirExiled(s, VANILLA);
      s.play(CURSE);
      expect(s.unit("p1", 1)?.id).toBe(vanilla.id);
      const recruited = s.card(vanilla);
      expect(recruited.controller).toBe("p1");
      expect(recruited.owner).toBe("p1");
      s.expectInZone(menace, "exile");
    });

    it("R53 a recruited Unit attacks the enemy hero at once, summoning sickness ignored", () => {
      const s = curse(true, { theirExile: [MENACE, VANILLA, STOCKPILE] });
      s.play(CURSE);
      // 3 for the exile of three, then the Vanilla's 4.
      expect(hitsOn(s, "p2")).toEqual([3, 4]);
      s.expectHealth("p2", 23);
      s.expectEvents("damage", "drawn", "summoned", "attackDeclared");
    });

    it("R53 the forced attack skips Taunt: it hits the hero past their Taunt Unit", () => {
      const s = curse(true, { theirExile: [VANILLA], theirField: [MENACE] });
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([1, 4]);
    });

    it("R1 a recruit fires no Cry", () => {
      const s = curse(true, { theirExile: [MR_TOKEN] });
      s.play(CURSE);
      expect(s.unit("p1", 1)?.defId).toBe(MR_TOKEN);
      expect(s.unit("p1", 2)).toBeNull();
      expect(s.events.some((event) => event.type === "summoned" && event.defId === RUSH_TOKEN)).toBe(false);
    });

    it("R12 when it leaves the field it goes to its current owner's piles", () => {
      const s = curse(true, { theirExile: [VANILLA] });
      const vanilla = theirExiled(s, VANILLA);
      s.play(CURSE);
      s.endTurn();
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectInZone(vanilla, "graveyard");
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toContain(vanilla.id);
      expect(s.pile("p2", "graveyard").map((card) => card.id)).not.toContain(vanilla.id);
    });

    it("no permanent in their exile: nothing is recruited", () => {
      const s = curse(true, { theirExile: [STOCKPILE, STOCKPILE] });
      s.play(CURSE);
      expect(s.unit("p1", 1)).toBeNull();
      expect(s.backrow("p1", 1)).toBeNull();
      expect(hitsOn(s, "p2")).toEqual([2]);
    });

    it("no open zone: nothing is recruited, and nothing attacks", () => {
      const s = curse(true, { theirExile: [VANILLA], myField: [MENACE, MENACE, MENACE, MENACE, MENACE] });
      const vanilla = theirExiled(s, VANILLA);
      s.play(CURSE);
      s.expectInZone(vanilla, "exile");
      expect(hitsOn(s, "p2")).toEqual([1]);
    });

    it("R33 a recruited trap is set face-down and read by you alone; it attacks nothing", () => {
      const s = curse(true, { theirExile: [VANILLA, SHEEPISH] });
      s.play(CURSE);
      const trap = s.backrow("p1", 1);
      expect(trap?.defId).toBe(SHEEPISH);
      expect(trap?.faceUp).not.toBe(true);
      expect(trap?.owner).toBe("p1");
      expect(s.unit("p1", 1)).toBeNull();
      expect(s.view("p2").opponent.backrow[0]).toEqual({ faceDown: true, cost: 1 });
      expect(s.view("p1").you.backrow[0]).toMatchObject({ defId: SHEEPISH });
      expect(hitsOn(s, "p2")).toEqual([2]);
      // Its owner's event stream names it nowhere once it is set (R97: judged where it is now).
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(`"${trap?.id ?? "missing"}"`);
      expect(theirs).not.toContain(SHEEPISH);
    });

    it("the newest permanent with no open zone for it: nothing is recruited, not an older one", () => {
      // The Trap is the newest permanent and your backrow is full, so nothing comes, though the older
      // Vanilla would fit a unit zone; and nothing attacks.
      const s = scenario({
        p1: {
          hand: [{ def: CURSE, radiant: true }, STOCKPILE],
          library: [VANILLA],
          backrow: [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
        },
        p2: { hand: [STOCKPILE], exile: [VANILLA, SHEEPISH], library: [STOCKPILE] },
      });
      s.play(CURSE);
      expect([1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane))).toEqual([null, null, null, null, null]);
      expect(s.pile("p2", "exile").map((card) => card.defId)).toEqual([VANILLA, SHEEPISH]);
      expect(hitsOn(s, "p2")).toEqual([2]);
    });

    it("a recruited Field Spell comes face-up to your backrow", () => {
      const s = curse(true, { theirExile: [MANA_WELL] });
      s.play(CURSE);
      expect(s.backrow("p1", 1)?.defId).toBe(MANA_WELL);
      expect(s.view("p2").opponent.backrow[0]).toMatchObject({ defId: MANA_WELL });
    });

    it("R386 an Upgrade of damage per card deals 2 for each, and the recruit still happens", () => {
      const s = curse(true, { theirExile: [VANILLA, STOCKPILE] });
      stepParam(s.card(CURSE), "damage", 1);
      s.play(CURSE);
      expect(hitsOn(s, "p2")).toEqual([4, 4]);
    });
  });
});
