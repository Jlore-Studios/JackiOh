// C #22 Mid Runner — SPEC §8.6 row 22, BUILD M9 Classic row C 22: "Cry, two independent checks at
// resolution: in midlane (computed from the lane count, R685) it Tributes itself (a death), anywhere
// else it stays; if you had 4 or more mana before paying for it (recorded as the play begins, §10.5
// step 1), two different random enemy permanents (R60; fewer if fewer) return to their controllers'
// hands (R747), tokens ceasing to exist (R11) and a full hand burning (R317); both may happen in one Cry;
// `conditionMet` answers in hand whether your mana is 4 or more now (R195); a bounced face-down trap
// is never named in your view (R97); radiant 4/2 returning 3; its tuned numbers (mana threshold,
// bounces) read through `param()` (R386)".
//
// The `conditionMet` proofs (R195) are in `../condition-active.test.ts`, with the other cards'.

import { midlaneLanes, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/022-mid-runner";

const RUNNER = "classic-022";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9.
const TEMPO = "core-011"; // (1) Unit 3/3.
const RUSH_TOKEN = "core-t-rush";
const BIG_FELINOR = "core-043"; // (4) Unit 3/10, under the Stack pile below.
const FIENDER = "core-092"; // Felinor Fiender: Stack.
const PAWN = "core-096"; // (1) Trap; answers only a lethal attack.
const FIELD_SPELL = "core-073"; // (2) Field Spell.
const FILLER = "core-005";
const ANCHOR = "core-010";

function bouncedIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "bounced" ? [event.instanceId] : []));
}

function enemyBoardIds(s: Scenario): string[] {
  const units = [1, 2, 3, 4, 5].flatMap((lane) => {
    const card = s.unit("p2", lane);
    return card === null ? [] : [card.id];
  });
  const backrow = [1, 2, 3, 4, 5].flatMap((lane) => {
    const card = s.backrow("p2", lane);
    return card === null ? [] : [card.id];
  });
  return [...units, ...backrow];
}

