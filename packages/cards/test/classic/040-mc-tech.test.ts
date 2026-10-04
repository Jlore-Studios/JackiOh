// C #40 MC Tech — SPEC §8.6 row 40, BUILD M9 Classic row C 40: "Cry: at resolution, if your opponent
// controls 4 or more permanents (tops of unit piles and backrow cards, face-down ones included;
// dormant cards don't count, R13), steal one at random (R60), placed per R15 (no free zone: it stays
// with them), an entry (R171); 3 or fewer → nothing; `conditionMet` answers in hand whether they
// control 4 or more now (R195); a stolen face-down trap is read by you from then on (R33); radiant
// 6/6: you pick it in a prompt at resolution, a face-down option carrying only its id (R177); its
// tuned number (threshold) reads through `param()` (R386)".
//
// The `conditionMet` proofs (R195) are in `../condition-active.test.ts`, with the other cards'.
//
// The face-down traps used below never fire during these plays: My Pawn answers only a lethal
// attack, Bread and Butter and Intern Stimmy the end of a turn, and Unlicensed Experimentation a
// permanent of a type its controller controls (p2 controls no Unit when they hold only traps).

import { reduce, stepParam, type GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/040-mc-tech";

const TECH = "classic-040";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9.
const TEMPO = "core-011"; // (1) Unit 3/3 Rush.
const FIENDER = "core-092"; // Felinor Fiender: Stack.
const FIELD_SPELL = "core-073"; // (2) Field Spell.
const PAWN = "core-096";
const BREAD = "core-018";
const STIMMY = "core-071";
const UNLICENSED = "core-085";
const ANCHOR = "core-010";

const FACE_DOWN_TRAPS = [PAWN, BREAD, STIMMY, UNLICENSED].map((d) => ({ def: d, faceUp: false }));

function stolenIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "controlChanged" ? [event.instanceId] : []));
}

