// C #81 The Power to Thrive — SPEC §8.6 row 81, BUILD M9 Classic row C 81: "Activate, once per turn
// (R384), the mode carried in the `activate` action: heal your hero 3 (no cap, R19), draw 1, or gain 1 mana
// this turn; usable the turn it is played; a second activation that turn is refused; not a play; radiant:
// heal 6, draw 2, or 2 mana; its tuned numbers (heal, draw, mana) read through `param()` (R386)".

import { legalActions, stepParam } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/081-the-power-to-thrive";

const THRIVE = "classic-081";
const FILLER = "core-005"; // (1) Spell (§2.5).
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const X = "core-020"; // library filler.

type Activate = Extract<ActionBody, { type: "activate" }>;

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

function offeredModes(s: Scenario): string[] {
  return legalActions(s.state, "p1")
    .filter((action): action is Activate => action.type === "activate" && action.instanceId === s.card(THRIVE).id)
    .flatMap((action) => action.modes ?? [])
    .sort();
}

function standing(radiantFace = false, extra: { health?: number; library?: number; hand?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: { hand: [...(extra.hand ?? [ANCHOR])], backrow: [{ def: THRIVE, radiant: radiantFace }], library: lib(extra.library ?? 4), health: extra.health ?? 20 },
    p2: { hand: [ANCHOR], library: lib(4) },
  });
}

describe("C #81 The Power to Thrive", () => {
  it("is a Field Spell with one once-a-turn ability of three modes, its three numbers, one script on both faces", () => {
    expect(def.id).toBe(THRIVE);
    expect(def.type).toBe("Field Spell");
    expect(def.params).toEqual([
      { key: "heal", base: 3, radiant: 6, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "mana", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    expect(base.activations?.map((ability) => [ability.uses, ability.modes])).toEqual([[1, [{ kind: "mode", options: ["heal", "draw", "mana"] }]]]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R384 legalActions offers the ability once per mode, the mode carried in the action", () => {
      const s = standing();
      expect(offeredModes(s)).toEqual(["draw", "heal", "mana"]);
    });

    it("heal: your hero 3", () => {
      const s = standing();
      s.activate(THRIVE, { modes: ["heal"] });
      s.expectHealth("p1", 23);
      s.expectEvents("activated", "healed");
    });

    it("R19 heal has no cap on a hero: at 30 it goes to 33", () => {
      const s = standing(false, { health: 30 });
      s.activate(THRIVE, { modes: ["heal"] });
      s.expectHealth("p1", 33);
    });

    it("draw: 1", () => {
      const s = standing();
      s.activate(THRIVE, { modes: ["draw"] });
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("§2.3 mana: 1 this turn, above the maximum of 4, and gone at the next refresh", () => {
      const s = standing(false, { hand: [ANCHOR, FILLER] });
      s.activate(THRIVE, { modes: ["mana"] });
      s.expectMana("p1", 5);

      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p1");
      s.expectMana("p1", 4);
    });

    it("R384 it is usable the turn it is played", () => {
      const s = scenario({ p1: { hand: [THRIVE, ANCHOR], health: 20 }, p2: { hand: [ANCHOR] } });
      s.play(THRIVE);

      s.activate(THRIVE, { modes: ["heal"] });

      s.expectHealth("p1", 23);
    });

    it("R384 a second activation that turn is refused, whatever its mode; the next turn it is usable again", () => {
      const s = standing(false, { hand: [ANCHOR, FILLER] });
      s.activate(THRIVE, { modes: ["heal"] });

      expect(offeredModes(s)).toEqual([]);
      expect(() => s.activate(THRIVE, { modes: ["draw"] })).toThrow(/already been used/);

      s.endTurn();
      s.endTurn();
      expect(offeredModes(s)).toEqual(["draw", "heal", "mana"]);
    });

    it("R81 a mode it does not offer is refused", () => {
      const s = standing();
      expect(() => s.activate(THRIVE, { modes: ["steal"] })).toThrow();
      expect(() => s.activate(THRIVE, { modes: [] })).toThrow();
    });

    it("R384 not a play: nothing counts it", () => {
      const s = standing();
      const before = s.state.players.p1.turnLog.cardsPlayed;

      s.activate(THRIVE, { modes: ["draw"] });

      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(before);
    });

    it("§2.4 a full hand burns the draw", () => {
      const s = standing(false, { hand: Array.from({ length: 10 }, () => ANCHOR) });
      s.activate(THRIVE, { modes: ["draw"] });
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R386 an Upgrade heals 4, draws 2, gives 2 mana", () => {
      for (const [mode, check] of [
        ["heal", (s: Scenario) => s.expectHealth("p1", 24)],
        ["draw", (s: Scenario) => expect(drawsBy(s.lastEvents, "p1")).toBe(2)],
        ["mana", (s: Scenario) => s.expectMana("p1", 6)],
      ] as const) {
        const s = standing();
        for (const key of ["heal", "draw", "mana"]) stepParam(s.card(THRIVE), key, 1);
        s.activate(THRIVE, { modes: [mode] });
        check(s);
      }
    });
  });

  describe("radiant", () => {
    it("heal 6, draw 2, or 2 mana", () => {
      const healed = standing(true);
      healed.activate(THRIVE, { modes: ["heal"] });
      healed.expectHealth("p1", 26);

      const drew = standing(true);
      drew.activate(THRIVE, { modes: ["draw"] });
      expect(drawsBy(drew.lastEvents, "p1")).toBe(2);

      const mana = standing(true);
      mana.activate(THRIVE, { modes: ["mana"] });
      mana.expectMana("p1", 6);
    });

    it("R384 still once per turn", () => {
      const s = standing(true);
      s.activate(THRIVE, { modes: ["mana"] });
      expect(() => s.activate(THRIVE, { modes: ["mana"] })).toThrow(/already been used/);
    });

    it("R386 a Degrade heals 5", () => {
      const s = standing(true);
      stepParam(s.card(THRIVE), "heal", -1);
      s.activate(THRIVE, { modes: ["heal"] });
      s.expectHealth("p1", 25);
    });
  });
});