describe("C #22 Mid Runner", () => {
  it("runs one script on both faces, and midlane of 5 lanes is lane 3 (R685)", () => {
    expect(def.id).toBe(RUNNER);
    expect(radiant).toBe(base);
    expect(midlaneLanes(5)).toEqual([3]);
  });

  it("R685 computes midlane from the lane count: odd counts center, even counts both centers", () => {
    expect(midlaneLanes(5)).toEqual([3]);
    expect(midlaneLanes(3)).toEqual([2]);
    expect(midlaneLanes(1)).toEqual([1]);
    expect(midlaneLanes(4)).toEqual([2, 3]);
    expect(midlaneLanes(6)).toEqual([3, 4]);
  });

  describe("base", () => {
    it("is a 2/1", () => {
      const s = scenario({ p1: { field: [RUNNER], hand: [ANCHOR] } });
      s.expectStats(RUNNER, { attack: 2, health: 1 });
    });

    it("played into lane 3 it Tributes itself: a death", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR], mana: 3 }, p2: { hand: [ANCHOR], field: [VANILLA] } });

      s.play(RUNNER, { zone: 3 });

      s.expectInZone(RUNNER, "graveyard");
      expect(s.events.some((event) => event.type === "destroyed" && event.defId === RUNNER)).toBe(true);
      expect(bouncedIds(s)).toEqual([]);
    });

    it("played anywhere else it stays", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR], mana: 3 }, p2: { hand: [ANCHOR] } });

      s.play(RUNNER, { zone: 2 });

      s.expectInZone(RUNNER, "field");
      expect(s.unit("p1", 2)?.defId).toBe(RUNNER);
    });

    it("with 4 mana before paying it bounces two different random enemy permanents to their controller's hand", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO], backrow: [FIELD_SPELL] } });
      const before = enemyBoardIds(s);

      s.play(RUNNER, { zone: 1 });

      const bounced = bouncedIds(s);
      expect(bounced).toHaveLength(2);
      expect(new Set(bounced).size).toBe(2);
      for (const id of bounced) {
        expect(before).toContain(id);
        expect(s.hand("p2").map((card) => card.id)).toContain(id);
      }
      expect(enemyBoardIds(s)).toHaveLength(2);
    });

    it("with 3 mana before paying it bounces nothing", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR], mana: 3 }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE] } });

      s.play(RUNNER, { zone: 1 });

      expect(bouncedIds(s)).toEqual([]);
    });

    it("§10.5 step 1 it reads the mana before paying the price this play paid: a Runner made to cost 2 still had 4", () => {
      const s = scenario({ p1: { hand: [{ def: RUNNER, costMod: 1 }, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE] } });

      s.play(RUNNER, { zone: 1 });

      s.expectMana("p1", 2);
      expect(bouncedIds(s)).toHaveLength(2);
    });

    it("R60 fewer enemy permanents than two: it bounces what there is, and nothing on an empty board", () => {
      const one = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA] } });
      one.play(RUNNER, { zone: 1 });
      expect(bouncedIds(one)).toHaveLength(1);
      one.expectInZone(VANILLA, "hand");

      const none = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR] } });
      none.play(RUNNER, { zone: 1 });
      expect(bouncedIds(none)).toEqual([]);
    });

    it("R11 a bounced unit token ceases to exist", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [RUSH_TOKEN, RUSH_TOKEN] } });
      const tokens = [s.unit("p2", 1), s.unit("p2", 2)];

      s.play(RUNNER, { zone: 1 });

      for (const token of tokens) {
        if (token === null) throw new Error("two tokens");
        s.expectInZone(token, "gone");
      }
      expect(s.hand("p2").map((card) => card.defId)).toEqual([ANCHOR]);
    });

    it("R13 only the top of an enemy Stack pile is a permanent: it is bounced, and the card beneath resumes and stays", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [BIG_FELINOR, { def: FIENDER, stack: true }] } });

      s.play(RUNNER, { zone: 1 });

      expect(bouncedIds(s)).toEqual([s.card(FIENDER).id]);
      s.expectInZone(FIENDER, "hand");
      expect(s.unit("p2", 1)?.defId).toBe(BIG_FELINOR);
    });

    it("R4 R317 a full hand burns the bounced card into its owner's graveyard", () => {
      const s = scenario({
        p1: { hand: [RUNNER, ANCHOR] },
        p2: { hand: [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], field: [VANILLA] },
      });

      s.play(RUNNER, { zone: 1 });

      s.expectInZone(VANILLA, "graveyard");
      expect(s.events.some((event) => event.type === "burned")).toBe(true);
    });

    it("both checks may pass in one Cry: in lane 3 with 4 mana it dies and bounces two", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO] } });

      s.play(RUNNER, { zone: 3 });

      s.expectInZone(RUNNER, "graveyard");
      expect(bouncedIds(s)).toHaveLength(2);
    });

    it("R97 a bounced face-down trap is never named in your view", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], backrow: [{ def: PAWN, faceUp: false }] } });

      s.play(RUNNER, { zone: 1 });

      s.expectInZone(PAWN, "hand");
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      expect(JSON.stringify(s.view("p2"))).toContain(PAWN);
    });

    it("R386 a Degrade of the threshold to 5 stops 4 mana bouncing; an Upgrade to 3 lets 3 mana bounce", () => {
      const harder = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE] } });
      stepParam(harder.card(RUNNER), "threshold", 1);
      harder.play(RUNNER, { zone: 1 });
      expect(bouncedIds(harder)).toEqual([]);

      const easier = scenario({ p1: { hand: [RUNNER, ANCHOR], mana: 3 }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE] } });
      stepParam(easier.card(RUNNER), "threshold", -1);
      easier.play(RUNNER, { zone: 1 });
      expect(bouncedIds(easier)).toHaveLength(2);
    });

    it("R386 an Upgrade of bounces makes it bounce 3", () => {
      const s = scenario({ p1: { hand: [RUNNER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO, VANILLA] } });
      stepParam(s.card(RUNNER), "bounces", 1);

      s.play(RUNNER, { zone: 1 });

      expect(new Set(bouncedIds(s)).size).toBe(3);
    });
  });

  describe("radiant", () => {
    it("R275 is a 4/2 with the same text (the designer's word, docs/radiant-audit.md)", () => {
      const s = scenario({ p1: { field: [{ def: RUNNER, radiant: true }], hand: [ANCHOR] } });
      s.expectStats(RUNNER, { attack: 4, health: 2 });
      expect(def.radiant.text).toBe(def.base.text);
    });

    it("in lane 3 it Tributes itself, and with 4 mana it bounces three", () => {
      const s = scenario({ p1: { hand: [{ def: RUNNER, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO] } });

      s.play(RUNNER, { zone: 3 });

      s.expectInZone(RUNNER, "graveyard");
      expect(new Set(bouncedIds(s)).size).toBe(3);
    });

    it("with 3 mana it stays out of lane 3 and bounces nothing", () => {
      const s = scenario({ p1: { hand: [{ def: RUNNER, radiant: true }, ANCHOR], mana: 3 }, p2: { hand: [ANCHOR], field: [VANILLA] } });

      s.play(RUNNER, { zone: 5 });

      s.expectInZone(RUNNER, "field");
      expect(bouncedIds(s)).toEqual([]);
    });
  });
});
