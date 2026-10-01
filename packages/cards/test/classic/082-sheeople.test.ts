// C #82 Sheeople — SPEC §8.6 row 82, BUILD M9 Classic row C 82: "1/1 worth 2 Tributes, read off the face
// that is up, toward a Tribute X only (a script's Tribute, C #21's included, counts it as one Unit, §6.3),
// so it alone pays a Tribute 2 and overpays a Tribute 1 (R101); Death: draw 2, so tributing it draws; a
// bounce or an exile draws nothing; radiant 2/2 worth 3, draw 3; its tuned numbers (draw, worth) read
// through `param()` (R386)".
//
// A Tribute 2 is #66 The Rock with its Tribute X upgraded once (R101: "printed, or as a Degrade or
// Upgrade left it"). The R386 worth case waits on the engine reading `tributeWorth` through `param`.

import { addStep, legalActions, stepParam, tuningOf, type CardInstance } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/082-sheeople";

const SHEEOPLE = "classic-082";
const ROCK = "core-066"; // (4) Unit 10/10: Tribute 1, Indestructible.
const GOLEM = "core-055"; // (3) Unit 10/5: Taunt, Tribute 3.
const CUBE = "core-022"; // (3) Unit: Cry: Tribute one of your other Units and remember it.
const VANILLA = "core-008"; // (1) Unit 4/4.
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const COLLATERAL = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
const FILLER = "core-005"; // (1) Spell (§2.5).
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const X = "core-020"; // library filler.

type Play = Extract<ActionBody, { type: "play" }>;

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

/** The Tribute sets legalActions offers for a play of `card`, each sorted, deduplicated. */
function tributeSets(s: Scenario, card: CardInstance): string[][] {
  const sets = legalActions(s.state, "p1")
    .filter((action): action is Play => action.type === "play" && action.instanceId === card.id)
    .map((play) => [...(play.tributes ?? [])].sort().join(","));
  return [...new Set(sets)].map((set) => (set === "" ? [] : set.split(",")));
}

/** The Rock, its Tribute X upgraded from 1 to 2. */
function tribute2(s: Scenario): CardInstance {
  const rock = s.card(ROCK);
  const tuning = tuningOf(rock);
  tuning.x = addStep(tuning.x, "Tribute", 1);
  return rock;
}

