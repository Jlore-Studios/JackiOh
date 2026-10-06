// C #39 Outbreak — SPEC §8.6 row 39, BUILD M9 Classic row C 39: "A declared target, any permanent
// either side, face-down included; one placement of 1 (C #27 doubles it); then, if it is an enemy
// permanent with at least as many Plague Counters as its cost (R65 on the field; an X card its X, 0 with
// none chosen, R396), steal it (§6.3, R15), an entry (R171); no free zone → it stays with them and
// nothing is drawn; otherwise draw one card per token on it (hand cap); your own permanent always
// draws; a (0) Cost enemy permanent is always stolen; a stolen face-down trap is read by you alone
// from then on (R33); a face-down option, and the placement on it, never name it to you (R177);
// radiant: 2 tokens; its tuned number (tokens) reads through `param()` (R386)".
//
// The R396 cases read costs through the engine's `costNow`. A Plague Chalice played for X enters with
// X Plague Counters (SPEC §8.6 row 87), so any placement brings it to its X; the cases that need an X card
// played for 3 with fewer tokens than its X use C+ #69 Buff Billy, the other X permanent R396 names.

import { stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/039-outbreak";

const OUTBREAK = "classic-039";
const SLIME = "classic-027"; // (0) Unit 1/1: placements on it doubled.
const CHALICE = "classic-087"; // (X) Field Spell.
const BILLY = "classicplus-069"; // (X) Unit 3X/3X; its Cry Upgrades it, never its cost (R396).
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9.
const PAWN = "core-096"; // (1) Trap, answers only a lethal attack.
const UNLICENSED = "core-085"; // (2) Trap; answers a permanent of a type its controller controls.
const FILLER = "core-005";
const ANCHOR = "core-010";
const X = "core-020";

function at(s: Scenario, ref: string): Selection[] {
  return [{ pick: "instance", instanceId: s.card(ref).id }];
}

function stolenIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "controlChanged" ? [event.instanceId] : []));
}

function drawn(s: Scenario): number {
  return s.events.filter((event) => event.type === "drawn").length;
}

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

