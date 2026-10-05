// C+ #34 Memory Leak — SPEC §8.7 row 34, BUILD M9 Classic+ row C+ 34: "Field Spell; the mode is chosen
// with the play (R81) and kept in `memory.mode`: "End of turn" Locks one random zone of the opponent's
// ten not already Locked at each end of your turn (an occupied zone is fine, a Lock evicts nothing; all
// ten Locked, nothing and no random draw, R129); "After your opponent plays a Unit or Field Spell" Locks
// the zone that card went into (a cast counts, R70; a Spell, Trap or Field Trap doesn't; the card
// stays); a Locked unit zone stops a Reborn return there (R175); radiant both at once, no choice".

import { describe, expect, it } from "vitest";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";

const LEAK = "classicplus-034";
const WARDRUM = "classicplus-037";
const BONE_STORM = "classicplus-036-1"; // (1) Spell: 1 damage to each enemy.
const DEFENDER = "core-003"; // 1/1 Taunt, Divine Shield, Reborn.
const MENACE = "core-019"; // (3) 9/9.
const MANA_WELL = "core-006"; // (3) Field Spell.
const LUNAR_ECLIPSE = "core-035"; // (1) Spell: 3 damage to a target.
const SHEEPISH = "core-041"; // (1) Trap.
const BREAD = "core-018"; // (1) Field Trap.
const FILLER = "core-005";

type Lane = { row: "units" | "backrow"; lane: number };

function lockedOf(s: Scenario, player: PlayerId): Lane[] {
  const locks = s.view("p1")[player === "p1" ? "you" : "opponent"].locks;
  return [
    ...locks.units.flatMap((on, i) => (on ? [{ row: "units" as const, lane: i + 1 }] : [])),
    ...locks.backrow.flatMap((on, i) => (on ? [{ row: "backrow" as const, lane: i + 1 }] : [])),
  ];
}

/** p1 plays Memory Leak (base with `mode`, or the Radiant face with none). */
function leak(mode: "endOfTurn" | "afterPlay" | null, p2: { hand?: string[]; field?: string[]; backrow?: string[] } = {}): Scenario {
  const s = scenario({
    p1: { hand: [{ def: LEAK, radiant: mode === null }, FILLER], library: [FILLER, FILLER, FILLER, FILLER], mana: 4 },
    p2: { hand: p2.hand ?? [FILLER], field: p2.field ?? [], backrow: p2.backrow ?? [], library: [FILLER, FILLER, FILLER, FILLER] },
  });
  s.play(LEAK, mode === null ? {} : { modes: [mode] });
  return s;
}