describe("C #82 Sheeople", () => {
  it("declares its two numbers; the worth is the face's static flag, the draw its Death", () => {
    expect(def.id).toBe(SHEEOPLE);
    expect(def.params).toEqual([
      { key: "draw", base: 2, radiant: 3, better: "up", step: 1, min: 1 },
      { key: "worth", base: 2, radiant: 3, better: "up", step: 1, min: 1 },
    ]);
    expect(base.staticFlags).toEqual({ tributeWorth: 2 });
    expect(radiant.staticFlags).toEqual({ tributeWorth: 3 });
    expect(base.death).toBe(radiant.death);
  });

  describe("base", () => {
    it("is a 1/1", () => {
      scenario({ p1: { field: [SHEEOPLE], hand: [FILLER] } }).expectStats(SHEEOPLE, { attack: 1, health: 1 });
    });

    it("R101 worth 2: it alone pays a Tribute 2, and tributing it draws 2", () => {
      const s = scenario({ p1: { hand: [ROCK, ANCHOR], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const rock = tribute2(s);
      const sheeople = s.card(SHEEOPLE);
      expect(tributeSets(s, rock)).toEqual([[sheeople.id]]);

      s.play(rock, { tributes: [sheeople.id] });

      s.expectInZone(sheeople, "graveyard");
      s.expectInZone(rock, "field");
      expect(drawsBy(s.events, "p1")).toBe(2);
    });

    it("R101 it overpays a Tribute 1: alone it is a minimal set, never paired with another Unit", () => {
      const s = scenario({ p1: { hand: [ROCK, ANCHOR], field: [SHEEOPLE, VANILLA], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const sheeople = s.card(SHEEOPLE);
      const vanilla = s.card(VANILLA);

      expect(tributeSets(s, s.card(ROCK)).sort()).toEqual([[sheeople.id], [vanilla.id]].sort());
      s.play(ROCK, { tributes: [sheeople.id] });

      s.expectInZone(sheeople, "graveyard");
      s.expectInZone(vanilla, "field");
    });

    it("R101 it does not alone pay a Tribute 3; with one more Unit it does", () => {
      const s = scenario({ p1: { hand: [GOLEM, ANCHOR], field: [SHEEOPLE, VANILLA], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const sheeople = s.card(SHEEOPLE);
      const vanilla = s.card(VANILLA);

      expect(tributeSets(s, s.card(GOLEM))).toEqual([[sheeople.id, vanilla.id].sort()]);
      expect(() => s.play(GOLEM, { tributes: [sheeople.id] })).toThrow(/Tribute/);
    });

    it("§6.3 a script's Tribute counts Units: a Carnivorous Cube's Cry takes it as one Unit, and it draws 2", () => {
      const s = scenario({ p1: { hand: [CUBE, ANCHOR], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const sheeople = s.card(SHEEOPLE);

      s.play(CUBE, { targets: [{ pick: "instance", instanceId: sheeople.id }] });

      s.expectInZone(sheeople, "graveyard");
      expect(drawsBy(s.events, "p1")).toBe(2);
    });

    it("§4.5 Death: destroyed, it draws its controller 2", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [HIT_JOB, ANCHOR] } });

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(SHEEOPLE).id }] });

      expect(drawsBy(s.events, "p1")).toBe(2);
      expect(drawsBy(s.events, "p2")).toBe(0);
    });

    it("a bounce draws nothing", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [FLOOD, ANCHOR] } });

      s.play(FLOOD);

      s.expectInZone(SHEEOPLE, "hand");
      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("an exile draws nothing", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [COLLATERAL, ANCHOR] } });

      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(SHEEOPLE).id }] });

      s.expectInZone(SHEEOPLE, "exile");
      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("§2.4 a full hand burns the draws; an empty deck makes them fatigue", () => {
      const full = scenario({
        active: "p2",
        p1: { hand: Array.from({ length: 10 }, () => FILLER), field: [SHEEOPLE], library: lib(3) },
        p2: { hand: [HIT_JOB, ANCHOR] },
      });
      full.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: full.card(SHEEOPLE).id }] });
      expect(full.events.filter((event) => event.type === "burned")).toHaveLength(2);

      const empty = scenario({ active: "p2", p1: { hand: [FILLER], field: [SHEEOPLE] }, p2: { hand: [HIT_JOB, ANCHOR] } });
      empty.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: empty.card(SHEEOPLE).id }] });
      expect(empty.events.filter((event) => event.type === "fatigue" && event.player === "p1")).toHaveLength(2);
    });

    it("R386 an Upgrade of its draw draws 3", () => {
      const s = scenario({ p1: { hand: [ROCK, ANCHOR], field: [SHEEOPLE], library: lib(4) }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(SHEEOPLE), "draw", 1);

      s.play(ROCK, { tributes: [s.card(SHEEOPLE).id] });

      expect(drawsBy(s.events, "p1")).toBe(3);
    });

    it("R386 an Upgrade of its worth makes it worth 3: alone it pays a Tribute 3", () => {
      const s = scenario({ p1: { hand: [GOLEM, ANCHOR], field: [SHEEOPLE], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const sheeople = s.card(SHEEOPLE);
      stepParam(sheeople, "worth", 1);

      expect(tributeSets(s, s.card(GOLEM))).toEqual([[sheeople.id]]);
      s.play(GOLEM, { tributes: [sheeople.id] });
      s.expectInZone(GOLEM, "field");
    });
  });

  describe("radiant", () => {
    it("is a 2/2 worth 3: alone it pays a Tribute 3, and tributing it draws 3", () => {
      const s = scenario({ p1: { hand: [GOLEM, ANCHOR], field: [{ def: SHEEOPLE, radiant: true }], library: lib(4) }, p2: { hand: [ANCHOR] } });
      const sheeople = s.card(SHEEOPLE);
      s.expectStats(sheeople, { attack: 2, health: 2 });

      expect(tributeSets(s, s.card(GOLEM))).toEqual([[sheeople.id]]);
      s.play(GOLEM, { tributes: [sheeople.id] });

      s.expectInZone(GOLEM, "field");
      expect(drawsBy(s.events, "p1")).toBe(3);
    });

    it("a script's Tribute still counts it as one Unit, and a bounce draws nothing", () => {
      const s = scenario({ p1: { hand: [CUBE, FLOOD, ANCHOR], field: [{ def: SHEEOPLE, radiant: true }, VANILLA], library: lib(4), mana: 10 }, p2: { hand: [ANCHOR] } });

      s.play(CUBE, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] });
      s.play(FLOOD);

      s.expectInZone(SHEEOPLE, "hand");
      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("R386 a Degrade of its draw draws 2", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], field: [{ def: SHEEOPLE, radiant: true }], library: lib(4) }, p2: { hand: [HIT_JOB, ANCHOR] } });
      stepParam(s.card(SHEEOPLE), "draw", -1);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(SHEEOPLE).id }] });

      expect(drawsBy(s.events, "p1")).toBe(2);
    });
  });
});
