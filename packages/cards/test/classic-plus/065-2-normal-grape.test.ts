// C+ #65.2 Normal Grape — SPEC §8.7 row 65.2, BUILD M9 Classic+ row C+ 65.2: "A target Unit or hero
// chosen with the play: an enemy takes 2 damage (Spell Damage raises it), one of yours is healed 2;
// then you draw 1 and that card gets `costMod` −1; a card cast on draw, a burned card or a fatigue draw
// gets no discount; amount and draw read through `param()`; radiant 4 and 4, draw 4, each drawn card
// costs (1) less".

import { hashState, stepParam, type GameState } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-2-normal-grape";

const GRAPE = "classicplus-065-2";
const FILLER = "core-005"; // (1) Spell
const MENACE = "core-019"; // (3) Unit 9/9 Taunt
const HINDER = "core-021"; // Cast on draw; base face makes its caster discard 1 at random (R682)
const SOLARIUS = "classicplus-038"; // Unit printing Spell Damage +2
const DECK_A = "core-011"; // Tempo Timmy, (1) Unit
const DECK_B = "core-001"; // Big D-fender, (2) Unit
const DECK_C = "core-002"; // Bigot, (2) Unit
const DECK_D = "core-003"; // Right-house defender, (1) Unit
const BILLY = "classicplus-069"; // Buff Billy, an (X) Unit

