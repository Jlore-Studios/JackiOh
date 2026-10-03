// C+ #72 Book of Nerf — SPEC §8.7 row 72, BUILD M9 row C+ 72: Degrades a target permanent on either
// side 5 times, each its own draw (R386): attack floors at 0, current health never below 1 (it never
// kills), cost never above (4), no harmful keyword removed, what the floors refuse lost; an Immutable
// target is unchanged; the count reads through `param()`; radiant 10 times on a permanent or a card in
// your hand (the hand clause is the Radiant face's alone).

import { describe, expect, it } from "vitest";
import { HIDDEN_ID, costNow, legalActions, stepParam } from "@jackioh/engine";
import type { GameEvent, Selection } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/072-book-of-nerf";
import { scenario, type Scenario } from "../_harness";

const BOOK = "classicplus-072";
const UNIT = "core-008"; // Mr. Vanilla 4/4.
const MROW = "core-086"; // "Miss" Mrow: (1) 1/1, Can't attack.
const MENACE = "core-019"; // Midrange Menace; Radiant it is Immutable.
const TRAP = "core-041";
const FILLER = "core-005";

function degrades(events: readonly GameEvent[]): Extract<GameEvent, { type: "degraded" }>[] {
  return events.filter((event): event is Extract<GameEvent, { type: "degraded" }> => event.type === "degraded");
}

function pick(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

function offered(s: Scenario): string[] {
  const self = s.card(BOOK);
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "play" && action.instanceId === self.id
      ? (action.targets ?? []).flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []))
      : [],
  );
}

function book(opts: { radiant?: boolean; seed?: string } = {}): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: [{ def: BOOK, radiant: opts.radiant === true }, UNIT, FILLER], field: [UNIT] },
    p2: { hand: [FILLER], field: [MROW, UNIT], backrow: [TRAP] },
  });
}

