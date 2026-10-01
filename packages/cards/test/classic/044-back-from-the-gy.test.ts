// C #44 Back from the GY — SPEC §8.6 row 44, BUILD M9 Classic row C 44: "A budgeted pick from your
// graveyard: Units whose costs (R65 out of play: an X Unit 0) total (5) or less, a pick over the
// budget refused; each is summoned without a Cry into your leftmost open zones, a full board leaving
// the rest in the graveyard; no Unit there → nothing; then this Spell is exiled, not sent to the
// graveyard (§5.1); radiant: every Unit there, oldest first, until the board is full; its tuned number
// (budget) reads through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { reduce, stepParam, type GameState, type PendingChoice } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/044-back-from-the-gy";

const BACK = "classic-044";
const FILLER = "core-005"; // (1) Spell, a spare card (§2.5); also a non-Unit in the graveyard.
const MR_TOKEN = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
const TIMMY = "core-011"; // (1) Unit 3/3 Rush, First Strike
const POINTMASTER = "core-020"; // (2) Unit 7/1
const MENACE = "core-019"; // (3) Unit 9/9
const SEVEN = "core-025"; // (4) Unit 7/7
const VANILLA = "core-008"; // (1) Unit 4/4
const BILLY = "classicplus-069"; // (X) Unit, "This is a 3X/3X."
const RUSH_TOKEN = "core-t-rush";

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the scenario has no ${what}`);
  return value;
}

function open(s: Scenario): PendingChoice {
  return must(s.state.pending, "open prompt");
}

function back(
  radiantFace: boolean,
  graveyard: readonly (string | { def: string; costMod?: number })[],
  field: readonly string[] = [],
): Scenario {
  return scenario({
    p1: { hand: [{ def: BACK, radiant: radiantFace }, FILLER], graveyard: [...graveyard], field: [...field] },
    p2: { hand: [FILLER] },
  });
}

function unitsOnBoard(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null);
}

describe("C #44 Back from the GY", () => {
  it("declares its one number, budget (R386)", () => {
    expect(def.params).toEqual([{ key: "budget", base: 5, radiant: 5, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("E18 opens a budgeted pick of your graveyard's Units only, each carrying its cost, budget 5", () => {
      const s = back(false, [MR_TOKEN, FILLER, POINTMASTER, MENACE, SEVEN]);
      s.play(BACK);
      const pending = open(s);
      expect(pending.playerId).toBe("p1");
      expect(pending.kind).toBe("pick");
      expect(pending.budget).toBe(5);
      expect(pending.min).toBe(0);
      const offered = pending.options.map((option) =>
        option.selection.pick === "instance" ? [s.card(option.selection.instanceId).defId, option.cost] : ["?"],
      );
      expect(offered).toEqual([
        [MR_TOKEN, 1],
        [POINTMASTER, 2],
        [MENACE, 3],
        [SEVEN, 4],
      ]);
    });

    it("summons the Units you pick within the budget into your leftmost open zones", () => {
      const s = back(false, [POINTMASTER, MENACE, SEVEN], [VANILLA]);
      s.play(BACK);
      s.answer([s.card(POINTMASTER).id, s.card(MENACE).id]);
      expect(unitsOnBoard(s)).toEqual([VANILLA, POINTMASTER, MENACE, null, null]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([SEVEN]);
    });

    it("R1 a summon fires no Cry, and the Units arrive summoning sick", () => {
      const s = back(false, [MR_TOKEN]);
      s.play(BACK);
      s.answer([s.card(MR_TOKEN).id]);
      expect(unitsOnBoard(s)).toEqual([MR_TOKEN, null, null, null, null]);
      expect(s.events.some((event) => event.type === "summoned" && "defId" in event && event.defId === RUSH_TOKEN)).toBe(false);
      expect(s.card(MR_TOKEN).summonedTurn).toBe(s.state.turn);
      expect(() => s.attack(MR_TOKEN, "hero")).toThrow();
    });

    it("a pick whose costs total more than the budget is refused", () => {
      const s = back(false, [POINTMASTER, SEVEN]);
      s.play(BACK);
      expect(() => s.answer([s.card(POINTMASTER).id, s.card(SEVEN).id])).toThrow();
      expect(open(s).kind).toBe("pick");
    });

    it("R65 a graveyard card's own cost counts, its costMod included", () => {
      // The 7/7 (4) with costMod −3 costs (1) there, so it and a (4) fit in 5.
      const s = back(false, [{ def: SEVEN, costMod: -3 }, MENACE, VANILLA]);
      s.play(BACK);
      const cost = open(s).options.map((option) => option.cost);
      expect(cost).toEqual([1, 3, 1]);
      s.answer([s.card(SEVEN).id, s.card(MENACE).id, s.card(VANILLA).id]);
      expect(unitsOnBoard(s)).toEqual([SEVEN, MENACE, VANILLA, null, null]);
    });

    it("R396 an X Unit in the graveyard costs 0 toward the budget", () => {
      const s = back(false, [BILLY, SEVEN]);
      s.play(BACK);
      const billy = open(s).options.find(
        (option) => option.selection.pick === "instance" && s.card(option.selection.instanceId).defId === BILLY,
      );
      expect(billy?.cost).toBe(0);
    });

    it("R64 a full board leaves the rest in the graveyard", () => {
      const s = back(false, [VANILLA, TIMMY], [MENACE, SEVEN, POINTMASTER, MENACE]);
      s.play(BACK);
      const vanilla = s.card(VANILLA);
      const timmy = s.card(TIMMY);
      s.answer([vanilla.id, timmy.id]);
      expect(s.unit("p1", 5)?.id).toBe(vanilla.id);
      s.expectInZone(timmy, "graveyard");
    });

    it("no Unit in your graveyard: no prompt, nothing summoned", () => {
      const s = back(false, [FILLER]);
      s.play(BACK);
      expect(s.state.pending).toBeNull();
      expect(unitsOnBoard(s)).toEqual([null, null, null, null, null]);
    });

    it("R178 then this Spell is exiled, not sent to the graveyard", () => {
      const s = back(false, [VANILLA]);
      const spell = s.card(BACK);
      s.play(BACK);
      s.answer([s.card(VANILLA).id]);
      s.expectInZone(spell, "exile");
      expect(s.pile("p1", "graveyard").map((card) => card.id)).not.toContain(spell.id);
    });

    it("R178 exiled with nothing to summon too", () => {
      const s = back(false, []);
      s.play(BACK);
      s.expectInZone(BACK, "exile");
    });

    it("R177 the opponent reads only that a prompt is open for you", () => {
      const s = back(false, [VANILLA]);
      s.play(BACK);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("§9.3 the open pick survives a JSON round trip and resumes through reduce", () => {
      const s = back(false, [POINTMASTER, MENACE]);
      const spell = s.card(BACK);
      s.play(BACK);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const pending = must(revived.pending, "revived prompt");
      const menace = s.card(MENACE);
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: pending.id,
        selection: [{ pick: "instance", instanceId: menace.id }],
        nonce: "back-from-the-gy-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(result.events.some((event) => event.type === "exiled" && event.instanceId === spell.id)).toBe(true);
    });

    it("R386 an Upgrade of the budget lets 6 in", () => {
      const s = back(false, [POINTMASTER, SEVEN]);
      stepParam(s.card(BACK), "budget", 1);
      s.play(BACK);
      expect(open(s).budget).toBe(6);
      s.answer([s.card(POINTMASTER).id, s.card(SEVEN).id]);
      expect(unitsOnBoard(s)).toEqual([POINTMASTER, SEVEN, null, null, null]);
    });

    it("R386 a Degrade of the budget makes it 4", () => {
      const s = back(false, [POINTMASTER, MENACE]);
      stepParam(s.card(BACK), "budget", -1);
      s.play(BACK);
      expect(open(s).budget).toBe(4);
      expect(() => s.answer([s.card(POINTMASTER).id, s.card(MENACE).id])).toThrow();
    });
  });

  describe("radiant", () => {
    it("summons every Unit in your graveyard, oldest first, with no prompt", () => {
      const s = back(true, [SEVEN, FILLER, MENACE, VANILLA]);
      s.play(BACK);
      expect(s.state.pending).toBeNull();
      expect(unitsOnBoard(s)).toEqual([SEVEN, MENACE, VANILLA, null, null]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([FILLER]);
    });

    it("no budget: Units of any cost come back", () => {
      const s = back(true, [SEVEN, MENACE, POINTMASTER]);
      s.play(BACK);
      expect(unitsOnBoard(s)).toEqual([SEVEN, MENACE, POINTMASTER, null, null]);
    });

    it("R64 until the board is full: the newest stay in the graveyard", () => {
      const s = back(true, [SEVEN, MENACE, POINTMASTER, TIMMY], [VANILLA, VANILLA, VANILLA]);
      s.play(BACK);
      expect(unitsOnBoard(s)).toEqual([VANILLA, VANILLA, VANILLA, SEVEN, MENACE]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([POINTMASTER, TIMMY]);
    });

    it("R1 no Cry fires", () => {
      const s = back(true, [MR_TOKEN, MR_TOKEN]);
      s.play(BACK);
      expect(unitsOnBoard(s)).toEqual([MR_TOKEN, MR_TOKEN, null, null, null]);
    });

    it("R178 then this Spell is exiled", () => {
      const s = back(true, [VANILLA]);
      s.play(BACK);
      s.expectInZone(BACK, "exile");
    });

    it("an empty graveyard summons nothing, and the Spell is still exiled", () => {
      const s = back(true, []);
      s.play(BACK);
      expect(unitsOnBoard(s)).toEqual([null, null, null, null, null]);
      s.expectInZone(BACK, "exile");
    });
  });
});