function unitAt(s: Scenario, player: "p1" | "p2", lane = 1): Selection[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

function grape(radiant = false): { def: string; radiant?: boolean } {
  return radiant ? { def: GRAPE, radiant: true } : { def: GRAPE };
}

describe("C+ #65.2 Normal Grape", () => {
  it("is a (1) Fruit Spell token (printed Common) that targets any Unit or hero, one script on both faces", () => {
    expect(def.id).toBe(GRAPE);
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["Fruit", "Token"]);
    expect(def.printedRarity).toBe("Common");
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("an enemy Unit takes 2 damage, then you draw 1 that costs (1) less", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(GRAPE, { targets: unitAt(s, "p2") });

      s.expectStats(MENACE, { health: 7 });
      const drawn = s.card(DECK_B);
      expect(drawn.zone.z).toBe("hand");
      expect(drawn.costMod).toBe(-1);
      expect(s.view("p1").you.hand).toEqual(expect.arrayContaining([expect.objectContaining({ instanceId: drawn.id, cost: 1 })]));
      s.expectEvents("damage", "drawn", "costChanged");
    });

    it("the enemy hero takes 2 damage", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", 28).expectHealth("p1", 30);
    });

    it("R19 one of your Units is healed 2, never hit", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], field: [{ def: MENACE, damage: 5 }], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: unitAt(s, "p1") });
      s.expectStats(MENACE, { health: 6 });
      expect(s.lastEvents.some((event) => event.type === "damage")).toBe(false);
    });

    it("R19 your own hero is healed 2, past 30", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 32).expectHealth("p2", 30);
    });

    it("E6 Spell Damage raises the hit on an enemy (Spell Damage +2: 4)", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], field: [SOLARIUS], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", 26);
    });

    it("E6 Spell Damage never raises the heal on a friend", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], field: [SOLARIUS], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 32);
    });

    it("§2.4 R4 a burned card gets no discount", () => {
      const s = scenario({
        p1: { hand: [GRAPE, ...Array.from({ length: 10 }, () => FILLER)], library: [DECK_B] },
        p2: { hand: [FILLER] },
      });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      const drawn = s.card(DECK_B);
      expect(drawn.zone.z).toBe("graveyard");
      expect(drawn.costMod).toBe(0);
    });

    it("R65 an X-cost card it draws gets no discount: Buff Billy played for 2 still costs 2", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [BILLY] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectMana("p1", 3);
      s.play(BILLY, { zone: 1, x: 2 });
      s.expectMana("p1", 1);
    });

    it("§2.4 a fatigue draw brings no card, so nothing is discounted", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.lastEvents.some((event) => event.type === "fatigue")).toBe(true);
      expect(s.lastEvents.some((event) => event.type === "costChanged")).toBe(false);
      expect(s.hand("p1").every((card) => card.costMod === 0)).toBe(true);
    });

    it("R596 a card cast on draw never reaches the hand: neither it nor the card its draw then brings is discounted", () => {
      const s = scenario({
        p1: { hand: [GRAPE, FILLER], library: [{ def: HINDER, radiant: true }, DECK_B] },
        p2: { hand: [FILLER] },
      });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectInZone(HINDER, "graveyard");
      expect(s.card(HINDER).costMod).toBe(0);
      expect(s.card(DECK_B).zone.z).toBe("hand");
      expect(s.card(DECK_B).costMod).toBe(0);
    });

    it("R97 the opponent sees the draw under the sentinel and no price on a card they can't read", () => {
      const s = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_B] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).not.toContain(s.card(DECK_B).id);
      expect(theirs.opponent.hand).toEqual({ count: 2 });
    });

    it("R386 an Upgrade makes it 3 and 2 draws; a Degrade never takes either below 1", () => {
      const up = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      stepParam(up.card(GRAPE), "amount", 1);
      stepParam(up.card(GRAPE), "draw", 1);
      up.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      up.expectHealth("p2", 27);
      expect([up.card(DECK_A).costMod, up.card(DECK_B).costMod]).toEqual([-1, -1]);

      const down = scenario({ p1: { hand: [GRAPE, FILLER], library: [DECK_A, DECK_B] }, p2: { hand: [FILLER] } });
      stepParam(down.card(GRAPE), "amount", -3);
      stepParam(down.card(GRAPE), "draw", -3);
      down.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      down.expectHealth("p2", 29);
      expect(down.card(DECK_A).zone.z).toBe("hand");
      expect(down.card(DECK_B).zone.z).toBe("library");
    });
  });

  describe("radiant", () => {
    it("an enemy takes 4; you draw 4 and each costs (1) less", () => {
      const s = scenario({ p1: { hand: [grape(true), FILLER], library: [DECK_A, DECK_B, DECK_C, DECK_D] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(GRAPE, { targets: unitAt(s, "p2") });
      s.expectStats(MENACE, { health: 5 });
      expect([s.card(DECK_A).costMod, s.card(DECK_B).costMod, s.card(DECK_C).costMod, s.card(DECK_D).costMod]).toEqual([-1, -1, -1, -1]);
    });

    it("R19 a friend is healed 4", () => {
      const s = scenario({ p1: { hand: [grape(true), FILLER], library: [DECK_A, DECK_B, DECK_C, DECK_D] }, p2: { hand: [FILLER] } });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 34);
      expect([s.card(DECK_A).zone.z, s.card(DECK_B).zone.z, s.card(DECK_C).zone.z, s.card(DECK_D).zone.z]).toEqual(["hand", "hand", "hand", "hand"]);
    });

    it("§2.4 each draw prices its own card: the first keeps its discount, the rest burn", () => {
      const s = scenario({
        p1: { hand: [grape(true), ...Array.from({ length: 9 }, () => FILLER)], library: [DECK_A, DECK_B, DECK_C, DECK_D] },
        p2: { hand: [FILLER] },
      });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      expect(s.card(DECK_A).zone.z).toBe("hand");
      expect(s.card(DECK_A).costMod).toBe(-1);
      for (const id of [DECK_B, DECK_C, DECK_D]) {
        expect(s.card(id).zone.z).toBe("graveyard");
        expect(s.card(id).costMod).toBe(0);
      }
    });

    it("R58 a cast-on-draw card casts free with no prompt (R682); its chain's repeat brings no discount, the grape's own draws do", () => {
      const s = scenario({
        p1: { hand: [grape(true), FILLER], library: [HINDER, DECK_A, DECK_B, DECK_C, DECK_D] },
        p2: { hand: [FILLER] },
      });
      s.play(GRAPE, { targets: [{ pick: "hero", player: "p2" }] });
      // Base Hinder's discard is random (R682): no prompt opens mid-list.
      expect(s.state.pending).toBeNull();

      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      expect(hashState(revived)).toBe(hashState(s.state));

      // Hinder was cast (no discount) and discarded the one card held; its chain's repeat brought
      // DECK_A (no discount, R596); the grape's own remaining draws brought DECK_B/C/D, discounted.
      expect(s.card(HINDER).zone.z).toBe("graveyard");
      expect(s.card(HINDER).costMod).toBe(0);
      expect(s.card(FILLER).zone.z).toBe("graveyard");
      expect(s.card(DECK_A).zone.z).toBe("hand");
      expect(s.card(DECK_A).costMod).toBe(0);
      for (const id of [DECK_B, DECK_C, DECK_D]) {
        expect(s.card(id).zone.z).toBe("hand");
        expect(s.card(id).costMod).toBe(-1);
      }
      s.expectHealth("p2", 26);
    });
  });
});