describe("C #40 MC Tech", () => {
  it("has a script per face, and the Radiant pick answers into its resume step", () => {
    expect(def.id).toBe(TECH);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.resume?.steal).toBeTypeOf("function");
  });

  describe("base", () => {
    it("is a 3/3", () => {
      const s = scenario({ p1: { field: [TECH], hand: [ANCHOR] } });
      s.expectStats(TECH, { attack: 3, health: 3 });
    });

    it("R60 with 4 enemy permanents it steals one at random, which is now yours", () => {
      const s = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO], backrow: [FIELD_SPELL] } });

      s.play(TECH, { zone: 5 });

      const stolen = stolenIds(s);
      expect(stolen).toHaveLength(1);
      const card = s.card(stolen[0] ?? "");
      // R662: "which is now yours" — its controller and current owner.
      expect(card.controller).toBe("p1");
      expect(card.owner).toBe("p1");
    });

    it("R15 a stolen Unit lands in the same lane of your row when it is free", () => {
      const s = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA] } });

      s.play(TECH, { zone: 5 });

      const stolen = s.card(stolenIds(s)[0] ?? "");
      const lane = [1, 2, 3, 4].find((l) => s.unit("p1", l)?.id === stolen.id);
      expect(lane).toBeDefined();
      expect(s.unit("p2", lane ?? 0)).toBeNull();
    });

    it("R15 with no free zone in your row the pick stays with them and nothing is stolen", () => {
      const s = scenario({
        p1: { hand: [TECH, ANCHOR], field: [MENACE, MENACE, MENACE, MENACE] },
        p2: { hand: [ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA] },
      });

      s.play(TECH, { zone: 5 });

      expect(stolenIds(s)).toEqual([]);
      expect([1, 2, 3, 4].map((l) => s.unit("p2", l)?.defId)).toEqual([VANILLA, VANILLA, VANILLA, VANILLA]);
    });

    it("3 or fewer enemy permanents: nothing", () => {
      const s = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE], backrow: [FIELD_SPELL] } });
      const cursor = s.state.rngCursor;

      s.play(TECH, { zone: 5 });

      expect(stolenIds(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R13 a card dormant under a Stack pile does not count; a face-down trap does", () => {
      const dormant = scenario({
        p1: { hand: [TECH, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }, MENACE, TEMPO] },
      });
      dormant.play(TECH, { zone: 5 });
      expect(stolenIds(dormant)).toEqual([]);

      const trap = scenario({
        p1: { hand: [TECH, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }, MENACE, TEMPO], backrow: [{ def: PAWN, faceUp: false }] },
      });
      trap.play(TECH, { zone: 5 });
      expect(stolenIds(trap)).toHaveLength(1);
    });

    it("R171 the steal is an entry: a stolen Unit without Rush or Charge can't attack this turn", () => {
      const s = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA] } });

      s.play(TECH, { zone: 5 });

      const stolen = s.card(stolenIds(s)[0] ?? "");
      expect(() => s.attack(stolen, "hero")).toThrow();
    });

    it("R33 a stolen face-down trap stays face-down and is read by you from then on, not by them", () => {
      const s = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], backrow: FACE_DOWN_TRAPS } });

      s.play(TECH, { zone: 5 });

      const stolen = s.card(stolenIds(s)[0] ?? "");
      expect(stolen.zone).toMatchObject({ z: "field", player: "p1", row: "backrow" });
      expect(stolen.faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(stolen.defId);
      expect(JSON.stringify(s.view("p2"))).not.toContain(stolen.defId);
    });

    it("R386 a Degrade of the threshold to 5 stops 4 permanents; an Upgrade to 3 lets 3 be enough", () => {
      const harder = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO, VANILLA] } });
      stepParam(harder.card(TECH), "threshold", 1);
      harder.play(TECH, { zone: 5 });
      expect(stolenIds(harder)).toEqual([]);

      const easier = scenario({ p1: { hand: [TECH, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO] } });
      stepParam(easier.card(TECH), "threshold", -1);
      easier.play(TECH, { zone: 5 });
      expect(stolenIds(easier)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("is a 6/6 that, with 4 enemy permanents, asks which one to steal and steals that one", () => {
      const s = scenario({
        p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO], backrow: [FIELD_SPELL] },
      });

      s.play(TECH, { zone: 5 });

      s.expectStats(TECH, { attack: 6, health: 6 });
      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p1");
      expect(pending?.options).toHaveLength(4);
      const menace = s.card(MENACE);
      s.answer([{ pick: "instance", instanceId: menace.id }]);

      expect(stolenIds(s)).toEqual([menace.id]);
      expect(s.card(menace.id).controller).toBe("p1");
    });

    it("R13 the prompt offers the top of a Stack pile and never the card beneath it", () => {
      const s = scenario({
        p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }, MENACE, TEMPO], backrow: [FIELD_SPELL] },
      });

      s.play(TECH, { zone: 5 });

      const offered = (s.state.pending?.options ?? []).map((option) => option.selection);
      expect(offered).toHaveLength(4);
      expect(offered).toContainEqual({ pick: "instance", instanceId: s.card(FIENDER).id });
      expect(offered).not.toContainEqual({ pick: "instance", instanceId: s.card(VANILLA).id });
    });

    it("R15 with your unit row full the prompt still opens, and a Unit you pick stays with them", () => {
      const s = scenario({
        p1: { hand: [{ def: TECH, radiant: true }, ANCHOR], field: [MENACE, MENACE, MENACE, MENACE] },
        p2: { hand: [ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA] },
      });
      s.play(TECH, { zone: 5 });
      const picked = s.unit("p2", 1);
      if (picked === null) throw new Error("a Vanilla in lane 1");

      s.answer([{ pick: "instance", instanceId: picked.id }]);

      expect(stolenIds(s)).toEqual([]);
      expect(s.unit("p2", 1)?.id).toBe(picked.id);
      expect(s.card(picked.id).controller).toBe("p2");
    });

    it("3 or fewer: no prompt opens and nothing is stolen", () => {
      const s = scenario({ p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO] } });

      s.play(TECH, { zone: 5 });

      expect(s.state.pending).toBeNull();
      expect(stolenIds(s)).toEqual([]);
    });

    it("R177 a face-down option carries only its id: your view of the prompt never names it", () => {
      const s = scenario({ p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], backrow: FACE_DOWN_TRAPS } });

      s.play(TECH, { zone: 5 });

      const prompt = JSON.stringify(s.view("p1").pending ?? null);
      expect(prompt).not.toBe("null");
      for (const trap of [PAWN, BREAD, STIMMY, UNLICENSED]) expect(prompt).not.toContain(trap);
    });

    it("R33 the trap you pick is read by you once it is yours", () => {
      const s = scenario({ p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], backrow: FACE_DOWN_TRAPS } });
      s.play(TECH, { zone: 5 });
      const pawn = s.card(PAWN);

      s.answer([{ pick: "instance", instanceId: pawn.id }]);

      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(PAWN);
      expect(JSON.stringify(s.view("p2"))).not.toContain(PAWN);
    });

    it("R113 the paused pick survives a JSON round trip and resumes through reduce", () => {
      const s = scenario({
        p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO], backrow: [FIELD_SPELL] },
      });
      s.play(TECH, { zone: 5 });
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const tempo = s.card(TEMPO);

      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: revived.pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: tempo.id }],
        nonce: "rt-040",
      });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.events.some((event) => event.type === "controlChanged" && event.instanceId === tempo.id)).toBe(true);
    });

    it("R386 a Degrade of the threshold to 5 stops 4 permanents asking", () => {
      const s = scenario({
        p1: { hand: [{ def: TECH, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, MENACE, TEMPO], backrow: [FIELD_SPELL] },
      });
      stepParam(s.card(TECH), "threshold", 1);

      s.play(TECH, { zone: 5 });

      expect(s.state.pending).toBeNull();
    });
  });
});
