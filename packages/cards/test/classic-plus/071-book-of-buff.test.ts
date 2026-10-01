// C+ #71 Book of Buff — SPEC §8.7 row 71, BUILD M9 row C+ 71: Upgrades one card 5 times, the target
// chosen with the play (R81) — a permanent on either side (an enemy face-down trap offered by id only,
// R177) or a card in your hand; each Upgrade its own draw (R386); an Immutable target is unchanged; a
// hand card's changes are hidden from the opponent (R97); the count reads through `param()`; radiant
// 10 times.

import { describe, expect, it } from "vitest";
import { HIDDEN_ID, legalActions, stepParam } from "@jackioh/engine";
import type { GameEvent, Selection } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/071-book-of-buff";
import { scenario, type Scenario } from "../_harness";

const BOOK = "classicplus-071";
const UNIT = "core-008"; // Mr. Vanilla 4/4.
const MENACE = "core-019"; // Midrange Menace; Radiant it is Immutable.
const TRAP = "core-041"; // Sheepish, set face-down.
const FIELD = "core-006"; // Mana Well.
const FILLER = "core-005";

function upgrades(events: readonly GameEvent[]): Extract<GameEvent, { type: "upgraded" }>[] {
  return events.filter((event): event is Extract<GameEvent, { type: "upgraded" }> => event.type === "upgraded");
}

function pick(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

function book(opts: { radiant?: boolean } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: BOOK, radiant: opts.radiant === true }, UNIT, FILLER], field: [UNIT], backrow: [FIELD] },
    p2: { hand: [FILLER], field: [UNIT], backrow: [TRAP] },
  });
}

describe("C+ #71 Book of Buff", () => {
  it("is a (1) Spell, Book, with one script on both faces", () => {
    expect(def).toMatchObject({ cost: 1, type: "Spell", tags: ["Book"] });
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R81 R177 the target is a permanent on either side or a card in your hand; an enemy face-down trap is offered by id only", () => {
      const s = book();
      const self = s.card(BOOK);
      const offered = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === self.id ? (action.targets ?? []) : [],
      );
      const ids = offered.map((selection) => (selection.pick === "instance" ? selection.instanceId : selection.pick));
      const expected = [s.unit("p1", 1)!, s.backrow("p1", 1)!, s.unit("p2", 1)!, s.backrow("p2", 1)!].map((card) => card.id);
      const hand = s.hand("p1").filter((card) => card.id !== self.id).map((card) => card.id);
      expect(new Set(ids)).toEqual(new Set([...expected, ...hand]));
      for (const selection of offered) expect(Object.keys(selection).sort()).toEqual(["instanceId", "pick"]);
      expect(ids).not.toContain(s.hand("p2")[0]!.id);
    });

    it("R386 Upgrades a unit 5 times, each its own draw", () => {
      const s = book();
      const unit = s.unit("p2", 1)!;
      s.play(BOOK, { targets: pick(unit.id) });
      const events = upgrades(s.events);
      expect(events).toHaveLength(5);
      expect(events.every((event) => event.instanceId === unit.id)).toBe(true);
      expect(events.every((event) => event.change.kind !== "none")).toBe(true);
    });

    it("R97 Upgrades a card in your hand, and the opponent reads only that a card of your hand changed", () => {
      const s = book();
      const held = s.hand("p1").find((card) => card.defId === UNIT)!;
      s.play(BOOK, { targets: pick(held.id) });
      expect(upgrades(s.events)).toHaveLength(5);
      expect(s.card(held).tuning ?? s.card(held).costMod).toBeTruthy();
      const theirs = upgrades(s.view("p2").events);
      expect(theirs).toHaveLength(5);
      expect(theirs.every((event) => event.instanceId === HIDDEN_ID && event.defId === HIDDEN_ID)).toBe(true);
      expect(upgrades(s.view("p1").events).every((event) => event.instanceId === held.id)).toBe(true);
    });

    it("R177 an enemy face-down trap may be its target; its changes are its controller's to read", () => {
      const s = book();
      const trap = s.backrow("p2", 1)!;
      s.play(BOOK, { targets: pick(trap.id) });
      expect(upgrades(s.events)).toHaveLength(5);
      expect(upgrades(s.view("p1").events).every((event) => event.instanceId === HIDDEN_ID)).toBe(true);
      expect(upgrades(s.view("p2").events).every((event) => event.instanceId === trap.id)).toBe(true);
    });

    it("R386 an Immutable target is unchanged, with no draw", () => {
      const s = scenario({ p1: { hand: [BOOK, FILLER] }, p2: { hand: [FILLER], field: [{ def: MENACE, radiant: true }] } });
      const menace = s.unit("p2", 1)!;
      s.play(BOOK, { targets: pick(menace.id) });
      expect(upgrades(s.events)).toEqual([]);
      expect(s.card(menace).tuning).toBeUndefined();
      s.expectStats(menace, { attack: 18, maxHealth: 18 });
    });

    it("R90 with nothing to target the play is still legal and Upgrades nothing", () => {
      const s = scenario({ p1: { hand: [BOOK] }, p2: { hand: [FILLER] } });
      s.play(BOOK);
      expect(upgrades(s.events)).toEqual([]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R386 the count reads through param(): an Upgrade of `times` makes 6, a Degrade 4", () => {
      for (const [steps, count] of [
        [1, 6],
        [-1, 4],
      ] as const) {
        const s = book();
        stepParam(s.card(BOOK), "times", steps);
        s.play(BOOK, { targets: pick(s.unit("p1", 1)!.id) });
        expect(upgrades(s.events)).toHaveLength(count);
      }
    });
  });

  describe("radiant", () => {
    it("R386 Upgrades its target 10 times", () => {
      const s = book({ radiant: true });
      const unit = s.unit("p1", 1)!;
      s.play(BOOK, { targets: pick(unit.id) });
      expect(upgrades(s.events)).toHaveLength(10);
      expect(upgrades(s.events).every((event) => event.instanceId === unit.id)).toBe(true);
    });

    it("may still target a card in your hand", () => {
      const s = book({ radiant: true });
      const held = s.hand("p1").find((card) => card.defId === UNIT)!;
      s.play(BOOK, { targets: pick(held.id) });
      expect(upgrades(s.events)).toHaveLength(10);
    });
  });
});
