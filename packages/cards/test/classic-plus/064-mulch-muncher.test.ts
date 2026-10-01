// C+ #64 Mulch Muncher — SPEC §8.7 row 64, BUILD M9 row C+ 64: Rush, Trample; a `cost` hook (R55's
// pattern) that costs (1) less for each Fruit-tagged card its controller has played this game (casts
// included, R70; a Grape counts; the opponent's Fruits don't; never reset), floored at 0 and shown as
// its current cost in hand; out of play it costs (10) (R65, R584); the Trample excess reaches the hero
// (R63); the discount reads through `param()`; radiant 18/18 with Divine Shield too.

import { describe, expect, it } from "vitest";
import {
  applyEffects,
  costNow,
  createRng,
  effectiveCost,
  makeContext,
  query,
  settle,
  stepParam,
  type EngineSink,
} from "@jackioh/engine";
import { castNew } from "@jackioh/engine/effects";
import type { PlayerId } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/064-mulch-muncher";
import { scenario, type Scenario } from "../_harness";

const MULCH = "classicplus-064";
const FIG = "core-047"; // Fig of Life: (3) Spell, Fruit. "Heal a target 20."
const GRAPE = "classicplus-065-1"; // Rotten Grape: (1) Spell, Fruit, Token.
const FILLER = "core-005";

function handCost(s: Scenario, player: PlayerId = "p1"): number {
  const card = s.hand(player).find((held) => held.defId === MULCH);
  if (card === undefined) throw new Error("no Mulch Muncher in hand");
  const view = s.view(player).you.hand;
  if (!Array.isArray(view)) throw new Error("own hand is a list");
  const shown = view.find((held) => held.instanceId === card.id)?.cost;
  expect(shown, "the view shows the cost the engine charges").toBe(effectiveCost(s.state, card));
  return effectiveCost(s.state, card);
}

function playFig(s: Scenario): void {
  const fig = s.hand(s.state.active).find((card) => card.defId === FIG);
  if (fig === undefined) throw new Error("no Fig of Life in hand");
  s.play(fig, { targets: [{ pick: "hero", player: s.state.active }] });
}

function withFruit(opts: { radiant?: boolean; figs?: number } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: MULCH, radiant: opts.radiant === true }, ...Array.from({ length: opts.figs ?? 2 }, () => FIG), FILLER], mana: 30 },
    p2: { hand: [FILLER, FIG] },
  });
}

describe("C+ #64 Mulch Muncher", () => {
  it("prints a (10) 9/9 Rush, Trample (Radiant 18/18 with Divine Shield) and runs one script on both faces", () => {
    expect(def.cost).toBe(10);
    expect(def.base).toMatchObject({ attack: 9, health: 9, keywords: [{ kind: "Rush" }, { kind: "Trample" }] });
    expect(def.radiant).toMatchObject({
      attack: 18,
      health: 18,
      keywords: [{ kind: "Rush" }, { kind: "Trample" }, { kind: "Divine Shield" }],
    });
    expect(def.tags).not.toContain("Fruit");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R55 costs (1) less for each Fruit its controller has played this game, shown as its cost in hand", () => {
      const s = withFruit();
      expect(handCost(s)).toBe(10);
      playFig(s);
      expect(handCost(s)).toBe(9);
      playFig(s);
      expect(handCost(s)).toBe(8);
    });

    it("R382 a Grape is a Fruit and counts", () => {
      const s = scenario({ p1: { hand: [MULCH, GRAPE, FILLER], mana: 10 }, p2: { hand: [FILLER] } });
      s.play(GRAPE);
      expect(handCost(s)).toBe(9);
    });

    it("R70 a cast Fruit counts as a play", () => {
      const s = scenario({ p1: { hand: [MULCH, FILLER] }, p2: { hand: [FILLER] } });
      const state = s.state;
      const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
      applyEffects([castNew({ def: FIG, random: true })], makeContext(sink, null, { controller: "p1" }));
      settle(sink);
      state.rngCursor = sink.rng.cursor;
      expect(handCost(s)).toBe(9);
    });

    it("the opponent's Fruits don't count", () => {
      const s = scenario({ active: "p2", p1: { hand: [MULCH, FILLER] }, p2: { hand: [FIG, FILLER], mana: 10 } });
      playFig(s);
      expect(handCost(s, "p1")).toBe(10);
    });

    it("the per-game count never resets: the discount outlives the turn and the Fruit", () => {
      const s = withFruit({ figs: 1 });
      playFig(s);
      s.expectInZone(FIG, "graveyard");
      s.endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      expect(handCost(s)).toBe(9);
    });

    it("§2.3 the cost floors at 0", () => {
      const s = withFruit({ figs: 2 });
      stepParam(s.card(MULCH), "discount", 5);
      playFig(s);
      expect(handCost(s)).toBe(4);
      playFig(s);
      expect(handCost(s)).toBe(0);
    });

    it("R386 the discount reads through param(): an Upgrade makes each Fruit (2) off, a Degrade never takes it below (1)", () => {
      const up = withFruit({ figs: 1 });
      stepParam(up.card(MULCH), "discount", 1);
      playFig(up);
      expect(handCost(up)).toBe(8);
      const down = withFruit({ figs: 1 });
      stepParam(down.card(MULCH), "discount", -1);
      playFig(down);
      expect(handCost(down)).toBe(9);
    });

    it("R584 R65 out of play it costs (10): in a pool, in a deck, in a graveyard and on the field, whatever was played", () => {
      expect(query({ cost: 10 }).map((card) => card.id)).toContain(MULCH);
      const s = scenario({
        p1: { hand: [MULCH, FIG, FIG, FILLER], library: [MULCH], graveyard: [MULCH], mana: 30 },
        p2: { hand: [FILLER] },
      });
      playFig(s);
      playFig(s);
      expect(handCost(s)).toBe(8);
      const [inDeck] = s.pile("p1", "library");
      const [inGrave] = s.pile("p1", "graveyard");
      expect(effectiveCost(s.state, inDeck!)).toBe(10);
      expect(effectiveCost(s.state, inGrave!)).toBe(10);
      s.play(MULCH, { zone: 1 });
      expect(costNow(s.state, s.unit("p1", 1)!)).toBe(10);
      s.expectMana("p1", 30 - 3 - 3 - 8);
    });

    it("R63 Rush lets it attack a Unit at once, and its Trample excess reaches the hero", () => {
      // One mana left over for the Stockpile, so the turn does not end itself (§2.5) and fatigue p2.
      const s = scenario({ p1: { hand: [MULCH, FILLER], mana: 11 }, p2: { hand: [FILLER], field: ["core-008"] } });
      s.play(MULCH, { zone: 1 });
      s.attack(s.unit("p1", 1)!, s.unit("p2", 1)!);
      // 9 into Mr. Vanilla's 4/4: 5 tramples over.
      s.expectHealth("p2", 25);
      expect(s.unit("p2", 1)).toBeNull();
    });
  });

  describe("radiant", () => {
    it("is an 18/18 with Divine Shield, and its cost falls the same way", () => {
      const s = withFruit({ radiant: true, figs: 1 });
      expect(handCost(s)).toBe(10);
      playFig(s);
      expect(handCost(s)).toBe(9);
      s.play(MULCH, { zone: 2 });
      const unit = s.unit("p1", 2)!;
      expect(s.stats(unit)).toMatchObject({ attack: 18, health: 18 });
      expect(s.stats(unit).keywords.map((keyword) => keyword.kind)).toEqual(["Rush", "Trample", "Divine Shield"]);
    });
  });
});
