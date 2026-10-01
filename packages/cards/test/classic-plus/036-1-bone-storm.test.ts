// C+ #36.1 Bone Storm — SPEC §8.7 row 36.1, BUILD M9 Classic+ row C+ 36.1: "Cast on draw (R70: free,
// counts as played): 1 damage to the enemy hero and to each enemy Unit, separate instances all
// landing before the state check, then you draw again; one draw casts at most 20 in a row
// (`CAST_ON_DRAW_CHAIN_CAP`, R58), the next going to hand uncast (burned when the hand is full);
// played from a hand it does the same; Spell Damage raises each hit; the damage reads through
// `param()`; radiant Echo: it resolves a second time".

import { CAST_ON_DRAW_CHAIN_CAP, HAND_CAP, HERO_HEALTH, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type PileSetup, type Scenario } from "../_harness";

const BONE_STORM = "classicplus-036-1";
const SOLARIUS = "classicplus-038";
const MENACE = "core-019"; // 9/9 Taunt.
const TIMMY = "core-011"; // a 3/3 Unit that keeps p1's turns alive.
const FILLER = "core-005";

const storm = (radiant: boolean): PileSetup => ({ def: BONE_STORM, radiant });

/** p1 with the given library, an enemy Menace on the board; run to p1's next draw. */
function drawStorm(library: readonly PileSetup[], extra: { field?: string[]; hand?: string[] } = {}): Scenario {
  const s = scenario({
    p1: { hand: extra.hand ?? [FILLER], field: extra.field ?? [TIMMY], library },
    p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER, FILLER] },
  });
  s.endTurn(); // p2's turn.
  s.endTurn(); // p1's turn: the draw.
  return s;
}

function damageEvents(s: Scenario): Extract<GameEvent, { type: "damage" }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage");
}

describe("C+ #36.1 Bone Storm", () => {
  describe("base", () => {
    it("R70 drawn, it casts itself for free: 1 to the enemy hero and 1 to each enemy Unit, then you draw again", () => {
      const s = drawStorm([storm(false), FILLER, FILLER]);
      s.expectHealth("p2", HERO_HEALTH - 1).expectHealth("p1", HERO_HEALTH);
      s.expectStats(MENACE, { health: 8 });
      s.expectStats(TIMMY, { health: 3 }); // your own side untouched
      expect(s.card(BONE_STORM).zone.z).toBe("graveyard");
      // Then you draw again: the next card arrived in hand.
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER, FILLER]);
      expect(s.pile("p1", "library")).toHaveLength(1);
      // It counts as played and paid nothing (R70).
      expect(s.state.players.p1.turnLog.playedIds).toContain(s.card(BONE_STORM).id);
      s.expectMana("p1", 4);
    });

    it("R59 each enemy takes its own hit, all landing before the state check", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [TIMMY], library: [storm(false), FILLER] },
        p2: { hand: [FILLER], field: ["core-t-sheep", "core-t-sheep", MENACE], library: [FILLER, FILLER] },
      });
      s.endTurn();
      const from = s.events.length;
      s.endTurn();
      const after = s.events.slice(from);
      const hits = after.flatMap((event, index) => (event.type === "damage" ? [index] : []));
      const deaths = after.flatMap((event, index) => (event.type === "destroyed" ? [index] : []));
      expect(hits).toHaveLength(4); // two Sheep, Menace, the hero
      expect(deaths).toHaveLength(2);
      expect(Math.max(...hits)).toBeLessThan(Math.min(...deaths));
    });

    it("R58 one draw casts at most 20 in a row; the next goes to hand uncast", () => {
      const library = Array.from({ length: CAST_ON_DRAW_CHAIN_CAP + 3 }, () => storm(false));
      const s = drawStorm(library);
      s.expectHealth("p2", HERO_HEALTH - CAST_ON_DRAW_CHAIN_CAP);
      const inHand = s.hand("p1").filter((card) => card.defId === BONE_STORM);
      expect(inHand).toHaveLength(1);
      expect(s.pile("p1", "library")).toHaveLength(2);
    });

    it("R58 it is cast even with a full hand, and the card after the cap is burned when the hand is full", () => {
      const full = Array.from({ length: HAND_CAP }, () => FILLER);
      const library = Array.from({ length: CAST_ON_DRAW_CHAIN_CAP + 1 }, () => storm(false));
      const s = drawStorm(library, { hand: full });
      s.expectHealth("p2", HERO_HEALTH - CAST_ON_DRAW_CHAIN_CAP);
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.events.some((event) => event.type === "burned" && event.defId === BONE_STORM)).toBe(true);
    });

    it("played from a hand it does the same, for its cost (1)", () => {
      const s = scenario({ p1: { hand: [BONE_STORM, FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(BONE_STORM);
      s.expectHealth("p2", HERO_HEALTH - 1).expectStats(MENACE, { health: 8 }).expectMana("p1", 3);
      expect(s.card(BONE_STORM).zone.z).toBe("graveyard");
    });

    it("Spell Damage raises each hit: with Solarius (+2) every enemy takes 3", () => {
      const s = drawStorm([storm(false), FILLER], { field: [SOLARIUS] });
      s.expectHealth("p2", HERO_HEALTH - 3).expectStats(MENACE, { health: 6 });
    });

    it("the opponent's Bone Storm hits you, never its own side", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [TIMMY], library: [FILLER, FILLER] },
        p2: { hand: [FILLER], field: [MENACE], library: [storm(false), FILLER] },
      });
      s.endTurn(); // p2 draws the storm.
      s.expectHealth("p1", HERO_HEALTH - 1).expectHealth("p2", HERO_HEALTH);
      s.expectStats(TIMMY, { health: 2 }).expectStats(MENACE, { health: 9 });
    });

    it("R386 the damage reads through param(): an Upgrade makes each hit 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [TIMMY], library: [storm(false), FILLER] },
        p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER] },
      });
      stepParam(s.card(BONE_STORM), "damage", 1);
      s.endTurn();
      s.endTurn();
      s.expectHealth("p2", HERO_HEALTH - 2).expectStats(MENACE, { health: 7 });
    });
  });

  describe("radiant", () => {
    it("Echo: it resolves a second time, hitting every enemy twice", () => {
      const s = drawStorm([storm(true), FILLER, FILLER]);
      s.expectHealth("p2", HERO_HEALTH - 2).expectStats(MENACE, { health: 7 });
      expect(damageEvents(s).filter((event) => event.amount === 1)).toHaveLength(4);
    });

    it("played from hand, Echo repeats it too", () => {
      const s = scenario({ p1: { hand: [{ def: BONE_STORM, radiant: true }, FILLER] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(BONE_STORM);
      s.expectHealth("p2", HERO_HEALTH - 2).expectStats(MENACE, { health: 7 });
    });
  });
});
