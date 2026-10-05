// C+ #65.3 Large Grape — SPEC §8.7 row 65.3, BUILD M9 Classic+ row C+ 65.3: "As Normal Grape with 5 and
// 5, and the drawn card costs (0) (`costOverride` 0); amount and draw read through `param()`; radiant 10
// and 10, draw 2, and each drawn card costs (0)".

import { hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { askingCastOnDraw } from "../_askingCast";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-3-large-grape";

const GRAPE = "classicplus-065-3";
const FILLER = "core-005";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const HINDER = "core-021"; // Cast on draw; its Radiant face asks nothing
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const DECK_A = "core-043"; // (4) Unit, Big Felinor
const DECK_B = "core-025"; // (4) Unit
const BILLY = "classicplus-069"; // Buff Billy, an (X) Unit

function unitAt(s: Scenario, player: "p1" | "p2"): Selection[] {
  const unit = s.unit(player, 1);
  if (unit === null) throw new Error(`no unit in ${player} lane 1`);
  return [{ pick: "instance", instanceId: unit.id }];
}

const RADIANT = { def: GRAPE, radiant: true };

describe("C+ #65.3 Large Grape", () => {
  it("is a (3) Fruit Spell token (printed Rare) that targets any Unit or hero, one script on both faces", () => {
    expect(def.id).toBe(GRAPE);
    expect(def.cost).toBe(3);
    expect(def.tags).toEqual(["Fruit", "Token"]);
    expect(def.printedRarity).toBe("Rare");
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("an enemy Unit takes 5, then you draw 1 that costs (0)", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(GRAPE, { targets: unitAt(s, "p2") });
      s.expectStats(MENACE, { health: 4 });
      const drawn = s.card(DECK_B);
      expect(drawn.zone.z).toBe("hand");
      expect(drawn.costOverride).toBe(0);
      expect(s.view("p1").you.hand).toEqual(expect.arrayContaining([expect.objectContaining({ instanceId: drawn.id, cost: 0 })]));
    });

    it("the enemy hero takes 5; Spell Damage +2 makes it 7", () => {
      const plain = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER] } });
      plain.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      plain.expectHealth("p2", 25);

      const raised = scenario({ p1: { hand: [GRAPE, FILLER], field: [SOLARIUS], library: [DECK_B] }, p2: { hand: [FILLER] } });
      raised.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      raised.expectHealth("p2", 23);
    });

    it("R19 a friendly Unit is healed 5, and your hero 5 past 30", () => {
      const unit = scenario({ p1: { hand: [GRAPE, FILLER], field: [{ def: MENACE, damage: 6 }], library: [DECK_B] }, p2: { hand: [FILLER] } });
      unit.play(GRAPE, { targets: unitAt(unit, "p1") });
      unit.expectStats(MENACE, { health: 8 });

      const hero = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER] } });
      hero.play(GRAPE, { targets: [{ pick: "hero", player: "p1" }] });
      hero.expectHealth("p1", 35);
    });

    it("R596 a burned card, a fatigue draw and a card cast on draw — nor the card its draw then brings — get no price", () => {
      const burned = scenario({ p1: { hand: [GRAPE, ...Array.from({ length: 10 }, () => FILLER)], library: [DECK_B] }, p2: { hand: [FILLER] } });
      burned.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(burned.card(DECK_B).zone.z).toBe("graveyard");
      expect(burned.card(DECK_B).costOverride).toBeUndefined();

      const fatigue = scenario({ p1: { hand: [GRAPE, FILLER], library: [] }, p2: { hand: [FILLER] } });
      fatigue.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(fatigue.lastEvents.some((event) => event.type === "costChanged")).toBe(false);

      const cast = scenario({ p1: { hand: [GRAPE, FILLER], library: [{ def: HINDER, radiant: true }, DECK_B] }, p2: { hand: [FILLER] } });
      cast.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(cast.card(HINDER).zone.z).toBe("graveyard");
      expect(cast.card(DECK_B).costOverride).toBeUndefined();
    });

    it("R65 an X-cost card it draws is free to play, its X still chosen: Buff Billy played for 1 with 1 mana left costs nothing", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [BILLY] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.card(BILLY).costOverride).toBe(0);
      s.expectMana("p1", 1);
      s.play(BILLY, { zone: 1, x: 1 });
      s.expectMana("p1", 1);
      expect(s.card(BILLY).x).toBe(1);
    });

    it("R386 an Upgrade makes it 6 and 2 draws; a Degrade 4", () => {
      const up = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      stepParam(up.card(GRAPE), "amount", 1);
      stepParam(up.card(GRAPE), "draw", 1);
      up.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      up.expectHealth("p2", 24);
      expect([up.card(DECK_A).costOverride, up.card(DECK_B).costOverride]).toEqual([0, 0]);

      const down = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      stepParam(down.card(GRAPE), "amount", -1);
      down.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      down.expectHealth("p2", 26);
    });
  });

  describe("radiant", () => {
    it("an enemy takes 10; you draw 2 and each costs (0)", () => {
      const s = scenario({ p1: { hand: [RADIANT, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER], field: [{ def: MENACE }] } });
      s.play(GRAPE, { targets: unitAt(s, "p2") });
      expect(s.unit("p2", 1)).toBeNull();
      expect(s.card(DECK_A).costOverride).toBe(0);
      expect(s.card(DECK_B).costOverride).toBe(0);
    });

    it("R19 a friend is healed 10", () => {
      const s = scenario({ p1: { hand: [RADIANT, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 40);
    });

    it("R113 a cast-on-draw prompt pauses the second draw; after a JSON round trip the answer prices it (0)", () => {
      const s = scenario({ p1: { hand: [RADIANT, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      askingCastOnDraw(s);
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.state.pending?.kind).toBe("hand");
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      const filler = s.hand("p1").find((card) => card.defId === FILLER);
      const selection = [{ pick: "instance" as const, instanceId: filler?.id ?? "" }];
      const resumed = reduce(revived, { type: "answer", playerId: "p1", choiceId: revived.pending?.id ?? "", selection, nonce: "large-pause" });
      expect(resumed.error).toBeUndefined();
      s.answer(selection);
      expect(hashState(resumed.state)).toBe(hashState(s.state));
      expect(s.card(DECK_A).costOverride).toBeUndefined();
      expect(s.card(DECK_B).costOverride).toBe(0);
    });
  });
});