describe("C+ #34 Memory Leak", () => {
  describe("base", () => {
    it("R81 declares its two modes and keeps the chosen one on the instance", () => {
      const s = leak("endOfTurn");
      expect(s.card(LEAK).memory.mode).toBe("endOfTurn");
      expect(s.card(LEAK).zone.z).toBe("field");
      expect(leak("afterPlay").card(LEAK).memory.mode).toBe("afterPlay");
    });

    it("refuses a play without a mode", () => {
      const s = scenario({ p1: { hand: [LEAK, FILLER] }, p2: { hand: [FILLER] } });
      expect(() => s.play(LEAK)).toThrow();
    });

    it("end of turn: Locks one random zone of the opponent's, at each end of your turn", () => {
      const s = leak("endOfTurn");
      expect(lockedOf(s, "p2")).toHaveLength(0);
      s.endTurn(); // p1's end of turn: one Lock.
      expect(lockedOf(s, "p2")).toHaveLength(1);
      expect(lockedOf(s, "p1")).toHaveLength(0);
      s.endTurn(); // p2's end of turn: none.
      expect(lockedOf(s, "p2")).toHaveLength(1);
      s.endTurn(); // p1's again.
      expect(lockedOf(s, "p2")).toHaveLength(2);
      expect(s.events.filter((event) => event.type === "locked").every((event) => event.type === "locked" && event.player === "p2")).toBe(true);
    });

    it("R60 the zone is a uniform pick among the ten, occupied zones included: a Lock evicts nothing", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 16; i += 1) {
        const s = scenario({
          seed: `leak-${i}`,
          p1: { hand: [LEAK, FILLER], library: [FILLER, FILLER], mana: 4 },
          p2: { hand: [FILLER], field: [MENACE, MENACE, MENACE, MENACE, MENACE], backrow: [MANA_WELL] },
        });
        s.play(LEAK, { modes: ["endOfTurn"] });
        s.endTurn();
        const [lock] = lockedOf(s, "p2");
        if (lock === undefined) throw new Error("no lock");
        seen.add(`${lock.row}${lock.lane}`);
        expect(s.pile("p2", "graveyard")).toHaveLength(0);
        expect([1, 2, 3, 4, 5].every((lane) => s.unit("p2", lane)?.defId === MENACE)).toBe(true);
      }
      expect(seen.size).toBeGreaterThan(3);
      expect([...seen].some((key) => key.startsWith("units"))).toBe(true);
    });

    it("never a zone that is Locked already, and R129 with all ten Locked it draws nothing", () => {
      const s = leak("endOfTurn");
      const locks = s.state.players.p2.locks;
      locks.units = locks.units.map(() => true);
      locks.backrow = locks.backrow.map((_, i) => i !== 2);
      s.endTurn();
      expect(lockedOf(s, "p2")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "locked").at(-1)).toMatchObject({ player: "p2", row: "backrow", lane: 3 });

      const full = leak("endOfTurn");
      const all = full.state.players.p2.locks;
      all.units = all.units.map(() => true);
      all.backrow = all.backrow.map(() => true);
      const cursor = full.state.rngCursor;
      const from = full.events.length;
      full.endTurn();
      expect(full.events.slice(from).some((event) => event.type === "locked")).toBe(false);
      // p2's turn draws nothing random here either, so the cursor is untouched by the Leak.
      expect(full.state.rngCursor).toBe(cursor);
    });

    it("the end-of-turn mode does nothing after the opponent plays a Unit", () => {
      const s = leak("endOfTurn", { hand: [MENACE, FILLER] });
      s.endTurn();
      const before = lockedOf(s, "p2").length;
      s.play(MENACE);
      expect(lockedOf(s, "p2")).toHaveLength(before);
    });

    it("after play: the opponent's Unit has its zone Locked once it resolves, and stays in it", () => {
      const s = leak("afterPlay", { hand: [MENACE, FILLER] });
      s.endTurn();
      s.play(MENACE, { zone: 3 });
      expect(lockedOf(s, "p2")).toEqual([{ row: "units", lane: 3 }]);
      expect(s.unit("p2", 3)?.defId).toBe(MENACE);
      const resolved = s.events.findIndex((event) => event.type === "cardResolved" && event.defId === MENACE);
      const locked = s.events.findIndex((event) => event.type === "locked");
      expect(locked).toBeGreaterThan(resolved);
      // No end-of-turn Lock in this mode.
      s.endTurn();
      expect(lockedOf(s, "p2")).toHaveLength(1);
    });

    it("after play: a Field Spell's backrow zone is Locked", () => {
      const s = leak("afterPlay", { hand: [MANA_WELL, FILLER] });
      s.endTurn();
      s.play(MANA_WELL, { zone: 2 });
      expect(lockedOf(s, "p2")).toEqual([{ row: "backrow", lane: 2 }]);
      expect(s.backrow("p2", 2)?.defId).toBe(MANA_WELL);
    });

    it("after play: a Spell, a Trap or a Field Trap Locks nothing", () => {
      const s = leak("afterPlay", { hand: [LUNAR_ECLIPSE, SHEEPISH, BREAD, FILLER] });
      s.endTurn();
      s.play(LUNAR_ECLIPSE, { targets: [{ pick: "hero", player: "p1" }] });
      s.play(SHEEPISH);
      s.play(BREAD);
      expect(lockedOf(s, "p2")).toHaveLength(0);
    });

    it("after play: your own plays Lock nothing", () => {
      const s = scenario({
        p1: { hand: [LEAK, MENACE, FILLER], library: [FILLER], mana: 7 },
        p2: { hand: [FILLER] },
      });
      s.play(LEAK, { modes: ["afterPlay"] });
      s.play(MENACE);
      expect(lockedOf(s, "p1")).toHaveLength(0);
      expect(lockedOf(s, "p2")).toHaveLength(0);
    });

    it("R70 a cast counts: the opponent's Wardrum's copy of their Field Spell is Locked too", () => {
      const s = scenario({
        p1: { hand: [{ def: LEAK }, FILLER], library: [FILLER, FILLER], mana: 4 },
        p2: { hand: [MANA_WELL, FILLER], field: [{ def: WARDRUM, radiant: true }], library: [FILLER, FILLER] },
      });
      s.play(LEAK, { modes: ["afterPlay"] });
      s.endTurn();
      s.play(MANA_WELL, { zone: 1 });
      expect(lockedOf(s, "p2")).toEqual([{ row: "backrow", lane: 1 }]);
      s.endTurn(); // p2's end of turn: Wardrum casts a copy of Mana Well into an open zone.
      const wells = [1, 2, 3, 4, 5].filter((lane) => s.backrow("p2", lane)?.defId === MANA_WELL);
      expect(wells).toHaveLength(2);
      expect(lockedOf(s, "p2").map((lock) => lock.lane).sort()).toEqual(wells);
    });

    it("R175 R688 a Locked unit zone still takes a Reborn return: the return is no play", () => {
      const s = scenario({
        p1: { hand: [LEAK, BONE_STORM, BONE_STORM, FILLER], library: [FILLER, FILLER], mana: 4 },
        p2: { hand: [DEFENDER, FILLER], library: [FILLER, FILLER] },
      });
      s.play(LEAK, { modes: ["afterPlay"] });
      s.endTurn();
      s.play(DEFENDER, { zone: 2 });
      expect(lockedOf(s, "p2")).toEqual([{ row: "units", lane: 2 }]);
      s.endTurn();
      // Two Bone Storms: the first takes the Divine Shield, the second the body.
      const [first, second] = s.hand("p1").filter((card) => card.defId === BONE_STORM);
      if (first === undefined || second === undefined) throw new Error("two Bone Storms in hand");
      s.play(first);
      s.play(second);
      expect(s.unit("p2", 2)?.defId).toBe(DEFENDER);
      expect(lockedOf(s, "p2")).toEqual([{ row: "units", lane: 2 }]);
    });
  });

  describe("radiant", () => {
    it("both at once, with no choice: an end-of-turn Lock and a Lock after each Unit or Field Spell", () => {
      const s = leak(null, { hand: [MENACE, FILLER] });
      expect(s.card(LEAK).memory.mode).toBeUndefined();
      s.endTurn(); // p1's end of turn: one random Lock.
      const after = lockedOf(s, "p2");
      expect(after).toHaveLength(1);
      const free = [1, 2, 3, 4, 5].find((lane) => !after.some((lock) => lock.row === "units" && lock.lane === lane));
      s.play(MENACE, { zone: free });
      expect(lockedOf(s, "p2")).toHaveLength(2);
      expect(lockedOf(s, "p2")).toContainEqual({ row: "units", lane: free });
    });
  });
});