describe("C+ #72 Book of Nerf", () => {
  it("is a (1) Spell, Book; the Radiant face declares a wider target", () => {
    expect(def).toMatchObject({ cost: 1, type: "Spell", tags: ["Book"] });
    expect(base.targets?.[0]?.filter?.of).toEqual(["unit", "backrow"]);
    expect(radiant.targets?.[0]?.filter?.of).toEqual(["unit", "backrow", "hand"]);
  });

  describe("base", () => {
    it("R81 its target is a permanent on either side, never a card in a hand", () => {
      const s = book();
      const expected = [s.unit("p1", 1)!, s.unit("p2", 1)!, s.unit("p2", 2)!, s.backrow("p2", 1)!].map((card) => card.id);
      expect(new Set(offered(s))).toEqual(new Set(expected));
    });

    it("R386 Degrades a unit 5 times, each its own draw", () => {
      const s = book();
      const unit = s.unit("p2", 2)!;
      s.play(BOOK, { targets: pick(unit.id) });
      const events = degrades(s.events);
      expect(events).toHaveLength(5);
      expect(events.every((event) => event.instanceId === unit.id)).toBe(true);
    });

    it("R386 it never kills: attack floors at 0, current health at 1, what the floors refuse is lost; the harmful keyword stays; cost stops at (4)", () => {
      for (let n = 0; n < 8; n += 1) {
        const s = book({ seed: `nerf-floors-${n}` });
        const mrow = s.unit("p2", 1)!;
        s.play(BOOK, { targets: pick(mrow.id) });
        expect(s.unit("p2", 1)?.id).toBe(mrow.id);
        const stats = s.stats(mrow);
        expect(stats.attack).toBeGreaterThanOrEqual(0);
        expect(stats.health).toBe(1);
        expect(stats.keywords.map((keyword) => keyword.kind)).toContain("Can't attack");
        expect(costNow(s.state, s.card(mrow))).toBeLessThanOrEqual(4);
        for (const event of degrades(s.events)) {
          expect(event.change.kind).not.toBe("keyword");
          if (event.change.kind === "stats") {
            expect(event.change.health).toBe(0);
            expect(event.change.attack).toBeGreaterThanOrEqual(-1);
          }
        }
      }
    });

    it("R386 it may Degrade your own permanent, or an enemy face-down trap", () => {
      const own = book();
      own.play(BOOK, { targets: pick(own.unit("p1", 1)!.id) });
      expect(degrades(own.events)).toHaveLength(5);
      const trap = book();
      const set = trap.backrow("p2", 1)!;
      trap.play(BOOK, { targets: pick(set.id) });
      expect(degrades(trap.events)).toHaveLength(5);
      expect(degrades(trap.view("p1").events).every((event) => event.instanceId === HIDDEN_ID)).toBe(true);
    });

    it("R386 an Immutable target is unchanged", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] } });
      const menace = s.unit("p2", 1)!;
      s.play(BOOK, { targets: pick(menace.id) });
      expect(degrades(s.events)).toEqual([]);
      s.expectStats(menace, { attack: 18, maxHealth: 18 });
    });

    it("R386 the count reads through param(): an Upgrade of `times` makes 6, a Degrade 4", () => {
      for (const [steps, count] of [
        [1, 6],
        [-1, 4],
      ] as const) {
        const s = book();
        stepParam(s.card(BOOK), "times", steps);
        s.play(BOOK, { targets: pick(s.unit("p2", 2)!.id) });
        expect(degrades(s.events)).toHaveLength(count);
      }
    });
  });

  describe("radiant", () => {
    it("R275 may also target a card in your hand", () => {
      const s = book({ radiant: true });
      const hand = s.hand("p1").filter((card) => card.defId !== BOOK).map((card) => card.id);
      expect(offered(s)).toEqual(expect.arrayContaining(hand));
    });

    it("R386 R97 Degrades a hand card 10 times, which the opponent reads only as a card of your hand changing", () => {
      const s = book({ radiant: true });
      const held = s.hand("p1").find((card) => card.defId === UNIT)!;
      s.play(BOOK, { targets: pick(held.id) });
      expect(degrades(s.events)).toHaveLength(10);
      expect(degrades(s.view("p2").events).every((event) => event.instanceId === HIDDEN_ID)).toBe(true);
      // Cost never above (4): Mr. Vanilla costs (1).
      expect(s.card(held).costMod).toBeLessThanOrEqual(3);
    });

    it("R386 R440 Degrades a permanent 10 times: an enemy face-down trap is cued each time", () => {
      const s = book({ radiant: true });
      const trap = s.backrow("p2", 1)!;
      s.play(BOOK, { targets: pick(trap.id) });
      expect(degrades(s.events)).toHaveLength(10);
    });

    it("R386 R129 a public card the floors have emptied is left alone, with no event: Mr. Vanilla bottoms out at 0 attack, 1 health and (4)", () => {
      let emptied = 0;
      for (let n = 0; n < 8; n += 1) {
        const s = scenario({
          seed: `nerf-empty-${n}`,
          p1: { hand: [{ def: BOOK, radiant: true }, { def: BOOK, radiant: true }, FILLER], mana: 10 },
          p2: { hand: [FILLER], field: [UNIT] },
        });
        const unit = s.unit("p2", 1)!;
        const [first, second] = s.hand("p1");
        s.play(first!, { targets: pick(unit.id) });
        const stats = s.stats(unit);
        expect(s.unit("p2", 1)?.id).toBe(unit.id);
        expect(stats.attack).toBeGreaterThanOrEqual(0);
        expect(stats.health).toBeGreaterThanOrEqual(1);
        expect(costNow(s.state, s.card(unit))).toBeLessThanOrEqual(4);
        if (stats.attack !== 0 || stats.health !== 1 || costNow(s.state, s.card(unit)) !== 4) continue;
        emptied += 1;
        const cursor = s.state.rngCursor;
        s.play(second!, { targets: pick(unit.id) });
        expect(degrades(s.lastEvents)).toEqual([]);
        expect(s.state.rngCursor).toBe(cursor);
      }
      expect(emptied).toBeGreaterThan(0);
    });
  });
});