describe("C #39 Outbreak", () => {
  it("declares one target, any permanent on either side, and runs one script on both faces", () => {
    expect(def.id).toBe(OUTBREAK);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("places 1 Plague Counter on your own permanent, then draws one card per token on it", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [{ def: VANILLA, counters: { plague: 2 } }], library: lib(5) }, p2: { hand: [ANCHOR] } });

      s.play(OUTBREAK, { targets: at(s, VANILLA) });

      expect(s.card(VANILLA).counters.plague).toBe(3);
      expect(drawn(s)).toBe(3);
      expect(stolenIds(s)).toEqual([]);
    });

    it("your own permanent always draws, even at or above its cost", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [VANILLA], library: lib(3) }, p2: { hand: [ANCHOR] } });

      s.play(OUTBREAK, { targets: at(s, VANILLA) });

      expect(drawn(s)).toBe(1);
      expect(s.card(VANILLA).controller).toBe("p1");
    });

    it("an enemy permanent with fewer tokens than its cost: it draws one per token", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], field: [MENACE] } });

      s.play(OUTBREAK, { targets: at(s, MENACE) });

      expect(s.card(MENACE).counters.plague).toBe(1);
      expect(drawn(s)).toBe(1);
      expect(stolenIds(s)).toEqual([]);
    });

    it("R15 R171 an enemy permanent reaching its cost is stolen into the same lane, an entry, and nothing is drawn", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(5) }, p2: { hand: [ANCHOR], field: [{ def: MENACE, lane: 2, counters: { plague: 2 } }] } });
      const menace = s.card(MENACE);

      s.play(OUTBREAK, { targets: at(s, MENACE) });

      expect(stolenIds(s)).toEqual([menace.id]);
      expect(s.unit("p1", 2)?.id).toBe(menace.id);
      expect(s.card(menace.id).counters.plague).toBe(3);
      expect(drawn(s)).toBe(0);
      expect(() => s.attack(menace, "hero")).toThrow();
    });

    it("a (0) Cost enemy permanent is always stolen", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], field: [SLIME] } });

      s.play(OUTBREAK, { targets: at(s, SLIME) });

      expect(stolenIds(s)).toEqual([s.card(SLIME).id]);
    });

    it("C #27 a Pestilent Slime doubles the placement: your own Slime takes 2 and you draw 2", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [SLIME], library: lib(3) }, p2: { hand: [ANCHOR] } });

      s.play(OUTBREAK, { targets: at(s, SLIME) });

      expect(s.card(SLIME).counters.plague).toBe(2);
      expect(drawn(s)).toBe(2);
    });

    it("R15 with no free zone in your row the stolen card stays with them, and nothing is drawn", () => {
      const s = scenario({
        p1: { hand: [OUTBREAK, ANCHOR], field: [MENACE, MENACE, MENACE, MENACE, MENACE], library: lib(3) },
        p2: { hand: [ANCHOR], field: [VANILLA] },
      });

      s.play(OUTBREAK, { targets: at(s, VANILLA) });

      expect(stolenIds(s)).toEqual([]);
      expect(s.card(VANILLA).controller).toBe("p2");
      expect(drawn(s)).toBe(0);
    });

    it("§2.4 the hand cap burns the draws that do not fit", () => {
      const s = scenario({
        p1: { hand: [OUTBREAK, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], library: lib(3) },
        p2: { hand: [ANCHOR] },
      });

      s.play(OUTBREAK, { targets: at(s, VANILLA) });

      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R33 a stolen face-down trap is read by you alone from then on", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], backrow: [{ def: PAWN, faceUp: false }] } });
      const pawn = s.card(PAWN);

      s.play(OUTBREAK, { targets: [{ pick: "instance", instanceId: pawn.id }] });

      expect(stolenIds(s)).toEqual([pawn.id]);
      expect(s.card(pawn.id).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(PAWN);
      expect(JSON.stringify(s.view("p2"))).not.toContain(PAWN);
    });

    it("R177 a face-down enemy option, and the placement on it, never name it to you", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], backrow: [{ def: UNLICENSED, faceUp: false }] } });
      const trap = s.card(UNLICENSED);

      expect(JSON.stringify(s.view("p1"))).not.toContain(UNLICENSED);
      s.play(OUTBREAK, { targets: [{ pick: "instance", instanceId: trap.id }] });

      // (2) Cost with 1 token: not stolen, so it stays theirs, face-down, and you draw 1.
      expect(stolenIds(s)).toEqual([]);
      expect(drawn(s)).toBe(1);
      expect(s.card(trap.id).counters.plague).toBe(1);
      expect(JSON.stringify(s.view("p1"))).not.toContain(UNLICENSED);
    });

    it("R396 an X card on the field costs the X it was played for: a Buff Billy played for 3 is not stolen by 1 token", () => {
      const s = scenario({ active: "p2", p1: { hand: [OUTBREAK, ANCHOR], library: lib(4) }, p2: { hand: [BILLY, ANCHOR], library: lib(2) } });
      s.play(BILLY, { x: 3 });
      s.endTurn();
      expect(s.state.active).toBe("p1");

      s.play(OUTBREAK, { targets: at(s, BILLY) });

      expect(s.card(BILLY).counters.plague).toBe(1);
      expect(stolenIds(s)).toEqual([]);
      expect(s.card(BILLY).controller).toBe("p2");
      expect(s.lastEvents.filter((event) => event.type === "drawn")).toHaveLength(1);
    });

    it("R396 C #87 a Plague Chalice played for 3 enters with 3 Plague Counters, so 1 more reaches its X and steals it", () => {
      const s = scenario({ active: "p2", p1: { hand: [OUTBREAK, ANCHOR], library: lib(4) }, p2: { hand: [CHALICE, ANCHOR], library: lib(2) } });
      s.play(CHALICE, { x: 3 });
      s.endTurn();

      s.play(OUTBREAK, { targets: at(s, CHALICE) });

      expect(s.card(CHALICE).counters.plague).toBe(4);
      expect(stolenIds(s)).toEqual([s.card(CHALICE).id]);
      expect(s.lastEvents.filter((event) => event.type === "drawn")).toHaveLength(0);
    });

    it("R396 an X card that arrived with no X chosen costs 0 on the field, so it is stolen", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], backrow: [CHALICE] } });

      s.play(OUTBREAK, { targets: at(s, CHALICE) });

      expect(stolenIds(s)).toEqual([s.card(CHALICE).id]);
    });

    it("R386 an Upgrade places 2: an enemy (2) Cost permanent is stolen", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], backrow: [{ def: UNLICENSED, faceUp: false }] } });
      stepParam(s.card(OUTBREAK), "tokens", 1);
      const trap = s.card(UNLICENSED);

      s.play(OUTBREAK, { targets: [{ pick: "instance", instanceId: trap.id }] });

      expect(s.card(trap.id).counters.plague).toBe(2);
      expect(stolenIds(s)).toEqual([trap.id]);
    });
  });

  describe("radiant", () => {
    it("places 2: your own permanent draws 2", () => {
      const s = scenario({ p1: { hand: [{ def: OUTBREAK, radiant: true }, ANCHOR], field: [VANILLA], library: lib(3) }, p2: { hand: [ANCHOR] } });

      s.play(OUTBREAK, { targets: at(s, VANILLA) });

      expect(s.card(VANILLA).counters.plague).toBe(2);
      expect(drawn(s)).toBe(2);
    });

    it("places 2: an enemy (3) Cost permanent with 1 already reaches 3 and is stolen", () => {
      const s = scenario({ p1: { hand: [{ def: OUTBREAK, radiant: true }, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], field: [{ def: MENACE, counters: { plague: 1 } }] } });

      s.play(OUTBREAK, { targets: at(s, MENACE) });

      expect(stolenIds(s)).toEqual([s.card(MENACE).id]);
      expect(drawn(s)).toBe(0);
    });

    it("R396 a Buff Billy played for 3 is not stolen by 2 tokens; it draws 2", () => {
      const s = scenario({ active: "p2", p1: { hand: [{ def: OUTBREAK, radiant: true }, ANCHOR], library: lib(4) }, p2: { hand: [BILLY, ANCHOR], library: lib(2) } });
      s.play(BILLY, { x: 3 });
      s.endTurn();

      s.play(OUTBREAK, { targets: at(s, BILLY) });

      expect(stolenIds(s)).toEqual([]);
      expect(s.lastEvents.filter((event) => event.type === "drawn")).toHaveLength(2);
    });

    it("R386 an Upgrade places 3: a Buff Billy played for 3 reaches its X and is stolen", () => {
      const s = scenario({ active: "p2", p1: { hand: [{ def: OUTBREAK, radiant: true }, ANCHOR], library: lib(4) }, p2: { hand: [BILLY, ANCHOR], library: lib(2) } });
      s.play(BILLY, { x: 3 });
      s.endTurn();
      stepParam(s.card(OUTBREAK), "tokens", 1);

      s.play(OUTBREAK, { targets: at(s, BILLY) });

      expect(stolenIds(s)).toEqual([s.card(BILLY).id]);
    });
  });
});
