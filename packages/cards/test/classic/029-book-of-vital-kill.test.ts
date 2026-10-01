// C #29 Book of Vital Kill — SPEC §8.6 row 29, BUILD M9 Classic row C 29: "A declared hero target,
// either side; its health becomes 13 from above or below, its Armor unchanged: not damage and not a
// heal, so Armor, hit caps, C #75, Lifesteal, C #52's lethal window and Fed Fauci's tokens never see
// it; a hero at 13 stays; `healthSet` is public; radiant: also add a Book of Flame (C #16, base face)
// to your hand, burned at a full hand (R317) and never named in the opponent's view once there (R97);
// no tuned numbers".

import { describe, expect, it } from "vitest";
import { heroOf, legalActions } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/029-book-of-vital-kill";

const VITAL = "classic-029";
const FLAME = "classic-016"; // C #16 Book of Flame, (1) Spell: "Deal {damage} damage."
const GAMBIT = "classic-052"; // C #52 Final Gambit, (2) Trap: a hit that would bring your hero to 0 or less.
const TWINSPELL = "core-079"; // (2) Field Spell: "Your next Spell gains Echo +1."
const FAUCI = "core-091"; // (2) Unit: "Whenever this takes damage, it gets a Plague Token."
const VANILLA = "core-008"; // (1) Unit 4/4, no text.
const FILLER = "core-005"; // (1) Spell, a spare card so a hand never runs out (§2.5).

function eventsOf<T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

function playAt(s: Scenario, player: "p1" | "p2"): Scenario {
  return s.play(VITAL, { targets: [{ pick: "hero", player }] });
}

describe("C #29 Book of Vital Kill", () => {
  it("declares one hero target on either side, and 13 is the card's own number (no params)", () => {
    expect(def.id).toBe(VITAL);
    expect(def.params).toBeUndefined();
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["hero"] } }]);
    expect(radiant.targets).toEqual(base.targets);
  });

  describe("base", () => {
    it("R81 legalActions offers the play at each hero and at no Unit", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER], field: [VANILLA] }, p2: { hand: [FILLER], field: [VANILLA] } });
      const book = s.card(VITAL);
      const plays = legalActions(s.state, "p1").filter(
        (action) => action.type === "play" && action.instanceId === book.id,
      );
      const targets = plays.map((action) => (action.type === "play" ? action.targets : undefined));
      expect(targets).toEqual([[{ pick: "hero", player: "p1" }], [{ pick: "hero", player: "p2" }]]);
      const unit = s.unit("p2", 1);
      if (unit === null) throw new Error("p2's Vanilla should be on the board");
      expect(() => s.play(VITAL, { targets: [{ pick: "instance", instanceId: unit.id }] })).toThrow();
    });

    it("sets the enemy hero's health down to 13 from above", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER], health: 30 } });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
      s.expectHealth("p1", 30);
      s.expectInZone(VITAL, "graveyard");
    });

    it("sets your own hero's health up to 13 from below — not a heal", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER], health: 4 }, p2: { hand: [FILLER] } });
      playAt(s, "p1");
      s.expectHealth("p1", 13);
      expect(eventsOf(s, "healed")).toEqual([]);
    });

    it("a hero above its starting health comes down to 13 too", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER], health: 45 } });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
    });

    it("a hero at 13 stays at 13", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER], health: 13 } });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
      expect(eventsOf(s, "healthSet")).toHaveLength(1);
    });

    it("E7 not damage: its Armor is left as it was and no damage event is made", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER], health: 30, armor: 5 } });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
      expect(heroOf(s.state, "p2").armor).toBe(5);
      expect(eventsOf(s, "damage")).toEqual([]);
      expect(eventsOf(s, "healed")).toEqual([]);
      expect(eventsOf(s, "redirected")).toEqual([]);
    });

    it("E7 nothing that answers a hit or a heal sees it: a set Final Gambit stays set, Fed Fauci gets no token", () => {
      const s = scenario({
        p1: { hand: [VITAL, FILLER] },
        p2: { hand: [FILLER], field: [FAUCI], backrow: [{ def: GAMBIT, faceUp: false }], health: 30 },
      });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
      const gambit = s.backrow("p2", 1);
      expect(gambit?.defId).toBe(GAMBIT);
      expect(gambit?.faceUp).toBe(false);
      expect(eventsOf(s, "trapFired")).toEqual([]);
      const fauci = s.unit("p2", 1);
      expect(fauci?.counters.plague ?? 0).toBe(0);
    });

    it("R97 healthSet is public: both players read it, naming the Book as its source", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER] } });
      const book = s.card(VITAL);
      playAt(s, "p2");
      const expected = { type: "healthSet", player: "p2", health: 13, sourceId: book.id };
      for (const viewer of ["p1", "p2"] as const) {
        expect(s.view(viewer).events.filter((event) => event.type === "healthSet")).toEqual([expected]);
      }
    });

    it("the base face adds nothing to the hand", () => {
      const s = scenario({ p1: { hand: [VITAL, FILLER] }, p2: { hand: [FILLER] } });
      playAt(s, "p2");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
    });
  });

  describe("radiant", () => {
    it("sets the chosen hero's health to 13 and adds a Book of Flame on its base face to your hand", () => {
      const s = scenario({ p1: { hand: [{ def: VITAL, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      playAt(s, "p2");
      s.expectHealth("p2", 13);
      const flame = s.hand("p1").find((card) => card.defId === FLAME);
      expect(flame).toBeDefined();
      expect(flame?.radiant).toBe(false);
      expect(flame?.owner).toBe("p1");
    });

    it("sets your own hero too, and still adds the Book of Flame", () => {
      const s = scenario({ p1: { hand: [{ def: VITAL, radiant: true }, FILLER], health: 2 }, p2: { hand: [FILLER] } });
      playAt(s, "p1");
      s.expectHealth("p1", 13);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER, FLAME]);
    });

    it("R97 the opponent's view never names the Book of Flame once it is in your hand", () => {
      const s = scenario({ p1: { hand: [{ def: VITAL, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      playAt(s, "p2");
      const flame = s.hand("p1").find((card) => card.defId === FLAME);
      if (flame === undefined) throw new Error("the Book of Flame should be in p1's hand");
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(`"${flame.id}"`);
      expect(theirs).not.toContain(FLAME);
      // Its owner reads it.
      expect(JSON.stringify(s.view("p1"))).toContain(`"${flame.id}"`);
    });

    it("R317 a full hand burns the Book of Flame into your graveyard, and both players read which", () => {
      // A Radiant Twinspell's Echo +2 resolves the Book three times: 8 cards left in hand, then three
      // Flames — the first two fill the hand to 10 and the third burns.
      const fillers = Array.from({ length: 8 }, () => FILLER);
      const s = scenario({
        p1: { hand: [{ def: TWINSPELL, radiant: true }, { def: VITAL, radiant: true }, ...fillers] },
        p2: { hand: [FILLER] },
      });
      s.play(TWINSPELL);
      playAt(s, "p2");
      // §6.2 Echo: each repeat asks its target afresh.
      for (let repeat = 0; repeat < 2; repeat += 1) {
        expect(s.state.pending?.kind).toBe("target");
        s.answer([{ pick: "hero", player: "p2" }]);
      }
      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.hand("p1").filter((card) => card.defId === FLAME)).toHaveLength(2);
      const burned = eventsOf(s, "burned");
      expect(burned).toHaveLength(1);
      const burnedFlame = s.pile("p1", "graveyard").find((card) => card.defId === FLAME);
      expect(burnedFlame).toBeDefined();
      for (const viewer of ["p1", "p2"] as const) {
        const seen = s.view(viewer).events.filter((event) => event.type === "burned");
        expect(seen).toHaveLength(1);
        expect(JSON.stringify(seen)).toContain(FLAME);
      }
    });
  });
});
